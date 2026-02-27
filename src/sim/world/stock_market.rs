use super::World;
use crate::sim::{event::SimEvent, money::round_cents};

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
    }
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}

fn run_country_stock_market(country: &mut crate::sim::country::Country, day: u64) {
    let params = country.sim_params.stock_market;
    let cap = params.daily_participant_cap;
    let reserve_ratio = params.investor_cash_reserve_ratio;

    // Build list of listed company IDs and their current prices
    let listed_ids: Vec<u16> = country.stock_market.listed.keys().copied().collect();
    if listed_ids.is_empty() {
        return;
    }

    // -----------------------------------------------------------------------
    // Sample participants deterministically
    // -----------------------------------------------------------------------
    let people_count = country.people.len();
    let sample_count = cap.min(people_count);

    // Per stock: collect net buy/sell pressure
    // We process each sampled person for each stock
    let mut buy_orders: std::collections::HashMap<u16, Vec<(u32, u64, f64)>> =
        std::collections::HashMap::new(); // company_id -> [(person_id, shares, limit_price)]
    let mut sell_orders: std::collections::HashMap<u16, Vec<(u32, u64, f64)>> =
        std::collections::HashMap::new();

    for slot in 0..sample_count {
        let seed = hash64(
            day.wrapping_mul(0xb5ad4eceda1ce2a9) ^ (u64::from(country.id) << 32) ^ slot as u64,
        );
        let person_idx = (hash64(seed ^ 0x1234) as usize) % people_count;
        let person = &country.people[person_idx];
        let person_id = person.id;

        for &cid in &listed_ids {
            let last_price = country
                .stock_market
                .listed
                .get(&cid)
                .map_or(0.0, |e| e.last_price);
            if last_price <= 0.0 {
                continue;
            }

            let has_position = country
                .stock_market
                .person_portfolios
                .get(&person_id)
                .and_then(|p| p.position(cid))
                .map_or(false, |pos| pos.shares > 0);

            // Simple momentum signal: compare last price to avg of price history
            let price_history = country
                .stock_market
                .listed
                .get(&cid)
                .map(|e| e.price_history.as_slice())
                .unwrap_or(&[]);
            let avg_price = if price_history.is_empty() {
                last_price
            } else {
                price_history.iter().sum::<f64>() / price_history.len() as f64
            };
            let signal = (last_price - avg_price) / avg_price.max(0.01); // positive = above trend

            // Sell intent: person has position AND (price is above trend OR needs cash)
            if has_position && signal > 0.05 {
                let pos = country
                    .stock_market
                    .person_portfolios
                    .get(&person_id)
                    .and_then(|p| p.position(cid))
                    .cloned();
                if let Some(pos) = pos {
                    let free_shares = pos.shares.saturating_sub(pos.pledged_shares);
                    if free_shares > 0 {
                        let sell_seed = hash64(seed ^ u64::from(cid) ^ 0xabcd);
                        let fraction = unit_interval(sell_seed) * 0.3 + 0.05; // sell 5–35%
                        let shares = ((pos.shares as f64 * fraction).ceil() as u64)
                            .min(free_shares)
                            .max(1);
                        sell_orders
                            .entry(cid)
                            .or_default()
                            .push((person_id, shares, last_price));
                    }
                }
            }

            // Buy intent: person has surplus cash AND price is below trend
            if signal < -0.02 || (!has_position && signal < 0.10) {
                let reserve = person.cash_balance * reserve_ratio;
                let investable = round_cents((person.cash_balance - reserve).max(0.0));
                if investable >= last_price {
                    let buy_seed = hash64(seed ^ u64::from(cid) ^ 0x5678);
                    let fraction = unit_interval(buy_seed) * 0.4 + 0.05;
                    let amount = round_cents(investable * fraction);
                    let shares = (amount / last_price) as u64;
                    if shares > 0 {
                        buy_orders
                            .entry(cid)
                            .or_default()
                            .push((person_id, shares, last_price));
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Clear each stock: match buy/sell, update price
    // -----------------------------------------------------------------------
    let mut total_market_cap = 0.0_f64;
    let mut total_turnover = 0.0_f64;
    let mut listed_count = 0_usize;

    for cid in &listed_ids {
        let last_price = country
            .stock_market
            .listed
            .get(cid)
            .map_or(0.0, |e| e.last_price);
        if last_price <= 0.0 {
            continue;
        }

        let buys = buy_orders.remove(cid).unwrap_or_default();
        let sells = sell_orders.remove(cid).unwrap_or_default();

        let total_buy_shares: u64 = buys.iter().map(|(_, s, _)| s).sum();
        let total_sell_shares: u64 = sells.iter().map(|(_, s, _)| s).sum();

        let matched_shares: u64;
        let clearing_price: f64;
        let net_demand = total_buy_shares as i64 - total_sell_shares as i64;

        if net_demand == 0 && total_buy_shares == 0 {
            // No activity on this listing.
            continue;
        }

        // Determine matched volume and price impact
        if total_buy_shares >= total_sell_shares {
            matched_shares = total_sell_shares;
            // Excess demand → price up, capped at +5%
            let demand_ratio = if total_sell_shares == 0 {
                1.0
            } else {
                total_buy_shares as f64 / total_sell_shares as f64
            };
            let impact = ((demand_ratio - 1.0) * 0.01).min(0.05);
            clearing_price = round_cents(last_price * (1.0 + impact));
        } else {
            matched_shares = total_buy_shares;
            // Excess supply → price down, capped at -5%
            let supply_ratio = total_sell_shares as f64 / total_buy_shares.max(1) as f64;
            let impact = ((supply_ratio - 1.0) * 0.01).min(0.05);
            clearing_price = round_cents(last_price * (1.0 - impact)).max(0.01);
        };

        if matched_shares == 0 {
            // Do not synthesize trades or move price on one-sided/no-execution days.
            continue;
        }

        // Apply sell transactions
        let mut sell_shares_remaining = matched_shares;
        for (pid, shares, _) in &sells {
            if sell_shares_remaining == 0 {
                break;
            }
            let sell = (*shares).min(sell_shares_remaining);
            if sell == 0 {
                continue;
            }
            let proceeds = round_cents(sell as f64 * clearing_price);
            if let Some(portfolio) = country.stock_market.person_portfolios.get_mut(pid) {
                portfolio.remove_shares(*cid, sell, clearing_price);
            }
            if let Some(person) = country.people.iter_mut().find(|p| p.id == *pid) {
                person.cash_balance = round_cents(person.cash_balance + proceeds);
            }
            // Bank absorbs shares (acts as buyer for unmatched sells in excess-supply case)
            country
                .stock_market
                .bank_portfolio
                .add_shares(*cid, sell, clearing_price);
            country.bank.reserve_balance =
                round_cents(country.bank.reserve_balance - proceeds).max(0.0);
            sell_shares_remaining = sell_shares_remaining.saturating_sub(sell);
        }

        // Apply buy transactions
        let mut buy_shares_remaining = matched_shares;
        for (pid, shares, _) in &buys {
            if buy_shares_remaining == 0 {
                break;
            }
            let buy = (*shares).min(buy_shares_remaining);
            if buy == 0 {
                continue;
            }
            let cost = round_cents(buy as f64 * clearing_price);
            if let Some(person) = country.people.iter_mut().find(|p| p.id == *pid) {
                if person.cash_balance < cost {
                    continue;
                }
                person.cash_balance = round_cents(person.cash_balance - cost);
            } else {
                continue;
            }
            country
                .stock_market
                .person_portfolio_mut(*pid)
                .add_shares(*cid, buy, clearing_price);
            buy_shares_remaining = buy_shares_remaining.saturating_sub(buy);
        }

        // Record the trade in the listing
        if let Some(equity) = country.stock_market.listed.get_mut(cid) {
            equity.record_trade(matched_shares, clearing_price);
        }

        if let Some(equity) = country.stock_market.listed.get(cid) {
            total_market_cap += equity.market_cap;
            total_turnover += equity.day_turnover;
            listed_count += 1;
        }
    }

    // Close all equities for the day (archives price/volume history)
    country.stock_market.record_day_close(day);

    // Emit market close event (only if there is active trading)
    if listed_count > 0 && total_turnover > 0.0 {
        country.pending_events.push(SimEvent::StockMarketClose {
            country_name: country.name.clone(),
            day,
            total_market_cap: round_cents(total_market_cap),
            total_turnover: round_cents(total_turnover),
            listed_count,
        });
    }

    // GC empty portfolios periodically (every 30 days)
    if day.is_multiple_of(30) {
        country.stock_market.gc_portfolios();
    }
}
