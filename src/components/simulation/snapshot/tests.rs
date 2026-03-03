use super::*;
use crate::sim::{GameCommand, GameEngine, GamePhase, runtime::SimulationMode};

fn runtime_stats() -> RuntimeStatsSnapshot {
    RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0)
}

fn setup_running_engine(days: usize) -> GameEngine {
    let mut engine = GameEngine::default();
    engine.apply(GameCommand::SetupEditChar('1'));
    engine.apply(GameCommand::SetupNextField);
    engine.apply(GameCommand::SetupEditChar('1'));
    engine.apply(GameCommand::SetupEditChar('0'));
    engine.apply(GameCommand::SetupSubmit);
    for _ in 0..days {
        engine.apply(GameCommand::Tick);
    }
    engine
}

fn running_snapshot_for_tab(engine: &GameEngine, active_tab: RunningTab) -> RunningSnapshot {
    let mut ui_state = UiState {
        active_tab,
        ..UiState::default()
    };
    let snapshot = UiSnapshot::from_phase_and_ui(engine.phase(), &mut ui_state, runtime_stats());
    let SnapshotPhase::Running(world) = snapshot.phase else {
        panic!("running snapshot expected");
    };
    world
}

#[test]
fn inactive_tabs_skip_heavy_data() {
    let engine = setup_running_engine(35);
    let main = running_snapshot_for_tab(&engine, RunningTab::Main);
    assert!(main.event_log_lines.is_empty());
    assert!(main.stock_rows.is_empty());
    assert!(main.chart_history.is_empty());

    let stocks = running_snapshot_for_tab(&engine, RunningTab::Stocks);
    assert!(!stocks.stock_summary_lines.is_empty());

    let events = running_snapshot_for_tab(&engine, RunningTab::Events);
    assert_eq!(events.events_total_count, events.event_log_lines.len());

    let expected_chart_points = match engine.phase() {
        GamePhase::Running(world) => world.country_history.first().map_or(0, Vec::len),
        GamePhase::Setup(_) => 0,
    };
    let charts = running_snapshot_for_tab(&engine, RunningTab::Charts);
    assert_eq!(charts.chart_history.len(), expected_chart_points);
    assert_eq!(charts.chart_page, 0);
    assert_eq!(charts.total_chart_pages, 3);
}
