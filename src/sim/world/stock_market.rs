use std::collections::{HashMap, HashSet};

use super::World;
use crate::sim::{event::SimEvent, money::round_cents, stock_market::StockMarket};

// ---------------------------------------------------------------------------
// Order book types
// ---------------------------------------------------------------------------

/// Identifies a participant in a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderId {
    Person(u32),
    Bank,
}

impl TraderId {
    /// Stable key for deterministic tie-breaking in sort.
    fn stable_key(self) -> u64 {
        match self {
            TraderId::Person(id) => id as u64,
            TraderId::Bank => u64::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct LimitOrder {
    pub trader: TraderId,
    #[allow(dead_code)]
    pub company_id: u16,
    #[allow(dead_code)]
    pub side: OrderSide,
    /// Maximum price a buyer will pay / minimum price a seller will accept.
    pub limit_price: f64,
    pub remaining_shares: u64,
    /// Tracks remaining cash committed for this buy order (0.0 for sells).
    pub remaining_cash_budget: f64,
}

#[derive(Debug, Clone)]
pub struct TradeFill {
    pub company_id: u16,
    pub buyer: TraderId,
    pub seller: TraderId,
    pub shares: u64,
    pub price: f64,
}

// ---------------------------------------------------------------------------
// World entry point
// ---------------------------------------------------------------------------

impl World {
    /// Run the daily stock market for all countries after the goods market.
    pub(super) fn run_daily_stock_market(&mut self) {
        for country in &mut self.countries {
            if !country.sim_params.stock_market.enabled {
                continue;
            }
            if country.stock_market.listed.is_empty() {
                continue;
            }
            run_country_stock_market(country, self.day);
        }
        // Export must run before day-close reset so turnover/volume/day% reflect
        // today's actual trading activity.
        if self.export_queue_active {
            self.push_daily_stock_snapshot();
        }
        // Close every active market day after exports are captured.
        for country in &mut self.countries {
            if !country.sim_params.stock_market.enabled {
                continue;
            }
            if country.stock_market.listed.is_empty() {
                continue;
            }
            country.stock_market.record_day_close(self.day);
            if self.day.is_multiple_of(30) {
                country.stock_market.gc_portfolios();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic hash helpers
// ---------------------------------------------------------------------------

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}

fn deterministic_sample_people_indices(
    people_count: usize,
    sample_count: usize,
    day: u64,
    country_id: u16,
) -> Vec<usize> {
    if people_count == 0 || sample_count == 0 {
        return Vec::new();
    }

    let mut selected = Vec::with_capacity(sample_count);
    let mut seen = HashSet::with_capacity(sample_count.saturating_mul(2));
    let base_seed =
        day.wrapping_mul(0xb5ad4eceda1ce2a9) ^ (u64::from(country_id) << 32) ^ 0x9e37_79b9;
    let mut nonce = 0_u64;
    while selected.len() < sample_count {
        let candidate = (hash64(base_seed ^ nonce) as usize) % people_count;
        nonce = nonce.wrapping_add(1);
        if seen.insert(candidate) {
            selected.push(candidate);
        }
    }
    selected
}

// ---------------------------------------------------------------------------
// Main daily market function
// ---------------------------------------------------------------------------

fn run_country_stock_market(country: &mut crate::sim::country::Country, day: u64) {
    let params = country.sim_params.stock_market;
    let cap = params.daily_participant_cap;
    let reserve_ratio = params.investor_cash_reserve_ratio;
    let spread_frac = params.maker_spread_bps as f64 / 10_000.0;
    let max_move = params.max_daily_price_move_pct;
    let noise_prob = params.retail_noise_trade_prob;
    let max_float_frac = params.max_order_fraction_of_float;

    let listed_ids: Vec<u16> = country.stock_market.listed.keys().copied().collect();
    if listed_ids.is_empty() {
        return;
    }

    let people_count = country.people.len();
    if people_count == 0 {
        return;
    }

    let sample_count = cap.min(people_count);

    // -----------------------------------------------------------------------
    // Build per-symbol order books
    // -----------------------------------------------------------------------
    let mut buy_orders: HashMap<u16, Vec<LimitOrder>> = HashMap::new();
    let mut sell_orders: HashMap<u16, Vec<LimitOrder>> = HashMap::new();

    // --- Retail orders ---
    let sampled_people = deterministic_sample_people_indices(people_count, sample_count, day, country.id);
    for (slot, person_idx) in sampled_people.into_iter().enumerate() {
        let slot_seed =
            hash64(day.wrapping_mul(0xb5ad4eceda1ce2a9) ^ (u64::from(country.id) << 32) ^ slot as u64);
        let person = &country.people[person_idx];
        let person_id = person.id;
        let person_cash = person.cash_balance;

        for &cid in &listed_ids {
            let equity = match country.stock_market.listed.get(&cid) {
                Some(e) => e,
                None => continue,
            };
            let last_price = equity.last_price;
            let float_shares = equity.free_float_shares;
            if last_price <= 0.0 || float_shares == 0 {
                continue;
            }

            let max_order_by_float =
                ((float_shares as f64 * max_float_frac).floor() as u64).max(1);

            // MA20 trend signal
            let history = equity.price_history.as_slice();
            let ma_window = history.len().min(20);
            let avg_price = if ma_window == 0 {
                last_price
            } else {
                history[history.len() - ma_window..].iter().sum::<f64>() / ma_window as f64
            };
            let trend = if avg_price > 0.0 {
                (last_price - avg_price) / avg_price
            } else {
                0.0
            };

            // Value gap signal from company fundamentals
            let company = match country.companies.get(cid as usize) {
                Some(c) => c,
                None => continue,
            };
            let debt = company.loan.as_ref().map_or(0.0, |l| l.debt_balance);
            let anchor = StockMarket::ipo_valuation_anchor(
                company.average_monthly_revenue(),
                company.last_month_profit,
                company.cash_balance,
                debt,
                equity.shares_outstanding,
            );
            let value_gap = if last_price > 0.0 {
                (anchor - last_price) / last_price
            } else {
                0.0
            };

            let order_seed = hash64(slot_seed ^ u64::from(cid) ^ 0xf00d);
            let noise = (unit_interval(hash64(order_seed ^ 0x1111)) - 0.5) * 2.0 * noise_prob;

            let has_position = country
                .stock_market
                .person_portfolios
                .get(&person_id)
                .and_then(|p| p.position(cid))
                .map_or(false, |pos| pos.shares > 0);

            // Positive directional score means undervalued/down-trending => buy pressure.
            // Negative score means overvalued/up-trending => sell pressure.
            let directional = (value_gap - trend).clamp(-0.25, 0.25) + noise;
            let buy_threshold = if has_position { 0.03 } else { 0.08 };
            let sell_threshold = 0.03;
            let want_buy = directional > buy_threshold;
            let want_sell = has_position && directional < -sell_threshold;

            if want_sell {
                let free_shares = country
                    .stock_market
                    .person_portfolios
                    .get(&person_id)
                    .and_then(|p| p.position(cid))
                    .map_or(0, |pos| pos.shares.saturating_sub(pos.pledged_shares));

                if free_shares > 0 {
                    let sell_frac = unit_interval(hash64(order_seed ^ 0x2222)) * 0.30 + 0.05;
                    let shares = ((free_shares as f64 * sell_frac).ceil() as u64)
                        .min(free_shares)
                        .min(max_order_by_float)
                        .max(1);
                    // Sell limit: slight discount below last (0–1%) for eager sellers
                    let disc = unit_interval(hash64(order_seed ^ 0x3333)) * 0.01;
                    let limit_price =
                        round_cents((last_price * (1.0 - disc)).max(0.01));
                    sell_orders.entry(cid).or_default().push(LimitOrder {
                        trader: TraderId::Person(person_id),
                        company_id: cid,
                        side: OrderSide::Sell,
                        limit_price,
                        remaining_shares: shares,
                        remaining_cash_budget: 0.0,
                    });
                }
            } else if want_buy {
                let investable =
                    round_cents((person_cash - person_cash * reserve_ratio).max(0.0));
                if investable < last_price {
                    continue;
                }
                let buy_frac = unit_interval(hash64(order_seed ^ 0x4444)) * 0.35 + 0.05;
                let budget = round_cents(investable * buy_frac);
                let shares = (budget / last_price) as u64;
                let shares = shares.min(max_order_by_float).max(1);
                if shares == 0 {
                    continue;
                }
                // Buy limit: slight premium above last (0–1%) for eager buyers
                let prem = unit_interval(hash64(order_seed ^ 0x5555)) * 0.01;
                let limit_price = round_cents(last_price * (1.0 + prem));
                let cash_budget = round_cents(shares as f64 * limit_price);
                buy_orders.entry(cid).or_default().push(LimitOrder {
                    trader: TraderId::Person(person_id),
                    company_id: cid,
                    side: OrderSide::Buy,
                    limit_price,
                    remaining_shares: shares,
                    remaining_cash_budget: cash_budget,
                });
            }
        }
    }

    // --- Bank maker orders ---
    let bank_equity_exposure = country.stock_market.bank_equity_exposure();
    let bank_equity_budget =
        params.bank_max_equity_reserve_fraction * country.bank.reserve_balance;
    let bank_equity_headroom = (bank_equity_budget - bank_equity_exposure).max(0.0);

    for &cid in &listed_ids {
        let equity = match country.stock_market.listed.get(&cid) {
            Some(e) => e,
            None => continue,
        };
        let last_price = equity.last_price;
        let float_shares = equity.free_float_shares;
        if last_price <= 0.0 || float_shares == 0 {
            continue;
        }
        let max_order_by_float =
            ((float_shares as f64 * max_float_frac).floor() as u64).max(1);

        let half = spread_frac / 2.0;
        let bid_price = round_cents((last_price * (1.0 - half)).max(0.01));
        let ask_price = round_cents(last_price * (1.0 + half));

        // Bank bid (buy)
        let bid_shares_by_headroom = if bid_price > 0.0 {
            (bank_equity_headroom / bid_price).floor() as u64
        } else {
            0
        };
        let bid_shares = bid_shares_by_headroom.min(max_order_by_float);
        if bid_shares > 0 {
            buy_orders.entry(cid).or_default().push(LimitOrder {
                trader: TraderId::Bank,
                company_id: cid,
                side: OrderSide::Buy,
                limit_price: bid_price,
                remaining_shares: bid_shares,
                remaining_cash_budget: round_cents(bid_shares as f64 * bid_price),
            });
        }

        // Bank ask (sell from inventory)
        let bank_inventory = country
            .stock_market
            .bank_portfolio
            .position(cid)
            .map_or(0, |p| p.shares);
        let ask_shares = bank_inventory.min(max_order_by_float);
        if ask_shares > 0 {
            sell_orders.entry(cid).or_default().push(LimitOrder {
                trader: TraderId::Bank,
                company_id: cid,
                side: OrderSide::Sell,
                limit_price: ask_price,
                remaining_shares: ask_shares,
                remaining_cash_budget: 0.0,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Match and settle per symbol
    // -----------------------------------------------------------------------
    let mut total_market_cap = 0.0_f64;
    let mut total_turnover = 0.0_f64;
    let mut listed_count = 0_usize;

    for &cid in &listed_ids {
        let (prev_close, last_price) = match country.stock_market.listed.get(&cid) {
            Some(e) => (e.prev_close, e.last_price),
            None => continue,
        };
        if last_price <= 0.0 {
            continue;
        }

        let base = if prev_close > 0.0 { prev_close } else { last_price };
        let max_price = round_cents(base * (1.0 + max_move));
        let min_price = round_cents((base * (1.0 - max_move)).max(0.01));

        let mut buys = buy_orders.remove(&cid).unwrap_or_default();
        let mut sells = sell_orders.remove(&cid).unwrap_or_default();

        // Sort: buys descending by limit_price, tiebreak by stable_key ascending
        buys.sort_by(|a, b| {
            b.limit_price
                .partial_cmp(&a.limit_price)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.trader.stable_key().cmp(&b.trader.stable_key()))
        });
        // Sort: sells ascending by limit_price, tiebreak by stable_key ascending
        sells.sort_by(|a, b| {
            a.limit_price
                .partial_cmp(&b.limit_price)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.trader.stable_key().cmp(&b.trader.stable_key()))
        });

        // Price-time priority matching
        let fills = match_books(&mut buys, &mut sells, min_price, max_price, cid);

        if fills.is_empty() {
            continue;
        }

        // Settle all fills strictly
        settle_fills(country, &fills);

        if let Some(equity) = country.stock_market.listed.get(&cid) {
            total_market_cap += equity.market_cap;
            total_turnover += equity.day_turnover;
            listed_count += 1;
        }
    }

    // Emit market close event (only when there is active trading)
    if listed_count > 0 && total_turnover > 0.0 {
        country.pending_events.push(SimEvent::StockMarketClose {
            country_name: country.name.clone(),
            day,
            total_market_cap: round_cents(total_market_cap),
            total_turnover: round_cents(total_turnover),
            listed_count,
        });
    }
}

// ---------------------------------------------------------------------------
// Price-time priority matching
// ---------------------------------------------------------------------------

fn match_books(
    buys: &mut Vec<LimitOrder>,
    sells: &mut Vec<LimitOrder>,
    min_price: f64,
    max_price: f64,
    cid: u16,
) -> Vec<TradeFill> {
    let mut fills = Vec::new();
    let mut bi = 0;
    let mut si = 0;

    while bi < buys.len() && si < sells.len() {
        if buys[bi].remaining_shares == 0 {
            bi += 1;
            continue;
        }
        if sells[si].remaining_shares == 0 {
            si += 1;
            continue;
        }

        let buy_limit = buys[bi].limit_price;
        let sell_limit = sells[si].limit_price;

        if buy_limit < sell_limit {
            break; // Book no longer crosses
        }

        // Midpoint price clamped by daily move bound
        let midpoint = (buy_limit + sell_limit) / 2.0;
        let trade_price = round_cents(midpoint.clamp(min_price, max_price));

        // Skip if clamped price violates either party's limit
        if trade_price > buy_limit || trade_price < sell_limit {
            break;
        }

        let fill_shares = buys[bi].remaining_shares.min(sells[si].remaining_shares);
        if fill_shares == 0 {
            break;
        }

        let cost = round_cents(fill_shares as f64 * trade_price);
        fills.push(TradeFill {
            company_id: cid,
            buyer: buys[bi].trader,
            seller: sells[si].trader,
            shares: fill_shares,
            price: trade_price,
        });

        buys[bi].remaining_shares -= fill_shares;
        buys[bi].remaining_cash_budget = (buys[bi].remaining_cash_budget - cost).max(0.0);
        sells[si].remaining_shares -= fill_shares;

        if buys[bi].remaining_shares == 0 {
            bi += 1;
        }
        if sells[si].remaining_shares == 0 {
            si += 1;
        }
    }

    fills
}

// ---------------------------------------------------------------------------
// Strict settlement — no synthetic fills, one path only
// ---------------------------------------------------------------------------

fn settle_fills(country: &mut crate::sim::country::Country, fills: &[TradeFill]) {
    for fill in fills {
        let cid = fill.company_id;
        let cost = round_cents(fill.shares as f64 * fill.price);
        let proceeds = cost; // same amount, different direction

        // Check buyer feasibility before touching any state
        let buyer_ok = match fill.buyer {
            TraderId::Person(pid) => country
                .people
                .iter()
                .find(|p| p.id == pid)
                .map_or(false, |p| p.cash_balance >= cost),
            TraderId::Bank => country.bank.reserve_balance >= cost,
        };

        // Check seller feasibility — prevents phantom-share creation when the same
        // person appears as seller in multiple fills (duplicate slot sampling).
        let seller_ok = match fill.seller {
            TraderId::Person(pid) => country
                .stock_market
                .person_portfolios
                .get(&pid)
                .and_then(|p| p.position(cid))
                .map_or(false, |pos| pos.shares >= fill.shares),
            TraderId::Bank => country
                .stock_market
                .bank_portfolio
                .position(cid)
                .map_or(false, |pos| pos.shares >= fill.shares),
        };

        if !buyer_ok || !seller_ok {
            continue;
        }

        // --- Buyer: pay cash / reserve, receive shares ---
        match fill.buyer {
            TraderId::Person(pid) => {
                if let Some(person) = country.people.iter_mut().find(|p| p.id == pid) {
                    person.cash_balance = round_cents(person.cash_balance - cost);
                }
                country
                    .stock_market
                    .person_portfolio_mut(pid)
                    .add_shares(cid, fill.shares, fill.price);
            }
            TraderId::Bank => {
                country.bank.reserve_balance =
                    round_cents(country.bank.reserve_balance - cost).max(0.0);
                country
                    .stock_market
                    .bank_portfolio
                    .add_shares(cid, fill.shares, fill.price);
            }
        }

        // --- Seller: give up shares, receive cash / reserve ---
        match fill.seller {
            TraderId::Person(pid) => {
                if let Some(portfolio) =
                    country.stock_market.person_portfolios.get_mut(&pid)
                {
                    portfolio.remove_shares(cid, fill.shares, fill.price);
                }
                if let Some(person) = country.people.iter_mut().find(|p| p.id == pid) {
                    person.cash_balance = round_cents(person.cash_balance + proceeds);
                }
            }
            TraderId::Bank => {
                country
                    .stock_market
                    .bank_portfolio
                    .remove_shares(cid, fill.shares, fill.price);
                country.bank.reserve_balance =
                    round_cents(country.bank.reserve_balance + proceeds);
            }
        }

        // Record the trade in the listing
        if let Some(equity) = country.stock_market.listed.get_mut(&cid) {
            equity.record_trade(fill.shares, fill.price);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{
        country::Country,
        person::Person,
        stock_market::ListedEquity,
        world::World,
    };

    fn make_person(id: u32, cash: f64) -> Person {
        let mut p = Person::new(id, 30, 0, id + 1);
        p.cash_balance = cash;
        p
    }

    fn make_world_with_listed(people: Vec<Person>, initial_price: f64) -> World {
        let mut country = Country::new(0, "Test".to_string(), people);
        let equity = ListedEquity::new(0, "TST".to_string(), initial_price, 1_000_000, 200_000);
        country.stock_market.listed.insert(0, equity);
        // Give bank some reserves for maker orders
        country.bank.reserve_balance = 500_000.0;
        World::new(vec![country])
    }

    #[test]
    fn retail_buy_sell_crosses_and_conserves_cash_and_shares() {
        // Person 0: has 1000 shares of company 0, wants to sell
        // Person 1: has cash, wants to buy
        let seller = make_person(0, 0.0);
        let buyer = make_person(1, 100_000.0);
        let mut world = make_world_with_listed(vec![seller, buyer], 10.0);
        world.countries[0].sim_params.stock_market.retail_noise_trade_prob = 1.0;

        // Manually give seller shares
        world.countries[0]
            .stock_market
            .person_portfolio_mut(0)
            .add_shares(0, 1000, 10.0);
        // Track free_float used: set seller's shares as part of free float
        // (already set to 200_000 in equity constructor)

        let initial_buyer_cash = world.countries[0]
            .people
            .iter()
            .find(|p| p.id == 1)
            .unwrap()
            .cash_balance;
        let initial_total_cash: f64 = world.countries[0]
            .people
            .iter()
            .map(|p| p.cash_balance)
            .sum::<f64>()
            + world.countries[0].bank.reserve_balance;

        world.run_daily_stock_market();

        let final_total_cash: f64 = world.countries[0]
            .people
            .iter()
            .map(|p| p.cash_balance)
            .sum::<f64>()
            + world.countries[0].bank.reserve_balance;

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
            .unwrap()
            .cash_balance;
        assert!(
            buyer_cash_after < initial_buyer_cash,
            "buyer did not spend any cash"
        );
    }

    #[test]
    fn noncrossing_books_produce_zero_fills() {
        // Buyer willing to pay 9.0, seller asking 11.0 — books don't cross
        let mut buys = vec![LimitOrder {
            trader: TraderId::Person(1),
            company_id: 0,
            side: OrderSide::Buy,
            limit_price: 9.0,
            remaining_shares: 100,
            remaining_cash_budget: 900.0,
        }];
        let mut sells = vec![LimitOrder {
            trader: TraderId::Person(0),
            company_id: 0,
            side: OrderSide::Sell,
            limit_price: 11.0,
            remaining_shares: 100,
            remaining_cash_budget: 0.0,
        }];

        let fills = match_books(&mut buys, &mut sells, 5.0, 15.0, 0);
        assert!(fills.is_empty(), "expected no fills for non-crossing book");
    }

    #[test]
    fn daily_price_move_is_bounded() {
        // Even if buyer bids 200 and seller asks 100 (midpoint 150), prev_close=100 → max cap 108
        let mut buys = vec![LimitOrder {
            trader: TraderId::Person(1),
            company_id: 0,
            side: OrderSide::Buy,
            limit_price: 200.0,
            remaining_shares: 50,
            remaining_cash_budget: 10_000.0,
        }];
        let mut sells = vec![LimitOrder {
            trader: TraderId::Person(0),
            company_id: 0,
            side: OrderSide::Sell,
            limit_price: 100.0,
            remaining_shares: 50,
            remaining_cash_budget: 0.0,
        }];

        // prev_close=100, max_move=0.08 → max_price=108, min_price=92
        let fills = match_books(&mut buys, &mut sells, 92.0, 108.0, 0);
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
        let mut world2 = make_world_with_listed(people.clone(), 10.0);

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
}
