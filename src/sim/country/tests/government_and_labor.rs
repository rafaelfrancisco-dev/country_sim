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
        ..Default::default()
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
fn unemployed_with_unmet_essential_needs_can_receive_benefit_and_top_up_in_same_month() {
    let fiscal = FiscalParams {
        unemployment_benefit_rate: 0.2, // 200/month with reference 1_000
        median_income_reference: 1_000.0,
        basic_needs_cash_threshold: 500.0,
        ..Default::default()
    };
    let mut person = unemployed_adult(1, 100.0);
    person.last_month_unmet_needs_mask = 1; // unmet water need (essential)
    let mut country = make_gov_country(vec![person], 10_000.0, fiscal);

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 500.0).abs() < 0.01,
        "expected stacked benefit+top-up to reach threshold; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.monthly_gov_spending - 400.0).abs() < 0.01,
        "expected 200 benefit + 200 top-up; got {}",
        country.monthly_gov_spending
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
    person.set_employment_with_salary(0, 0, 3_200.0, 0);

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
    person.set_employment_with_salary(0, 0, 4_000.0, 0);

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

#[test]
fn monthly_labor_churn_retirements_are_age_gated_to_65_plus() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 1_000_000.0;

    let younger = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        111,
        64 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    let older = Person::new_with_profile(
        2,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        222,
        66 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;
    params.company_growth.min_profitable_months = u8::MAX;
    params.company_formation.max_startups_per_month = Some(0);
    params.labor_mobility.switch_raise_threshold = 10.0;
    params.labor_churn.quit_rate_monthly = 0.0;
    params.labor_churn.layoff_rate_monthly = 0.0;
    params.labor_churn.retirement_rate_monthly = 1.0;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![younger, older],
        companies,
        0.0,
        params,
    );

    country.process_month_end();

    assert!(country.people[0].is_employed());
    assert!(!country.people[1].is_employed());
    assert!(country.pending_events.iter().any(|event| matches!(
        event,
        SimEvent::LaborChurnMonthly {
            quits: 0,
            retirements: 1,
            layoffs_non_distress: 0,
            replacement_openings: 1,
            ..
        }
    )));
}

#[test]
fn monthly_labor_churn_non_distress_layoffs_create_replacement_openings() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 1_000_000.0;

    let mut people = Vec::new();
    for id in 1..=3_u32 {
        people.push(Person::new_with_profile(
            id,
            EmploymentStatus::Employed {
                job_type_id: 0,
                employer_company_id: 0,
            },
            id + 500,
            35 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        ));
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;
    params.company_growth.min_profitable_months = u8::MAX;
    params.company_formation.max_startups_per_month = Some(0);
    params.labor_mobility.switch_raise_threshold = 10.0;
    params.labor_churn.quit_rate_monthly = 0.0;
    params.labor_churn.retirement_rate_monthly = 0.0;
    params.labor_churn.layoff_rate_monthly = 1.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);

    country.process_month_end();

    assert!(country.people.iter().all(|person| !person.is_employed()));
    let replacement_slots: u16 = country.companies[0]
        .open_positions
        .iter()
        .map(|opening| opening.slots)
        .sum();
    assert_eq!(replacement_slots, 3);
    assert!(country.pending_events.iter().any(|event| matches!(
        event,
        SimEvent::LaborChurnMonthly {
            quits: 0,
            retirements: 0,
            layoffs_non_distress: 3,
            replacement_openings: 3,
            ..
        }
    )));
    assert!(
        !country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyLayoffs { .. }))
    );
}

#[test]
fn monthly_labor_churn_channel_order_prefers_retirement_then_layoff_over_quit() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 1_000_000.0;

    let older = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        901,
        70 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    let younger = Person::new_with_profile(
        2,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        902,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.labor_demand.replacement_opening_rate = 0.0;
    params.labor_demand.min_openings_per_active_company = 0;
    params.company_growth.min_profitable_months = u8::MAX;
    params.company_formation.max_startups_per_month = Some(0);
    params.labor_mobility.switch_raise_threshold = 10.0;
    params.labor_churn.quit_rate_monthly = 1.0;
    params.labor_churn.retirement_rate_monthly = 1.0;
    params.labor_churn.layoff_rate_monthly = 1.0;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![older, younger],
        companies,
        0.0,
        params,
    );

    country.process_month_end();

    assert!(country.pending_events.iter().any(|event| matches!(
        event,
        SimEvent::LaborChurnMonthly {
            quits: 0,
            retirements: 1,
            layoffs_non_distress: 1,
            replacement_openings: 2,
            ..
        }
    )));
}

// ---------------------------------------------------------------------------
// Feature: Corporate profit tax
// ---------------------------------------------------------------------------

#[test]
fn corporate_tax_collected_from_profitable_company() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 200_000.0;
    companies[0].last_month_profit = 100_000.0; // bracket_1_max = 50k, rest in bracket 2

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);

    country.process_month_end();

    // Expected tax: 50_000 * 0.15 + 50_000 * 0.25 = 7_500 + 12_500 = 20_000
    let expected_tax = 50_000.0 * 0.15 + 50_000.0 * 0.25;
    assert!(
        (country.monthly_corporate_tax_collected - expected_tax).abs() < 0.01,
        "corporate tax should be {expected_tax}; got {}",
        country.monthly_corporate_tax_collected
    );
    assert!(
        (country.monthly_tax_collected - expected_tax).abs() < 0.01,
        "monthly_tax_collected should include corporate tax; got {}",
        country.monthly_tax_collected
    );
    assert!(
        (country.companies[0].cash_balance - (200_000.0 - expected_tax)).abs() < 0.01,
        "company cash should decrease by tax; got {}",
        country.companies[0].cash_balance
    );
}

#[test]
fn corporate_tax_not_collected_from_unprofitable_company() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 100_000.0;
    companies[0].last_month_profit = -5_000.0; // loss

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);

    country.process_month_end();

    assert!(
        (country.monthly_corporate_tax_collected - 0.0).abs() < 0.01,
        "no corporate tax should be collected on a loss; got {}",
        country.monthly_corporate_tax_collected
    );
}

// ---------------------------------------------------------------------------
// Feature: Unemployment benefit duration cap
// ---------------------------------------------------------------------------

#[test]
fn benefit_not_paid_after_duration_cap_exceeded() {
    let fiscal = FiscalParams {
        max_benefit_months: 2,
        ..FiscalParams::default()
    };
    let mut person = unemployed_adult(1, 0.0);
    // Simulate person already at the cap: set months_unemployed to cap value
    // (process_government_spending will increment it to cap+1, blocking payout)
    person.months_unemployed = 2;
    let treasury_start = 50_000.0;

    let mut country = make_gov_country(vec![person], treasury_start, fiscal);
    // Disable bank recapitalization so treasury changes reflect only gov spending.
    country.sim_params.banking.public_bank_recapitalize_amount = f64::MAX;
    country.process_month_end();

    // months_unemployed will become 3, which exceeds cap of 2 → no benefit
    assert!(
        (country.monthly_gov_spending - 0.0).abs() < 0.01,
        "no gov spending should occur after duration cap; got {}",
        country.monthly_gov_spending
    );
    assert!(
        country.people[0].months_unemployed > fiscal.max_benefit_months,
        "months_unemployed should exceed cap; got {}",
        country.people[0].months_unemployed
    );
}

#[test]
fn benefit_paid_while_within_duration_cap() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    let fiscal = FiscalParams {
        max_benefit_months: 12,
        ..FiscalParams::default()
    };
    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 5; // will become 6 — within cap
    let treasury_start = 50_000.0;

    let mut country = make_gov_country(vec![person], treasury_start, fiscal);
    // Disable bank recapitalization so gov spending is the only treasury outflow.
    country.sim_params.banking.public_bank_recapitalize_amount = f64::MAX;
    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - benefit).abs() < 0.01,
        "benefit should be paid within duration cap; got {}",
        country.monthly_gov_spending
    );
    assert!(
        country.people[0].months_unemployed <= fiscal.max_benefit_months,
        "months_unemployed should be within cap; got {}",
        country.people[0].months_unemployed
    );
}

#[test]
fn months_unemployed_resets_to_zero_on_employment() {
    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 10;

    let employed = employed_adult(2, 0.0);

    let fiscal = FiscalParams::default();
    let treasury_start = 50_000.0;
    let mut country = make_gov_country(vec![person, employed], treasury_start, fiscal);

    country.process_month_end();

    // The employed person (index 1) should have months_unemployed reset to 0
    assert_eq!(
        country.people[1].months_unemployed, 0,
        "employed person months_unemployed should reset to 0"
    );
    // The unemployed person (index 0) should have months_unemployed incremented
    assert!(
        country.people[0].months_unemployed > 0,
        "unemployed person months_unemployed should be incremented"
    );
}

// ---------------------------------------------------------------------------
// Feature: Debt-to-GDP austerity trigger
// ---------------------------------------------------------------------------

#[test]
fn austerity_activates_when_debt_exceeds_gdp_threshold() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].last_month_sales = 1_000.0; // monthly GDP = 1_000, annual = 12_000

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.fiscal.austerity_debt_gdp_threshold = 0.80;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    // Set debt above 80% of annual GDP (80% of 12_000 = 9_600)
    country.gov_debt = 10_000.0;

    country.process_month_end();

    assert!(
        country.austerity_active,
        "austerity should be active when debt/GDP > threshold"
    );
}

#[test]
fn austerity_does_not_activate_below_threshold() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].last_month_sales = 1_000_000.0; // large GDP

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.fiscal.austerity_debt_gdp_threshold = 0.80;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    country.gov_debt = 1_000.0; // well below 80% of 12_000_000

    country.process_month_end();

    assert!(
        !country.austerity_active,
        "austerity should not be active below threshold"
    );
}

#[test]
fn austerity_reduces_benefit_payout() {
    let fiscal = FiscalParams {
        austerity_benefit_reduction: 0.30,
        austerity_debt_gdp_threshold: 0.80,
        ..FiscalParams::default()
    };
    let base_benefit = fiscal.monthly_unemployment_benefit();
    let expected_reduced = round_cents(base_benefit * 0.70);

    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 0;
    let treasury_start = 50_000.0;

    let mut country = make_gov_country(vec![person], treasury_start, fiscal);
    // Disable bank recapitalization so gov spending is the only treasury outflow.
    country.sim_params.banking.public_bank_recapitalize_amount = f64::MAX;
    // Force small GDP and large debt to trigger austerity
    country.gov_debt = 1_000_000.0;
    // Use a tiny GDP so debt/annual_GDP > 0.80 (annual GDP = 1_200; debt/GDP = 833)
    country.companies[0].last_month_sales = 100.0;

    country.process_month_end();

    assert!(country.austerity_active, "austerity should be active");
    assert!(
        (country.monthly_gov_spending - expected_reduced).abs() < 0.01,
        "benefit should be reduced 30% under austerity; expected {expected_reduced}, got {}",
        country.monthly_gov_spending
    );
}
