use super::*;
use crate::sim::{
    bank::{DEFAULT_LOAN_APR, DEFAULT_MAX_DEBT_TO_INCOME},
    params::{DemographicParams, LaborParams, SimulationParams},
};

#[test]
fn daily_job_market_headcounts_match_full_recompute_reference() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening(5, 1);

    let mut country = Country::with_economy(
        1,
        "A".to_string(),
        vec![
            unemployed_adult(1, 0.0),
            unemployed_adult(2, 0.0),
            unemployed_adult(3, 0.0),
        ],
        companies,
        0.0,
    );
    country.primary_bank_mut().open_positions.clear();
    country.primary_bank_mut().add_opening(2, 1);
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

    let company_count_before = country.companies[0].employee_count;
    let manufacturing_before = country.companies[0].manufacturing_employee_count;
    let bank_count_before = country.primary_bank().employee_count;

    country.recompute_company_headcounts();
    country.recompute_bank_headcounts();

    assert_eq!(company_count_before, country.companies[0].employee_count);
    assert_eq!(
        manufacturing_before,
        country.companies[0].manufacturing_employee_count
    );
    assert_eq!(bank_count_before, country.primary_bank().employee_count);
}

#[test]
fn daily_deaths_keep_incremental_aggregates_aligned_with_full_recompute() {
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
    let mut loan_slot = country.people[0].loan.take();
    country.primary_bank_mut().originate_loan(
        1_000.0,
        3_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        DEFAULT_LOAN_APR,
        &mut loan_slot,
    );
    country.people[0].loan = loan_slot;

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

    let incremental_household = country.household_cash_total;
    let incremental_company = country.company_cash_total;
    let incremental_total = country.total_money_supply;

    country.recompute_aggregates();

    assert!((country.household_cash_total - incremental_household).abs() < 0.01);
    assert!((country.company_cash_total - incremental_company).abs() < 0.01);
    assert!((country.total_money_supply - incremental_total).abs() < 0.01);
    assert!(country.validate_money_invariants().is_empty());
}

#[test]
fn eligible_unemployed_index_tracks_hire_and_birthday_eligibility() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening(5, 1);

    let mut almost_adult = Person::new_with_profile(
        2,
        EmploymentStatus::Unemployed,
        22,
        18 * DAYS_PER_YEAR - 1,
        EducationTrack::BasicOnly,
    );
    almost_adult.cash_balance = 0.0;

    let mut country = Country::with_economy(
        1,
        "A".to_string(),
        vec![unemployed_adult(1, 0.0), almost_adult],
        companies,
        0.0,
    );
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

    assert!(country.eligible_unemployed_ids().contains(&1));
    assert!(!country.eligible_unemployed_ids().contains(&2));

    country.tick_day_without_month();

    assert!(country.employed_ids().contains(&1));
    assert!(!country.eligible_unemployed_ids().contains(&1));
    assert!(country.eligible_unemployed_ids().contains(&2));
}
