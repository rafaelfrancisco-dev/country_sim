use std::collections::VecDeque;

use super::*;
use crate::sim::bank::{
    DEFAULT_LOAN_TERM_MONTHS, DEFAULT_MAX_DEBT_TO_INCOME, max_principal_for_payment,
};

fn make_employed_person(id: u32, company_id: u16, salary: f64, cash_balance: f64) -> Person {
    let mut person = Person::new_with_profile(
        id,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: company_id,
        },
        10_000 + id,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.set_employment_with_salary(company_id, 0, salary, 0);
    person.cash_balance = cash_balance;
    person
}

fn make_unemployed_person(id: u32, cash_balance: f64) -> Person {
    let mut person = Person::new_with_profile(
        id,
        EmploymentStatus::Unemployed,
        20_000 + id,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.cash_balance = cash_balance;
    person
}

#[test]
fn end_of_month_loan_approval_is_higher_when_policy_rate_is_lower() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    let person = make_employed_person(1, 0, 3_000.0, -100_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;

    let mut low_rate_country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![person.clone()],
        companies.clone(),
        0.0,
        params.clone(),
    );
    low_rate_country.policy_rate = 0.01;
    low_rate_country.process_end_of_month_loan_requests();
    let low_issued = low_rate_country.people[0].last_month_new_loan;

    let mut high_rate_country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    high_rate_country.policy_rate = 0.15;
    high_rate_country.process_end_of_month_loan_requests();
    let high_issued = high_rate_country.people[0].last_month_new_loan;

    assert!(low_issued > 0.0);
    assert!(low_issued > high_issued);
}

#[test]
fn company_loan_capacity_math_is_tighter_when_rates_are_higher() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].sales_history = VecDeque::from(vec![10_000.0, 10_000.0, 10_000.0]);
    companies[0].loan = Some(LoanAccount::new(35_000.0, 0.12, 12));

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;

    let mut low_rate_country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        companies.clone(),
        0.0,
        params.clone(),
    );
    low_rate_country.policy_rate = 0.01;

    let mut high_rate_country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    high_rate_country.policy_rate = 0.15;

    let existing_debt = low_rate_country.companies[0]
        .loan
        .as_ref()
        .map_or(0.0, |loan| loan.debt_balance);
    let monthly_income = low_rate_country.companies[0].average_monthly_revenue();

    let low_max_payment =
        monthly_income * low_rate_country.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);
    let high_max_payment =
        monthly_income * high_rate_country.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);

    let low_capacity = max_principal_for_payment(
        low_max_payment,
        low_rate_country.bank_loan_rate(),
        DEFAULT_LOAN_TERM_MONTHS,
    );
    let high_capacity = max_principal_for_payment(
        high_max_payment,
        high_rate_country.bank_loan_rate(),
        DEFAULT_LOAN_TERM_MONTHS,
    );

    assert!(low_capacity > existing_debt);
    assert!(high_capacity < existing_debt);
}

#[test]
fn growth_openings_are_more_conservative_when_rates_are_high() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].consecutive_profitable_months = 3;
    companies[0].monthly_payroll_budget = 5_000.0;
    companies[0].cash_balance = 2_000_000.0;
    companies[0].last_month_sales = 200_000.0;
    companies[0].last_month_profit = 200_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;

    let mut low_rate_country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        companies.clone(),
        0.0,
        params.clone(),
    );
    low_rate_country.policy_rate = 0.01;
    low_rate_country.process_month();
    let low_slots: u16 = low_rate_country.companies[0]
        .open_positions
        .iter()
        .map(|opening| opening.slots)
        .sum();

    let mut high_rate_country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    high_rate_country.policy_rate = 0.15;
    high_rate_country.process_month();
    let high_slots: u16 = high_rate_country.companies[0]
        .open_positions
        .iter()
        .map(|opening| opening.slots)
        .sum();

    assert!(low_slots > 0);
    assert!(low_slots > high_slots);
}

#[test]
fn higher_policy_rates_reduce_price_pressure_and_inflation() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].produced_good_id = 0;
    companies[0].unit_price = 20.0;
    companies[0].inventory_units = 100;
    companies[0].last_month_units_sold = 1_000;
    companies[0].last_month_sales = 20_000.0;

    let person = make_employed_person(10, 0, 3_000.0, 1_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;

    let mut low_rate_country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![person.clone()],
        companies.clone(),
        0.0,
        params.clone(),
    );
    low_rate_country.policy_rate = 0.01;
    low_rate_country.process_month();

    let mut high_rate_country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    high_rate_country.policy_rate = 0.15;
    high_rate_country.process_month();

    assert!(low_rate_country.companies[0].unit_price > high_rate_country.companies[0].unit_price);
    assert!(low_rate_country.monthly_inflation_rate > high_rate_country.monthly_inflation_rate);
}

#[test]
fn auto_money_minting_caps_and_tracks_monthly_and_cumulative_amounts() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;
    params.monetary.mint_inflation_gap_weight = 0.20;
    params.monetary.mint_unemployment_gap_weight = 0.10;
    params.monetary.mint_max_fraction_expected_supply = 0.01;
    params.monetary.deposit_spread = 1.0;
    // Disable bank recapitalization so it doesn't create spurious bank-created money
    // that would inflate expected_money_supply independently of minting.
    params.banking.public_bank_recapitalize_amount = f64::MAX;

    let people = vec![
        make_employed_person(1, 0, 3_000.0, 10_000.0),
        make_unemployed_person(2, 10_000.0),
    ];
    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 100_000.0, params);
    let expected_before = country.expected_money_supply;
    let expected_mint = round_cents(expected_before * 0.01);

    country.process_month_end();

    assert!((country.monthly_money_minted - expected_mint).abs() <= 0.01);
    assert!((country.cumulative_money_minted - expected_mint).abs() <= 0.01);
    assert!((country.expected_money_supply - (expected_before + expected_mint)).abs() <= 0.01);
    assert!(country.validate_money_invariants().is_empty());
    assert!(country.pending_events.iter().any(|event| {
        matches!(event, SimEvent::MoneyMinted { amount, .. } if (*amount - expected_mint).abs() <= 0.01)
    }));
}

#[test]
fn auto_money_minting_skips_when_no_positive_inflation_or_unemployment_gap() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;
    params.monetary.mint_inflation_gap_weight = 0.50;
    params.monetary.mint_unemployment_gap_weight = 0.50;
    params.monetary.mint_max_fraction_expected_supply = 0.05;
    params.monetary.deposit_spread = 1.0;
    // Disable bank recapitalization so it doesn't create spurious bank-created money
    // that would inflate expected_money_supply independently of minting.
    params.banking.public_bank_recapitalize_amount = f64::MAX;

    let people = vec![make_employed_person(1, 0, 3_000.0, 10_000.0)];
    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 100_000.0, params);
    country.monthly_inflation_rate = 0.04;
    let expected_before = country.expected_money_supply;

    country.process_month_end();

    assert_eq!(country.monthly_money_minted, 0.0);
    assert_eq!(country.cumulative_money_minted, 0.0);
    assert!((country.expected_money_supply - expected_before).abs() <= 0.01);
    assert!(
        !country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::MoneyMinted { .. }))
    );
}

#[test]
fn unemployment_minting_is_blocked_when_inflation_exceeds_cap() {
    // Set up a country with high unemployment (would normally trigger minting)
    // but with inflation above target * cap_multiplier (should block unemployment minting).
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;
    params.monetary.mint_inflation_gap_weight = 0.0; // disable inflation-gap minting
    params.monetary.mint_unemployment_gap_weight = 1.0;
    params.monetary.mint_max_fraction_expected_supply = 0.05;
    params.monetary.deposit_spread = 1.0;
    params.monetary.target_inflation_rate = 0.02;
    params.monetary.mint_unemployment_inflation_cap_multiplier = 1.5; // cap at 3%
    // Disable bank recapitalization so it doesn't create spurious bank-created money
    // that would inflate expected_money_supply independently of minting.
    params.banking.public_bank_recapitalize_amount = f64::MAX;

    // 1 employed + 1 unemployed → 50% unemployment → large unemployment gap
    let people = vec![
        make_employed_person(1, 0, 3_000.0, 10_000.0),
        make_unemployed_person(2, 10_000.0),
    ];
    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 100_000.0, params);
    // Pre-set inflation well above the 3% overshoot threshold
    country.monthly_inflation_rate = 0.10; // 10% — far above cap
    let expected_before = country.expected_money_supply;

    country.process_month_end();

    // Unemployment minting should be fully blocked; inflation-gap minting is also 0
    // (since inflation > target). So no minting should occur.
    assert!(
        country.monthly_money_minted < 0.01,
        "no minting should occur when inflation exceeds cap; got {}",
        country.monthly_money_minted
    );
    assert!(
        country.last_mint_unemployment_gap < 0.01,
        "last_mint_unemployment_gap should be zeroed when inflation exceeds cap; got {}",
        country.last_mint_unemployment_gap
    );
    // expected_money_supply should not have grown from minting
    assert!(
        (country.expected_money_supply - expected_before).abs() < 100.0,
        "expected_money_supply should not grow from minting; delta = {}",
        country.expected_money_supply - expected_before
    );
}

#[test]
fn local_taylor_rule_uses_country_macro_state_and_params() {
    let mut low_params = SimulationParams::default();
    low_params.monetary.policy_rate_step = 0.005;
    low_params.monetary.rate_change_event_threshold = 0.001;

    let mut high_params = low_params.clone();
    high_params.monetary.inflation_coefficient = 2.0;

    let mut low_country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        build_default_companies("A", 1),
        0.0,
        low_params,
    );
    low_country.monthly_inflation_rate = 0.00;

    let mut high_country = Country::with_economy_and_params(
        2,
        "B".to_string(),
        vec![],
        build_default_companies("B", 1),
        0.0,
        high_params,
    );
    high_country.monthly_inflation_rate = 0.08;

    low_country.update_policy_rate();
    high_country.update_policy_rate();

    assert!(high_country.policy_rate() > low_country.policy_rate());
}

#[test]
fn local_taylor_rule_applies_step_limit_and_emits_event() {
    let mut params = SimulationParams::default();
    params.monetary.policy_rate_step = 0.005;
    params.monetary.rate_change_event_threshold = 0.001;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        build_default_companies("A", 1),
        0.0,
        params,
    );
    country.policy_rate = 0.04;
    country.monthly_inflation_rate = 0.10;

    country.update_policy_rate();

    assert!((country.policy_rate() - 0.045).abs() < 1e-12);
    assert!(country.pending_events.iter().any(|event| {
        matches!(
            event,
            SimEvent::RatePolicyChange {
                old_rate,
                new_rate,
                ..
            } if (*old_rate - 0.04).abs() < 1e-12 && (*new_rate - 0.045).abs() < 1e-12
        )
    }));
}

#[test]
fn local_taylor_rule_uses_natural_unemployment_without_labor_force() {
    let mut params = SimulationParams::default();
    params.monetary.natural_unemployment_rate = 0.06;

    let country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        build_default_companies("A", 1),
        0.0,
        params,
    );

    assert!((country.policy_unemployment_rate() - 0.06).abs() < f64::EPSILON);
}
