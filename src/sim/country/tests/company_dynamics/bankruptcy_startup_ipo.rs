use std::collections::VecDeque;

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
    p0.set_employment_with_salary(0, 0, 3_200.0, 0);

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
    p1.set_employment_with_salary(1, 0, 3_200.0, 0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.stock_market.enabled = false;
    params
        .company_distress
        .countercyclical_layoff_unemployment_guard = 1.1;
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
                job_type_id: 5,
                employer_company_id: 0,
            },
            idx + 100,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.set_employment_with_salary(0, 5, 3_900.0, 0);
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
    assert!(
        !country.companies[startup_id as usize]
            .founder_stakes
            .is_empty()
    );
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
    companies[0].sales_history = VecDeque::from(vec![900_000.0, 1_000_000.0, 1_100_000.0]);
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
    founder.set_employment_with_salary(0, 0, 3_200.0, 0);
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
    params
        .company_distress
        .countercyclical_layoff_unemployment_guard = 1.1;
    params.company_distress.bankruptcy_min_loss_months = 0;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;
    params.stock_market.ipo_min_company_age_months = 0;
    params.stock_market.ipo_growth_min_profitable_months = 1;
    params.stock_market.ipo_growth_min_monthly_revenue = 100_000.0;
    params.stock_market.ipo_growth_max_cash_runway_months = 12.0;
    params.stock_market.ipo_max_new_listings_per_month = 2;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
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
            } => Some((
                reason.clone(),
                *secondary_shares_sold,
                *founder_payout_total,
            )),
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
    companies[0].sales_history = VecDeque::from(vec![0.0, 0.0, 0.0]);
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
    params
        .company_distress
        .countercyclical_layoff_unemployment_guard = 1.1;
    params.company_distress.bankruptcy_min_loss_months = 0;
    params.company_distress.bankruptcy_missed_payment_threshold = 3;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    let maybe_ipo = country.pending_events.iter().find_map(|event| match event {
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
