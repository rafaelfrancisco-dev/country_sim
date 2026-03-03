use crossterm::event::{KeyCode, KeyEvent};
use std::{
    sync::{Arc, atomic::AtomicU8},
    thread,
    time::{Duration, Instant},
};

use super::{RunningTab, Simulation, SnapshotPhase};
use crate::{
    action::Action,
    components::Component,
    sim::{GameCommand, SimulationMode, engine::DEFAULT_GENERATION_SEED},
};

#[derive(Debug, Clone, PartialEq)]
pub struct UiProbeReport {
    pub day_start: u64,
    pub day_end: u64,
    pub max_snapshot_age_ms: f64,
    pub avg_command_latency_ms: f64,
    pub p95_command_latency_ms: f64,
    pub samples: usize,
}

pub fn run_max_speed_ui_probe(probe_duration: Duration) -> UiProbeReport {
    let sim_mode_state = Arc::new(AtomicU8::new(SimulationMode::MaxSpeed.as_atomic_u8()));
    let mut simulation =
        Simulation::with_runtime(4.0, 2.0, sim_mode_state, DEFAULT_GENERATION_SEED, None);
    simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
    simulation.apply_command_and_wait(GameCommand::SetupNextField);
    simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
    simulation.apply_command_and_wait(GameCommand::SetupEditChar('0'));
    simulation.apply_command_and_wait(GameCommand::SetupSubmit);

    let started_at = Instant::now();
    let mut max_snapshot_age_observed = 0.0_f64;
    let mut day_start = 0_u64;
    let mut day_end = 0_u64;
    let mut latencies_ms = Vec::new();

    if let SnapshotPhase::Running(world) = simulation.snapshot_phase() {
        day_start = world.day;
    }

    while started_at.elapsed() < probe_duration {
        let _ = simulation.update(Action::Render);

        if let SnapshotPhase::Running(world) = simulation.snapshot_phase() {
            day_end = world.day;
            max_snapshot_age_observed = max_snapshot_age_observed.max(world.max_snapshot_age_ms);
        }

        let cmd_start = Instant::now();
        let _ = simulation.handle_key_event(KeyEvent::from(KeyCode::Tab));
        let initial_tab = match simulation.snapshot_phase() {
            SnapshotPhase::Running(world) => world.active_tab,
            SnapshotPhase::Setup(_) => RunningTab::Main,
        };

        let deadline = Instant::now() + Duration::from_millis(1200);
        while Instant::now() < deadline {
            let _ = simulation.update(Action::Render);
            if let SnapshotPhase::Running(world) = simulation.snapshot_phase()
                && world.active_tab != initial_tab
            {
                latencies_ms.push(cmd_start.elapsed().as_secs_f64() * 1_000.0);
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        thread::sleep(Duration::from_millis(120));
    }

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let avg_latency = if latencies_ms.is_empty() {
        0.0
    } else {
        latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64
    };
    let p95_latency = if latencies_ms.is_empty() {
        0.0
    } else {
        let idx = ((latencies_ms.len() - 1) as f64 * 0.95).round() as usize;
        latencies_ms[idx]
    };

    UiProbeReport {
        day_start,
        day_end,
        max_snapshot_age_ms: max_snapshot_age_observed,
        avg_command_latency_ms: avg_latency,
        p95_command_latency_ms: p95_latency,
        samples: latencies_ms.len(),
    }
}
