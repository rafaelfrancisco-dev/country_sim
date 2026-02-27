use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{
    action::Action,
    components::{Component, fps::FpsCounter, simulation::Simulation},
    config::Config,
    sim::SimulationMode,
    tui::{Event, Tui},
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    sim_mode_state: Arc<AtomicU8>,
    max_mode_frame_rate: f64,
    last_render_at: Option<Instant>,
    input_boost_until: Option<Instant>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
}

impl App {
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        sim_mode: SimulationMode,
        max_mode_frame_rate: f64,
        initial_generation_seed: u64,
    ) -> color_eyre::Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let sim_mode_state = Arc::new(AtomicU8::new(sim_mode.as_atomic_u8()));
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(Simulation::with_runtime(
                    tick_rate,
                    max_mode_frame_rate,
                    sim_mode_state.clone(),
                    initial_generation_seed,
                )),
                Box::new(FpsCounter::default()),
            ],
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Home,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            sim_mode_state,
            max_mode_frame_rate: max_mode_frame_rate.max(0.1),
            last_render_at: None,
            input_boost_until: None,
        })
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => {
                self.input_boost_until = Some(Instant::now() + Duration::from_millis(250));
                self.handle_key_event(key)?;
            }
            _ => {}
        }
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.0.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        let mut saw_tick = false;
        let mut saw_render = false;

        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                Action::Tick => saw_tick = true,
                Action::Render => saw_render = true,
                other => self.apply_action(other, tui)?,
            }
        }

        if saw_tick {
            self.apply_action(Action::Tick, tui)?;
        }
        if saw_render && self.should_render_now() {
            self.apply_action(Action::Render, tui)?;
        }
        Ok(())
    }

    fn should_render_now(&mut self) -> bool {
        let mode = SimulationMode::from_atomic_u8(self.sim_mode_state.load(Ordering::Acquire));
        let now = Instant::now();
        let interactive_boost = self
            .input_boost_until
            .is_some_and(|deadline| now < deadline);
        let target_rate = if interactive_boost {
            self.frame_rate
        } else {
            match mode {
                SimulationMode::Realtime => self.frame_rate,
                SimulationMode::MaxSpeed => self.max_mode_frame_rate,
            }
        }
        .max(0.1);

        let min_interval = Duration::from_secs_f64(1.0 / target_rate);
        let should_render = self
            .last_render_at
            .is_none_or(|last| now.saturating_duration_since(last) >= min_interval);
        if should_render {
            self.last_render_at = Some(now);
        }
        should_render
    }

    fn apply_action(&mut self, action: Action, tui: &mut Tui) -> color_eyre::Result<()> {
        if action != Action::Tick && action != Action::Render {
            debug!("{action:?}");
        }
        match action {
            Action::Tick => {
                self.last_tick_key_events.drain(..);
            }
            Action::Quit => self.should_quit = true,
            Action::Suspend => self.should_suspend = true,
            Action::Resume => self.should_suspend = false,
            Action::ClearScreen => tui.terminal.clear()?,
            Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
            Action::Render => self.render(tui)?,
            Action::Error(ref message) => error!("Application action error: {message}"),
            _ => {}
        }

        for component in self.components.iter_mut() {
            if let Some(next_action) = component.update(action.clone())? {
                self.action_tx.send(next_action)?
            };
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> color_eyre::Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {:?}", err)));
                }
            }
        })?;
        Ok(())
    }
}
