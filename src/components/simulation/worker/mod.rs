#[cfg(any(test, feature = "manual-harness"))]
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

mod helpers;
mod runtime;
#[cfg(test)]
mod tests;
mod types;

pub(in crate::components::simulation) use types::{RenderSnapshot, SimulationWorker, WorkerError};
