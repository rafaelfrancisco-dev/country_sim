use rayon::prelude::*;

use super::*;
use crate::sim::world::fx::CrossBorderFlowKind;

pub(in crate::sim::world) fn run_world_stock_market(world: &mut World, day: u64) {
    let symbol_contexts = build_symbol_contexts(world, day);
    if symbol_contexts.is_empty() {
        return;
    }

    let mut scratch = WorldStockScratch::with_symbol_count(symbol_contexts.len());
    let mut person_index_lookups = Vec::with_capacity(world.country_count());
    let mut per_country_stats = Vec::with_capacity(world.country_count());

    for country in world.countries() {
        person_index_lookups.push(build_person_index_lookup(country));
    }

    for country in world.countries() {
        let params = country.sim_params().stock_market;
        if !params.enabled || country.people().is_empty() {
            continue;
        }

        let people_count = country.people().len();
        let scaled_by_population =
            (people_count as f64 * params.participant_population_fraction).round() as usize;
        let sample_target = params.daily_participant_cap.max(scaled_by_population);
        let sample_count = sample_target.min(people_count);
        let sampled_people =
            deterministic_sample_people_indices(people_count, sample_count, day, country.id());
        let should_parallel =
            sampled_people.len() >= RETAIL_PAR_PARTICIPANT_THRESHOLD && symbol_contexts.len() > 1;

        let country_scratch = if should_parallel {
            sampled_people
                .par_iter()
                .enumerate()
                .fold(
                    || WorldStockScratch::with_symbol_count(symbol_contexts.len()),
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
                    || WorldStockScratch::with_symbol_count(symbol_contexts.len()),
                    |mut left, right| {
                        left.merge_from(right);
                        left
                    },
                )
        } else {
            let mut local = WorldStockScratch::with_symbol_count(symbol_contexts.len());
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
        per_country_stats.push((
            country.id(),
            country_scratch.active_traders.len(),
            country_scratch.buy_orders_count,
            country_scratch.sell_orders_count,
        ));
        scratch.merge_from(country_scratch);

        append_bank_market_maker_orders(
            world,
            country.id(),
            &symbol_contexts,
            &params,
            &mut scratch,
        );
    }

    for (country_id, active_traders, buy_orders_count, sell_orders_count) in per_country_stats {
        if let Some(country) = world
            .countries
            .iter_mut()
            .find(|country| country.id() == country_id)
        {
            country.stock_market.day_active_traders_count = active_traders;
            country.stock_market.day_buy_orders_count = buy_orders_count;
            country.stock_market.day_sell_orders_count = sell_orders_count;
        }
    }

    let should_parallel_match = symbol_contexts.len() >= MATCH_PAR_SYMBOL_THRESHOLD
        && scratch
            .buy_orders_count
            .saturating_add(scratch.sell_orders_count)
            >= MATCH_PAR_ORDER_THRESHOLD;

    let WorldStockScratch {
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

    for result in matched_symbols {
        if result.fills.is_empty() {
            continue;
        }
        settle_fills(world, &person_index_lookups, &result.fills);
        let context = symbol_contexts[result.symbol_idx];
        if let Some(country) = world.country_mut(context.listing_country_idx)
            && let Some(equity) = country
                .stock_market
                .listed
                .get(&context.security_id.company_id)
            && equity.day_turnover > 0.0
        {
            country.push_pending_event(SimEvent::StockMarketClose {
                country_name: country.name().into(),
                day,
                total_market_cap: round_cents(country.stock_market.total_market_cap()),
                total_turnover: round_cents(country.stock_market.total_daily_turnover()),
                listed_count: country.stock_market.listed_count(),
            });
        }
    }
}

fn append_bank_market_maker_orders(
    world: &World,
    country_id: u16,
    symbol_contexts: &[SymbolContext],
    params: &crate::sim::params::StockMarketParams,
    scratch: &mut WorldStockScratch,
) {
    let Some(country) = world.countries().iter().find(|c| c.id() == country_id) else {
        return;
    };
    let stock_bank_id = country
        .stock_market_bank_id()
        .unwrap_or(country.primary_bank().id);
    let Some(bank) = country.bank_by_id(stock_bank_id) else {
        return;
    };

    let spread_frac = params.maker_spread_bps as f64 / 10_000.0;
    let bank_equity_exposure = world.market_bank_portfolio_value(country_id);
    let bank_equity_budget = params.bank_max_equity_reserve_fraction * bank.reserve_balance;
    // Each bid placed consumes headroom so the bank cannot commit its full
    // equity budget to every symbol at once; remaining headroom is re-divided
    // across the symbols not yet quoted to spread bids fairly.
    let mut bank_equity_headroom = (bank_equity_budget - bank_equity_exposure).max(0.0);
    let mut open_symbols_remaining = symbol_contexts
        .iter()
        .filter(|context| context.market_open)
        .count() as u64;

    for (symbol_idx, context) in symbol_contexts.iter().copied().enumerate() {
        if !context.market_open {
            continue;
        }
        let bid_budget_this_symbol = bank_equity_headroom / open_symbols_remaining.max(1) as f64;
        open_symbols_remaining = open_symbols_remaining.saturating_sub(1);

        // Quote around a fundamental-blended mid (instead of pure last price) so
        // the maker gradually pulls quotes toward value in both directions.
        let anchor_shift = 0.05 * context.value_gap.clamp(-0.5, 0.5);
        let quote_mid = (context.last_price * (1.0 + anchor_shift))
            .clamp(context.min_price, context.max_price);
        let half_spread = spread_frac / 2.0;
        let bid_price = round_cents((quote_mid * (1.0 - half_spread)).max(0.01));
        let ask_price = round_cents(quote_mid * (1.0 + half_spread));

        let bid_shares_by_headroom = if bid_price > 0.0 {
            (bid_budget_this_symbol / bid_price).floor() as u64
        } else {
            0
        };
        let bid_shares = bid_shares_by_headroom.min(context.max_order_by_float);
        if bid_shares > 0 {
            let bid_commitment = round_cents(bid_shares as f64 * bid_price);
            bank_equity_headroom = (bank_equity_headroom - bid_commitment).max(0.0);
            scratch.push_buy(
                symbol_idx,
                LimitOrder {
                    trader: TraderId::Bank {
                        country_id,
                        bank_id: stock_bank_id,
                    },
                    security_id: context.security_id,
                    side: OrderSide::Buy,
                    limit_price: bid_price,
                    remaining_shares: bid_shares,
                    remaining_cash_budget: bid_commitment,
                },
            );
        }

        let bank_inventory = country
            .stock_market()
            .bank_portfolio
            .position(context.security_id)
            .map_or(0, |position| position.shares);
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
        // When investing mode is active only offer excess above the per-symbol target for
        // sale; when over-exposed sell everything to reduce risk.
        let target_per_symbol = if params.cross_country_bank_investing_enabled
            && exposure_ratio <= 1.0
            && bank_equity_budget > 0.0
            && context.last_price > 0.0
            && !symbol_contexts.is_empty()
        {
            (bank_equity_budget / symbol_contexts.len() as f64 / context.last_price) as u64
        } else {
            0
        };
        let ask_shares = bank_inventory
            .saturating_sub(target_per_symbol)
            .min(ask_cap);
        if ask_shares > 0 {
            scratch.push_sell(
                symbol_idx,
                LimitOrder {
                    trader: TraderId::Bank {
                        country_id,
                        bank_id: stock_bank_id,
                    },
                    security_id: context.security_id,
                    side: OrderSide::Sell,
                    limit_price: ask_price,
                    remaining_shares: ask_shares,
                    remaining_cash_budget: 0.0,
                },
            );
        }
    }
}

pub(in crate::sim::world) fn match_books(
    buys: &mut [LimitOrder],
    sells: &mut [LimitOrder],
    min_price: f64,
    max_price: f64,
    security_id: crate::sim::stock_market::StockSecurityId,
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
            security_id,
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

fn settle_fills(
    world: &mut World,
    person_index_lookups: &[PersonIndexLookup],
    fills: &[TradeFill],
) {
    for fill in fills {
        let security_id = fill.security_id;
        let buyer_country_id = match fill.buyer {
            TraderId::Person { country_id, .. } | TraderId::Bank { country_id, .. } => country_id,
        };
        let seller_country_id = match fill.seller {
            TraderId::Person { country_id, .. } | TraderId::Bank { country_id, .. } => country_id,
        };
        let Some(buyer_country_idx) = world
            .countries()
            .iter()
            .position(|country| country.id() == buyer_country_id)
        else {
            continue;
        };
        let Some(seller_country_idx) = world
            .countries()
            .iter()
            .position(|country| country.id() == seller_country_id)
        else {
            continue;
        };
        let Some(listing_country_idx) = world
            .countries()
            .iter()
            .position(|country| country.id() == security_id.country_id)
        else {
            continue;
        };

        let cost = round_cents(fill.shares as f64 * fill.price);
        let buyer_cost = if buyer_country_idx == listing_country_idx {
            cost
        } else {
            world.local_amount_from_base(
                buyer_country_idx,
                world.base_amount_from_local(listing_country_idx, cost),
            )
        };
        let seller_proceeds = if seller_country_idx == listing_country_idx {
            cost
        } else {
            world.local_amount_from_base(
                seller_country_idx,
                world.base_amount_from_local(listing_country_idx, cost),
            )
        };
        let buyer_ok = match fill.buyer {
            TraderId::Person {
                country_id: _,
                person_id,
            } => person_index_lookups
                .get(buyer_country_idx)
                .and_then(|lookup| lookup.get(&person_id))
                .and_then(|idx| world.countries[buyer_country_idx].people().get(*idx))
                .is_some_and(|person| person.cash_balance >= buyer_cost),
            TraderId::Bank { bank_id, .. } => world.countries[buyer_country_idx]
                .bank_by_id(bank_id)
                .is_some_and(|bank| bank.reserve_balance >= buyer_cost),
        };
        let seller_ok = match fill.seller {
            TraderId::Person {
                country_id: _,
                person_id,
            } => world.countries[seller_country_idx]
                .stock_market()
                .person_portfolios
                .get(&person_id)
                .and_then(|portfolio| portfolio.position(security_id))
                .is_some_and(|position| position.shares >= fill.shares),
            TraderId::Bank { .. } => world.countries[seller_country_idx]
                .stock_market()
                .bank_portfolio
                .position(security_id)
                .is_some_and(|position| position.shares >= fill.shares),
        };
        if !buyer_ok || !seller_ok {
            continue;
        }

        if buyer_country_idx != listing_country_idx {
            let buyer_settlement =
                world.convert_between_countries(buyer_country_idx, listing_country_idx, buyer_cost);
            let buyer_fx_available = world.countries[buyer_country_idx]
                .public_bank()
                .map(|bank| bank.fx_reserve_assets_base)
                .unwrap_or(0.0);
            if buyer_fx_available + 0.01 < buyer_settlement.payer_amount_base {
                continue;
            }
        }
        if seller_country_idx != listing_country_idx {
            let seller_settlement =
                world.convert_between_countries(listing_country_idx, seller_country_idx, cost);
            let seller_fx_available = world.countries[listing_country_idx]
                .public_bank()
                .map(|bank| bank.fx_reserve_assets_base)
                .unwrap_or(0.0);
            if seller_fx_available + 0.01 < seller_settlement.payer_amount_base {
                continue;
            }
        }

        if buyer_country_idx != listing_country_idx {
            world.settle_cross_border_cash(
                CrossBorderFlowKind::StockTrade,
                buyer_country_idx,
                listing_country_idx,
                buyer_cost,
            );
        }
        if seller_country_idx != listing_country_idx {
            world.settle_cross_border_cash(
                CrossBorderFlowKind::StockTrade,
                listing_country_idx,
                seller_country_idx,
                cost,
            );
        }

        match fill.buyer {
            TraderId::Person { person_id, .. } => {
                if let Some(&person_idx) = person_index_lookups[buyer_country_idx].get(&person_id)
                    && let Some(person) = world.countries[buyer_country_idx]
                        .people_mut()
                        .get_mut(person_idx)
                {
                    person.cash_balance = round_cents(person.cash_balance - buyer_cost);
                }
                world.countries[buyer_country_idx]
                    .stock_market_mut()
                    .person_portfolio_mut(person_id)
                    .add_shares(security_id, fill.shares, fill.price);
            }
            TraderId::Bank { bank_id, .. } => {
                if let Some(bank) = world.countries[buyer_country_idx].bank_by_id_mut(bank_id) {
                    bank.reserve_balance = round_cents(bank.reserve_balance - buyer_cost).max(0.0);
                }
                world.countries[buyer_country_idx]
                    .stock_market_mut()
                    .bank_portfolio
                    .add_shares(security_id, fill.shares, fill.price);
            }
        }

        match fill.seller {
            TraderId::Person { person_id, .. } => {
                if let Some(portfolio) = world.countries[seller_country_idx]
                    .stock_market_mut()
                    .person_portfolios
                    .get_mut(&person_id)
                {
                    portfolio.remove_shares(security_id, fill.shares, fill.price);
                }
                if let Some(&person_idx) = person_index_lookups[seller_country_idx].get(&person_id)
                    && let Some(person) = world.countries[seller_country_idx]
                        .people_mut()
                        .get_mut(person_idx)
                {
                    person.cash_balance = round_cents(person.cash_balance + seller_proceeds);
                }
            }
            TraderId::Bank { bank_id, .. } => {
                world.countries[seller_country_idx]
                    .stock_market_mut()
                    .bank_portfolio
                    .remove_shares(security_id, fill.shares, fill.price);
                if let Some(bank) = world.countries[seller_country_idx].bank_by_id_mut(bank_id) {
                    bank.reserve_balance = round_cents(bank.reserve_balance + seller_proceeds);
                }
            }
        }

        if let Some(equity) = world.countries[listing_country_idx]
            .stock_market_mut()
            .listed
            .get_mut(&security_id.company_id)
        {
            equity.record_trade(fill.shares, fill.price);
        }
    }
}
