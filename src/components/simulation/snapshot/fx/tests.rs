use crate::components::simulation::snapshot::{
    RunningSnapshot, RunningTabSnapshot, RuntimeStatsSnapshot, SnapshotPhase, UiSnapshot,
};
use crate::components::simulation::ui_state::{RunningTab, UiState};
use crate::sim::{
    GamePhase, country::Country, currency::CurrencySpec, person::Person, runtime::SimulationMode,
    world::MetricPoint, world::World,
};
use std::collections::{HashMap, VecDeque};

fn sample_world_two_countries() -> World {
    World::new(vec![
        Country::new(1, "A".to_string(), vec![Person::new(1, 30, 0, 11)]),
        Country::new(2, "B".to_string(), vec![Person::new(2, 30, 0, 12)]),
    ])
}

fn sample_world_with_shared_foreign_currency() -> World {
    let selected = Country::new(1, "A".to_string(), vec![Person::new(1, 30, 0, 11)]);
    let mut foreign_a = Country::new(2, "B".to_string(), vec![Person::new(2, 30, 0, 12)]);
    let mut foreign_b = Country::new(3, "C".to_string(), vec![Person::new(3, 30, 0, 13)]);
    let shared_currency = CurrencySpec::new("EUR", "Euro", "€");
    foreign_a.currency_spec = shared_currency.clone();
    foreign_b.currency_spec = shared_currency;
    World::new(vec![selected, foreign_a, foreign_b])
}

fn snapshot_from_world_with_ui(world: World, mut ui_state: UiState) -> RunningSnapshot {
    let snapshot = UiSnapshot::from_phase_and_ui(
        &GamePhase::Running(world),
        &mut ui_state,
        RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0),
    );
    let SnapshotPhase::Running(world) = snapshot.phase else {
        panic!("running snapshot expected");
    };
    *world
}

#[test]
fn fx_snapshot_populates_selected_summary_from_country_fields() {
    let mut world = World::new(vec![Country::new(
        1,
        "A".to_string(),
        vec![Person::new(1, 30, 0, 11)],
    )]);
    let country = world.country_mut(0).unwrap();
    country.monthly_goods_export_revenue_base = 320.0;
    country.monthly_goods_import_spending_base = 180.0;
    country.monthly_fx_financial_inflows_base = 75.0;
    country.monthly_fx_financial_outflows_base = 40.0;
    country.monthly_fx_net_flow_base = 35.0;
    country.fx_pressure = 0.22;
    country.set_public_bank_fx_reserves(999.0, HashMap::new(), 999.0);

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Fx,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Fx(fx_tab) = snapshot.tab else {
        panic!("fx tab payload expected");
    };

    let text = fx_tab.selected_fx_summary_lines.join("\n");
    assert!(text.contains("FX Reserves:"));
    assert!(text.contains("Financial In / Out:"));
    assert!(text.contains("Net FX Flow:"));
    assert!(text.contains("FX Pressure: 0.22"));
}

#[test]
fn fx_snapshot_uses_reserve_header_when_breakdown_exists() {
    let mut world = sample_world_two_countries();
    world.country_mut(0).unwrap().set_public_bank_fx_reserves(
        42.0,
        HashMap::from([(1, 42.0)]),
        42.0,
    );
    world.country_mut(1).unwrap().fx_rate_local_per_base = 1.5;

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Fx,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Fx(fx_tab) = snapshot.tab else {
        panic!("fx tab payload expected");
    };

    assert_eq!(
        fx_tab.selected_fx_summary_lines.last(),
        Some(&"FX Reserves:".to_string())
    );
    assert_eq!(fx_tab.selected_reserves_breakdown.len(), 1);
    assert!((fx_tab.selected_reserves_breakdown[0].amount_local - 63.0).abs() < f64::EPSILON);
}

#[test]
fn fx_snapshot_seeds_initial_reserves_into_foreign_currency_lines() {
    let world = sample_world_two_countries();

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Fx,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Fx(fx_tab) = snapshot.tab else {
        panic!("fx tab payload expected");
    };

    assert_eq!(fx_tab.selected_reserves_breakdown.len(), 1);
    assert!(fx_tab.selected_reserves_breakdown[0].amount_local > 0.0);
}

#[test]
fn fx_snapshot_sorts_rows_and_formats_missing_history_and_ratio() {
    let mut world = sample_world_two_countries();
    let template = world.country_history[0][0];
    let c0 = world.country_mut(0).unwrap();
    c0.fx_rate_local_per_base = 2.50;
    c0.monthly_goods_export_revenue_base = 200.0;
    c0.monthly_goods_import_spending_base = 100.0;
    let c1 = world.country_mut(1).unwrap();
    c1.fx_rate_local_per_base = 1.10;
    c1.monthly_goods_export_revenue_base = 50.0;
    c1.monthly_goods_import_spending_base = 0.0;
    world.country_history[0].clear();
    world.country_history[0].push_back(crate::sim::world::MetricPoint {
        fx_rate_local_per_base: 2.0,
        ..template
    });
    world.country_history[0].push_back(crate::sim::world::MetricPoint {
        fx_rate_local_per_base: 2.5,
        ..template
    });
    world.country_history[1].truncate(1);

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Fx,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Fx(fx_tab) = snapshot.tab else {
        panic!("fx tab payload expected");
    };

    assert_eq!(fx_tab.fx_rows[0].country, "A");
    assert_eq!(fx_tab.fx_rows[0].month_move_pct, "+25.00%");
    assert_eq!(fx_tab.fx_rows[0].export_import_ratio, "2.00x");
    assert_eq!(fx_tab.fx_rows[1].month_move_pct, "N/A");
    assert_eq!(fx_tab.fx_rows[1].export_import_ratio, "N/A");
}

#[test]
fn fx_snapshot_includes_zero_value_foreign_lines() {
    let mut world = sample_world_two_countries();
    world
        .country_mut(0)
        .unwrap()
        .set_public_bank_fx_reserves(120.0, HashMap::new(), 120.0);

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Fx,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Fx(fx_tab) = snapshot.tab else {
        panic!("fx tab payload expected");
    };

    assert_eq!(fx_tab.selected_reserves_breakdown.len(), 1);
    assert_eq!(fx_tab.selected_reserves_breakdown[0].amount_local, 0.0);
}

#[test]
fn fx_snapshot_aggregates_shared_currency_reserves_and_history() {
    let mut world = sample_world_with_shared_foreign_currency();
    let history_template = world.country_history[0][0];
    world.country_history[0] = VecDeque::from([
        MetricPoint {
            fx_rate_local_per_base: 2.0,
            ..history_template
        },
        MetricPoint {
            fx_rate_local_per_base: 4.0,
            ..history_template
        },
    ]);
    world.country_fx_source_history[0].insert(1, VecDeque::from([2.0, 3.0]));
    world.country_fx_source_history[0].insert(2, VecDeque::from([5.0, 7.0]));

    world.country_mut(1).unwrap().fx_rate_local_per_base = 2.0;
    world.country_mut(2).unwrap().fx_rate_local_per_base = 2.0;
    world.country_mut(0).unwrap().set_public_bank_fx_reserves(
        25.0,
        HashMap::from([(1, 10.0), (2, 15.0)]),
        25.0,
    );

    let snapshot = snapshot_from_world_with_ui(
        world,
        UiState {
            active_tab: RunningTab::Fx,
            ..UiState::default()
        },
    );
    let RunningTabSnapshot::Fx(fx_tab) = snapshot.tab else {
        panic!("fx tab payload expected");
    };

    assert_eq!(fx_tab.selected_reserves_breakdown.len(), 1);
    assert_eq!(fx_tab.selected_reserves_breakdown[0].currency_code, "EUR");
    assert!((fx_tab.selected_reserves_breakdown[0].amount_local - 50.0).abs() < f64::EPSILON);
    assert_eq!(fx_tab.selected_fx_reserves_by_currency.len(), 1);
    assert_eq!(fx_tab.selected_fx_reserves_by_currency[0].0, "EUR");
    assert_eq!(
        fx_tab.selected_fx_reserves_by_currency[0].1.as_ref(),
        [14.0, 40.0]
    );
}
