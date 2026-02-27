#[cfg(test)]
use std::sync::mpsc;
use std::{
    fmt,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tracing::warn;

use super::{
    snapshot::{RuntimeStatsSnapshot, UiSnapshot},
    ui_state::UiState,
};
use crate::{
    csv_export::CsvExporter,
    sim::{GameCommand, GameEngine, GamePhase, SimulationMode, UiCommand},
};

#[derive(Debug, Clone)]
pub(super) struct RenderSnapshot {
    pub ui_snapshot: UiSnapshot,
}

#[derive(Debug, Copy, Clone)]
enum WorkerCommand {
    Domain(GameCommand),
    Ui(UiCommand),
}

#[derive(Debug)]
enum WorkerMsg {
    Apply {
        command: WorkerCommand,
        enqueued_at: Instant,
        is_user_initiated: bool,
    },
    #[cfg(test)]
    ApplyAndAck(WorkerCommand, mpsc::Sender<()>),
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
            snapshot: RuntimeStatsSnapshot::new(mode, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0),
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

    fn record_snapshot_publish(&mut self, snapshot_age_ms: f64, snapshot_build_ms: f64) {
        self.snapshot.snapshot_publish_count =
            self.snapshot.snapshot_publish_count.saturating_add(1);
        self.snapshot.max_snapshot_age_ms = self.snapshot.max_snapshot_age_ms.max(snapshot_age_ms);
        self.snapshot.avg_snapshot_build_ms = if self.snapshot.avg_snapshot_build_ms <= 0.0 {
            snapshot_build_ms
        } else {
            self.snapshot.avg_snapshot_build_ms * 0.9 + snapshot_build_ms * 0.1
        };
    }

    fn snapshot(&self) -> RuntimeStatsSnapshot {
        self.snapshot
    }
}

impl SimulationWorker {
    pub(super) fn new(
        initial_engine: GameEngine,
        snapshot: Arc<RwLock<RenderSnapshot>>,
        realtime_tick_rate: f64,
        max_speed_publish_interval: Duration,
        sim_mode_state: Arc<AtomicU8>,
        export_csv: Option<PathBuf>,
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
            let mut ui_state = UiState::default();
            let mut runtime_stats = RuntimeStats::new(SimulationMode::from_atomic_u8(
                worker_sim_mode_state.load(Ordering::Acquire),
            ));
            let mut last_snapshot_published_at = Instant::now();
            let export_csv_path = export_csv;
            let mut csv_exporter: Option<CsvExporter> = None;
            let mut export_seed: Option<u64> = None;
            let mut export_queue_activated = export_csv_path.is_none(); // skip if no path

            while let Some(message) = rx.blocking_recv() {
                if worker_shutdown.load(Ordering::Acquire) {
                    break;
                }

                let mode =
                    SimulationMode::from_atomic_u8(worker_sim_mode_state.load(Ordering::Acquire));
                runtime_stats.update_mode_and_queue(mode, rx.len());

                match message {
                    WorkerMsg::Apply {
                        command,
                        enqueued_at,
                        is_user_initiated,
                    } => {
                        let command_started_at = Instant::now();
                        let queue_lag_ms = command_started_at
                            .saturating_duration_since(enqueued_at)
                            .as_secs_f64()
                            * 1_000.0;

                        let mut is_tick = false;
                        match command {
                            WorkerCommand::Domain(game_command) => {
                                is_tick = matches!(game_command, GameCommand::Tick);
                                let outcome = engine.apply(game_command);
                                for warning in outcome.warnings {
                                    warn!("simulation invariant warning: {warning}");
                                }
                            }
                            WorkerCommand::Ui(ui_command) => {
                                Self::apply_ui_command(
                                    ui_command,
                                    &engine,
                                    &mut ui_state,
                                    &worker_sim_mode_state,
                                );
                            }
                        }

                        update_phase_flags(
                            &engine,
                            &worker_phase_is_running,
                            &worker_world_is_paused,
                        );

                        // Activate the export queue on the world once the engine is Running
                        if !export_queue_activated
                            && matches!(engine.phase(), GamePhase::Running(_))
                        {
                            engine.enable_export_queue();
                            export_queue_activated = true;
                        }

                        // Drain export queue and write CSV rows if enabled
                        let export_rows = engine.drain_export_queue();
                        if !export_rows.is_empty() {
                            if export_seed.is_none() {
                                export_seed =
                                    export_rows.first().map(|r| r.generation_seed);
                            }
                            if let (Some(path), Some(seed)) =
                                (&export_csv_path, export_seed)
                            {
                                if csv_exporter.is_none() {
                                    match CsvExporter::new(path, seed) {
                                        Ok(e) => csv_exporter = Some(e),
                                        Err(err) => tracing::error!(
                                            "failed to open CSV export: {err}"
                                        ),
                                    }
                                }
                                if let Some(exporter) = &mut csv_exporter {
                                    for export_data in &export_rows {
                                        for country_data in &export_data.countries {
                                            if let Err(err) =
                                                exporter.write_row(country_data)
                                            {
                                                tracing::error!(
                                                    "CSV write error: {err}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if is_tick {
                            let tick_duration_ms =
                                command_started_at.elapsed().as_secs_f64() * 1_000.0;
                            runtime_stats.record_tick(queue_lag_ms, tick_duration_ms);
                        }

                        let mode_after = SimulationMode::from_atomic_u8(
                            worker_sim_mode_state.load(Ordering::Acquire),
                        );
                        runtime_stats.update_mode_and_queue(mode_after, rx.len());

                        let should_publish = if is_user_initiated {
                            true
                        } else if is_tick {
                            should_publish_snapshot_for_tick(
                                mode_after,
                                command_started_at,
                                last_snapshot_published_at,
                                max_speed_publish_interval,
                            )
                        } else {
                            false
                        };

                        if should_publish {
                            let snapshot_age_ms = command_started_at
                                .saturating_duration_since(last_snapshot_published_at)
                                .as_secs_f64()
                                * 1_000.0;

                            let snapshot_started_at = Instant::now();
                            let mut ui_snapshot = UiSnapshot::from_engine_and_ui(
                                &engine,
                                &mut ui_state,
                                runtime_stats.snapshot(),
                            );
                            let snapshot_build_ms =
                                snapshot_started_at.elapsed().as_secs_f64() * 1_000.0;

                            runtime_stats
                                .record_snapshot_publish(snapshot_age_ms, snapshot_build_ms);
                            let runtime_snapshot = runtime_stats.snapshot();
                            ui_snapshot.apply_runtime_stats(runtime_snapshot);

                            if let Ok(mut shared_snapshot) = worker_snapshot.write() {
                                *shared_snapshot = RenderSnapshot { ui_snapshot };
                                last_snapshot_published_at = Instant::now();
                            } else {
                                break;
                            }
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
                        match command {
                            WorkerCommand::Domain(game_command) => {
                                let _ = engine.apply(game_command);
                            }
                            WorkerCommand::Ui(ui_command) => {
                                Self::apply_ui_command(
                                    ui_command,
                                    &engine,
                                    &mut ui_state,
                                    &worker_sim_mode_state,
                                );
                            }
                        }

                        update_phase_flags(
                            &engine,
                            &worker_phase_is_running,
                            &worker_world_is_paused,
                        );

                        let mode_after = SimulationMode::from_atomic_u8(
                            worker_sim_mode_state.load(Ordering::Acquire),
                        );
                        runtime_stats.update_mode_and_queue(mode_after, rx.len());

                        let mut ui_snapshot = UiSnapshot::from_engine_and_ui(
                            &engine,
                            &mut ui_state,
                            runtime_stats.snapshot(),
                        );
                        let runtime_snapshot = runtime_stats.snapshot();
                        ui_snapshot.apply_runtime_stats(runtime_snapshot);

                        if let Ok(mut shared_snapshot) = worker_snapshot.write() {
                            *shared_snapshot = RenderSnapshot { ui_snapshot };
                            last_snapshot_published_at = Instant::now();
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
                                command: WorkerCommand::Domain(GameCommand::Tick),
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
        }
    }

    fn apply_ui_command(
        ui_command: UiCommand,
        engine: &GameEngine,
        ui_state: &mut UiState,
        sim_mode_state: &AtomicU8,
    ) {
        if matches!(ui_command, UiCommand::ToggleSimMode) {
            let current_mode =
                SimulationMode::from_atomic_u8(sim_mode_state.load(Ordering::Acquire));
            let next_mode = match current_mode {
                SimulationMode::Realtime => SimulationMode::MaxSpeed,
                SimulationMode::MaxSpeed => SimulationMode::Realtime,
            };
            sim_mode_state.store(next_mode.as_atomic_u8(), Ordering::Release);
            return;
        }

        if let GamePhase::Running(world) = engine.phase() {
            ui_state.apply(ui_command, world);
        }
    }

    fn send_internal(
        &self,
        command: WorkerCommand,
        is_user_initiated: bool,
    ) -> Result<(), WorkerError> {
        if is_user_initiated {
            self.pending_user_commands.fetch_add(1, Ordering::AcqRel);
        }

        if self
            .tx
            .send(WorkerMsg::Apply {
                command,
                enqueued_at: Instant::now(),
                is_user_initiated,
            })
            .is_err()
        {
            if is_user_initiated {
                self.pending_user_commands.fetch_sub(1, Ordering::AcqRel);
            }
            return Err(WorkerError::CHANNEL_CLOSED);
        }

        Ok(())
    }

    pub(super) fn send_domain(&self, command: GameCommand) -> Result<(), WorkerError> {
        self.send_internal(WorkerCommand::Domain(command), true)
    }

    pub(super) fn send_ui(&self, command: UiCommand) -> Result<(), WorkerError> {
        self.send_internal(WorkerCommand::Ui(command), true)
    }

    #[cfg(test)]
    pub(super) fn send_and_wait(&self, command: GameCommand) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self
            .tx
            .send(WorkerMsg::ApplyAndAck(
                WorkerCommand::Domain(command),
                ack_tx,
            ))
            .is_ok()
        {
            let _ = ack_rx.recv();
        }
    }
}

fn update_phase_flags(
    engine: &GameEngine,
    phase_is_running: &AtomicBool,
    world_is_paused: &AtomicBool,
) {
    if let GamePhase::Running(world) = engine.phase() {
        phase_is_running.store(true, Ordering::Release);
        world_is_paused.store(world.paused, Ordering::Release);
    } else {
        phase_is_running.store(false, Ordering::Release);
        world_is_paused.store(true, Ordering::Release);
    }
}

fn should_publish_snapshot_for_tick(
    mode: SimulationMode,
    now: Instant,
    last_snapshot_published_at: Instant,
    max_speed_publish_interval: Duration,
) -> bool {
    match mode {
        SimulationMode::Realtime => true,
        SimulationMode::MaxSpeed => {
            now.saturating_duration_since(last_snapshot_published_at) >= max_speed_publish_interval
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
        let mut initial_ui_state = UiState::default();
        let runtime_stats =
            RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0);
        let snapshot = Arc::new(RwLock::new(RenderSnapshot {
            ui_snapshot: UiSnapshot::from_engine_and_ui(
                &initial_engine,
                &mut initial_ui_state,
                runtime_stats,
            ),
        }));
        let worker = SimulationWorker::new(
            initial_engine,
            snapshot.clone(),
            0.0,
            Duration::from_millis(500),
            Arc::new(AtomicU8::new(SimulationMode::Realtime.as_atomic_u8())),
            None,
        );

        worker.send_and_wait(GameCommand::SetupEditChar('1'));
        worker.send_and_wait(GameCommand::SetupNextField);
        worker.send_and_wait(GameCommand::SetupEditChar('1'));
        worker.send_and_wait(GameCommand::SetupSubmit);
        worker.send_and_wait(GameCommand::Tick);
        worker.send_and_wait(GameCommand::Tick);

        let snapshot = snapshot.read().expect("snapshot lock poisoned").clone();
        let crate::components::simulation::snapshot::SnapshotPhase::Running(world) =
            snapshot.ui_snapshot.phase
        else {
            panic!("expected running phase");
        };
        assert_eq!(world.day, 2);
    }

    #[test]
    fn max_speed_auto_tick_publish_is_cadenced() {
        let now = Instant::now();
        assert!(should_publish_snapshot_for_tick(
            SimulationMode::Realtime,
            now,
            now,
            Duration::from_millis(500)
        ));
        assert!(!should_publish_snapshot_for_tick(
            SimulationMode::MaxSpeed,
            now,
            now,
            Duration::from_millis(500)
        ));
        assert!(should_publish_snapshot_for_tick(
            SimulationMode::MaxSpeed,
            now + Duration::from_millis(600),
            now,
            Duration::from_millis(500)
        ));
    }
}
