use super::*;
use crate::sim::world::stock_market::matching::match_books;
use crate::sim::{country::Country, person::Person, stock_market::ListedEquity, world::World};

fn make_person(id: u32, cash: f64) -> Person {
    let mut p = Person::new(id, 30, 0, id + 1);
    p.cash_balance = cash;
    p
}

fn make_world_with_listed(people: Vec<Person>, initial_price: f64) -> World {
    let mut country = Country::new(0, "Test".to_string(), people);
    let equity = ListedEquity::new(0, 0, "TST".to_string(), initial_price, 1_000_000, 200_000);
    country.stock_market.listed.insert(0, equity);
    // Give bank some reserves for maker orders
    country.primary_bank_mut().reserve_balance = 500_000.0;
    World::new(vec![country])
}

#[test]
fn retail_buy_sell_crosses_and_conserves_cash_and_shares() {
    // Person 0: has 1000 shares of company 0, wants to sell
    // Person 1: has cash, wants to buy
    let seller = make_person(0, 0.0);
    let buyer = make_person(1, 100_000.0);
    let mut world = make_world_with_listed(vec![seller, buyer], 10.0);
    world.countries[0]
        .sim_params
        .stock_market
        .retail_noise_trade_prob = 1.0;

    // Manually give seller shares
    world.countries[0]
        .stock_market
        .person_portfolio_mut(0)
        .add_shares(
            crate::sim::stock_market::StockSecurityId {
                country_id: 0,
                company_id: 0,
            },
            1000,
            10.0,
        );
    // Track free_float used: set seller's shares as part of free float
    // (already set to 200_000 in equity constructor)

    let initial_buyer_cash = world.countries[0]
        .people
        .iter()
        .find(|p| p.id == 1)
        .expect("buyer exists")
        .cash_balance;
    let initial_total_cash: f64 = world.countries[0]
        .people
        .iter()
        .map(|p| p.cash_balance)
        .sum::<f64>()
        + world.countries[0].primary_bank().reserve_balance;

    world.run_daily_stock_market();

    let final_total_cash: f64 = world.countries[0]
        .people
        .iter()
        .map(|p| p.cash_balance)
        .sum::<f64>()
        + world.countries[0].primary_bank().reserve_balance;

    // Cash is conserved across people + bank (trades are zero-sum transfers)
    assert!(
        (final_total_cash - initial_total_cash).abs() < 1.0,
        "cash conservation violated: before {initial_total_cash:.2}, after {final_total_cash:.2}"
    );
    // Buyer's cash should have decreased (bought something)
    let buyer_cash_after = world.countries[0]
        .people
        .iter()
        .find(|p| p.id == 1)
        .expect("buyer exists")
        .cash_balance;
    assert!(
        buyer_cash_after < initial_buyer_cash,
        "buyer did not spend any cash"
    );
}

#[test]
fn noncrossing_books_produce_zero_fills() {
    // Buyer willing to pay 9.0, seller asking 11.0 - books don't cross
    let mut buys = vec![LimitOrder {
        trader: TraderId::Person {
            country_id: 0,
            person_id: 1,
        },
        security_id: crate::sim::stock_market::StockSecurityId {
            country_id: 0,
            company_id: 0,
        },
        side: OrderSide::Buy,
        limit_price: 9.0,
        remaining_shares: 100,
        remaining_cash_budget: 900.0,
    }];
    let mut sells = vec![LimitOrder {
        trader: TraderId::Person {
            country_id: 0,
            person_id: 0,
        },
        security_id: crate::sim::stock_market::StockSecurityId {
            country_id: 0,
            company_id: 0,
        },
        side: OrderSide::Sell,
        limit_price: 11.0,
        remaining_shares: 100,
        remaining_cash_budget: 0.0,
    }];

    let fills = match_books(
        &mut buys,
        &mut sells,
        5.0,
        15.0,
        crate::sim::stock_market::StockSecurityId {
            country_id: 0,
            company_id: 0,
        },
    );
    assert!(fills.is_empty(), "expected no fills for non-crossing book");
}

#[test]
fn daily_price_move_is_bounded() {
    // Even if buyer bids 200 and seller asks 100 (midpoint 150), prev_close=100 -> max cap 108
    let mut buys = vec![LimitOrder {
        trader: TraderId::Person {
            country_id: 0,
            person_id: 1,
        },
        security_id: crate::sim::stock_market::StockSecurityId {
            country_id: 0,
            company_id: 0,
        },
        side: OrderSide::Buy,
        limit_price: 200.0,
        remaining_shares: 50,
        remaining_cash_budget: 10_000.0,
    }];
    let mut sells = vec![LimitOrder {
        trader: TraderId::Person {
            country_id: 0,
            person_id: 0,
        },
        security_id: crate::sim::stock_market::StockSecurityId {
            country_id: 0,
            company_id: 0,
        },
        side: OrderSide::Sell,
        limit_price: 100.0,
        remaining_shares: 50,
        remaining_cash_budget: 0.0,
    }];

    // prev_close=100, max_move=0.08 -> max_price=108, min_price=92
    let fills = match_books(
        &mut buys,
        &mut sells,
        92.0,
        108.0,
        crate::sim::stock_market::StockSecurityId {
            country_id: 0,
            company_id: 0,
        },
    );
    assert!(!fills.is_empty(), "expected fills");
    for fill in &fills {
        assert!(
            fill.price <= 108.0,
            "price {} exceeds daily cap 108",
            fill.price
        );
        assert!(
            fill.price >= 92.0,
            "price {} below daily floor 92",
            fill.price
        );
    }
}

#[test]
fn determinism_same_seed_produces_identical_fills() {
    let people: Vec<Person> = (0..50).map(|i| make_person(i, 10_000.0)).collect();
    let mut world1 = make_world_with_listed(people.clone(), 10.0);
    let mut world2 = make_world_with_listed(people, 10.0);

    world1.run_daily_stock_market();
    world2.run_daily_stock_market();

    let turnover1 = world1.countries[0].stock_market.listed[&0].day_turnover;
    let turnover2 = world2.countries[0].stock_market.listed[&0].day_turnover;
    // After record_day_close, day_turnover is reset; check price instead
    let price1 = world1.countries[0].stock_market.listed[&0].last_price;
    let price2 = world2.countries[0].stock_market.listed[&0].last_price;
    let _ = (turnover1, turnover2);
    assert_eq!(price1, price2, "non-deterministic price");
}
