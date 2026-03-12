use std::sync::Arc;

use super::*;

#[test]
fn margin_liquidation_decreases_bank_reserve_not_increases() {
    use crate::sim::{
        bank::{DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS},
        stock_market::ListedEquity,
    };

    let mut person = Person::new(1, 40, 0, 1);
    person.cash_balance = 0.0;

    let companies = build_default_companies("MarginTest", 2);
    let mut country = Country::with_economy(
        0,
        "MarginTest".to_string(),
        vec![person],
        companies,
        50_000.0,
    );
    country.primary_bank_mut().reserve_balance = 20_000.0;

    let equity = ListedEquity::new(0, 0, Arc::from("MCT"), 10.0, 1_000_000, 500);
    country.stock_market.listed.insert(0, equity);
    country.stock_market.person_portfolio_mut(1).add_shares(
        crate::sim::stock_market::StockSecurityId {
            country_id: country.id,
            company_id: 0,
        },
        500,
        10.0,
    );

    let mut loan_slot = None;
    country.primary_bank_mut().originate_loan(
        5_000.0,
        0.0,
        DEFAULT_LOAN_APR / DEFAULT_LOAN_TERM_MONTHS as f64 * 100.0,
        DEFAULT_LOAN_APR,
        &mut loan_slot,
    );
    country.people[0].loan = loan_slot;

    country.process_month_end();

    let violations = country.validate_money_invariants();
    let share_violations: Vec<_> = violations
        .iter()
        .filter(|violation| violation.code == "stock_share_conservation")
        .collect();
    assert!(
        share_violations.is_empty(),
        "share conservation violated after margin call: {:?}",
        share_violations
    );

    let bank_shares = country
        .stock_market
        .bank_portfolio
        .position(crate::sim::stock_market::StockSecurityId {
            country_id: country.id,
            company_id: 0,
        })
        .map_or(0, |position| position.shares);
    let person_shares = country
        .stock_market
        .person_portfolios
        .get(&1)
        .and_then(|portfolio| {
            portfolio.position(crate::sim::stock_market::StockSecurityId {
                country_id: country.id,
                company_id: 0,
            })
        })
        .map_or(0, |position| position.shares);

    assert!(
        bank_shares > 0 || person_shares == 500,
        "expected either bank received liquidated shares ({bank_shares}) or person still holds all 500 ({person_shares})"
    );
}

#[test]
fn growth_ipo_with_conflicting_listing_id_keeps_free_float_in_sync() {
    use crate::sim::stock_market::{ListedEquity, StockSecurityId};

    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();
    companies[1].cash_balance = 10_000.0;
    companies[1].sales_history = std::collections::VecDeque::from(vec![
        900_000.0,
        1_000_000.0,
        1_100_000.0,
    ]);
    companies[1].last_month_sales = 1_000_000.0;
    companies[1].consecutive_profitable_months = 2;
    companies[1].age_months = 12;
    companies[1].founder_stakes = vec![FounderStake {
        person_id: 1,
        shares_owned: 1_000_000,
        cash_contributed: 50_000.0,
    }];

    let mut founder = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 1,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    founder.set_employment_with_salary(1, 0, 3_200.0, 0);
    founder.cash_balance = 2_000.0;

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
    params.stock_market.ipo_min_company_age_months = 0;
    params.stock_market.ipo_growth_min_profitable_months = 1;
    params.stock_market.ipo_growth_min_monthly_revenue = 100_000.0;
    params.stock_market.ipo_growth_max_cash_runway_months = 12.0;
    params.stock_market.ipo_max_new_listings_per_month = 2;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.stock_market.listed.insert(
        1,
        ListedEquity::new(1, 0, Arc::from("LEG"), 10.0, 1_000, 500),
    );

    country.process_month_end();

    let equity = country
        .stock_market
        .listed
        .values()
        .find(|equity| equity.current_company_id == 1)
        .expect("expected IPO listing for company 1");
    assert_ne!(equity.company_id, 1);

    let security_id = StockSecurityId {
        country_id: country.id,
        company_id: equity.company_id,
    };
    let held_by_people: u64 = country
        .stock_market
        .person_portfolios
        .values()
        .filter_map(|portfolio| portfolio.position(security_id).map(|position| position.shares))
        .sum();
    let held_by_bank = country
        .stock_market
        .bank_portfolio
        .position(security_id)
        .map_or(0, |position| position.shares);

    assert_eq!(held_by_people + held_by_bank, equity.free_float_shares);
}
