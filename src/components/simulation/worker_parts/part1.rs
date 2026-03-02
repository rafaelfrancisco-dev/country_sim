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

