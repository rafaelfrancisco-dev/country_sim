use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};

use super::Component;
use crate::{
    action::Action,
    sim::{
        GameCommand, GameEngine,
        world::{DetailsView, RunningTab, SelectionFocus},
    },
};

mod input;
mod monthly;
mod render;
mod snapshot;
mod theme;
mod worker;

use snapshot::{SnapshotPhase, UiSnapshot};
use worker::SimulationWorker;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) struct RunningInputState {
    pub details_view: DetailsView,
    pub selection_focus: SelectionFocus,
    pub active_tab: RunningTab,
}

#[derive(Debug)]
pub struct Simulation {
    snapshot: Arc<RwLock<UiSnapshot>>,
    last_snapshot: UiSnapshot,
    worker: SimulationWorker,
    tick_in_flight: Arc<AtomicBool>,
    phase_is_running: bool,
    running_input_state: RunningInputState,
    pending_tab_intent: Option<GameCommand>,
}

impl Default for Simulation {
    fn default() -> Self {
        let engine_state = GameEngine::default();
        let initial_snapshot = UiSnapshot::from_engine(&engine_state);
        let snapshot = Arc::new(RwLock::new(initial_snapshot.clone()));
        let tick_in_flight = Arc::new(AtomicBool::new(false));
        let worker = SimulationWorker::new(engine_state, snapshot.clone(), tick_in_flight.clone());
        Self {
            snapshot,
            last_snapshot: initial_snapshot,
            worker,
            tick_in_flight,
            phase_is_running: false,
            running_input_state: RunningInputState {
                details_view: DetailsView::List,
                selection_focus: SelectionFocus::Country,
                active_tab: RunningTab::Main,
            },
            pending_tab_intent: None,
        }
    }
}

impl Simulation {
    pub fn new() -> Self {
        Self::default()
    }

    fn refresh_cached_input_state(&mut self) {
        if let Ok(snapshot) = self.snapshot.try_read() {
            self.last_snapshot = snapshot.clone();
        }

        match &self.last_snapshot.phase {
            SnapshotPhase::Setup(_) => self.phase_is_running = false,
            SnapshotPhase::Running(world) => {
                self.phase_is_running = true;
                self.running_input_state = RunningInputState {
                    details_view: world.details_view,
                    selection_focus: world.selection_focus,
                    active_tab: world.active_tab,
                };
            }
        }
    }

    fn queue_command(&mut self, command: GameCommand) {
        match command {
            GameCommand::NextTab | GameCommand::PrevTab => {
                self.pending_tab_intent = Some(command);
            }
            _ => {
                self.flush_pending_tab_intent();
                let _ = self.worker.send(command);
            }
        }
    }

    fn flush_pending_tab_intent(&mut self) {
        if let Some(command) = self.pending_tab_intent.take() {
            let _ = self.worker.send(command);
        }
    }

    #[cfg(test)]
    pub(super) fn apply_command_and_wait(&self, command: GameCommand) {
        self.worker.send_and_wait(command);
    }

    #[cfg(test)]
    fn snapshot_phase(&self) -> SnapshotPhase {
        self.snapshot
            .read()
            .expect("snapshot lock poisoned")
            .phase
            .clone()
    }

    fn money(amount: f64) -> String {
        format!("{amount:.2}")
    }
}

impl Component for Simulation {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        self.refresh_cached_input_state();

        let command = if self.phase_is_running {
            self.map_running_key(key, self.running_input_state)
        } else {
            self.map_setup_key(key)
        };

        if let Some(command) = command {
            self.queue_command(command);
            self.reconcile_monthly_processing();
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        self.flush_pending_tab_intent();

        if matches!(action, Action::Tick) {
            if self
                .tick_in_flight
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                if !self.worker.send(GameCommand::Tick) {
                    self.tick_in_flight.store(false, Ordering::Release);
                }
            }
            self.reconcile_monthly_processing();
        } else if matches!(action, Action::Render) {
            self.reconcile_monthly_processing();
            self.refresh_cached_input_state();
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        self.refresh_cached_input_state();

        match &self.last_snapshot.phase {
            SnapshotPhase::Setup(setup) => self.draw_setup(frame, area, setup),
            SnapshotPhase::Running(world) => self.draw_running(frame, area, world),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent};

    use super::*;
    use crate::{action::Action, components::Component};

    fn setup_running_world(simulation: &Simulation) {
        simulation.apply_command_and_wait(GameCommand::SetupEditDigit(1));
        simulation.apply_command_and_wait(GameCommand::SetupNextField);
        simulation.apply_command_and_wait(GameCommand::SetupEditDigit(1));
        simulation.apply_command_and_wait(GameCommand::SetupSubmit);
    }

    #[test]
    fn repeated_tab_presses_collapse_to_latest_intent() {
        let mut simulation = Simulation::new();
        setup_running_world(&simulation);

        let _ = simulation.handle_key_event(KeyEvent::from(KeyCode::Tab));
        let _ = simulation.handle_key_event(KeyEvent::from(KeyCode::BackTab));
        let _ = simulation.handle_key_event(KeyEvent::from(KeyCode::Tab));

        let _ = simulation.update(Action::Render);
        simulation.apply_command_and_wait(GameCommand::StepOneDay);

        let SnapshotPhase::Running(world) = simulation.snapshot_phase() else {
            panic!("running world should exist");
        };
        assert_eq!(world.active_tab, RunningTab::Charts);
    }
}
