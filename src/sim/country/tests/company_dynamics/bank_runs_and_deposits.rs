use super::*;
use crate::sim::bank::BankKind;

#[test]
fn private_bank_can_be_formed_from_founder_capital() {
    let mut country = build_deterministic_country();
    for person in &mut country.people {
        person.cash_balance = 250_000.0;
    }
    country.sim_params.banking.min_private_bank_founders = 4;
    country.sim_params.banking.min_private_bank_seed_capital = 200_000.0;
    country.sim_params.banking.max_new_banks_per_month = 1;
    country
        .sim_params
        .banking
        .min_private_seed_to_deposits_ratio = 0.005;

    country.process_bank_formation();

    let private_count = std::iter::once(&country.bank)
        .chain(country.banks.iter())
        .filter(|bank| bank.kind == BankKind::Private)
        .count();
    assert!(private_count >= 1);
    assert!(country.pending_events.iter().any(|event| matches!(
        event,
        SimEvent::BankFounded {
            kind: BankKind::Private,
            ..
        }
    )));
}

#[test]
fn bank_run_moves_depositors_and_applies_losses() {
    let mut country = build_deterministic_country();
    for person in &mut country.people {
        person.cash_balance = 80_000.0;
    }
    for company in &mut country.companies {
        company.cash_balance = 300_000.0;
    }

    let weak_bank_id = country
        .create_bank(BankKind::Private, 5_000.0)
        .expect("private bank should be created");
    for person in &mut country.people {
        person.deposit_bank_id = Some(weak_bank_id);
    }
    for company in &mut country.companies {
        company.deposit_bank_id = Some(weak_bank_id);
    }
    country.treasury_bank_id = Some(weak_bank_id);
    country.treasury_balance = 500_000.0;

    country.sim_params.banking.run_health_threshold = 0.95;
    country.sim_params.banking.run_withdraw_fraction = 0.90;
    country.sim_params.banking.run_switch_fraction = 1.0;
    country.sim_params.banking.deposit_insurance_cap = 20_000.0;
    country.sim_params.banking.uninsured_haircut_rate = 0.50;
    country.recompute_bank_deposit_liabilities();
    country.recompute_aggregates();
    let cash_before = round_cents(
        country.household_cash_total + country.company_cash_total + country.treasury_balance,
    );

    country.process_bank_runs();
    country.recompute_aggregates();

    let cash_after = round_cents(
        country.household_cash_total + country.company_cash_total + country.treasury_balance,
    );
    assert!(cash_after < cash_before);
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::BankRunStarted { bank_id, .. } if *bank_id == weak_bank_id))
    );
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::BankDepositHaircut { .. }))
    );
    assert!(
        country
            .people
            .iter()
            .any(|person| person.deposit_bank_id != Some(weak_bank_id))
    );
}

#[test]
fn ten_year_monthly_banking_simulation_keeps_invariants() {
    let mut country = build_deterministic_country();
    country.sim_params.banking.max_new_banks_per_month = 1;
    country.sim_params.banking.min_private_bank_founders = 3;
    country.sim_params.banking.min_private_bank_seed_capital = 100_000.0;
    country.sim_params.banking.run_health_threshold = 0.70;
    country.sim_params.banking.run_withdraw_fraction = 0.35;
    country.sim_params.banking.uninsured_haircut_rate = 0.25;

    for _ in 0..120 {
        country.process_month();
        let violations = country.validate_money_invariants();
        assert!(
            violations.is_empty(),
            "money invariant violation after month {}: {violations:#?}",
            country.month
        );
    }

    assert!(country.total_bank_reserves().is_finite());
    assert!(country.total_bank_created_money().is_finite());
    assert!(
        std::iter::once(&country.bank)
            .chain(country.banks.iter())
            .count()
            >= 1
    );
}

#[test]
fn moving_deposit_bank_transfers_reserve_assets() {
    let mut country = build_deterministic_country();
    let private_bank_id = country
        .create_bank(BankKind::Private, 1_000.0)
        .expect("private bank should be created");
    country.bank.reserve_balance = 50_000.0;
    country.people[0].cash_balance = 10_000.0;
    country.people[0].deposit_bank_id = Some(country.bank.id);
    let public_before = country.bank.reserve_balance;
    let private_before = country
        .bank_by_id(private_bank_id)
        .map(|bank| bank.reserve_balance)
        .unwrap_or(0.0);

    country.move_person_deposit_bank(0, private_bank_id);

    let public_after = country.bank.reserve_balance;
    let private_after = country
        .bank_by_id(private_bank_id)
        .map(|bank| bank.reserve_balance)
        .unwrap_or(0.0);
    assert!(public_after < public_before);
    assert!(private_after > private_before);
}

#[test]
fn public_bank_health_is_not_pinned_near_zero() {
    let country = build_deterministic_country();
    assert!(country.bank_health_score(country.bank.id) > 0.20);
}
