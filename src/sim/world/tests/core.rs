use super::*;
use crate::sim::{
    event::{IpoReason, SimEvent},
    generator::{GenerationConfig, generate_countries},
};
use rayon::ThreadPoolBuilder;
use std::collections::HashMap;

#[test]
fn world_tick_increments_day_and_ticks_countries() {
    let country = Country::new(1, "Axion".to_string(), build_people(2));
    let mut world = World::new(vec![country]);

    world.tick_day();

    assert_eq!(world.day, 1);
    assert_eq!(world.countries[0].day, 1);
}

#[test]
fn world_starts_with_initial_history() {
    let world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

    assert_eq!(world.world_history.len(), 1);
    assert_eq!(world.country_history.len(), 1);
    assert_eq!(world.country_history[0].len(), 1);
    assert_eq!(world.world_history[0].month, 0);
}

#[test]
fn world_tick_advances_days_and_processes_monthly_boundaries() {
    let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

    for _ in 0..DAYS_PER_MONTH {
        world.tick_day();
    }
    assert_eq!(world.world_history.len(), 2);
    assert_eq!(world.countries[0].month, 1);
    assert_eq!(world.completed_month_boundaries(), 1);
}

#[test]
fn world_history_retains_only_latest_limit() {
    let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(1))]);

    for _ in 0..(HISTORY_LIMIT as u64 + 10) {
        for _ in 0..DAYS_PER_MONTH {
            world.tick_day();
        }
    }

    assert_eq!(world.world_history.len(), HISTORY_LIMIT);
    assert_eq!(world.country_history[0].len(), HISTORY_LIMIT);
}

#[test]
fn aggregate_history_matches_country_sums() {
    let countries = vec![
        Country::new(1, "A".to_string(), build_people(2)),
        Country::new(2, "B".to_string(), build_people(3)),
    ];
    let world = World::new(countries);
    let aggregate = world.world_history[0];
    let country_sum = world.country_history.iter().map(|series| series[0]).fold(
        MetricPoint {
            month: 0,
            population: 0.0,
            total_money_supply: 0.0,
            household_cash_total: 0.0,
            company_cash_total: 0.0,
            expected_money_supply: 0.0,
            money_supply_gap: 0.0,
            created_money_outstanding: 0.0,
            bank_reserve: 0.0,
            treasury_balance: 0.0,
            budget_balance: 0.0,
            monthly_tax_collected: 0.0,
            monthly_gov_spending: 0.0,
            gov_debt: 0.0,
            monthly_goods_import_spending: 0.0,
            monthly_goods_export_revenue: 0.0,
            monthly_unmet_need_events: 0.0,
            monthly_essential_debt_spending: 0.0,
            cpi_index: 0.0,
            inflation_rate: 0.0,
            policy_rate: 0.0,
            bonds_outstanding: 0.0,
            stock_market_cap: 0.0,
            stock_bank_equity_exposure: 0.0,
        },
        |acc, point| MetricPoint {
            month: acc.month.max(point.month),
            population: acc.population + point.population,
            total_money_supply: acc.total_money_supply + point.total_money_supply,
            household_cash_total: acc.household_cash_total + point.household_cash_total,
            company_cash_total: acc.company_cash_total + point.company_cash_total,
            expected_money_supply: acc.expected_money_supply + point.expected_money_supply,
            money_supply_gap: acc.money_supply_gap + point.money_supply_gap,
            created_money_outstanding: acc.created_money_outstanding
                + point.created_money_outstanding,
            bank_reserve: acc.bank_reserve + point.bank_reserve,
            treasury_balance: acc.treasury_balance + point.treasury_balance,
            budget_balance: acc.budget_balance + point.budget_balance,
            monthly_tax_collected: acc.monthly_tax_collected + point.monthly_tax_collected,
            monthly_gov_spending: acc.monthly_gov_spending + point.monthly_gov_spending,
            gov_debt: acc.gov_debt + point.gov_debt,
            monthly_goods_import_spending: acc.monthly_goods_import_spending
                + point.monthly_goods_import_spending,
            monthly_goods_export_revenue: acc.monthly_goods_export_revenue
                + point.monthly_goods_export_revenue,
            monthly_unmet_need_events: acc.monthly_unmet_need_events
                + point.monthly_unmet_need_events,
            monthly_essential_debt_spending: acc.monthly_essential_debt_spending
                + point.monthly_essential_debt_spending,
            cpi_index: acc.cpi_index + point.cpi_index,
            inflation_rate: acc.inflation_rate + point.inflation_rate,
            policy_rate: acc.policy_rate + point.policy_rate,
            bonds_outstanding: acc.bonds_outstanding + point.bonds_outstanding,
            stock_market_cap: acc.stock_market_cap + point.stock_market_cap,
            stock_bank_equity_exposure: acc.stock_bank_equity_exposure
                + point.stock_bank_equity_exposure,
        },
    );

    assert_eq!(aggregate.month, country_sum.month);
    assert!((aggregate.population - country_sum.population).abs() < f64::EPSILON);
    assert!((aggregate.total_money_supply - country_sum.total_money_supply).abs() < f64::EPSILON);
    assert!(
        (aggregate.created_money_outstanding - country_sum.created_money_outstanding).abs()
            < f64::EPSILON
    );
    assert!((aggregate.bank_reserve - country_sum.bank_reserve).abs() < f64::EPSILON);
    assert!((aggregate.treasury_balance - country_sum.treasury_balance).abs() < f64::EPSILON);
    assert!((aggregate.budget_balance - country_sum.budget_balance).abs() < f64::EPSILON);
    assert!((aggregate.gov_debt - country_sum.gov_debt).abs() < f64::EPSILON);
    assert!(
        (aggregate.monthly_tax_collected - country_sum.monthly_tax_collected).abs() < f64::EPSILON
    );
    assert!((aggregate.stock_market_cap - country_sum.stock_market_cap).abs() < f64::EPSILON);
}

#[test]
fn world_tick_parallel_paths_keep_invariants_small_world() {
    let single_thread = run_world_in_pool(2, 25, 120, 1);
    let multi_thread = run_world_in_pool(2, 25, 120, 4);

    assert_eq!(single_thread.day, multi_thread.day);
    assert_eq!(single_thread.countries.len(), multi_thread.countries.len());
    assert!(single_thread.validate_invariants().is_empty());
    assert!(multi_thread.validate_invariants().is_empty());
}

#[test]
fn world_tick_parallel_paths_keep_invariants_large_world() {
    let single_thread = run_world_in_pool(8, 120, 90, 1);
    let multi_thread = run_world_in_pool(8, 120, 90, 8);

    assert_eq!(single_thread.day, multi_thread.day);
    assert_eq!(single_thread.countries.len(), multi_thread.countries.len());
    assert!(single_thread.validate_invariants().is_empty());
    assert!(multi_thread.validate_invariants().is_empty());
}

#[test]
fn history_capture_counts_remain_consistent_at_month_boundaries_parallel_mode() {
    let world_one_thread = run_world_in_pool(3, 30, DAYS_PER_MONTH * 3, 1);
    let world_many_threads = run_world_in_pool(3, 30, DAYS_PER_MONTH * 3, 8);

    assert_eq!(
        world_one_thread.world_history.len(),
        world_many_threads.world_history.len()
    );
    assert_eq!(
        world_one_thread.country_history.len(),
        world_many_threads.country_history.len()
    );
}

#[test]
fn generated_world_hash_is_deterministic_for_same_seed_at_day_zero() {
    let first = run_generated_world_in_pool(41, 4, 2_500, 0, 4);
    let second = run_generated_world_in_pool(41, 4, 2_500, 0, 4);

    assert_eq!(world_state_hash(&first), world_state_hash(&second));
}

#[test]
fn generated_world_hash_differs_for_different_seeds_at_day_zero() {
    let first = run_generated_world_in_pool(77, 5, 2_000, 0, 4);
    let second = run_generated_world_in_pool(78, 5, 2_000, 0, 4);

    assert_ne!(world_state_hash(&first), world_state_hash(&second));
}

#[cfg(feature = "bench-trace")]
#[test]
fn traced_tick_path_processes_month_boundary_and_keeps_invariants() {
    let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(250))]);
    let mut month_trace = None;

    for expected_day in 1..=(DAYS_PER_MONTH + 1) {
        let trace = world.tick_day_with_trace();
        assert_eq!(trace.day, expected_day);

        if expected_day == DAYS_PER_MONTH {
            assert!(trace.is_month_boundary);
            month_trace = Some(trace);
        } else {
            assert!(!trace.is_month_boundary);
        }
    }

    let trace = month_trace.expect("month boundary trace must be captured");
    assert!(trace.world_monthly_total_ms >= 0.0);
    assert!(trace.world_monthly_process_countries_ms >= 0.0);
    assert!(trace.world_monthly_tariff_adjust_ms >= 0.0);
    assert!(trace.world_monthly_rate_update_ms >= 0.0);
    assert!(trace.world_monthly_capture_snapshot_ms >= 0.0);
    assert!(trace.world_monthly_reset_trackers_ms >= 0.0);

    assert_eq!(world.day, DAYS_PER_MONTH + 1);
    assert_eq!(world.completed_month_boundaries(), 1);
    assert_eq!(world.world_history.len(), 2);
    assert_eq!(world.country_history[0].len(), 2);
    assert_eq!(world.countries[0].month, 1);
    assert!(world.validate_invariants().is_empty());
}

fn labor_force_unemployment(country: &Country) -> f64 {
    let mut labor_force = 0_usize;
    let mut unemployed = 0_usize;
    for person in &country.people {
        if !person.is_labor_force_participant() {
            continue;
        }
        labor_force += 1;
        if !person.is_employed() {
            unemployed += 1;
        }
    }
    if labor_force == 0 {
        0.0
    } else {
        unemployed as f64 / labor_force as f64
    }
}

fn run_generated_world_with_acceptance_metrics_in_pool(
    seed: u64,
    country_count: u16,
    population_per_country: u32,
    days: u64,
    threads: usize,
) -> (World, u64, HashMap<String, u64>) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool should build");

    pool.install(|| {
        let countries = generate_countries(&GenerationConfig::with_defaults(
            country_count,
            population_per_country,
            seed,
        ));
        let mut world = World::new_with_generation_seed(countries, seed);
        let mut companies_founded_total = 0_u64;
        let mut growth_ipos_by_country: HashMap<String, u64> = HashMap::new();

        for _ in 0..days {
            world.tick_day();
            for event in world.event_log.drain(..) {
                match event {
                    SimEvent::CompanyFounded { .. } => {
                        companies_founded_total = companies_founded_total.saturating_add(1);
                    }
                    SimEvent::StockIpo {
                        country_name,
                        reason: IpoReason::Growth,
                        ..
                    } => {
                        let entry = growth_ipos_by_country.entry(country_name).or_insert(0);
                        *entry = entry.saturating_add(1);
                    }
                    _ => {}
                }
            }
        }

        (world, companies_founded_total, growth_ipos_by_country)
    })
}

#[test]
#[ignore = "long-run stability scenario (730 days, 3 x 15k)"]
fn generated_world_3x15k_730_days_meets_stability_floor() {
    let (world, companies_founded_total, growth_ipos_by_country) =
        run_generated_world_with_acceptance_metrics_in_pool(0x5eed_c0de, 3, 15_000, 730, 8);
    assert!(world.validate_invariants().is_empty());

    let rates = world
        .countries
        .iter()
        .map(labor_force_unemployment)
        .collect::<Vec<_>>();
    let avg = rates.iter().sum::<f64>() / rates.len() as f64;
    assert!(
        avg <= 0.15,
        "expected avg labor-force unemployment <= 15%, got {avg:.4}"
    );
    assert!(
        companies_founded_total > 0,
        "expected companies_founded_total > 0 over 730 days"
    );
    for country in &world.countries {
        let growth_ipos = growth_ipos_by_country
            .get(&country.name)
            .copied()
            .unwrap_or(0);
        assert!(
            growth_ipos > 0,
            "expected at least one growth IPO for country {}",
            country.name
        );
    }
}

#[test]
#[ignore = "long-run stability scenario (1095 days, 3 x 15k)"]
fn generated_world_3x15k_1095_days_keeps_country_unemployment_capped() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 3, 15_000, 1095, 8);
    assert!(world.validate_invariants().is_empty());

    for country in &world.countries {
        let rate = labor_force_unemployment(country);
        assert!(
            rate <= 0.20,
            "country {} unemployment exceeded 20%: {rate:.4}",
            country.id
        );
    }
}
