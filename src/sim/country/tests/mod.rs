use super::*;
use crate::sim::{
    company::build_default_companies,
    params::{DemographicParams, LaborParams, SimulationParams},
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus},
};
use rayon::ThreadPoolBuilder;

fn build_people(count: usize) -> Vec<Person> {
    (0..count)
        .map(|idx| Person::new(idx as u32, (idx % 10) as u8, (idx % 4) as u16, idx as u32))
        .collect()
}

fn build_deterministic_country() -> Country {
    let companies = build_default_companies("Deterministic", 4);
    let mut people = Vec::with_capacity(80);
    for idx in 0..80_u32 {
        let employment = if idx % 3 == 0 {
            EmploymentStatus::Unemployed
        } else {
            EmploymentStatus::Employed {
                job_type_id: (idx % 10) as u8,
                employer_company_id: (idx % 4) as u16,
            }
        };
        let mut person = Person::new_with_profile(
            idx,
            employment,
            idx.saturating_mul(13),
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = if idx % 2 == 0 { 2_000.0 } else { -100.0 };
        people.push(person);
    }
    let mut country =
        Country::with_economy(1, "Deterministic".to_string(), people, companies, 100_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    country.sim_params = params;
    country
}

fn run_country_with_threads(days: u64, threads: usize) -> Country {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool should build");
    pool.install(|| {
        let mut country = build_deterministic_country();
        for _ in 0..days {
            country.tick_day();
        }
        country
    })
}

#[test]
fn tax_brackets_follow_progressive_curve() {
    assert!((monthly_income_tax(2_000.0) - 200.0).abs() < f64::EPSILON);
    assert!((monthly_income_tax(5_000.0) - 700.0).abs() < f64::EPSILON);
    assert!((monthly_income_tax(9_000.0) - 1_700.0).abs() < f64::EPSILON);
}

#[test]
fn month_processing_runs_every_thirty_days() {
    let companies = build_default_companies("A", 4);
    let mut country = Country::with_economy(1, "A".to_string(), build_people(10), companies, 0.0);

    for _ in 0..29 {
        country.tick_day();
    }
    assert_eq!(country.month, 0);

    country.tick_day();
    assert_eq!(country.month, 1);
}

#[test]
fn payroll_and_taxes_only_apply_when_employed() {
    let companies = build_default_companies("A", 2);
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        10,
        25 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.cash_balance = 100.0;
    let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);

    country.process_month();

    assert!((country.monthly_tax_collected - 0.0).abs() < f64::EPSILON);
    assert!((country.people[0].last_month_gross_pay - 0.0).abs() < f64::EPSILON);
}

#[test]
fn money_is_created_by_loan_issuance_only() {
    let companies = build_default_companies("A", 2);
    let mut people = build_people(2);
    people[0].cash_balance = -500.0;
    let mut country = Country::with_economy(1, "A".to_string(), people, companies, 100_000.0);

    country.process_end_of_month_loan_requests();
    country.recompute_aggregates();

    assert!(country.bank.total_originated >= 0.0);
    let expected =
        round_cents(country.initial_money_supply + country.bank.created_money_outstanding);
    assert!((country.expected_money_supply - expected).abs() < f64::EPSILON);
}

#[test]
fn daily_job_market_hires_qualified_unemployed_people() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening(0, 10);
    let mut person = Person::new_with_profile(
        99,
        EmploymentStatus::Unemployed,
        10,
        35 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    person.cash_balance = -5_000.0;
    let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);

    for _ in 0..365 {
        country.tick_day_without_month();
        if country.people[0].is_employed() {
            break;
        }
    }

    assert!(country.people[0].is_employed());
}

#[test]
fn daily_deaths_remove_people_and_write_off_loans() {
    let companies = build_default_companies("A", 1);
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        11,
        99 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    person.cash_balance = 50.0;
    let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);
    country.bank.originate_loan(
        1_000.0,
        3_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        &mut country.people[0].loan,
    );

    let mut params = country.sim_params.clone();
    params.demographic = DemographicParams {
        births_per_1000_per_year: 0.0,
    };
    country.sim_params = params;

    for _ in 0..20_000 {
        country.tick_day_without_month();
        if country.people.is_empty() {
            break;
        }
    }

    assert!(country.people.is_empty());
    assert!(country.bank.created_money_outstanding <= 1_000.0);
}

#[test]
fn country_daily_lifecycle_deterministic_across_thread_counts() {
    let one_thread = run_country_with_threads(45, 1);
    let many_threads = run_country_with_threads(45, 8);
    assert_eq!(one_thread, many_threads);
}

#[test]
fn country_monthly_processing_deterministic_across_thread_counts() {
    let one_thread = run_country_with_threads(DAYS_PER_MONTH * 4, 1);
    let many_threads = run_country_with_threads(DAYS_PER_MONTH * 4, 8);
    assert_eq!(one_thread, many_threads);
}

#[test]
fn job_market_conflict_resolution_is_stable_by_person_index() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening(0, 1);

    let first = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        1,
        35 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    let second = Person::new_with_profile(
        2,
        EmploymentStatus::Unemployed,
        2,
        35 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    let mut country =
        Country::with_economy(1, "A".to_string(), vec![first, second], companies, 0.0);

    country.sim_params = SimulationParams {
        demographic: DemographicParams {
            births_per_1000_per_year: 0.0,
        },
        labor: LaborParams {
            seek_base: 1.0,
            seek_age_slope: 0.0,
            seek_cash_pressure_scale: 0.0,
            seek_cash_reference: 1.0,
            seek_probability_cap: 1.0,
            ..LaborParams::default()
        },
        ..SimulationParams::default()
    };

    country.tick_day_without_month();

    assert!(country.people[0].is_employed());
    assert!(!country.people[1].is_employed());
}
