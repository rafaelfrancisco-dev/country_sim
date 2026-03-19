use super::*;
use std::collections::VecDeque;

#[test]
fn dormant_private_company_without_staff_or_openings_is_resolved() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 25_000.0;
    companies[0].consecutive_loss_months = 3;
    companies[0].last_month_sales = 0.0;
    companies[0].last_month_units_produced = 0;
    companies[0].last_completed_month_units_produced = 0;
    companies[0].sales_history = VecDeque::from(vec![0.0, 0.0, 0.0]);

    let people = vec![Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    )];

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.company_distress.loss_distress_month_threshold = 2;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    assert!(country.companies.is_empty());
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyBankrupt { .. }))
    );
}

#[test]
fn chronic_private_loss_company_with_no_runway_is_resolved() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].employee_count = 20;
    companies[0].monthly_payroll_budget = 40_000.0;
    companies[0].last_month_payroll_paid = 40_000.0;
    companies[0].last_completed_month_payroll_paid = 40_000.0;
    companies[0].cash_balance = 10_000.0;
    companies[0].consecutive_loss_months = 18;
    companies[0].last_month_profit = -25_000.0;
    companies[0].profit_history = VecDeque::from(vec![-25_000.0; 6]);
    companies[0].last_month_sales = 0.0;
    companies[0].sales_history = VecDeque::from(vec![0.0, 0.0, 0.0]);

    let mut worker = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        301,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    worker.set_employment_with_salary(0, 0, 2_000.0, 0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.company_distress.countercyclical_layoff_unemployment_guard = 1.1;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![worker],
        companies,
        0.0,
        params,
    );
    country.process_month_end();

    assert!(country.companies.is_empty());
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyBankrupt { .. }))
    );
}

#[test]
fn housing_landlord_with_rent_revenue_is_not_resolved_as_no_demand_loss_company() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].produced_good_id = 2;
    companies[0].employee_count = 5;
    companies[0].monthly_payroll_budget = 20_000.0;
    companies[0].last_month_payroll_paid = 20_000.0;
    companies[0].last_completed_month_payroll_paid = 20_000.0;
    companies[0].cash_balance = 50_000.0;
    companies[0].consecutive_loss_months = 18;
    companies[0].last_month_profit = -1_000.0;
    companies[0].profit_history = VecDeque::from(vec![-1_000.0; 6]);
    companies[0].last_month_rent_revenue = 19_000.0;

    let mut worker = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        401,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    worker.set_employment_with_salary(0, 0, 4_000.0, 0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.company_distress.countercyclical_layoff_unemployment_guard = 1.1;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![worker],
        companies,
        0.0,
        params,
    );
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert!(
        country
            .pending_events
            .iter()
            .all(|event| !matches!(event, SimEvent::CompanyBankrupt { .. }))
    );
}
