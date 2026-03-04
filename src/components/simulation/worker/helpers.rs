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
