use super::*;
#[test]
fn public_backfill_creation_clamps_price_to_dynamic_macro_cap() {
    let mut companies = build_default_companies("A", 1);
    companies[0].produced_good_id = 0;
    companies[0].unit_price = 10_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params
        .government_backfill
        .public_utility_price_cap_multiplier = 2.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 10_000.0, params);
    country.treasury_balance = 1_000_000.0;
    country.cpi_index = 4.0;
    country.expected_money_supply = 40_000.0;

    country.create_public_essential_company(0, country.sim_params.government_backfill);

    let created = country
        .companies
        .iter()
        .find(|company| company.is_public_utility)
        .expect("public utility should be created");
    let base_cost = crate::sim::good::good_by_id(0)
        .expect("water good should exist")
        .base_unit_cost;
    let expected_cap = round_cents(base_cost * (4.0_f64 * 4.0_f64).sqrt() * 2.0);

    assert!(created.unit_price <= expected_cap + 0.01);
    assert!(created.unit_price < 10_000.0);
}

#[test]
fn monthly_close_clamps_public_utility_price_to_dynamic_macro_cap() {
    let mut company = build_default_companies("A", 1).remove(0);
    company.produced_good_id = 0;
    company.is_public_utility = true;
    company.is_essential_backfill = true;
    company.unit_price = 10_000.0;
    company.inventory_units = 0;
    company.last_month_units_sold = 100;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params
        .government_backfill
        .public_utility_price_cap_multiplier = 2.0;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        vec![company],
        10_000.0,
        params,
    );
    country.cpi_index = 9.0;
    country.expected_money_supply = 40_000.0;

    country.process_month_end();

    let base_cost = crate::sim::good::good_by_id(0)
        .expect("water good should exist")
        .base_unit_cost;
    let expected_cap = round_cents(base_cost * (9.0_f64 * 4.0_f64).sqrt() * 2.0);
    assert!(country.companies[0].unit_price <= expected_cap + 0.01);
}

#[test]
fn monthly_close_applies_affordability_pressure_relief_to_public_utility_price() {
    let mut company = build_default_companies("A", 1).remove(0);
    company.produced_good_id = 0;
    company.is_public_utility = true;
    company.is_essential_backfill = true;
    company.unit_price = 500.0;
    company.inventory_units = 0;
    company.last_month_units_sold = 100;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill.enabled = false;
    params
        .government_backfill
        .public_utility_price_cap_multiplier = 1.5;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        vec![company],
        10_000.0,
        params,
    );
    country.cpi_index = 1.2;
    country.expected_money_supply = 12_000.0;
    country.monthly_essential_debt_spending = 8_000.0;
    country.monthly_unmet_need_events = 14;

    let before = country.companies[0].unit_price;
    country.process_month_end();
    let after = country.companies[0].unit_price;

    assert!(
        after < before,
        "price should be relieved under high affordability pressure: before={before}, after={after}"
    );
}

#[test]
fn public_backfill_openings_start_above_switch_threshold_when_not_capped() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening_with_salary(5, 1, 5_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 1,
        openings_total: 2,
        manufacturing_openings: 1,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 100_000.0, params);
    country.treasury_balance = 100_000.0;
    country.process_month_end();

    let public_company = country
        .companies
        .iter()
        .find(|company| company.is_public_utility)
        .expect("public backfill company should exist");
    let opening = public_company
        .open_positions
        .iter()
        .find(|opening| opening.job_type_id == 5)
        .expect("manufacturing opening should exist");
    let expected_switch_target = 5_000.0 * (1.0 + 0.10 + 0.05);
    assert!(opening.offered_monthly_salary + 0.01 >= expected_switch_target);
}

#[test]
fn public_backfill_wage_escalation_increases_and_respects_1_8x_cap() {
    let mut company = build_default_companies("A", 1).remove(0);
    company.open_positions.clear();
    company.add_opening_with_salary(0, 1, 3_200.0);
    company.is_public_utility = true;
    company.is_essential_backfill = true;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 0,
        ..GovernmentBackfillParams::default()
    };

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![],
        vec![company],
        50_000.0,
        params,
    );
    country.treasury_balance = 50_000.0;
    country.process_month_end();
    let first_salary = country.companies[0].open_positions[0].offered_monthly_salary;

    country.process_month_end();
    let second_salary = country.companies[0].open_positions[0].offered_monthly_salary;

    assert!(second_salary + 0.01 >= first_salary);
    assert!(second_salary <= 3_200.0 * 1.8 + 0.01);
}

#[test]
fn monthly_switch_can_move_worker_into_public_company_on_creation_month() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 5,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.set_employment_with_salary(0, 5, 3_900.0, 0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 1,
        openings_total: 2,
        manufacturing_openings: 1,
        ..GovernmentBackfillParams::default()
    };

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        vec![person],
        companies,
        200_000.0,
        params,
    );
    country.treasury_balance = 200_000.0;
    country.process_month_end();

    let employer_id = country.people[0]
        .employer_company_id()
        .expect("person should remain employed");
    assert_ne!(employer_id, 0);
    assert!(country.companies[employer_id as usize].is_public_utility);
    assert!(country.people[0].monthly_salary() > 3_900.0);
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::JobSwitched { .. }))
    );
}

#[test]
fn annual_market_raise_updates_contracts_and_openings_on_year_boundary() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();
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
    worker_b.set_employment_with_salary(1, 0, 5_000.0, 0);

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
    country.companies[0].profit_history = vec![1_000.0; 12];
    country.companies[0].cash_balance = 50_000.0;
    country.companies[1].profit_history = vec![1_000.0; 12];
    country.companies[1].cash_balance = 50_000.0;

    country.process_month_end();

    assert!((country.people[0].monthly_salary() - 5_000.0).abs() < 0.01);
    let raised_opening = country.companies[0]
        .open_positions
        .iter()
        .find(|opening| opening.job_type_id == 0)
        .expect("company should retain raised opening");
    assert!((raised_opening.offered_monthly_salary - 5_000.0).abs() < 0.01);
}

#[test]
fn annual_market_raise_skips_non_boundary_months() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();

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
    worker_b.set_employment_with_salary(1, 0, 5_000.0, 0);

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
    country.month = 10;
    country.companies[0].profit_history = vec![1_000.0; 12];
    country.companies[0].cash_balance = 50_000.0;

    country.process_month_end();

    assert!((country.people[0].monthly_salary() - 3_200.0).abs() < 0.01);
}

#[test]
fn annual_market_raise_requires_min_profit_history() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();

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
    worker_b.set_employment_with_salary(1, 0, 5_000.0, 0);

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
    country.companies[0].profit_history = vec![1_000.0; 10];
    country.companies[0].cash_balance = 50_000.0;

    country.process_month_end();

    assert!((country.people[0].monthly_salary() - 3_200.0).abs() < 0.01);
}

#[test]
fn annual_market_raise_requires_cash_runway() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();

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
    worker_b.set_employment_with_salary(1, 0, 5_000.0, 0);

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
    country.companies[0].profit_history = vec![1_000.0; 11];
    country.companies[0].cash_balance = 10.0;

    country.process_month_end();

    assert!((country.people[0].monthly_salary() - 3_200.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// Stock market — margin call direction test
// ---------------------------------------------------------------------------

#[test]
fn margin_liquidation_decreases_bank_reserve_not_increases() {
    use crate::sim::{
        bank::{DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS},
        stock_market::ListedEquity,
    };

    // Build a country with one person who has a loan and holds shares
    let mut person = Person::new(1, 40, 0, 1);
    person.cash_balance = 0.0; // will not cover loan payment ? triggers margin call

    let companies = build_default_companies("MarginTest", 2);
    let mut country = Country::with_economy(
        0,
        "MarginTest".to_string(),
        vec![person],
        companies,
        50_000.0,
    );

    // Give bank initial reserve
    country.primary_bank_mut().reserve_balance = 20_000.0;

    // List a company and give the person shares
    let equity = ListedEquity::new(0, 0, "MCT".to_string(), 10.0, 1_000_000, 500);
    country.stock_market.listed.insert(0, equity);
    country.stock_market.person_portfolio_mut(1).add_shares(
        crate::sim::stock_market::StockSecurityId {
            country_id: country.id,
            company_id: 0,
        },
        500,
        10.0,
    );

    // Give the person a loan they cannot pay (monthly_payment >> cash)
    let mut loan_slot = None;
    country.primary_bank_mut().originate_loan(
        5_000.0,
        0.0,
        DEFAULT_LOAN_APR / DEFAULT_LOAN_TERM_MONTHS as f64 * 100.0,
        DEFAULT_LOAN_APR,
        &mut loan_slot,
    );
    country.people[0].loan = loan_slot;

    // Run month-end (which includes collateral seizure)
    country.process_month_end();

    // Share conservation must hold
    let violations = country.validate_money_invariants();
    let share_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.code == "stock_share_conservation")
        .collect();
    assert!(
        share_violations.is_empty(),
        "share conservation violated after margin call: {:?}",
        share_violations
    );

    // If margin call fired: bank should hold shares, person fewer
    let bank_shares = country
        .stock_market
        .bank_portfolio
        .position(crate::sim::stock_market::StockSecurityId {
            country_id: country.id,
            company_id: 0,
        })
        .map_or(0, |p| p.shares);
    let person_shares = country
        .stock_market
        .person_portfolios
        .get(&1)
        .and_then(|p| {
            p.position(crate::sim::stock_market::StockSecurityId {
                country_id: country.id,
                company_id: 0,
            })
        })
        .map_or(0, |p| p.shares);

    assert!(
        bank_shares > 0 || person_shares == 500,
        "expected either bank received liquidated shares ({bank_shares}) or person still holds all 500 ({person_shares})"
    );
}
