use super::*;

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

#[test]
fn corporate_tax_collected_from_profitable_company() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 200_000.0;
    companies[0].last_month_profit = 100_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);

    country.process_month_end();

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
    companies[0].last_month_profit = -5_000.0;

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
