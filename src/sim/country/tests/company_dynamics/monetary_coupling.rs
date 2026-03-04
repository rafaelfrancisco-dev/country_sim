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
    companies[0].sales_history = vec![10_000.0, 10_000.0, 10_000.0];
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
