use std::collections::VecDeque;

use super::health::mark_fatal;
use super::types::{WorkerCommand, WorkerMsg};
use super::*;

pub(super) fn should_self_tick_next(
    mode: SimulationMode,
    running: bool,
    unpaused: bool,
    no_pending: bool,
    alive: bool,
    healthy: bool,
) -> bool {
    matches!(mode, SimulationMode::MaxSpeed)
        && running
        && unpaused
        && no_pending
        && alive
        && healthy
}

#[expect(
    clippy::too_many_arguments,
    reason = "clock loop wiring needs explicit shared state handles"
)]
pub(super) fn spawn_clock_thread(
    tx: UnboundedSender<WorkerMsg>,
    tick_in_flight: Arc<AtomicBool>,
    phase_is_running: Arc<AtomicBool>,
    world_is_paused: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    sim_mode_state: Arc<AtomicU8>,
    pending_user_commands: Arc<AtomicUsize>,
    health: Arc<Mutex<WorkerHealth>>,
    events: Arc<Mutex<VecDeque<WorkerEvent>>>,
    realtime_tick_rate: f64,
) -> Option<thread::JoinHandle<()>> {
    if realtime_tick_rate <= 0.0 {
        return None;
    }

    let realtime_interval = Duration::from_secs_f64(1.0 / realtime_tick_rate.max(0.1));
    Some(thread::spawn(move || {
        let mut current_mode =
            SimulationMode::from_atomic_u8(sim_mode_state.load(Ordering::Acquire));
        let mut next_realtime_tick_at = Instant::now();

        while !shutdown.load(Ordering::Acquire) && is_healthy(&health) {
            let mode = SimulationMode::from_atomic_u8(sim_mode_state.load(Ordering::Acquire));
            if mode != current_mode {
                current_mode = mode;
                next_realtime_tick_at = Instant::now();
            }

            if !phase_is_running.load(Ordering::Acquire) || world_is_paused.load(Ordering::Acquire)
            {
                thread::sleep(Duration::from_millis(2));
                next_realtime_tick_at = Instant::now();
                continue;
            }

            if pending_user_commands.load(Ordering::Acquire) > 0 {
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

            if tick_in_flight
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                if tx
                    .send(WorkerMsg::Apply {
                        command: WorkerCommand::Domain(GameCommand::Tick),
                        enqueued_at: Instant::now(),
                        is_user_initiated: false,
                    })
                    .is_err()
                {
                    tick_in_flight.store(false, Ordering::Release);
                    if !shutdown.load(Ordering::Acquire) {
                        mark_fatal(&health, &events, WorkerFatalReason::CommandChannelClosed);
                    }
                    break;
                }
            } else if matches!(mode, SimulationMode::MaxSpeed) {
                thread::sleep(Duration::from_millis(1));
            }
        }
    }))
}

fn is_healthy(health: &Arc<Mutex<WorkerHealth>>) -> bool {
    health
        .lock()
        .map(|state| matches!(*state, WorkerHealth::Healthy))
        .unwrap_or(false)
}
