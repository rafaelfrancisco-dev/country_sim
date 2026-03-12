use std::collections::VecDeque;

use super::*;
use crate::sim::params::LaborDemandParams;

#[test]
fn annual_raise_ignores_public_utility_wage_leader_for_private_company() {
    let mut companies = build_default_companies("A", 3);
    for company in &mut companies {
        company.open_positions.clear();
    }
    companies[1].is_public_utility = true;
    companies[1].is_essential_backfill = true;
    companies[0].add_opening_with_salary(0, 1, 3_200.0);

    let mut worker_a = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    worker_a.set_employment_with_salary(0, 0, 3_200.0, 0);

    let mut public_worker = Person::new_with_profile(
        2,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 1,
        },
        202,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    public_worker.set_employment_with_salary(1, 0, 5_000.0, 0);

    let mut private_worker = Person::new_with_profile(
        3,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 2,
        },
        303,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    private_worker.set_employment_with_salary(2, 0, 3_600.0, 0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params.labor_mobility.switch_raise_threshold = 10.0;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![worker_a, public_worker, private_worker],
        companies,
        200_000.0,
        params,
    );
    country.month = 11;
    country.companies[0].profit_history = VecDeque::from(vec![1_000.0; 12]);
    country.companies[0].cash_balance = 50_000.0;
    country.companies[1].profit_history = VecDeque::from(vec![1_000.0; 12]);
    country.companies[1].cash_balance = 50_000.0;
    country.companies[2].profit_history = VecDeque::from(vec![1_000.0; 12]);
    country.companies[2].cash_balance = 50_000.0;

    country.process_month_end();

    assert!((country.people[0].monthly_salary() - 3_340.0).abs() < 0.01);
}

#[test]
fn annual_raise_skips_when_profit_margin_is_too_thin() {
    let mut companies = build_default_companies("A", 2);
    for company in &mut companies {
        company.open_positions.clear();
    }
    companies[0].add_opening_with_salary(0, 1, 3_200.0);

    let mut worker_a = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    worker_a.set_employment_with_salary(0, 0, 3_200.0, 0);

    let mut worker_b = Person::new_with_profile(
        2,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 1,
        },
        202,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    worker_b.set_employment_with_salary(1, 0, 3_600.0, 0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params.labor_mobility.switch_raise_threshold = 10.0;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![worker_a, worker_b],
        companies,
        200_000.0,
        params,
    );
    country.month = 11;
    country.companies[0].profit_history = VecDeque::from(vec![100.0; 12]);
    country.companies[0].cash_balance = 50_000.0;
    country.companies[1].profit_history = VecDeque::from(vec![1_000.0; 12]);
    country.companies[1].cash_balance = 50_000.0;

    country.process_month_end();

    assert!((country.people[0].monthly_salary() - 3_200.0).abs() < 0.01);
}

#[test]
fn monthly_job_switch_waits_for_minimum_tenure() {
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
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    country.day = 29;
    country.process_month_end();

    assert_eq!(country.people[0].employer_company_id(), Some(0));
    assert!(
        country
            .pending_events
            .iter()
            .all(|event| !matches!(event, SimEvent::JobSwitched { .. }))
    );
}

#[test]
fn unprofitable_company_does_not_add_baseline_openings() {
    let mut company = build_default_companies("A", 1).remove(0);
    company.open_positions.clear();
    company.employee_count = 120;
    company.last_month_profit = -1_000.0;
    company.profit_history = VecDeque::from(vec![-1_000.0; 3]);

    let added = company.ensure_baseline_openings(LaborDemandParams::default(), 7);

    assert_eq!(added, 0);
    assert!(company.open_positions.is_empty());
}

#[test]
fn profitable_company_without_recent_demand_does_not_add_baseline_openings() {
    let mut company = build_default_companies("A", 1).remove(0);
    company.open_positions.clear();
    company.employee_count = 80;
    company.cash_balance = 500_000.0;
    company.last_month_profit = 12_000.0;
    company.profit_history = VecDeque::from(vec![12_000.0; 3]);
    company.last_month_sales = 0.0;
    company.last_completed_month_sales = 0.0;
    company.last_month_units_sold = 0;
    company.last_completed_month_units_sold = 0;
    company.last_month_sell_through = 0.0;
    company.last_completed_month_sell_through = 0.0;
    company.sales_history = VecDeque::from(vec![0.0, 0.0, 0.0]);
    company.sold_units_history = VecDeque::from(vec![0, 0, 0]);

    let added = company.ensure_baseline_openings(LaborDemandParams::default(), 11);

    assert_eq!(added, 0);
    assert!(company.open_positions.is_empty());
}

#[test]
fn housing_landlord_with_rent_revenue_can_add_baseline_openings() {
    let mut company = build_default_companies("A", 1).remove(0);
    company.produced_good_id = 2;
    company.open_positions.clear();
    company.employee_count = 12;
    company.cash_balance = 100_000.0;
    company.monthly_payroll_budget = 12_000.0;
    company.last_month_payroll_paid = 10_000.0;
    company.last_month_rent_revenue = 24_000.0;
    company.last_month_profit = 14_000.0;

    let added = company.ensure_baseline_openings(LaborDemandParams::default(), 13);

    assert!(added > 0);
    assert!(!company.open_positions.is_empty());
}

#[test]
fn public_utility_is_skipped_by_generic_growth_openings() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].is_public_utility = true;
    companies[0].employee_count = 25;
    companies[0].cash_balance = 200_000.0;
    companies[0].consecutive_profitable_months = 12;
    companies[0].last_month_profit = 50_000.0;
    companies[0].last_month_sales = 40_000.0;
    companies[0].last_completed_month_sales = 40_000.0;
    companies[0].last_month_units_sold = 100;
    companies[0].last_completed_month_units_sold = 100;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params.company_distress.countercyclical_layoff_unemployment_guard = 1.1;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    country.process_month_end();

    assert!(country.companies[0].open_positions.is_empty());
}
