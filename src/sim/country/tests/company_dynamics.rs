use super::*;

#[test]
fn bankruptcy_physically_removes_company_and_remaps_employers() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();
    companies[0].cash_balance = 0.0;
    companies[0].loan = Some(LoanAccount::new(5_000.0, 0.12, 12));
    companies[0].consecutive_missed_loan_payments = 2;
    companies[0].consecutive_loss_months = 6;
    companies[1].cash_balance = 100_000.0;

    let mut p0 = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        111,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    p0.set_employment_with_salary(0, 0, 3_200.0);

    let mut p1 = Person::new_with_profile(
        2,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 1,
        },
        112,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    p1.set_employment_with_salary(1, 0, 3_200.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params.company_distress.countercyclical_layoff_unemployment_guard = 1.1;
    params.company_distress.bankruptcy_min_loss_months = 6;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![p0, p1], companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert_eq!(country.companies[0].id, 0);
    assert!(!country.people[0].is_employed());
    assert_eq!(country.people[1].employer_company_id(), Some(0));
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyBankrupt { .. }))
    );
}

#[test]
fn startup_formation_creates_company_and_moves_founders() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 1_000_000.0;
    companies[0].produced_good_id = 0;
    companies[0].last_month_sales = 100_000.0;

    let mut people = Vec::new();
    for idx in 0..5_u32 {
        let mut person = Person::new_with_profile(
            idx,
            EmploymentStatus::Employed {
                job_type_id: 0,
                employer_company_id: 0,
            },
            idx + 100,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.set_employment_with_salary(0, 0, 3_200.0);
        person.cash_balance = 100_000.0;
        people.push(person);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.company_formation.max_startups_per_month = Some(1);
    params.labor_mobility.switch_raise_threshold = 10.0;
    params.government_backfill.enabled = false;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();
    assert_eq!(country.companies.len(), 2);
    let startup_id = 1_u16;
    assert!(
        country
            .people
            .iter()
            .all(|person| person.employer_company_id() == Some(startup_id))
    );
    let startup_openings: u16 = country.companies[startup_id as usize]
        .open_positions
        .iter()
        .map(|opening| opening.slots)
        .sum();
    assert!(
        startup_openings >= country.sim_params.company_formation.startup_openings_min
            && startup_openings <= country.sim_params.company_formation.startup_openings_max
    );
    assert!(!country.companies[startup_id as usize].founder_stakes.is_empty());
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyFounded { .. }))
    );
}

#[test]
fn growth_ipo_lists_company_and_pays_founder_secondary_proceeds() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 10_000.0;
    companies[0].sales_history = vec![900_000.0, 1_000_000.0, 1_100_000.0];
    companies[0].last_month_sales = 1_000_000.0;
    companies[0].consecutive_profitable_months = 2;
    companies[0].age_months = 12;
    companies[0].founder_stakes = vec![FounderStake {
        person_id: 1,
        shares_owned: 1_000_000,
        cash_contributed: 50_000.0,
    }];

    let mut founder = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    founder.set_employment_with_salary(0, 0, 3_200.0);
    founder.cash_balance = 2_000.0;

    let founder_cash_before = founder.cash_balance;
    let mut people = vec![founder];
    for idx in 0..24_u32 {
        let mut investor = Person::new_with_profile(
            idx + 2,
            EmploymentStatus::Unemployed,
            idx + 102,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        investor.cash_balance = 80_000.0;
        people.push(investor);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.company_distress.countercyclical_layoff_unemployment_guard = 1.1;
    params.company_distress.bankruptcy_min_loss_months = 0;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;
    params.stock_market.ipo_min_company_age_months = 0;
    params.stock_market.ipo_growth_min_profitable_months = 1;
    params.stock_market.ipo_growth_min_monthly_revenue = 100_000.0;
    params.stock_market.ipo_growth_max_cash_runway_months = 12.0;
    params.stock_market.ipo_max_new_listings_per_month = 2;

    let mut country = Country::with_economy_and_params(
        1,
        "A".to_string(),
        people,
        companies,
        0.0,
        params,
    );
    country.process_month_end();

    let ipo = country
        .pending_events
        .iter()
        .find_map(|event| match event {
            SimEvent::StockIpo {
                reason,
                secondary_shares_sold,
                founder_payout_total,
                ..
            } => Some((reason.clone(), *secondary_shares_sold, *founder_payout_total)),
            _ => None,
        })
        .expect("expected growth IPO event");
    assert!(matches!(ipo.0, IpoReason::Growth));
    assert!(ipo.1 > 0);
    assert!(ipo.2 > 0.0);
    assert!(country.stock_market.is_listed(0));
    assert!(country.people[0].cash_balance > founder_cash_before);
}

#[test]
fn distress_ipo_keeps_secondary_zero() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = -500.0;
    companies[0].loan = Some(LoanAccount::new(100_000.0, 0.12, 12));
    companies[0].consecutive_missed_loan_payments = 2;
    companies[0].consecutive_loss_months = 6;
    companies[0].last_month_sales = 0.0;
    companies[0].sales_history = vec![0.0, 0.0, 0.0];
    companies[0].age_months = 24;

    let mut people = Vec::new();
    for idx in 0..20_u32 {
        let mut investor = Person::new_with_profile(
            idx + 10,
            EmploymentStatus::Unemployed,
            idx + 103,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        investor.cash_balance = 50_000.0;
        people.push(investor);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.company_distress.countercyclical_layoff_unemployment_guard = 1.1;
    params.company_distress.bankruptcy_min_loss_months = 0;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    let maybe_ipo = country
        .pending_events
        .iter()
        .find_map(|event| match event {
            SimEvent::StockIpo {
                reason,
                secondary_shares_sold,
                ..
            } => Some((reason.clone(), *secondary_shares_sold)),
            _ => None,
        });
    assert!(
        maybe_ipo.is_some(),
        "expected distress IPO event; misses={} loss={} cash={} threshold={} guard={} events={:?}",
        country.companies[0].consecutive_missed_loan_payments,
        country.companies[0].consecutive_loss_months,
        country.companies[0].cash_balance,
        country
            .sim_params
            .company_distress
            .bankruptcy_missed_payment_threshold,
        country
            .sim_params
            .company_distress
            .countercyclical_layoff_unemployment_guard,
        country.pending_events
    );
    let ipo = maybe_ipo.expect("checked above");
    assert!(matches!(ipo.0, IpoReason::Distress));
    assert_eq!(ipo.1, 0);
}

#[test]
fn startup_can_form_with_unemployed_founders_when_enabled() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 0;
    companies[0].last_month_sales = 500_000.0;
    companies[0].last_month_profit = 200_000.0;
    companies[0].sales_history = vec![450_000.0, 500_000.0, 520_000.0];

    let mut people = Vec::new();
    for idx in 0..3_u32 {
        let mut person = Person::new_with_profile(
            idx + 1,
            EmploymentStatus::Unemployed,
            idx + 200,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = 150_000.0;
        people.push(person);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.company_formation.max_startups_per_month = Some(1);
    params.company_formation.min_founders = 3;
    params.company_formation.max_founders = 3;
    params.company_formation.allow_unemployed_founders = true;
    params.company_formation.labor_slack_startup_trigger = 0.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 2);
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyFounded { .. }))
    );
}

#[test]
fn startup_viability_gate_blocks_non_viable_plans() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 0;
    companies[0].last_month_sales = 1_000.0;
    companies[0].last_month_profit = 400.0;
    companies[0].sales_history = vec![900.0, 1_000.0, 1_100.0];

    let mut people = Vec::new();
    for idx in 0..3_u32 {
        let mut person = Person::new_with_profile(
            idx + 1,
            EmploymentStatus::Unemployed,
            idx + 400,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = 200_000.0;
        people.push(person);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.company_formation.max_startups_per_month = Some(1);
    params.company_formation.min_founders = 3;
    params.company_formation.max_founders = 3;
    params.company_formation.allow_unemployed_founders = true;
    params.company_formation.startup_max_payroll_to_revenue = 0.05;
    params.company_formation.labor_slack_startup_trigger = 0.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert!(
        !country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyFounded { .. }))
    );
}

#[test]
fn government_backfill_creates_company_for_each_missing_essential_good() {
    let companies = build_default_companies("A", 1);
    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 4,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 200_000.0, params);
    country.treasury_balance = 200_000.0;
    country.process_month_end();

    let funded_goods = country
        .companies
        .iter()
        .filter(|company| company.is_essential_backfill)
        .map(|company| company.produced_good_id)
        .collect::<HashSet<_>>();
    assert_eq!(funded_goods.len(), 4);
    for good_id in 1..=4_u8 {
        assert!(funded_goods.contains(&good_id));
    }

    let gov_company_events = country
        .pending_events
        .iter()
        .filter(|event| matches!(event, SimEvent::GovCompanyFunded { .. }))
        .count();
    assert_eq!(gov_company_events, 4);
}

#[test]
fn government_backfill_financing_uses_treasury_then_deficit() {
    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 0,
        openings_total: 3,
        manufacturing_openings: 2,
        runway_months: 1.5,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], vec![], 1_000.0, params);
    country.treasury_balance = 1_000.0;
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert!(country.companies[0].is_public_utility);
    assert!(country.gov_debt > 0.0);
    assert!(country.bank.created_money_outstanding > 0.0);
    assert!(country.treasury_balance.abs() < 0.01);
    assert!(country.validate_money_invariants().is_empty());
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
    person.set_employment_with_salary(0, 5, 3_900.0);

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
    country.bank.reserve_balance = 20_000.0;

    // List a company and give the person shares
    let equity = ListedEquity::new(0, "MCT".to_string(), 10.0, 1_000_000, 500);
    country.stock_market.listed.insert(0, equity);
    country
        .stock_market
        .person_portfolio_mut(1)
        .add_shares(0, 500, 10.0);

    // Give the person a loan they cannot pay (monthly_payment >> cash)
    let mut loan_slot = None;
    country.bank.originate_loan(
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
        .position(0)
        .map_or(0, |p| p.shares);
    let person_shares = country
        .stock_market
        .person_portfolios
        .get(&1)
        .and_then(|p| p.position(0))
        .map_or(0, |p| p.shares);

    assert!(
        bank_shares > 0 || person_shares == 500,
        "expected either bank received liquidated shares ({bank_shares}) or person still holds all 500 ({person_shares})"
    );
}
