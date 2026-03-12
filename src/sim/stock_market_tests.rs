use std::sync::Arc;

use super::{ListedEquity, StockMarket};

#[test]
fn record_day_close_persists_completed_day_pct_change() {
    let mut equity = ListedEquity::new(1, 1, Arc::from("TST"), 100.0, 10_000, 3_000);
    equity.record_trade(100, 110.0);

    assert!((equity.day_pct_change() - 0.10).abs() < f64::EPSILON);

    equity.record_day_close();

    assert!((equity.day_pct_change() - 0.0).abs() < f64::EPSILON);
    assert!((equity.closed_day_pct_change() - 0.10).abs() < f64::EPSILON);
}

#[test]
fn insert_listing_preserve_index_assigns_distinct_ticker_symbols() {
    let mut market = StockMarket::new();

    let first_ticker = market.insert_listing_preserve_index(
        1,
        ListedEquity::new(1, 1, Arc::from("CMP"), 10.0, 1_000, 500),
    );
    let second_ticker = market.insert_listing_preserve_index(
        2,
        ListedEquity::new(2, 2, Arc::from("cmp"), 20.0, 2_000, 1_000),
    );

    assert_eq!(first_ticker.as_ref(), "CMP");
    assert_ne!(second_ticker.as_ref(), "CMP");
    assert_ne!(first_ticker, second_ticker);
    assert_eq!(
        market
            .listed
            .get(&2)
            .expect("second listing should be present")
            .ticker,
        second_ticker
    );
}

#[test]
fn next_listing_id_skips_existing_stable_listing_keys() {
    let mut market = StockMarket::new();
    market.insert_listing_preserve_index(
        7,
        ListedEquity::new(7, 3, Arc::from("OLD"), 10.0, 1_000, 500),
    );

    assert_eq!(market.next_listing_id(3), 3);
    assert_eq!(market.next_listing_id(7), 8);
}

#[test]
fn reset_mtd_trackers_preserves_last_completed_month_dividends() {
    let mut market = StockMarket::new();
    let portfolio = market.person_portfolio_mut(7);
    portfolio.dividends_mtd = 12.5;
    market.dividends_paid_mtd = 40.0;

    market.reset_mtd_trackers();

    assert!((market.dividends_paid_mtd - 0.0).abs() < f64::EPSILON);
    assert!((market.last_completed_month_dividends_paid - 40.0).abs() < f64::EPSILON);
    assert!(
        (market
            .person_portfolios
            .get(&7)
            .expect("portfolio should exist")
            .dividends_mtd
            - 0.0)
            .abs()
            < f64::EPSILON
    );
    assert!(
        (market
            .person_portfolios
            .get(&7)
            .expect("portfolio should exist")
            .last_completed_month_dividends
            - 12.5)
            .abs()
            < f64::EPSILON
    );
}
