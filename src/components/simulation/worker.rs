#[cfg(test)]
use std::sync::mpsc;
use std::{
    fmt,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tracing::warn;

use super::snapshot::RuntimeStatsSnapshot;
use crate::sim::{GameCommand, GameEngine, GamePhase, SimulationMode};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct EngineSnapshot {
    pub phase: GamePhase,
    pub runtime_stats: RuntimeStatsSnapshot,
}

#[derive(Debug)]
enum WorkerMsg {
    Apply {
        command: GameCommand,
        enqueued_at: Instant,
        is_user_initiated: bool,
    },
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
    clock_handle: Option<thread::JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    pending_user_commands: Arc<AtomicUsize>,
    sim_mode_state: Arc<AtomicU8>,
}

#[derive(Debug)]
struct RuntimeStats {
    snapshot: RuntimeStatsSnapshot,
    tick_window_started_at: Instant,
    ticks_in_window: u64,
}

impl RuntimeStats {
    fn new(mode: SimulationMode) -> Self {
        Self {
            snapshot: RuntimeStatsSnapshot::new(mode, 0.0, 0.0, 0, 0.0),
            tick_window_started_at: Instant::now(),
            ticks_in_window: 0,
        }
    }

    fn update_mode_and_queue(&mut self, mode: SimulationMode, queue_depth: usize) {
        self.snapshot.mode = mode;
        self.snapshot.queue_depth = queue_depth;
    }

    fn record_tick(&mut self, queue_lag_ms: f64, tick_duration_ms: f64) {
        self.snapshot.tick_queue_lag_ms = queue_lag_ms;
        self.snapshot.avg_tick_duration_ms = if self.snapshot.avg_tick_duration_ms <= 0.0 {
            tick_duration_ms
        } else {
            self.snapshot.avg_tick_duration_ms * 0.9 + tick_duration_ms * 0.1
        };

        self.ticks_in_window = self.ticks_in_window.saturating_add(1);
        let elapsed = self.tick_window_started_at.elapsed().as_secs_f64();
        if elapsed >= 0.5 {
            self.snapshot.ticks_per_second = self.ticks_in_window as f64 / elapsed;
            self.tick_window_started_at = Instant::now();
            self.ticks_in_window = 0;
        }
    }

    fn snapshot(&self) -> RuntimeStatsSnapshot {
        self.snapshot
    }
}

impl SimulationWorker {
    pub(super) fn new(
        initial_engine: GameEngine,
        snapshot: Arc<RwLock<EngineSnapshot>>,
        realtime_tick_rate: f64,
        sim_mode_state: Arc<AtomicU8>,
    ) -> Self {
        let tick_in_flight = Arc::new(AtomicBool::new(false));
        let pending_user_commands = Arc::new(AtomicUsize::new(0));
        let phase_is_running = Arc::new(AtomicBool::new(matches!(
            initial_engine.phase(),
            GamePhase::Running(_)
        )));
        let world_is_paused = Arc::new(AtomicBool::new(matches!(
            initial_engine.phase(),
            GamePhase::Running(world) if world.paused
        )));
        let shutdown = Arc::new(AtomicBool::new(false));

        let (tx, mut rx) = unbounded_channel::<WorkerMsg>();

        let worker_snapshot = snapshot.clone();
        let worker_phase_is_running = phase_is_running.clone();
        let worker_world_is_paused = world_is_paused.clone();
        let worker_tick_in_flight = tick_in_flight.clone();
        let worker_shutdown = shutdown.clone();
        let worker_sim_mode_state = sim_mode_state.clone();
        let worker_pending_user_commands = pending_user_commands.clone();

        let handle = thread::spawn(move || {
            let mut engine = initial_engine;
            let mut runtime_stats = RuntimeStats::new(SimulationMode::from_atomic_u8(
                worker_sim_mode_state.load(Ordering::Acquire),
            ));

            while let Some(message) = rx.blocking_recv() {
                if worker_shutdown.load(Ordering::Acquire) {
                    break;
                }

                let queue_depth = rx.len();
                let mode =
                    SimulationMode::from_atomic_u8(worker_sim_mode_state.load(Ordering::Acquire));
                runtime_stats.update_mode_and_queue(mode, queue_depth);

                match message {
                    WorkerMsg::Apply {
                        command,
                        enqueued_at,
                        is_user_initiated,
                    } => {
                        let is_tick = matches!(command, GameCommand::Tick);
                        let command_started_at = Instant::now();
                        let queue_lag_ms = command_started_at
                            .saturating_duration_since(enqueued_at)
                            .as_secs_f64()
                            * 1_000.0;

                        let outcome = engine.apply(command);
                        for warning in outcome.warnings {
                            warn!("simulation invariant warning: {warning}");
                        }

                        if let GamePhase::Running(world) = engine.phase() {
                            worker_phase_is_running.store(true, Ordering::Release);
                            worker_world_is_paused.store(world.paused, Ordering::Release);
                        } else {
                            worker_phase_is_running.store(false, Ordering::Release);
                            worker_world_is_paused.store(true, Ordering::Release);
                        }

                        if is_tick {
                            let tick_duration_ms =
                                command_started_at.elapsed().as_secs_f64() * 1_000.0;
                            runtime_stats.record_tick(queue_lag_ms, tick_duration_ms);
                        }

                        if let Ok(mut shared_snapshot) = worker_snapshot.write() {
                            *shared_snapshot = EngineSnapshot {
                                phase: engine.phase().clone(),
                                runtime_stats: runtime_stats.snapshot(),
                            };
                        } else {
                            break;
                        }

                        if is_tick {
                            worker_tick_in_flight.store(false, Ordering::Release);
                        }
                        if is_user_initiated {
                            worker_pending_user_commands.fetch_sub(1, Ordering::AcqRel);
                        }
                    }
                    #[cfg(test)]
                    WorkerMsg::ApplyAndAck(command, ack_tx) => {
                        let command_started_at = Instant::now();
                        let is_tick = matches!(command, GameCommand::Tick);
                        let outcome = engine.apply(command);
                        for warning in outcome.warnings {
                            warn!("simulation invariant warning: {warning}");
                        }

                        if let GamePhase::Running(world) = engine.phase() {
                            worker_phase_is_running.store(true, Ordering::Release);
                            worker_world_is_paused.store(world.paused, Ordering::Release);
                        } else {
                            worker_phase_is_running.store(false, Ordering::Release);
                            worker_world_is_paused.store(true, Ordering::Release);
                        }

                        if is_tick {
                            let tick_duration_ms =
                                command_started_at.elapsed().as_secs_f64() * 1_000.0;
                            runtime_stats.record_tick(0.0, tick_duration_ms);
                        }

                        if let Ok(mut shared_snapshot) = worker_snapshot.write() {
                            *shared_snapshot = EngineSnapshot {
                                phase: engine.phase().clone(),
                                runtime_stats: runtime_stats.snapshot(),
                            };
                        } else {
                            break;
                        }
                        let _ = ack_tx.send(());
                    }
                    WorkerMsg::Shutdown => break,
                }
            }
        });

        let clock_handle = if realtime_tick_rate > 0.0 {
            let clock_tx = tx.clone();
            let clock_tick_in_flight = tick_in_flight;
            let clock_phase_is_running = phase_is_running;
            let clock_world_is_paused = world_is_paused;
            let clock_shutdown = shutdown.clone();
            let clock_sim_mode_state = sim_mode_state.clone();
            let clock_pending_user_commands = pending_user_commands.clone();
            let realtime_interval = Duration::from_secs_f64(1.0 / realtime_tick_rate.max(0.1));

            Some(thread::spawn(move || {
                let mut current_mode =
                    SimulationMode::from_atomic_u8(clock_sim_mode_state.load(Ordering::Acquire));
                let mut next_realtime_tick_at = Instant::now();

                while !clock_shutdown.load(Ordering::Acquire) {
                    let mode = SimulationMode::from_atomic_u8(
                        clock_sim_mode_state.load(Ordering::Acquire),
                    );
                    if mode != current_mode {
                        current_mode = mode;
                        next_realtime_tick_at = Instant::now();
                    }

                    if !clock_phase_is_running.load(Ordering::Acquire)
                        || clock_world_is_paused.load(Ordering::Acquire)
                    {
                        thread::sleep(Duration::from_millis(2));
                        next_realtime_tick_at = Instant::now();
                        continue;
                    }

                    if clock_pending_user_commands.load(Ordering::Acquire) > 0 {
                        thread::sleep(Duration::from_micros(100));
                        continue;
                    }

                    if matches!(mode, SimulationMode::Realtime) {
                        let now = Instant::now();
                        if now < next_realtime_tick_at {
                            thread::sleep(next_realtime_tick_at.duration_since(now));
                            continue;
                        }
                        next_realtime_tick_at = now + realtime_interval;
                    }

                    if clock_tick_in_flight
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        if clock_tx
                            .send(WorkerMsg::Apply {
                                command: GameCommand::Tick,
                                enqueued_at: Instant::now(),
                                is_user_initiated: false,
                            })
                            .is_err()
                        {
                            clock_tick_in_flight.store(false, Ordering::Release);
                            break;
                        }
                    } else if matches!(mode, SimulationMode::MaxSpeed) {
                        thread::sleep(Duration::from_micros(50));
                    }
                }
            }))
        } else {
            None
        };

        Self {
            tx,
            handle: Some(handle),
            clock_handle,
            shutdown,
            pending_user_commands,
            sim_mode_state,
        }
    }

    pub(super) fn send(&self, command: GameCommand) -> Result<(), WorkerError> {
        self.pending_user_commands.fetch_add(1, Ordering::AcqRel);
        if self
            .tx
            .send(WorkerMsg::Apply {
                command,
                enqueued_at: Instant::now(),
                is_user_initiated: true,
            })
            .is_err()
        {
            self.pending_user_commands.fetch_sub(1, Ordering::AcqRel);
            return Err(WorkerError::CHANNEL_CLOSED);
        }

        Ok(())
    }

    pub(super) fn toggle_sim_mode(&self) {
        let current_mode =
            SimulationMode::from_atomic_u8(self.sim_mode_state.load(Ordering::Acquire));
        let next_mode = match current_mode {
            SimulationMode::Realtime => SimulationMode::MaxSpeed,
            SimulationMode::MaxSpeed => SimulationMode::Realtime,
        };
        self.sim_mode_state
            .store(next_mode.as_atomic_u8(), Ordering::Release);
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
        self.shutdown.store(true, Ordering::Release);
        let _ = self.tx.send(WorkerMsg::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.clock_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_applies_commands_in_order() {
        let initial_engine = GameEngine::default();
        let snapshot = Arc::new(RwLock::new(EngineSnapshot {
            phase: initial_engine.phase().clone(),
            runtime_stats: RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0),
        }));
        let worker = SimulationWorker::new(
            initial_engine,
            snapshot.clone(),
            0.0,
            Arc::new(AtomicU8::new(SimulationMode::Realtime.as_atomic_u8())),
        );

        worker.send_and_wait(GameCommand::SetupEditChar('1'));
        worker.send_and_wait(GameCommand::SetupNextField);
        worker.send_and_wait(GameCommand::SetupEditChar('1'));
        worker.send_and_wait(GameCommand::SetupSubmit);
        worker.send_and_wait(GameCommand::Tick);
        worker.send_and_wait(GameCommand::Tick);

        let snapshot = snapshot.read().expect("snapshot lock poisoned").clone();
        let GamePhase::Running(world) = snapshot.phase else {
            panic!("expected running phase");
        };
        assert_eq!(world.day, 2);
    }
}
