use super::*;

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
            created_money_outstanding: 0.0,
            bank_reserve: 0.0,
            treasury_balance: 0.0,
            budget_balance: 0.0,
            cpi_index: 0.0,
            inflation_rate: 0.0,
            policy_rate: 0.0,
            bonds_outstanding: 0.0,
        },
        |acc, point| MetricPoint {
            month: acc.month.max(point.month),
            population: acc.population + point.population,
            total_money_supply: acc.total_money_supply + point.total_money_supply,
            created_money_outstanding: acc.created_money_outstanding
                + point.created_money_outstanding,
            bank_reserve: acc.bank_reserve + point.bank_reserve,
            treasury_balance: acc.treasury_balance + point.treasury_balance,
            budget_balance: acc.budget_balance + point.budget_balance,
            cpi_index: acc.cpi_index + point.cpi_index,
            inflation_rate: acc.inflation_rate + point.inflation_rate,
            policy_rate: acc.policy_rate + point.policy_rate,
            bonds_outstanding: acc.bonds_outstanding + point.bonds_outstanding,
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
