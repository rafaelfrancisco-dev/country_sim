use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use super::Component;
use crate::{
    action::Action,
    sim::{GameCommand, GameEngine, SimulationMode, UiCommand, engine::DEFAULT_GENERATION_SEED},
};

mod hotkeys;
mod input;
#[cfg(feature = "manual-harness")]
pub mod manual_harness;
mod monthly;
mod render;
mod snapshot;
mod theme;
mod ui_state;
mod worker;

use snapshot::{SnapshotPhase, UiSnapshot};
use ui_state::{DetailsView, RunningTab, SelectionFocus};
use worker::{RenderSnapshot, SimulationWorker};

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
    pub events_search_active: bool,
}

#[derive(Debug)]
pub struct Simulation {
    snapshot: Arc<RwLock<RenderSnapshot>>,
    last_snapshot: UiSnapshot,
    worker: SimulationWorker,
    phase_is_running: bool,
    setup_generation_in_progress: bool,
    running_input_state: RunningInputState,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Default for Simulation {
    fn default() -> Self {
        let sim_mode = Arc::new(AtomicU8::new(SimulationMode::Realtime.as_atomic_u8()));
        Self::with_runtime(0.0, 2.0, sim_mode, DEFAULT_GENERATION_SEED, None)
    }
}

impl Simulation {
    pub fn with_runtime(
        realtime_tick_rate: f64,
        max_speed_ui_hz: f64,
        sim_mode_state: Arc<AtomicU8>,
        initial_generation_seed: u64,
        export_csv: Option<PathBuf>,
    ) -> Self {
        let engine_state = GameEngine::new(initial_generation_seed);
        let mut ui_state = ui_state::UiState::default();
        let runtime_stats = snapshot::RuntimeStatsSnapshot::new(
            SimulationMode::from_atomic_u8(sim_mode_state.load(Ordering::Acquire)),
            0.0,
            0.0,
            0,
            0.0,
            0,
            0.0,
            0.0,
        );
        let initial_snapshot =
            UiSnapshot::from_engine_and_ui(&engine_state, &mut ui_state, runtime_stats);
        let snapshot = Arc::new(RwLock::new(RenderSnapshot {
            ui_snapshot: initial_snapshot.clone(),
        }));
        let worker = SimulationWorker::new(
            engine_state,
            snapshot.clone(),
            realtime_tick_rate,
            Duration::from_secs_f64(1.0 / max_speed_ui_hz.max(0.1)).min(Duration::from_millis(500)),
            sim_mode_state,
            export_csv,
        );
        Self {
            snapshot,
            last_snapshot: initial_snapshot,
            worker,
            phase_is_running: false,
            setup_generation_in_progress: false,
            running_input_state: RunningInputState {
                details_view: DetailsView::List,
                selection_focus: SelectionFocus::Country,
                active_tab: RunningTab::Main,
                events_search_active: false,
            },
            action_tx: None,
        }
    }

    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    fn refresh_cached_input_state(&mut self) {
        if let Ok(snapshot) = self.snapshot.try_read() {
            self.last_snapshot = snapshot.ui_snapshot.clone();
        }

        match &mut self.last_snapshot.phase {
            SnapshotPhase::Setup(setup) => {
                self.phase_is_running = false;
                if setup.validation_error.is_some() {
                    self.setup_generation_in_progress = false;
                }
                setup.is_generating = self.setup_generation_in_progress;
            }
            SnapshotPhase::Running(world) => {
                self.phase_is_running = true;
                self.setup_generation_in_progress = false;
                self.running_input_state = RunningInputState {
                    details_view: world.details_view,
                    selection_focus: world.selection_focus,
                    active_tab: world.active_tab,
                    events_search_active: world.events_search_active,
                };
            }
        }
    }

    fn queue_domain_command(&mut self, command: GameCommand) -> bool {
        if let Err(err) = self.worker.send_domain(command) {
            warn!("failed to enqueue domain command: {err}");
            self.report_worker_error(err.to_string());
            return false;
        }
        true
    }

    fn queue_ui_command(&mut self, command: UiCommand) {
        if let Err(err) = self.worker.send_ui(command) {
            warn!("failed to enqueue ui command: {err}");
            self.report_worker_error(err.to_string());
        }
    }

    fn report_worker_error(&self, message: String) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(Action::Error(format!("Simulation worker error: {message}")));
        }
    }

    #[cfg(any(test, feature = "manual-harness"))]
    pub(in crate::components::simulation) fn apply_command_and_wait(&self, command: GameCommand) {
        self.worker.send_and_wait(command);
    }

    #[cfg(any(test, feature = "manual-harness"))]
    pub(in crate::components::simulation) fn snapshot_phase(&self) -> SnapshotPhase {
        let snapshot = self.snapshot.read().expect("snapshot lock poisoned");
        snapshot.ui_snapshot.phase.clone()
    }

    fn compact_num(n: f64) -> String {
        let abs = n.abs();
        let (divisor, suffix) = if abs >= 1_000_000.0 {
            (1_000_000.0, "M")
        } else if abs >= 10_000.0 {
            (1_000.0, "k")
        } else {
            return format!("{n:.2}");
        };
        let scaled = n / divisor;
        let r = (scaled * 10.0).round() / 10.0;
        if r.fract() == 0.0 {
            format!("{r:.0}{suffix}")
        } else {
            format!("{r:.1}{suffix}")
        }
    }

    fn money(amount: f64) -> String {
        Self::compact_num(amount)
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
                SimulationCommand::Domain(command) => {
                    let setup_submit_requested =
                        !self.phase_is_running && matches!(command, GameCommand::SetupSubmit);
                    if setup_submit_requested {
                        self.setup_generation_in_progress = true;
                    }
                    if !self.queue_domain_command(command) && setup_submit_requested {
                        self.setup_generation_in_progress = false;
                    }
                }
                SimulationCommand::Ui(command) => self.queue_ui_command(command),
            }
            self.reconcile_monthly_processing();
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if matches!(action, Action::Tick) {
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
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use super::*;
    use crate::{action::Action, components::Component};

    fn setup_running_world(simulation: &Simulation) {
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupNextField);
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('0'));
        simulation.apply_command_and_wait(GameCommand::SetupSubmit);

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if matches!(simulation.snapshot_phase(), SnapshotPhase::Running(_)) {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("timed out waiting for setup to transition into running phase");
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

    #[test]
    fn setup_submit_shows_generating_indicator_until_validation_finishes() {
        let mut simulation = Simulation::new();

        let _ = simulation.handle_key_event(KeyEvent::from(KeyCode::Enter));
        simulation.refresh_cached_input_state();

        let SnapshotPhase::Setup(setup) = &simulation.last_snapshot.phase else {
            panic!("setup phase should still be active immediately after submit");
        };
        assert!(
            setup.is_generating,
            "setup should display generating indicator while command is in flight"
        );

        let start = Instant::now();
        let timeout = Duration::from_secs(1);
        loop {
            simulation.refresh_cached_input_state();
            let SnapshotPhase::Setup(setup) = &simulation.last_snapshot.phase else {
                panic!("invalid setup should not transition to running");
            };
            if setup.validation_error.is_some() {
                assert!(
                    !setup.is_generating,
                    "setup should stop displaying generating indicator after validation fails"
                );
                break;
            }
            if start.elapsed() >= timeout {
                panic!("timed out waiting for setup validation error");
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
