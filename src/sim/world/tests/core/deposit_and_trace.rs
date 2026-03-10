use super::*;

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
    foreign.primary_bank_mut().reserve_balance = 2_500.0;

    let public_reserve_before = foreign.primary_bank().reserve_balance;
    let private_reserve_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .reserve_balance;

    let mut world = World::new(vec![home, foreign]);
    let expected_settlement = world.convert_between_countries(0, 1, 5_000.0);
    world.process_cross_country_deposits();

    let home = &world.countries[0];
    let foreign = &world.countries[1];
    let expected_amount = expected_settlement.payee_amount_local;

    assert_eq!(
        home.people[0].foreign_deposit,
        Some(ForeignDeposit {
            country_id: foreign.id,
            bank_id: foreign_private_bank_id,
            balance: expected_amount,
            home_amount_at_open: 5_000.0,
            opening_fx_rate_home_per_foreign: 5_000.0 / expected_amount,
        })
    );
    assert_eq!(home.people[0].cash_balance, 15_000.0);
    assert_eq!(
        foreign.public_bank().unwrap().reserve_balance,
        public_reserve_before
    );
    assert_eq!(
        foreign.public_bank().unwrap().foreign_deposit_liabilities,
        0.0
    );
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
            home_amount,
            ..
        } if *bank_id == foreign_private_bank_id && (*home_amount - 5_000.0).abs() < 0.01
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
        home_amount_at_open: 12_000.0,
        opening_fx_rate_home_per_foreign: 1.0,
    });
    foreign.policy_rate = 0.03;
    foreign.sim_params.monetary.deposit_spread = 0.01;
    foreign.sim_params.banking.private_bank_deposit_rate_bonus = 0.02;
    foreign.primary_bank_mut().reserve_balance = 9_000.0;
    foreign.primary_bank_mut().foreign_deposit_liabilities = 400.0;
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

    let public_reserve_before = foreign.primary_bank().reserve_balance;
    let public_liabilities_before = foreign.primary_bank().foreign_deposit_liabilities;
    let private_reserve_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .reserve_balance;
    let private_liabilities_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .foreign_deposit_liabilities;

    let mut world = World::new(vec![home, foreign]);
    let expected_home_interest = world
        .convert_between_countries(1, 0, expected_interest)
        .payee_amount_local;
    world.process_cross_country_deposits();

    let home = &world.countries[0];
    let foreign = &world.countries[1];
    assert_eq!(home.people[0].cash_balance, 1_000.0 + expected_home_interest);
    assert_eq!(
        home.people[0]
            .foreign_deposit
            .as_ref()
            .expect("foreign deposit should remain open")
            .balance,
        12_000.0
    );
    assert_eq!(
        foreign.public_bank().unwrap().reserve_balance,
        public_reserve_before
    );
    assert_eq!(
        foreign.public_bank().unwrap().foreign_deposit_liabilities,
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
        home_amount_at_open: 8_000.0,
        opening_fx_rate_home_per_foreign: 1.0,
    });
    foreign.policy_rate = 0.0;
    foreign.sim_params.monetary.deposit_spread = 0.0;
    foreign.sim_params.banking.private_bank_deposit_rate_bonus = 0.0;
    foreign.primary_bank_mut().reserve_balance = 7_500.0;
    foreign.primary_bank_mut().foreign_deposit_liabilities = 250.0;
    {
        let private_bank = foreign
            .bank_by_id_mut(foreign_private_bank_id)
            .expect("private foreign bank should exist");
        private_bank.reserve_balance = 14_000.0;
        private_bank.foreign_deposit_liabilities = 8_000.0;
    }

    let public_reserve_before = foreign.primary_bank().reserve_balance;
    let public_liabilities_before = foreign.primary_bank().foreign_deposit_liabilities;
    let private_reserve_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .reserve_balance;
    let private_liabilities_before = foreign
        .bank_by_id(foreign_private_bank_id)
        .expect("private foreign bank should exist")
        .foreign_deposit_liabilities;

    let mut world = World::new(vec![home, foreign]);
    let expected_returned_home = world
        .convert_between_countries(1, 0, 8_000.0)
        .payee_amount_local;
    let expected_interest_home = world
        .convert_between_countries(1, 0, 0.0)
        .payee_amount_local;
    world.process_cross_country_deposits();

    let home = &world.countries[0];
    let foreign = &world.countries[1];
    assert_eq!(home.people[0].cash_balance, 100.0 + expected_interest_home + expected_returned_home);
    assert!(home.people[0].foreign_deposit.is_none());
    assert_eq!(
        foreign.public_bank().unwrap().reserve_balance,
        public_reserve_before
    );
    assert_eq!(
        foreign.public_bank().unwrap().foreign_deposit_liabilities,
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
            returned_foreign_amount, ..
        } if (*returned_foreign_amount - 8_000.0).abs() < 0.01
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

    assert_eq!(world.day(), DAYS_PER_MONTH + 1);
    assert_eq!(world.completed_month_boundaries(), 1);
    assert_eq!(world.world_history().len(), 2);
    assert_eq!(world.country_history()[0].len(), 2);
    assert_eq!(world.countries()[0].month(), 1);
    assert!(world.validate_invariants().is_empty());
}
