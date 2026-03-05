use super::*;
use crate::sim::{
    bank::BankKind,
    event::{IpoReason, SimEvent},
    generator::{GenerationConfig, generate_countries},
    money::round_cents,
    person::ForeignDeposit,
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
    assert_eq!(world.bank_history.len(), 1);
    assert_eq!(world.bank_history.get(&(0, 0)).map_or(0, Vec::len), 1);
    assert_eq!(world.world_history[0].month, 0);
}

#[test]
fn world_tick_advances_days_and_processes_monthly_boundaries() {
    let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

    for _ in 0..DAYS_PER_MONTH {
        world.tick_day();
    }
    assert_eq!(world.world_history.len(), 2);
    assert_eq!(world.bank_history.get(&(0, 0)).map_or(0, Vec::len), 2);
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
        world.bank_history.get(&(0, 0)).map_or(0, Vec::len),
        HISTORY_LIMIT
    );
}

#[test]
fn bank_history_records_monthly_bank_metrics() {
    let mut country = Country::new(1, "A".to_string(), build_people(2));
    country.bank.reserve_balance = 25_000.0;
    country.bank.deposit_liabilities = 40_000.0;
    country.bank.created_money_outstanding = 18_000.0;
    country.bank.run_intensity = 0.35;
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
            monthly_money_minted: acc.monthly_money_minted + point.monthly_money_minted,
            cumulative_money_minted: acc.cumulative_money_minted + point.cumulative_money_minted,
            monthly_mint_fraction: acc.monthly_mint_fraction + point.monthly_mint_fraction,
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
            deposit_rate: acc.deposit_rate + point.deposit_rate,
            loan_rate: acc.loan_rate + point.loan_rate,
            monthly_savings_interest_paid: acc.monthly_savings_interest_paid
                + point.monthly_savings_interest_paid,
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

    let world = World::new(vec![country_a, country_b]);
    let aggregate = world.world_history[0];

    assert!((aggregate.deposit_rate - expected_deposit_rate).abs() < 1e-12);
    assert!((aggregate.loan_rate - expected_loan_rate).abs() < 1e-12);
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
fn cross_country_deposit_open_targets_best_foreign_private_bank() {
    let mut home = Country::new(1, "Home".to_string(), build_people(1));
    let mut foreign = Country::new(2, "Foreign".to_string(), build_people(1));

    home.people[0].cash_balance = 20_000.0;
    home.policy_rate = 0.02;
    home.sim_params.monetary.deposit_spread = 0.0;
    home.sim_params.banking.cross_country_deposit_min_balance = 10_000.0;
    home.sim_params.banking.cross_country_deposit_fraction = 0.50;
    home.sim_params.banking.cross_country_deposit_rate_premium = 0.005;

    foreign.policy_rate = 0.02;
    foreign.sim_params.monetary.deposit_spread = 0.01;
    foreign.sim_params.banking.private_bank_deposit_rate_bonus = 0.03;
    let foreign_private_bank_id = foreign
        .create_bank(BankKind::Private, 1_000.0)
        .expect("private foreign bank should be created");
    foreign.bank.reserve_balance = 2_500.0;

    let public_reserve_before = foreign.bank.reserve_balance;
    let private_reserve_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .reserve_balance;

    let mut world = World::new(vec![home, foreign]);
    world.process_cross_country_deposits();

    let home = &world.countries[0];
    let foreign = &world.countries[1];
    let expected_amount = 5_000.0;

    assert_eq!(
        home.people[0].foreign_deposit,
        Some(ForeignDeposit {
            country_id: foreign.id,
            bank_id: foreign_private_bank_id,
            balance: expected_amount,
        })
    );
    assert_eq!(home.people[0].cash_balance, 15_000.0);
    assert_eq!(foreign.bank.reserve_balance, public_reserve_before);
    assert_eq!(foreign.bank.foreign_deposit_liabilities, 0.0);
    assert_eq!(
        foreign
            .bank_by_id(foreign_private_bank_id)
            .expect("private foreign bank should exist")
            .reserve_balance,
        private_reserve_before + expected_amount
    );
    assert_eq!(
        foreign
            .bank_by_id(foreign_private_bank_id)
            .expect("private foreign bank should exist")
            .foreign_deposit_liabilities,
        expected_amount
    );
    assert!(home.pending_events.iter().any(|event| matches!(
        event,
        SimEvent::ForeignDepositOpened {
            bank_id,
            amount,
            ..
        } if *bank_id == foreign_private_bank_id && (*amount - expected_amount).abs() < 0.01
    )));
}

#[test]
fn cross_country_deposit_interest_uses_stored_foreign_bank_id() {
    let mut home = Country::new(1, "Home".to_string(), build_people(1));
    let mut foreign = Country::new(2, "Foreign".to_string(), build_people(1));
    let foreign_private_bank_id = foreign
        .create_bank(BankKind::Private, 1_000.0)
        .expect("private foreign bank should be created");

    home.people[0].cash_balance = 1_000.0;
    home.people[0].foreign_deposit = Some(ForeignDeposit {
        country_id: foreign.id,
        bank_id: foreign_private_bank_id,
        balance: 12_000.0,
    });
    foreign.policy_rate = 0.03;
    foreign.sim_params.monetary.deposit_spread = 0.01;
    foreign.sim_params.banking.private_bank_deposit_rate_bonus = 0.02;
    foreign.bank.reserve_balance = 9_000.0;
    foreign.bank.foreign_deposit_liabilities = 400.0;
    {
        let private_bank = foreign
            .bank_by_id_mut(foreign_private_bank_id)
            .expect("private foreign bank should exist");
        private_bank.reserve_balance = 20_000.0;
        private_bank.foreign_deposit_liabilities = 12_000.0;
    }

    let expected_interest = round_cents(
        12_000.0
            * ((1.0_f64 + foreign.deposit_rate_for(foreign_private_bank_id)).powf(1.0 / 12.0)
                - 1.0),
    );

    let public_reserve_before = foreign.bank.reserve_balance;
    let public_liabilities_before = foreign.bank.foreign_deposit_liabilities;
    let private_reserve_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .reserve_balance;
    let private_liabilities_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .foreign_deposit_liabilities;

    let mut world = World::new(vec![home, foreign]);
    world.process_cross_country_deposits();

    let home = &world.countries[0];
    let foreign = &world.countries[1];
    assert_eq!(home.people[0].cash_balance, 1_000.0 + expected_interest);
    assert_eq!(
        home.people[0]
            .foreign_deposit
            .as_ref()
            .expect("foreign deposit should remain open")
            .balance,
        12_000.0
    );
    assert_eq!(foreign.bank.reserve_balance, public_reserve_before);
    assert_eq!(
        foreign.bank.foreign_deposit_liabilities,
        public_liabilities_before
    );
    assert_eq!(
        foreign
            .bank_by_id(foreign_private_bank_id)
            .expect("private foreign bank should exist")
            .reserve_balance,
        private_reserve_before - expected_interest
    );
    assert_eq!(
        foreign
            .bank_by_id(foreign_private_bank_id)
            .expect("private foreign bank should exist")
            .foreign_deposit_liabilities,
        private_liabilities_before - expected_interest
    );
}

#[test]
fn cross_country_deposit_recall_uses_stored_foreign_bank_id() {
    let mut home = Country::new(1, "Home".to_string(), build_people(1));
    let mut foreign = Country::new(2, "Foreign".to_string(), build_people(1));
    let foreign_private_bank_id = foreign
        .create_bank(BankKind::Private, 1_000.0)
        .expect("private foreign bank should be created");

    home.people[0].cash_balance = 100.0;
    home.sim_params
        .banking
        .cross_country_deposit_distress_threshold = 500.0;
    home.people[0].foreign_deposit = Some(ForeignDeposit {
        country_id: foreign.id,
        bank_id: foreign_private_bank_id,
        balance: 8_000.0,
    });
    foreign.policy_rate = 0.0;
    foreign.sim_params.monetary.deposit_spread = 0.0;
    foreign.sim_params.banking.private_bank_deposit_rate_bonus = 0.0;
    foreign.bank.reserve_balance = 7_500.0;
    foreign.bank.foreign_deposit_liabilities = 250.0;
    {
        let private_bank = foreign
            .bank_by_id_mut(foreign_private_bank_id)
            .expect("private foreign bank should exist");
        private_bank.reserve_balance = 14_000.0;
        private_bank.foreign_deposit_liabilities = 8_000.0;
    }

    let public_reserve_before = foreign.bank.reserve_balance;
    let public_liabilities_before = foreign.bank.foreign_deposit_liabilities;
    let private_reserve_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .reserve_balance;
    let private_liabilities_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .foreign_deposit_liabilities;

    let mut world = World::new(vec![home, foreign]);
    world.process_cross_country_deposits();

    let home = &world.countries[0];
    let foreign = &world.countries[1];
    assert_eq!(home.people[0].cash_balance, 8_100.0);
    assert!(home.people[0].foreign_deposit.is_none());
    assert_eq!(foreign.bank.reserve_balance, public_reserve_before);
    assert_eq!(
        foreign.bank.foreign_deposit_liabilities,
        public_liabilities_before
    );
    assert_eq!(
        foreign
            .bank_by_id(foreign_private_bank_id)
            .expect("private foreign bank should exist")
            .reserve_balance,
        private_reserve_before - 8_000.0
    );
    assert_eq!(
        foreign
            .bank_by_id(foreign_private_bank_id)
            .expect("private foreign bank should exist")
            .foreign_deposit_liabilities,
        private_liabilities_before - 8_000.0
    );
    assert!(home.pending_events.iter().any(|event| matches!(
        event,
        SimEvent::ForeignDepositClosed {
            returned_amount, ..
        } if (*returned_amount - 8_000.0).abs() < 0.01
    )));
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
