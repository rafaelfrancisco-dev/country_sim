use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use tokio::sync::oneshot;

use super::Component;
use crate::{
    action::Action,
    sim::{GameCommand, GameEngine, GamePhase, country::Country},
};

mod input;
mod monthly;
mod render;
mod theme;

#[derive(Debug, Default)]
pub struct Simulation {
    engine: GameEngine,
    monthly_tick_inflight: Option<oneshot::Receiver<Vec<Country>>>,
    pending_monthly_ticks: u64,
    observed_month_boundaries: u64,
}

impl Simulation {
    pub fn new() -> Self {
        Self::default()
    }

    fn money(amount: f64) -> String {
        format!("{amount:.2}")
    }
}

impl Component for Simulation {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        let command = match self.engine.phase() {
            GamePhase::Setup(_) => self.map_setup_key(key),
            GamePhase::Running(_) => self.map_running_key(key),
        };

        if let Some(command) = command {
            let should_block_daily = matches!(command, GameCommand::StepOneDay)
                && self.should_pause_daily_ticks_for_monthly_catchup();
            if !should_block_daily {
                self.engine.apply(command);
            }
            self.reconcile_monthly_processing();
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if matches!(action, Action::Tick) {
            if !self.should_pause_daily_ticks_for_monthly_catchup() {
                self.engine.apply(GameCommand::Tick);
            }
            self.reconcile_monthly_processing();
        } else if matches!(action, Action::Render) {
            self.reconcile_monthly_processing();
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        match self.engine.phase() {
            GamePhase::Setup(_) => self.draw_setup(frame, area),
            GamePhase::Running(_) => self.draw_running(frame, area),
        }

        Ok(())
    }
}
