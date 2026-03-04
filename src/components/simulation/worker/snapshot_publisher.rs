use super::types::RuntimeStats;
use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "publish decisions and timing metadata are computed at the callsite"
)]
pub(super) fn publish_for_apply(
    is_user_initiated: bool,
    is_tick: bool,
    mode_after: SimulationMode,
    command_started_at: Instant,
    last_snapshot_published_at: &mut Instant,
    max_speed_publish_interval: Duration,
    engine: &GameEngine,
    ui_state: &mut UiState,
    runtime_stats: &mut RuntimeStats,
    worker_snapshot: &Arc<RwLock<RenderSnapshot>>,
) -> Result<(), WorkerFatalReason> {
    let should_publish = if is_user_initiated {
        true
    } else if is_tick {
        should_publish_snapshot_for_tick(
            mode_after,
            command_started_at,
            *last_snapshot_published_at,
            max_speed_publish_interval,
        )
    } else {
        false
    };

    if !should_publish {
        return Ok(());
    }

    let snapshot_age_ms = command_started_at
        .saturating_duration_since(*last_snapshot_published_at)
        .as_secs_f64()
        * 1_000.0;

    let snapshot_started_at = Instant::now();
    let mut ui_snapshot =
        UiSnapshot::from_engine_and_ui(engine, ui_state, runtime_stats.snapshot());
    let snapshot_build_ms = snapshot_started_at.elapsed().as_secs_f64() * 1_000.0;

    runtime_stats.record_snapshot_publish(snapshot_age_ms, snapshot_build_ms);
    ui_snapshot.apply_runtime_stats(runtime_stats.snapshot());
    publish_snapshot(worker_snapshot, ui_snapshot)?;
    *last_snapshot_published_at = Instant::now();

    Ok(())
}

#[cfg(any(test, feature = "manual-harness"))]
pub(super) fn force_publish(
    engine: &GameEngine,
    ui_state: &mut UiState,
    runtime_stats: &mut RuntimeStats,
    worker_snapshot: &Arc<RwLock<RenderSnapshot>>,
    last_snapshot_published_at: &mut Instant,
) -> Result<(), WorkerFatalReason> {
    let mut ui_snapshot =
        UiSnapshot::from_engine_and_ui(engine, ui_state, runtime_stats.snapshot());
    ui_snapshot.apply_runtime_stats(runtime_stats.snapshot());
    publish_snapshot(worker_snapshot, ui_snapshot)?;
    *last_snapshot_published_at = Instant::now();
    Ok(())
}

fn publish_snapshot(
    worker_snapshot: &Arc<RwLock<RenderSnapshot>>,
    ui_snapshot: UiSnapshot,
) -> Result<(), WorkerFatalReason> {
    if let Ok(mut shared_snapshot) = worker_snapshot.write() {
        *shared_snapshot = RenderSnapshot { ui_snapshot };
        Ok(())
    } else {
        Err(WorkerFatalReason::SnapshotLockPoisoned)
    }
}

pub(super) fn should_publish_snapshot_for_tick(
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
