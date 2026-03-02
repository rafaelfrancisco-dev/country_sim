use super::*;

#[test]
fn unemployment_benefit_increases_cash_and_decreases_treasury() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    let treasury_start = 10_000.0;
    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - benefit).abs() < 0.01,
        "cash_balance should equal benefit; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.treasury_balance - (treasury_start - benefit)).abs() < 0.01,
        "treasury should decrease by benefit; got {}",
        country.treasury_balance
    );
}

#[test]
fn employed_person_does_not_receive_unemployment_benefit() {
    // An employed adult with healthy cash and no unmet needs should trigger
    // zero government spending — they are not eligible for any benefit path.
    let treasury_start = 10_000.0;
    let person = employed_adult(1, 5_000.0);
    let mut country = make_gov_country(vec![person], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - 0.0).abs() < 0.01,
        "no gov spending should occur for employed person; got {}",
        country.monthly_gov_spending
    );
    let has_gov_event = country
        .pending_events
        .iter()
        .any(|e| matches!(e, SimEvent::GovTransfer { .. }));
    assert!(
        !has_gov_event,
        "no GovTransfer event should be emitted for employed person"
    );
}

#[test]
fn child_below_working_age_does_not_receive_unemployment_benefit() {
    let treasury_start = 10_000.0;
    let mut child = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        101,
        10 * DAYS_PER_YEAR, // age 10 — below 18
        EducationTrack::BasicOnly,
    );
    child.cash_balance = 0.0;
    let mut country = make_gov_country(vec![child], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 0.0).abs() < 0.01,
        "child should receive no benefit; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.treasury_balance - treasury_start).abs() < 0.01,
        "treasury should be unchanged; got {}",
        country.treasury_balance
    );
}

#[test]
fn senior_above_working_age_does_not_receive_unemployment_benefit() {
    let treasury_start = 10_000.0;
    let mut senior = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        101,
        70 * DAYS_PER_YEAR, // age 70 — above 65
        EducationTrack::BasicOnly,
    );
    senior.cash_balance = 0.0;
    let mut country = make_gov_country(vec![senior], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 0.0).abs() < 0.01,
        "senior should receive no benefit; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.treasury_balance - treasury_start).abs() < 0.01,
        "treasury should be unchanged; got {}",
        country.treasury_balance
    );
}

#[test]
fn monthly_gov_spending_and_budget_balance_are_tracked() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], 10_000.0, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - benefit).abs() < 0.01,
        "monthly_gov_spending should equal benefit; got {}",
        country.monthly_gov_spending
    );
    // No taxes collected (unemployed earns nothing), so budget_balance = 0 - benefit
    assert!(
        (country.monthly_budget_balance - (country.monthly_tax_collected - benefit)).abs() < 0.01,
        "budget_balance should be tax - spending; got {}",
        country.monthly_budget_balance
    );
}

#[test]
fn gov_transfer_event_is_emitted_when_benefits_are_paid() {
    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], 10_000.0, FiscalParams::default());

    country.process_month_end();

    let has_gov_event = country.pending_events.iter().any(
        |e| matches!(e, SimEvent::GovTransfer { total_recipients, .. } if *total_recipients > 0),
    );
    assert!(has_gov_event, "GovTransfer event should be emitted");
}

#[test]
fn no_gov_transfer_event_when_nobody_qualifies() {
    // All employed adults, full cash — no benefits should be distributed
    let person = employed_adult(1, 5_000.0);
    let mut country = make_gov_country(vec![person], 10_000.0, FiscalParams::default());

    country.process_month_end();

    let has_gov_event = country
        .pending_events
        .iter()
        .any(|e| matches!(e, SimEvent::GovTransfer { .. }));
    assert!(
        !has_gov_event,
        "no GovTransfer event should be emitted when nobody qualifies"
    );
}

#[test]
fn basic_needs_top_up_raises_cash_to_threshold_for_person_with_unmet_essential_needs() {
    let threshold = 500.0;
    let fiscal = FiscalParams {
        unemployment_benefit_rate: 0.0, // disable unemployment benefit path
        median_income_reference: 2_000.0,
        basic_needs_cash_threshold: threshold,
    };

    // Employed adult (no unemployment benefit), low cash, has unmet water need (bit 0)
    let mut person = employed_adult(1, 200.0);
    person.last_month_unmet_needs_mask = 1; // good id 0 = water (essential)

    let mut country = make_gov_country(vec![person], 10_000.0, fiscal);

    country.process_month_end();

    // Cash should be at least threshold (could be higher due to net pay)
    assert!(
        country.people[0].cash_balance >= threshold - 0.01,
        "cash should be topped up to at least threshold; got {}",
        country.people[0].cash_balance
    );
    assert!(
        country.monthly_gov_spending > 0.0,
        "gov spending should be nonzero after top-up"
    );
}

#[test]
fn no_benefit_paid_when_treasury_is_empty() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    assert!(
        benefit > 0.0,
        "benefit must be positive for this test to be meaningful"
    );

    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], 0.0, FiscalParams::default());
    // Explicitly zero out treasury
    country.treasury_balance = 0.0;

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 0.0).abs() < 0.01,
        "no benefit should be paid from empty treasury; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.monthly_gov_spending - 0.0).abs() < 0.01,
        "monthly_gov_spending should be zero; got {}",
        country.monthly_gov_spending
    );
}

#[test]
fn government_spending_does_not_violate_money_supply_invariant() {
    let treasury_start = 50_000.0;
    // Mix of unemployed adults and one employed to generate some tax
    let people = vec![
        unemployed_adult(1, 0.0),
        unemployed_adult(2, 0.0),
        employed_adult(3, 0.0),
    ];
    let mut country = make_gov_country(people, treasury_start, FiscalParams::default());

    country.process_month_end();
    country.recompute_aggregates();

    let violations = country.validate_money_invariants();
    assert!(
        violations.is_empty(),
        "money invariant violated after gov spending: {:?}",
        violations
    );
}

#[test]
fn monthly_job_switch_moves_worker_only_for_raise_and_backfills_old_opening() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();
    companies[0].cash_balance = 100_000.0;
    companies[1].cash_balance = 100_000.0;
    companies[1].add_opening_with_salary(0, 1, 4_000.0);

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.set_employment_with_salary(0, 0, 3_200.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.labor_mobility = LaborMobilityParams::default();
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.people[0].employer_company_id(), Some(1));
    assert!((country.people[0].monthly_salary() - 4_000.0).abs() < 0.01);
    assert!(
        country.companies[0]
            .open_positions
            .iter()
            .any(|opening| opening.job_type_id == 0
                && opening.slots > 0
                && (opening.offered_monthly_salary - 3_200.0).abs() < 0.01)
    );
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::JobSwitched { .. }))
    );
}

#[test]
fn first_missed_company_payment_applies_wage_cut_to_contracts_and_openings() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening_with_salary(0, 1, 5_000.0);
    companies[0].cash_balance = 0.0;
    companies[0].loan = Some(LoanAccount::new(5_000.0, 0.12, 12));

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.set_employment_with_salary(0, 0, 4_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies[0].consecutive_missed_loan_payments, 1);
    assert!((country.people[0].monthly_salary() - 3_600.0).abs() < 0.01);
    assert!(
        country.companies[0]
            .open_positions
            .iter()
            .any(|opening| (opening.offered_monthly_salary - 4_500.0).abs() < 0.01)
    );
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyWageCut { .. }))
    );
}


