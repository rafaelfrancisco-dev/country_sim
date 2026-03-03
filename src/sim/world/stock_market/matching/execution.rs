use rayon::prelude::*;

use super::*;

pub(in crate::sim::world) fn run_country_stock_market(
    country: &mut crate::sim::country::Country,
    day: u64,
) {
    let params = country.sim_params.stock_market;
    let cap = params.daily_participant_cap;
    let spread_frac = params.maker_spread_bps as f64 / 10_000.0;
    let max_float_frac = params.max_order_fraction_of_float;
    let max_move = params.max_daily_price_move_pct;

    let symbol_contexts = build_symbol_contexts(country, day, max_float_frac, max_move);
    if symbol_contexts.is_empty() {
        return;
    }

    let people_count = country.people.len();
    if people_count == 0 {
        return;
    }

    let scaled_by_population =
        (people_count as f64 * params.participant_population_fraction).round() as usize;
    let scaled_by_listings = params
        .min_participants_per_listed
        .saturating_mul(symbol_contexts.len());
    let sample_target = cap.max(scaled_by_population).max(scaled_by_listings);
    let sample_count = sample_target.min(people_count);

    let sampled_people =
        deterministic_sample_people_indices(people_count, sample_count, day, country.id);
    let should_parallel_retail =
        sampled_people.len() >= RETAIL_PAR_PARTICIPANT_THRESHOLD && symbol_contexts.len() > 1;
    let mut scratch = if should_parallel_retail {
        sampled_people
            .par_iter()
            .enumerate()
            .fold(
                || CountryStockScratch::with_symbol_count(symbol_contexts.len()),
                |mut local, (slot, person_idx)| {
                    append_retail_orders_for_person(
                        country,
                        day,
                        slot,
                        *person_idx,
                        &symbol_contexts,
                        &params,
                        &mut local,
                    );
                    local
                },
            )
            .reduce(
                || CountryStockScratch::with_symbol_count(symbol_contexts.len()),
                |mut left, right| {
                    left.merge_from(right);
                    left
                },
            )
    } else {
        let mut local = CountryStockScratch::with_symbol_count(symbol_contexts.len());
        for (slot, person_idx) in sampled_people.into_iter().enumerate() {
            append_retail_orders_for_person(
                country,
                day,
                slot,
                person_idx,
                &symbol_contexts,
                &params,
                &mut local,
            );
        }
        local
    };

    let bank_equity_exposure = country.stock_market.bank_equity_exposure();
    let bank_equity_budget = params.bank_max_equity_reserve_fraction * country.bank.reserve_balance;
    let bank_equity_headroom = (bank_equity_budget - bank_equity_exposure).max(0.0);

    for (symbol_idx, context) in symbol_contexts.iter().copied().enumerate() {
        if !context.market_open {
            continue;
        }

        let half_spread = spread_frac / 2.0;
        let bid_price = round_cents((context.last_price * (1.0 - half_spread)).max(0.01));
        let ask_price = round_cents(context.last_price * (1.0 + half_spread));

        let bid_shares_by_headroom = if bid_price > 0.0 {
            (bank_equity_headroom / bid_price).floor() as u64
        } else {
            0
        };
        let bid_shares = bid_shares_by_headroom.min(context.max_order_by_float);
        if bid_shares > 0 {
            scratch.push_buy(
                symbol_idx,
                LimitOrder {
                    trader: TraderId::Bank,
                    company_id: context.company_id,
                    side: OrderSide::Buy,
                    limit_price: bid_price,
                    remaining_shares: bid_shares,
                    remaining_cash_budget: round_cents(bid_shares as f64 * bid_price),
                },
            );
        }

        let bank_inventory = country
            .stock_market
            .bank_portfolio
            .position(context.company_id)
            .map_or(0, |position| position.shares);
        // Scale up ask size when bank is over its equity budget to actively reduce exposure.
        let exposure_ratio = if bank_equity_budget > 0.0 {
            bank_equity_exposure / bank_equity_budget
        } else {
            0.0
        };
        let pressure_multiplier = if exposure_ratio > 1.0 {
            exposure_ratio.min(5.0)
        } else {
            1.0
        };
        let ask_cap = ((context.max_order_by_float as f64 * pressure_multiplier).floor() as u64)
            .max(context.max_order_by_float);
        let ask_shares = bank_inventory.min(ask_cap);
        if ask_shares > 0 {
            scratch.push_sell(
                symbol_idx,
                LimitOrder {
                    trader: TraderId::Bank,
                    company_id: context.company_id,
                    side: OrderSide::Sell,
                    limit_price: ask_price,
                    remaining_shares: ask_shares,
                    remaining_cash_budget: 0.0,
                },
            );
        }
    }

    country.stock_market.day_active_traders_count = scratch.active_traders.len();
    country.stock_market.day_buy_orders_count = scratch.buy_orders_count;
    country.stock_market.day_sell_orders_count = scratch.sell_orders_count;

    let should_parallel_match = symbol_contexts.len() >= MATCH_PAR_SYMBOL_THRESHOLD
        && scratch
            .buy_orders_count
            .saturating_add(scratch.sell_orders_count)
            >= MATCH_PAR_ORDER_THRESHOLD;

    let CountryStockScratch {
        buy_orders,
        sell_orders,
        ..
    } = scratch;

    let matched_symbols = if should_parallel_match {
        buy_orders
            .into_par_iter()
            .zip(sell_orders.into_par_iter())
            .enumerate()
            .map(|(symbol_idx, (buys, sells))| SymbolMatchResult {
                symbol_idx,
                fills: run_matching_for_symbol(symbol_contexts[symbol_idx], buys, sells),
            })
            .collect::<Vec<_>>()
    } else {
        buy_orders
            .into_iter()
            .zip(sell_orders)
            .enumerate()
            .map(|(symbol_idx, (buys, sells))| SymbolMatchResult {
                symbol_idx,
                fills: run_matching_for_symbol(symbol_contexts[symbol_idx], buys, sells),
            })
            .collect::<Vec<_>>()
    };

    let person_index_lookup = build_person_index_lookup(country);
    let mut total_market_cap = 0.0_f64;
    let mut total_turnover = 0.0_f64;
    let mut listed_count = 0_usize;

    for result in matched_symbols {
        if result.fills.is_empty() {
            continue;
        }

        settle_fills(country, &person_index_lookup, &result.fills);
        let company_id = symbol_contexts[result.symbol_idx].company_id;
        if let Some(equity) = country.stock_market.listed.get(&company_id) {
            total_market_cap += equity.market_cap;
            total_turnover += equity.day_turnover;
            listed_count += 1;
        }
    }

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

pub(in crate::sim::world) fn match_books(
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
            break;
        }

        let midpoint = (buy_limit + sell_limit) / 2.0;
        let trade_price = round_cents(midpoint.clamp(min_price, max_price));
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

fn settle_fills(
    country: &mut crate::sim::country::Country,
    person_index_lookup: &PersonIndexLookup,
    fills: &[TradeFill],
) {
    for fill in fills {
        let cid = fill.company_id;
        let cost = round_cents(fill.shares as f64 * fill.price);
        let proceeds = cost;

        let buyer_ok = match fill.buyer {
            TraderId::Person(pid) => person_index_lookup
                .get(&pid)
                .and_then(|idx| country.people.get(*idx))
                .map_or(false, |person| person.cash_balance >= cost),
            TraderId::Bank => country.bank.reserve_balance >= cost,
        };

        let seller_ok = match fill.seller {
            TraderId::Person(pid) => country
                .stock_market
                .person_portfolios
                .get(&pid)
                .and_then(|portfolio| portfolio.position(cid))
                .map_or(false, |position| position.shares >= fill.shares),
            TraderId::Bank => country
                .stock_market
                .bank_portfolio
                .position(cid)
                .map_or(false, |position| position.shares >= fill.shares),
        };

        if !buyer_ok || !seller_ok {
            continue;
        }

        match fill.buyer {
            TraderId::Person(pid) => {
                if let Some(&person_idx) = person_index_lookup.get(&pid)
                    && let Some(person) = country.people.get_mut(person_idx)
                {
                    person.cash_balance = round_cents(person.cash_balance - cost);
                }
                country.stock_market.person_portfolio_mut(pid).add_shares(
                    cid,
                    fill.shares,
                    fill.price,
                );
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

        match fill.seller {
            TraderId::Person(pid) => {
                if let Some(portfolio) = country.stock_market.person_portfolios.get_mut(&pid) {
                    portfolio.remove_shares(cid, fill.shares, fill.price);
                }
                if let Some(&person_idx) = person_index_lookup.get(&pid)
                    && let Some(person) = country.people.get_mut(person_idx)
                {
                    person.cash_balance = round_cents(person.cash_balance + proceeds);
                }
            }
            TraderId::Bank => {
                country
                    .stock_market
                    .bank_portfolio
                    .remove_shares(cid, fill.shares, fill.price);
                country.bank.reserve_balance = round_cents(country.bank.reserve_balance + proceeds);
            }
        }

        if let Some(equity) = country.stock_market.listed.get_mut(&cid) {
            equity.record_trade(fill.shares, fill.price);
        }
    }
}
