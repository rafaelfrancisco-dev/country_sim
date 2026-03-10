use crate::components::simulation::snapshot::{
    RunningSnapshot, RunningTabSnapshot, RuntimeStatsSnapshot, SnapshotPhase, UiSnapshot,
};
use crate::components::simulation::ui_state::{RunningTab, UiState};
use crate::sim::{
    GamePhase, country::Country, person::Person, runtime::SimulationMode, world::World,
};

fn sample_world_two_countries() -> World {
    World::new(vec![
        Country::new(1, "A".to_string(), vec![Person::new(1, 30, 0, 11)]),
        Country::new(2, "B".to_string(), vec![Person::new(2, 30, 0, 12)]),
    ])
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
    country.fx_reserves_base = 999.0;

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
    world
        .country_mut(0)
        .unwrap()
        .fx_reserves_by_source
        .insert(1, 42.0);
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
    assert_eq!(fx_tab.selected_reserves_breakdown.len(), 2);
    assert!(fx_tab.selected_reserves_breakdown[0].is_unattributed);
    assert!((fx_tab.selected_reserves_breakdown[1].amount_local - 63.0).abs() < f64::EPSILON);
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
fn fx_snapshot_includes_unattributed_and_zero_value_foreign_lines() {
    let mut world = sample_world_two_countries();
    world.country_mut(0).unwrap().fx_reserves_base = 120.0;

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

    assert_eq!(fx_tab.selected_reserves_breakdown.len(), 2);
    assert!(fx_tab.selected_reserves_breakdown[0].is_unattributed);
    assert!(fx_tab.selected_reserves_breakdown[0].amount_local > 0.0);
    assert_eq!(fx_tab.selected_reserves_breakdown[1].amount_local, 0.0);
}
