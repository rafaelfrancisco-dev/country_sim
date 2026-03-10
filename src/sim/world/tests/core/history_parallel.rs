use super::*;
use crate::sim::{company::build_default_companies, params::SimulationParams};

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
    assert_eq!(world.bank_history.len(), 1);
    assert_eq!(world.bank_history.get(&(0, 0)).map_or(0, |v| v.len()), 1);
    assert_eq!(world.world_history[0].month, 0);
}

#[test]
fn world_tick_advances_days_and_processes_monthly_boundaries() {
    let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

    for _ in 0..DAYS_PER_MONTH {
        world.tick_day();
    }
    assert_eq!(world.world_history.len(), 2);
    assert_eq!(world.bank_history.get(&(0, 0)).map_or(0, |v| v.len()), 2);
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
    assert_eq!(
        world.bank_history.get(&(0, 0)).map_or(0, |v| v.len()),
        HISTORY_LIMIT
    );
}

#[test]
fn bank_history_records_monthly_bank_metrics() {
    let mut country = Country::new(1, "A".to_string(), build_people(2));
    country.primary_bank_mut().reserve_balance = 25_000.0;
    country.primary_bank_mut().deposit_liabilities = 40_000.0;
    country.primary_bank_mut().created_money_outstanding = 18_000.0;
    country.primary_bank_mut().run_intensity = 0.35;
    let world = World::new(vec![country]);

    let history = world
        .bank_history
        .get(&(0, 0))
        .expect("bank history for primary bank should be present");
    assert_eq!(history.len(), 1);
    let point = history[0];
    assert_eq!(point.month, 0);
    assert!((point.reserve - 25_000.0).abs() < f64::EPSILON);
    assert!((point.deposits - 40_000.0).abs() < f64::EPSILON);
    assert!((point.created_money - 18_000.0).abs() < f64::EPSILON);
    assert!((point.equity - 3_000.0).abs() < f64::EPSILON);
    assert!((point.run_intensity - 0.35).abs() < f64::EPSILON);
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
            fx_rate_local_per_base: 0.0,
            fx_reserves_base: 0.0,
            fx_net_flow_base: 0.0,
            fx_pressure: 0.0,
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
            monthly_money_minted: 0.0,
            cumulative_money_minted: 0.0,
            monthly_mint_fraction: 0.0,
            monthly_goods_import_spending: 0.0,
            monthly_goods_export_revenue: 0.0,
            monthly_unmet_need_events: 0.0,
            monthly_severe_unmet_need_events: 0.0,
            monthly_chronic_unmet_need_count: 0.0,
            monthly_essential_debt_spending: 0.0,
            cpi_index: 0.0,
            inflation_rate: 0.0,
            policy_rate: 0.0,
            deposit_rate: 0.0,
            loan_rate: 0.0,
            monthly_savings_interest_paid: 0.0,
            bonds_outstanding: 0.0,
            stock_market_cap: 0.0,
            stock_bank_equity_exposure: 0.0,
            housing_price_index: 0.0,
            monthly_rent: 0.0,
            homeownership_rate: 0.0,
            housing_vacancy_rate: 0.0,
            monthly_foreclosures: 0.0,
            monthly_home_purchases: 0.0,
            monthly_immigrants: 0.0,
            monthly_emigrants: 0.0,
            monthly_migrants_for_work: 0.0,
            monthly_migrants_for_distress: 0.0,
        },
        |acc, point| MetricPoint {
            month: acc.month.max(point.month),
            fx_rate_local_per_base: acc.fx_rate_local_per_base + point.fx_rate_local_per_base,
            fx_reserves_base: acc.fx_reserves_base + point.fx_reserves_base,
            fx_net_flow_base: acc.fx_net_flow_base + point.fx_net_flow_base,
            fx_pressure: acc.fx_pressure + point.fx_pressure,
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
            monthly_money_minted: acc.monthly_money_minted + point.monthly_money_minted,
            cumulative_money_minted: acc.cumulative_money_minted + point.cumulative_money_minted,
            monthly_mint_fraction: acc.monthly_mint_fraction + point.monthly_mint_fraction,
            monthly_goods_import_spending: acc.monthly_goods_import_spending
                + point.monthly_goods_import_spending,
            monthly_goods_export_revenue: acc.monthly_goods_export_revenue
                + point.monthly_goods_export_revenue,
            monthly_unmet_need_events: acc.monthly_unmet_need_events
                + point.monthly_unmet_need_events,
            monthly_severe_unmet_need_events: acc.monthly_severe_unmet_need_events
                + point.monthly_severe_unmet_need_events,
            monthly_chronic_unmet_need_count: acc.monthly_chronic_unmet_need_count
                + point.monthly_chronic_unmet_need_count,
            monthly_essential_debt_spending: acc.monthly_essential_debt_spending
                + point.monthly_essential_debt_spending,
            cpi_index: acc.cpi_index + point.cpi_index,
            inflation_rate: acc.inflation_rate + point.inflation_rate,
            policy_rate: acc.policy_rate + point.policy_rate,
            deposit_rate: acc.deposit_rate + point.deposit_rate,
            loan_rate: acc.loan_rate + point.loan_rate,
            monthly_savings_interest_paid: acc.monthly_savings_interest_paid
                + point.monthly_savings_interest_paid,
            bonds_outstanding: acc.bonds_outstanding + point.bonds_outstanding,
            stock_market_cap: acc.stock_market_cap + point.stock_market_cap,
            stock_bank_equity_exposure: acc.stock_bank_equity_exposure
                + point.stock_bank_equity_exposure,
            housing_price_index: acc.housing_price_index + point.housing_price_index,
            monthly_rent: acc.monthly_rent + point.monthly_rent,
            homeownership_rate: acc.homeownership_rate + point.homeownership_rate,
            housing_vacancy_rate: acc.housing_vacancy_rate + point.housing_vacancy_rate,
            monthly_foreclosures: acc.monthly_foreclosures + point.monthly_foreclosures,
            monthly_home_purchases: acc.monthly_home_purchases + point.monthly_home_purchases,
            monthly_immigrants: acc.monthly_immigrants + point.monthly_immigrants,
            monthly_emigrants: acc.monthly_emigrants + point.monthly_emigrants,
            monthly_migrants_for_work: acc.monthly_migrants_for_work
                + point.monthly_migrants_for_work,
            monthly_migrants_for_distress: acc.monthly_migrants_for_distress
                + point.monthly_migrants_for_distress,
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
        (aggregate.monthly_money_minted - country_sum.monthly_money_minted).abs() < f64::EPSILON
    );
    assert!(
        (aggregate.cumulative_money_minted - country_sum.cumulative_money_minted).abs()
            < f64::EPSILON
    );
    assert!(
        (aggregate.monthly_tax_collected - country_sum.monthly_tax_collected).abs() < f64::EPSILON
    );
    assert!((aggregate.stock_market_cap - country_sum.stock_market_cap).abs() < f64::EPSILON);
}

#[test]
fn aggregate_history_uses_population_weighted_rates() {
    let mut country_a = Country::new(1, "A".to_string(), build_people(2));
    country_a.policy_rate = 0.03;
    country_a.sim_params.monetary.deposit_spread = 0.01;
    country_a.sim_params.monetary.credit_spread = 0.04;

    let mut country_b = Country::new(2, "B".to_string(), build_people(5));
    country_b.policy_rate = 0.06;
    country_b.sim_params.monetary.deposit_spread = 0.015;
    country_b.sim_params.monetary.credit_spread = 0.02;

    let expected_deposit_rate = (country_a.deposit_rate() * country_a.population() as f64
        + country_b.deposit_rate() * country_b.population() as f64)
        / (country_a.population() + country_b.population()) as f64;
    let expected_loan_rate = (country_a.bank_loan_rate() * country_a.population() as f64
        + country_b.bank_loan_rate() * country_b.population() as f64)
        / (country_a.population() + country_b.population()) as f64;
    let expected_policy_rate = (country_a.policy_rate * country_a.population() as f64
        + country_b.policy_rate * country_b.population() as f64)
        / (country_a.population() + country_b.population()) as f64;

    let world = World::new(vec![country_a, country_b]);
    let aggregate = world.world_history[0];

    assert!((aggregate.policy_rate - expected_policy_rate).abs() < 1e-12);
    assert!((aggregate.deposit_rate - expected_deposit_rate).abs() < 1e-12);
    assert!((aggregate.loan_rate - expected_loan_rate).abs() < 1e-12);
}

#[test]
fn monthly_policy_update_keeps_country_rates_independent() {
    let mut low_inflation_params = SimulationParams::default();
    low_inflation_params.monetary.policy_rate_step = 0.005;
    low_inflation_params.monetary.rate_change_event_threshold = 0.001;

    let mut high_inflation_params = SimulationParams::default();
    high_inflation_params.monetary.policy_rate_step = 0.005;
    high_inflation_params.monetary.rate_change_event_threshold = 0.001;

    let mut country_a = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        build_default_companies("A", 1),
        0.0,
        low_inflation_params,
    );
    country_a.monthly_inflation_rate = 0.00;

    let mut country_b = Country::with_economy_and_params(
        2,
        "B".to_string(),
        vec![],
        build_default_companies("B", 1),
        0.0,
        high_inflation_params,
    );
    country_b.monthly_inflation_rate = 0.10;

    let mut world = World::new(vec![country_a, country_b]);
    world.update_country_policy_rates();

    let country_a_rate = world.countries[0].policy_rate();
    let country_b_rate = world.countries[1].policy_rate();
    let expected_world_rate = world.population_weighted_policy_rate();

    assert_ne!(country_a_rate, country_b_rate);
    assert!((world.policy_rate() - expected_world_rate).abs() < 1e-12);
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
