use super::*;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub(in crate::components::simulation) struct RenderSnapshot {
    pub ui_snapshot: UiSnapshot,
}

#[derive(Debug, Copy, Clone)]
pub(super) enum WorkerCommand {
    Domain(GameCommand),
    Ui(UiCommand),
}

#[derive(Debug)]
pub(super) enum WorkerMsg {
    Apply {
        command: WorkerCommand,
        enqueued_at: Instant,
        is_user_initiated: bool,
    },
    #[cfg(any(test, feature = "manual-harness"))]
    ApplyAndAck(WorkerCommand, mpsc::Sender<()>),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::components::simulation) struct WorkerError {
    message: &'static str,
}

impl WorkerError {
    pub(super) const CHANNEL_CLOSED: Self = Self {
        message: "worker channel is closed",
    };
    pub(super) const WORKER_UNHEALTHY: Self = Self {
        message: "worker is unhealthy",
    };
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug)]
pub(in crate::components::simulation) struct SimulationWorker {
    pub(super) tx: UnboundedSender<WorkerMsg>,
    pub(super) handle: Option<thread::JoinHandle<()>>,
    pub(super) clock_handle: Option<thread::JoinHandle<()>>,
    pub(super) shutdown: Arc<AtomicBool>,
    pub(super) pending_user_commands: Arc<AtomicUsize>,
    pub(super) health: Arc<Mutex<WorkerHealth>>,
    pub(super) events: Arc<Mutex<VecDeque<WorkerEvent>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::components::simulation) enum WorkerHealth {
    Healthy,
    Fatal(WorkerFatalReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::components::simulation) enum WorkerFatalReason {
    SnapshotLockPoisoned,
    CommandChannelClosed,
    InternalPanic(String),
}

impl WorkerFatalReason {
    pub(in crate::components::simulation) fn as_message(&self) -> String {
        match self {
            Self::SnapshotLockPoisoned => "snapshot lock poisoned".to_string(),
            Self::CommandChannelClosed => "worker command channel closed".to_string(),
            Self::InternalPanic(message) => format!("worker panicked: {message}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::components::simulation) enum WorkerEvent {
    Warning(String),
    ExportError(String),
    Fatal(String),
}

#[derive(Debug)]
pub(super) struct RuntimeStats {
    snapshot: RuntimeStatsSnapshot,
    tick_window_started_at: Instant,
    ticks_in_window: u64,
}

impl RuntimeStats {
    pub(super) fn new(mode: SimulationMode) -> Self {
        Self {
            snapshot: RuntimeStatsSnapshot::new(mode, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0),
            tick_window_started_at: Instant::now(),
            ticks_in_window: 0,
        }
    }

    pub(super) fn update_mode_and_queue(&mut self, mode: SimulationMode, queue_depth: usize) {
        self.snapshot.mode = mode;
        self.snapshot.queue_depth = queue_depth;
    }

    pub(super) fn record_tick(&mut self, queue_lag_ms: f64, tick_duration_ms: f64) {
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

    pub(super) fn record_snapshot_publish(&mut self, snapshot_age_ms: f64, snapshot_build_ms: f64) {
        self.snapshot.snapshot_publish_count =
            self.snapshot.snapshot_publish_count.saturating_add(1);
        self.snapshot.max_snapshot_age_ms = self.snapshot.max_snapshot_age_ms.max(snapshot_age_ms);
        self.snapshot.avg_snapshot_build_ms = if self.snapshot.avg_snapshot_build_ms <= 0.0 {
            snapshot_build_ms
        } else {
            self.snapshot.avg_snapshot_build_ms * 0.9 + snapshot_build_ms * 0.1
        };
    }

    pub(super) fn snapshot(&self) -> RuntimeStatsSnapshot {
        self.snapshot
    }
}
