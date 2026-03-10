use std::{collections::VecDeque, sync::Arc};

use super::*;
use crate::components::simulation::ui_state::{DetailsView, GoodsMainPage};
use crate::sim::{
    GameCommand, GameEngine, GamePhase,
    company::Company,
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

    let banks = running_snapshot_for_tab(&engine, RunningTab::Banks);
    let RunningTabSnapshot::Banks(banks_tab) = banks.tab else {
        panic!("banks tab payload expected");
    };
    assert!(!banks_tab.country_banks_summary_lines.is_empty());

    let stocks = running_snapshot_for_tab(&engine, RunningTab::Stocks);
    let RunningTabSnapshot::Stocks(stocks_tab) = stocks.tab else {
        panic!("stocks tab payload expected");
    };
    assert!(!stocks_tab.stock_summary_lines.is_empty());

    let disasters = running_snapshot_for_tab(&engine, RunningTab::Disasters);
    let RunningTabSnapshot::Disasters(disasters_tab) = disasters.tab else {
        panic!("disasters tab payload expected");
    };
    assert_eq!(
        disasters_tab.rows.len(),
        crate::sim::ALL_DISASTER_KINDS.len()
    );
    assert!(disasters_tab.last_apply_status.is_none());

    let events = running_snapshot_for_tab(&engine, RunningTab::Events);
    let RunningTabSnapshot::Events(events_tab) = events.tab else {
        panic!("events tab payload expected");
    };
    assert_eq!(
        events_tab.events_total_count,
        events_tab.event_log_lines.len()
    );

    let expected_chart_points = match engine.phase() {
        GamePhase::Running(world) => world.country_history.first().map_or(0, |v| v.len()),
        GamePhase::Setup(_) => 0,
    };
    let charts = running_snapshot_for_tab(&engine, RunningTab::Charts);
    let RunningTabSnapshot::Charts(charts_tab) = charts.tab else {
        panic!("charts tab payload expected");
    };
    assert_eq!(charts_tab.chart_history.len(), expected_chart_points);
    assert_eq!(charts_tab.chart_page, 0);
    assert_eq!(charts_tab.total_chart_pages, 8);
    assert!(!charts_tab.pie_page.population_by_country.slices.is_empty());
}

#[test]
fn charts_snapshot_uses_doctor_label_for_education_track() {
    use crate::sim::person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus};

    let doctor = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::Doctor,
    );
    let bachelor = Person::new_with_profile(
        2,
        EmploymentStatus::Unemployed,
        202,
        30 * DAYS_PER_YEAR,
        EducationTrack::Bachelor,
    );
    let world = World::new(vec![Country::new(
        1,
        "A".to_string(),
        vec![doctor, bachelor],
    )]);
    let ui = UiState {
        active_tab: RunningTab::Charts,
        ..UiState::default()
    };

    let snapshot = snapshot_from_world_with_ui(world, ui);
    let RunningTabSnapshot::Charts(charts_tab) = snapshot.tab else {
        panic!("charts tab payload expected");
    };
    let labels = charts_tab
        .pie_page
        .education_tracks
        .slices
        .iter()
        .map(|slice| slice.label.as_str())
        .collect::<Vec<_>>();

    assert!(labels.contains(&"Doctor"));
    assert!(!labels.contains(&"MedicalDoctor"));
}

#[test]
fn events_category_filter_limits_results() {
    let people = vec![Person::new(1, 30, 0, 11)];
    let mut world = World::new(vec![Country::new(1, "A".to_string(), people)]);
    world.event_log = VecDeque::from(vec![
        SimEvent::PopBirth {
            country_name: Arc::from("A"),
            count: 1,
            day: 1,
        },
        SimEvent::StockIpo {
            country_name: Arc::from("A"),
            company_id: 1,
            company_name: Arc::from("Comp"),
            ticker: Arc::from("CMP"),
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
            country_name: Arc::from("A"),
            entity_kind: EntityKind::Person,
            entity_id: 1,
            unpaid_amount: 5.0,
            day: 1,
        },
    ]);

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
    world.event_log = VecDeque::from(vec![
        SimEvent::CompanyBankrupt {
            country_name: Arc::from("A"),
            company_id: 1,
            company_name: Arc::from("Alpha"),
            laid_off_count: 2,
            principal_written_off: 10.0,
            day: 2,
        },
        SimEvent::DebtDistress {
            country_name: Arc::from("A"),
            entity_kind: EntityKind::Company,
            entity_id: 1,
            unpaid_amount: 3.0,
            day: 2,
        },
    ]);

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

#[test]
fn goods_snapshot_builds_production_demand_rows_for_each_good() {
    let people = (0..30)
        .map(|idx| Person::new(idx as u32 + 1, 30, 0, idx as u32 + 100))
        .collect::<Vec<_>>();
    let mut water_a = Company::new(1, "Water Works A".to_string(), vec![], 0);
    water_a.unit_price = 10.0;
    water_a.inventory_units = 30;
    water_a.last_month_units_produced = 40;
    water_a.last_month_units_sold = 35;
    water_a.last_month_export_units_sold = 5;
    let mut water_b = Company::new(2, "Water Works B".to_string(), vec![], 0);
    water_b.unit_price = 20.0;
    water_b.inventory_units = 5;
    water_b.last_month_units_produced = 10;
    water_b.last_month_units_sold = 8;
    water_b.last_month_export_units_sold = 2;
    let mut food = Company::new(3, "Food Co".to_string(), vec![], 1);
    food.unit_price = 12.0;
    food.inventory_units = 9;
    food.last_month_units_produced = 11;
    food.last_month_units_sold = 7;

    let country = Country::with_economy(
        1,
        "A".to_string(),
        people,
        vec![water_a, water_b, food],
        0.0,
    );
    let mut world = World::new(vec![country]);
    world.day = 9;
    let expected_currency = world
        .country(0)
        .expect("country")
        .currency_symbol()
        .to_string();

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Goods,
            goods_main_page: GoodsMainPage::ProductionDemand,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Goods(goods_tab) = snapshot.tab else {
        panic!("goods tab payload expected");
    };

    assert_eq!(goods_tab.total_goods_production_pages, 2);
    assert_eq!(goods_tab.goods_production_page, 0);

    let water_row = goods_tab
        .production_demand_rows
        .iter()
        .find(|row| row.good == "Water")
        .expect("water row");
    assert_eq!(water_row.producer_count, "2");
    assert_eq!(water_row.avg_price, format!("{expected_currency}15.00"));
    assert_eq!(water_row.inventory_units, "35");
    assert_eq!(water_row.produced_units, "50");
    assert_eq!(water_row.sold_units, "43");
    assert_eq!(water_row.export_units, "7");
    assert_eq!(water_row.est_daily_demand_units, "2");
    assert_eq!(water_row.est_mtd_demand_units, "22");

    let medicine_row = goods_tab
        .production_demand_rows
        .iter()
        .find(|row| row.good == "Medicine")
        .expect("medicine row");
    assert_eq!(medicine_row.producer_count, "0");
    assert_eq!(medicine_row.avg_price, "N/A");
    assert_eq!(medicine_row.inventory_units, "0");
}

#[test]
fn person_related_events_apply_shared_filter_and_country_scope() {
    let mut world = World::new(vec![
        Country::new(1, "A".to_string(), vec![Person::new(1, 30, 0, 11)]),
        Country::new(2, "B".to_string(), vec![Person::new(1, 30, 0, 22)]),
    ]);
    world.event_log = VecDeque::from(vec![
        SimEvent::StockMarginCall {
            country_name: Arc::from("A"),
            person_id: 1,
            shares_liquidated: 5,
            proceeds: 12.0,
            day: 3,
        },
        SimEvent::StockMarginCall {
            country_name: Arc::from("B"),
            person_id: 1,
            shares_liquidated: 3,
            proceeds: 9.0,
            day: 3,
        },
        SimEvent::StockIpo {
            country_name: Arc::from("A"),
            company_id: 0,
            company_name: Arc::from("Comp"),
            ticker: Arc::from("CMP"),
            reason: IpoReason::Growth,
            shares_issued: 10,
            primary_shares_issued: 8,
            secondary_shares_sold: 2,
            price_per_share: 1.0,
            total_raised: 10.0,
            primary_raised: 8.0,
            secondary_raised: 2.0,
            founder_payout_total: 1.0,
            day: 3,
        },
    ]);

    let ui = UiState {
        active_tab: RunningTab::Main,
        details_view: DetailsView::Person,
        events_category: EventsCategoryFilter::StockMarket,
        events_search_query: "margin".to_string(),
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Main(main_tab) = snapshot.tab else {
        panic!("main tab payload expected");
    };
    assert_eq!(main_tab.person_related_events.events_total_count, 1);
    assert_eq!(
        main_tab.person_related_events.events_category_label,
        "Stock Market"
    );
    assert!(main_tab.person_related_events.event_log_lines[0].contains("margin call"));
}

#[test]
fn company_related_events_match_company_references_and_selected_country_only() {
    let mut world = World::new(vec![
        Country::new(1, "A".to_string(), vec![Person::new(1, 30, 0, 11)]),
        Country::new(2, "B".to_string(), vec![Person::new(1, 30, 0, 22)]),
    ]);
    world.event_log = VecDeque::from(vec![
        SimEvent::JobSwitched {
            country_name: Arc::from("A"),
            person_id: 1,
            from_company_id: 0,
            to_company_id: 3,
            monthly_salary: 100.0,
            day: 4,
        },
        SimEvent::JobSwitched {
            country_name: Arc::from("B"),
            person_id: 1,
            from_company_id: 0,
            to_company_id: 2,
            monthly_salary: 110.0,
            day: 4,
        },
    ]);

    let ui = UiState {
        active_tab: RunningTab::Goods,
        details_view: DetailsView::Company,
        events_category: EventsCategoryFilter::CompaniesLabor,
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Goods(goods_tab) = snapshot.tab else {
        panic!("goods tab payload expected");
    };
    assert_eq!(goods_tab.company_related_events.events_total_count, 1);
    assert_eq!(
        goods_tab.company_related_events.events_category_label,
        "Companies & Labor"
    );
    assert!(goods_tab.company_related_events.event_log_lines[0].contains("Company #0 -> #3"));
}

#[test]
fn bank_related_events_match_selected_bank_only() {
    let mut world = World::new(vec![Country::new(
        1,
        "A".to_string(),
        vec![Person::new(1, 30, 0, 11)],
    )]);
    world.event_log = VecDeque::from(vec![
        SimEvent::BankRunStarted {
            country_name: Arc::from("A"),
            bank_id: 0,
            bank_name: Arc::from("A Public Bank"),
            health: 0.42,
            day: 5,
        },
        SimEvent::BankRunStarted {
            country_name: Arc::from("A"),
            bank_id: 1,
            bank_name: Arc::from("A Private Bank 1"),
            health: 0.31,
            day: 5,
        },
    ]);

    let ui = UiState {
        active_tab: RunningTab::Banks,
        details_view: DetailsView::Bank,
        events_category: EventsCategoryFilter::BankruptciesDistress,
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Banks(banks_tab) = snapshot.tab else {
        panic!("banks tab payload expected");
    };
    assert_eq!(banks_tab.bank_related_events.events_total_count, 1);
    assert!(banks_tab.bank_related_events.event_log_lines[0].contains("Bank #0"));
}

#[path = "tests_company_context.rs"]
mod company_context_tests;
