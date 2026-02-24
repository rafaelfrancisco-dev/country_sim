#[cfg(test)]
use std::sync::mpsc;
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::snapshot::UiSnapshot;
use crate::sim::{GameCommand, GameEngine};

#[derive(Debug)]
enum WorkerMsg {
    Apply(GameCommand),
    #[cfg(test)]
    ApplyAndAck(GameCommand, mpsc::Sender<()>),
    Shutdown,
}

#[derive(Debug)]
pub(super) struct SimulationWorker {
    tx: UnboundedSender<WorkerMsg>,
    handle: Option<thread::JoinHandle<()>>,
}

impl SimulationWorker {
    pub(super) fn new(
        initial_engine: GameEngine,
        snapshot: Arc<RwLock<UiSnapshot>>,
        tick_in_flight: Arc<AtomicBool>,
    ) -> Self {
        let (tx, mut rx) = unbounded_channel::<WorkerMsg>();
        let handle = thread::spawn(move || {
            let mut engine = initial_engine;
            while let Some(message) = rx.blocking_recv() {
                match message {
                    WorkerMsg::Apply(command) => {
                        let is_tick = matches!(command, GameCommand::Tick);
                        engine.apply(command);
                        if let Ok(mut ui_snapshot) = snapshot.write() {
                            *ui_snapshot = UiSnapshot::from_engine(&engine);
                        } else {
                            break;
                        }
                        if is_tick {
                            tick_in_flight.store(false, Ordering::Release);
                        }
                    }
                    #[cfg(test)]
                    WorkerMsg::ApplyAndAck(command, ack_tx) => {
                        let is_tick = matches!(command, GameCommand::Tick);
                        engine.apply(command);
                        if let Ok(mut ui_snapshot) = snapshot.write() {
                            *ui_snapshot = UiSnapshot::from_engine(&engine);
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

    pub(super) fn send(&self, command: GameCommand) -> bool {
        self.tx.send(WorkerMsg::Apply(command)).is_ok()
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
    use crate::components::simulation::snapshot::SnapshotPhase;
    use crate::sim::world::RunningTab;

    fn apply_setup(worker: &SimulationWorker) {
        worker.send_and_wait(GameCommand::SetupEditDigit(1));
        worker.send_and_wait(GameCommand::SetupNextField);
        worker.send_and_wait(GameCommand::SetupEditDigit(1));
        worker.send_and_wait(GameCommand::SetupSubmit);
    }

    #[test]
    fn worker_applies_commands_in_order() {
        let initial_engine = GameEngine::default();
        let snapshot = Arc::new(RwLock::new(UiSnapshot::from_engine(&initial_engine)));
        let worker = SimulationWorker::new(
            initial_engine,
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
    fn worker_snapshot_updates_after_command_burst() {
        let initial_engine = GameEngine::default();
        let snapshot = Arc::new(RwLock::new(UiSnapshot::from_engine(&initial_engine)));
        let worker = SimulationWorker::new(
            initial_engine,
            snapshot.clone(),
            Arc::new(AtomicBool::new(false)),
        );

        apply_setup(&worker);
        for _ in 0..5 {
            worker.send_and_wait(GameCommand::Tick);
        }
        worker.send_and_wait(GameCommand::NextTab);

        let snapshot = snapshot.read().expect("snapshot lock poisoned").clone();
        let SnapshotPhase::Running(world) = snapshot.phase else {
            panic!("expected running phase");
        };
        assert_eq!(world.day, 5);
        assert_eq!(world.active_tab, RunningTab::Charts);
    }
}
