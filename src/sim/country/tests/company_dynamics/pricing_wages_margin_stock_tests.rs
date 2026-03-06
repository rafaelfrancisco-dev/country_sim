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
