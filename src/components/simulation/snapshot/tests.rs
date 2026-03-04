use super::*;
use crate::sim::{
    GameCommand, GameEngine, GamePhase,
    country::Country,
    event::{EntityKind, IpoReason, SimEvent},
    person::Person,
    runtime::SimulationMode,
    world::World,
};

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
    *world
}

fn snapshot_from_world_with_ui(world: World, mut ui_state: UiState) -> RunningSnapshot {
    let snapshot =
        UiSnapshot::from_phase_and_ui(&GamePhase::Running(world), &mut ui_state, runtime_stats());
    let SnapshotPhase::Running(world) = snapshot.phase else {
        panic!("running snapshot expected");
    };
    *world
}

#[test]
fn inactive_tabs_skip_heavy_data() {
    let engine = setup_running_engine(35);
    let main = running_snapshot_for_tab(&engine, RunningTab::Main);
    assert!(matches!(main.tab, RunningTabSnapshot::Main(_)));

    let stocks = running_snapshot_for_tab(&engine, RunningTab::Stocks);
    let RunningTabSnapshot::Stocks(stocks_tab) = stocks.tab else {
        panic!("stocks tab payload expected");
    };
    assert!(!stocks_tab.stock_summary_lines.is_empty());

    let events = running_snapshot_for_tab(&engine, RunningTab::Events);
    let RunningTabSnapshot::Events(events_tab) = events.tab else {
        panic!("events tab payload expected");
    };
    assert_eq!(
        events_tab.events_total_count,
        events_tab.event_log_lines.len()
    );

    let expected_chart_points = match engine.phase() {
        GamePhase::Running(world) => world.country_history.first().map_or(0, Vec::len),
        GamePhase::Setup(_) => 0,
    };
    let charts = running_snapshot_for_tab(&engine, RunningTab::Charts);
    let RunningTabSnapshot::Charts(charts_tab) = charts.tab else {
        panic!("charts tab payload expected");
    };
    assert_eq!(charts_tab.chart_history.len(), expected_chart_points);
    assert_eq!(charts_tab.chart_page, 0);
    assert_eq!(charts_tab.total_chart_pages, 3);
}

#[test]
fn events_category_filter_limits_results() {
    let people = vec![Person::new(1, 30, 0, 11)];
    let mut world = World::new(vec![Country::new(1, "A".to_string(), people)]);
    world.event_log = vec![
        SimEvent::PopBirth {
            country_name: "A".to_string(),
            count: 1,
            day: 1,
        },
        SimEvent::StockIpo {
            country_name: "A".to_string(),
            company_id: 1,
            company_name: "Comp".to_string(),
            ticker: "CMP".to_string(),
            reason: IpoReason::Growth,
            shares_issued: 10,
            primary_shares_issued: 8,
            secondary_shares_sold: 2,
            price_per_share: 1.0,
            total_raised: 10.0,
            primary_raised: 8.0,
            secondary_raised: 2.0,
            founder_payout_total: 1.0,
            day: 1,
        },
        SimEvent::DebtDistress {
            country_name: "A".to_string(),
            entity_kind: EntityKind::Person,
            entity_id: 1,
            unpaid_amount: 5.0,
            day: 1,
        },
    ];

    let ui = UiState {
        active_tab: RunningTab::Events,
        events_category: EventsCategoryFilter::StockMarket,
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Events(events_tab) = snapshot.tab else {
        panic!("events tab payload expected");
    };
    assert_eq!(events_tab.events_total_count, 1);
    assert_eq!(events_tab.events_category_label, "Stock Market");
    assert!(events_tab.event_log_lines[0].starts_with("[IPO]"));
}

#[test]
fn events_search_intersects_with_category_filter() {
    let people = vec![Person::new(1, 30, 0, 11)];
    let mut world = World::new(vec![Country::new(1, "A".to_string(), people)]);
    world.event_log = vec![
        SimEvent::CompanyBankrupt {
            country_name: "A".to_string(),
            company_id: 1,
            company_name: "Alpha".to_string(),
            laid_off_count: 2,
            principal_written_off: 10.0,
            day: 2,
        },
        SimEvent::DebtDistress {
            country_name: "A".to_string(),
            entity_kind: EntityKind::Company,
            entity_id: 1,
            unpaid_amount: 3.0,
            day: 2,
        },
    ];

    let ui = UiState {
        active_tab: RunningTab::Events,
        events_category: EventsCategoryFilter::BankruptciesDistress,
        events_search_query: "bankrupt".to_string(),
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Events(events_tab) = snapshot.tab else {
        panic!("events tab payload expected");
    };
    assert_eq!(events_tab.events_total_count, 1);
    assert!(events_tab.event_log_lines[0].contains("bankrupt"));
}
