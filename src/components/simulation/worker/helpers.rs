use super::types::WorkerMsg;
use super::*;

pub(in crate::components::simulation) fn update_phase_flags(
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

pub(in crate::components::simulation) fn should_publish_snapshot_for_tick(
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
