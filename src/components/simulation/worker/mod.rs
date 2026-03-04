#[cfg(any(test, feature = "manual-harness"))]
use std::sync::mpsc;
use std::{
    collections::VecDeque,
    fmt,
    path::PathBuf,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::{
    snapshot::{RuntimeStatsSnapshot, UiSnapshot},
    ui_state::UiState,
};
use crate::{
    csv_export::CsvExporter,
    sim::{GameCommand, GameEngine, GamePhase, SimulationMode, UiCommand},
};

mod command_processor;
mod export_pipeline;
mod health;
mod helpers;
mod runtime;
mod scheduler;
mod snapshot_publisher;
#[cfg(test)]
mod tests;
mod types;

pub(in crate::components::simulation) use types::{
    RenderSnapshot, SimulationWorker, WorkerError, WorkerEvent, WorkerFatalReason, WorkerHealth,
};
