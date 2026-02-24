#[cfg(test)]
use std::sync::mpsc;
use std::{
    fmt,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tracing::warn;

use super::{snapshot::UiSnapshot, ui_state::UiState};
use crate::sim::{GameCommand, GameEngine, UiCommand};

#[derive(Debug)]
enum WorkerMsg {
    Apply(GameCommand),
    ApplyUi(UiCommand),
    #[cfg(test)]
    ApplyAndAck(GameCommand, mpsc::Sender<()>),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkerError {
    message: &'static str,
}

impl WorkerError {
    const CHANNEL_CLOSED: Self = Self {
        message: "worker channel is closed",
    };
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug)]
pub(super) struct SimulationWorker {
    tx: UnboundedSender<WorkerMsg>,
    handle: Option<thread::JoinHandle<()>>,
}

impl SimulationWorker {
    pub(super) fn new(
        initial_engine: GameEngine,
        initial_ui_state: UiState,
        snapshot: Arc<RwLock<UiSnapshot>>,
        tick_in_flight: Arc<AtomicBool>,
    ) -> Self {
        let (tx, mut rx) = unbounded_channel::<WorkerMsg>();
        let handle = thread::spawn(move || {
            let mut engine = initial_engine;
            let mut ui_state = initial_ui_state;
            while let Some(message) = rx.blocking_recv() {
                match message {
                    WorkerMsg::Apply(command) => {
                        let is_tick = matches!(command, GameCommand::Tick);
                        if matches!(command, GameCommand::SetupSubmit) {
                            if let Some(loading_snapshot) =
                                UiSnapshot::loading_setup_from_engine(&engine)
                            {
                                if let Ok(mut ui_snapshot) = snapshot.write() {
                                    *ui_snapshot = loading_snapshot;
                                } else {
                                    break;
                                }
                            }
                        }
                        let outcome = engine.apply(command);
                        for warning in outcome.warnings {
                            warn!("simulation invariant warning: {warning}");
                        }
                        if let Ok(mut ui_snapshot) = snapshot.write() {
                            *ui_snapshot = UiSnapshot::from_engine_and_ui(&engine, &ui_state);
                        } else {
                            break;
                        }
                        if is_tick {
                            tick_in_flight.store(false, Ordering::Release);
                        }
                    }
                    WorkerMsg::ApplyUi(command) => {
                        if let crate::sim::GamePhase::Running(world) = engine.phase() {
                            ui_state.apply(command, world);
                        }
                        if let Ok(mut ui_snapshot) = snapshot.write() {
                            *ui_snapshot = UiSnapshot::from_engine_and_ui(&engine, &ui_state);
                        } else {
                            break;
                        }
                    }
                    #[cfg(test)]
                    WorkerMsg::ApplyAndAck(command, ack_tx) => {
                        let is_tick = matches!(command, GameCommand::Tick);
                        if matches!(command, GameCommand::SetupSubmit) {
                            if let Some(loading_snapshot) =
                                UiSnapshot::loading_setup_from_engine(&engine)
                            {
                                if let Ok(mut ui_snapshot) = snapshot.write() {
                                    *ui_snapshot = loading_snapshot;
                                } else {
                                    break;
                                }
                            }
                        }
                        let outcome = engine.apply(command);
                        for warning in outcome.warnings {
                            warn!("simulation invariant warning: {warning}");
                        }
                        if let Ok(mut ui_snapshot) = snapshot.write() {
                            *ui_snapshot = UiSnapshot::from_engine_and_ui(&engine, &ui_state);
                        } else {
                            break;
                        }
                        if is_tick {
                            tick_in_flight.store(false, Ordering::Release);
                        }
                        let _ = ack_tx.send(());
                    }
                    WorkerMsg::Shutdown => break,
                }
            }
        });

        Self {
            tx,
            handle: Some(handle),
        }
    }

    pub(super) fn send(&self, command: GameCommand) -> Result<(), WorkerError> {
        self.tx
            .send(WorkerMsg::Apply(command))
            .map_err(|_| WorkerError::CHANNEL_CLOSED)
    }

    pub(super) fn send_ui_command(&self, command: UiCommand) -> Result<(), WorkerError> {
        self.tx
            .send(WorkerMsg::ApplyUi(command))
            .map_err(|_| WorkerError::CHANNEL_CLOSED)
    }

    #[cfg(test)]
    pub(super) fn send_and_wait(&self, command: GameCommand) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self
            .tx
            .send(WorkerMsg::ApplyAndAck(command, ack_tx))
            .is_ok()
        {
            let _ = ack_rx.recv();
        }
    }
}

impl Drop for SimulationWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(WorkerMsg::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{components::simulation::snapshot::SnapshotPhase, sim::UiCommand};

    fn apply_setup(worker: &SimulationWorker) {
        worker.send_and_wait(GameCommand::SetupEditChar('1'));
        worker.send_and_wait(GameCommand::SetupNextField);
        worker.send_and_wait(GameCommand::SetupEditChar('1'));
        worker.send_and_wait(GameCommand::SetupSubmit);
    }

    #[test]
    fn worker_applies_commands_in_order() {
        let initial_engine = GameEngine::default();
        let snapshot = Arc::new(RwLock::new(UiSnapshot::from_engine_and_ui(
            &initial_engine,
            &UiState::default(),
        )));
        let worker = SimulationWorker::new(
            initial_engine,
            UiState::default(),
            snapshot.clone(),
            Arc::new(AtomicBool::new(false)),
        );

        apply_setup(&worker);
        worker.send_and_wait(GameCommand::Tick);
        worker.send_and_wait(GameCommand::Tick);

        let snapshot = snapshot.read().expect("snapshot lock poisoned").clone();
        let SnapshotPhase::Running(world) = snapshot.phase else {
            panic!("expected running phase");
        };
        assert_eq!(world.day, 2);
    }

    #[test]
    fn worker_updates_ui_without_domain_mutation() {
        let initial_engine = GameEngine::default();
        let snapshot = Arc::new(RwLock::new(UiSnapshot::from_engine_and_ui(
            &initial_engine,
            &UiState::default(),
        )));
        let worker = SimulationWorker::new(
            initial_engine,
            UiState::default(),
            snapshot.clone(),
            Arc::new(AtomicBool::new(false)),
        );

        apply_setup(&worker);
        worker.send_and_wait(GameCommand::Tick);
        let before = snapshot.read().expect("snapshot lock poisoned").clone();
        let day_before = match before.phase {
            SnapshotPhase::Running(ref world) => world.day,
            _ => 0,
        };

        worker
            .send_ui_command(UiCommand::NextTab)
            .expect("ui command should send");

        let snapshot = snapshot.read().expect("snapshot lock poisoned").clone();
        let SnapshotPhase::Running(world) = snapshot.phase else {
            panic!("expected running phase");
        };
        assert_eq!(world.day, day_before);
    }
}
