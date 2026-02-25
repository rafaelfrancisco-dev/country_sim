use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use super::Component;
use crate::{
    action::Action,
    sim::{GameCommand, GameEngine, UiCommand},
};

mod input;
mod monthly;
mod render;
mod snapshot;
mod theme;
mod ui_state;
mod worker;

use snapshot::{SnapshotPhase, UiSnapshot};
use ui_state::{DetailsView, RunningTab, SelectionFocus};
use worker::SimulationWorker;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SimulationCommand {
    Domain(GameCommand),
    Ui(UiCommand),
}

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
    action_tx: Option<UnboundedSender<Action>>,
}

impl Default for Simulation {
    fn default() -> Self {
        let engine_state = GameEngine::default();
        let ui_state = ui_state::UiState::default();
        let initial_snapshot = UiSnapshot::from_engine_and_ui(&engine_state, &ui_state);
        let snapshot = Arc::new(RwLock::new(initial_snapshot.clone()));
        let tick_in_flight = Arc::new(AtomicBool::new(false));
        let worker = SimulationWorker::new(
            engine_state,
            ui_state,
            snapshot.clone(),
            tick_in_flight.clone(),
        );
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
            action_tx: None,
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

    fn queue_domain_command(&mut self, command: GameCommand) {
        if let Err(err) = self.worker.send(command) {
            warn!("failed to enqueue domain command: {err}");
            self.report_worker_error(err.to_string());
        }
    }

    fn apply_ui_command(&mut self, command: UiCommand) {
        if let Err(err) = self.worker.send_ui_command(command) {
            warn!("failed to enqueue ui command: {err}");
            self.report_worker_error(err.to_string());
        }
    }

    fn report_worker_error(&self, message: String) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(Action::Error(format!("Simulation worker error: {message}")));
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
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        self.refresh_cached_input_state();

        let command = if self.phase_is_running {
            self.map_running_key(key, self.running_input_state)
        } else {
            self.map_setup_key(key)
        };

        if let Some(command) = command {
            match command {
                SimulationCommand::Domain(command) => self.queue_domain_command(command),
                SimulationCommand::Ui(command) => self.apply_ui_command(command),
            }
            self.reconcile_monthly_processing();
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if matches!(action, Action::Tick) {
            if self
                .tick_in_flight
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
                && self.worker.send(GameCommand::Tick).is_err()
            {
                self.tick_in_flight.store(false, Ordering::Release);
                self.report_worker_error("failed to enqueue tick command".to_string());
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
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupNextField);
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupSubmit);
    }

    #[test]
    fn repeated_tab_presses_apply_latest_ui_state() {
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
        assert_eq!(world.active_tab, RunningTab::Goods);
    }
}
