use std::collections::VecDeque;

use tracing::warn;

use super::health::push_event;
use super::types::WorkerCommand;
use super::*;

pub(super) fn apply_command(
    command: WorkerCommand,
    engine: &mut GameEngine,
    ui_state: &mut UiState,
    sim_mode_state: &AtomicU8,
    events: &Arc<Mutex<VecDeque<WorkerEvent>>>,
) -> bool {
    match command {
        WorkerCommand::Domain(game_command) => {
            let is_tick = matches!(game_command, GameCommand::Tick);
            let outcome = engine.apply(game_command);
            for warning in outcome.warnings {
                let warning_message = warning.to_string();
                warn!("simulation invariant warning: {warning_message}");
                push_event(events, WorkerEvent::Warning(warning_message));
            }
            is_tick
        }
        WorkerCommand::Ui(ui_command) => {
            apply_ui_command(ui_command, engine, ui_state, sim_mode_state, events);
            false
        }
    }
}

pub(super) fn apply_ui_command(
    ui_command: UiCommand,
    engine: &mut GameEngine,
    ui_state: &mut UiState,
    sim_mode_state: &AtomicU8,
    events: &Arc<Mutex<VecDeque<WorkerEvent>>>,
) {
    if matches!(ui_command, UiCommand::ToggleSimMode) {
        let current_mode = SimulationMode::from_atomic_u8(sim_mode_state.load(Ordering::Acquire));
        let next_mode = match current_mode {
            SimulationMode::Realtime => SimulationMode::MaxSpeed,
            SimulationMode::MaxSpeed => SimulationMode::Realtime,
        };
        sim_mode_state.store(next_mode.as_atomic_u8(), Ordering::Release);
        return;
    }

    let domain_command = if let GamePhase::Running(world) = engine.phase() {
        ui_state.apply(ui_command, world)
    } else {
        None
    };
    if let Some(domain_command) = domain_command {
        let outcome = engine.apply(domain_command);
        for warning in outcome.warnings {
            let warning_message = warning.to_string();
            warn!("simulation invariant warning: {warning_message}");
            push_event(events, WorkerEvent::Warning(warning_message));
        }
    }
}
