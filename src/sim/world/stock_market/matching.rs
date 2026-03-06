mod execution;

use super::*;

pub(in crate::sim::world) use execution::{match_books, run_country_stock_market};

const RETAIL_PAR_PARTICIPANT_THRESHOLD: usize = 4_096;
const MATCH_PAR_SYMBOL_THRESHOLD: usize = 8;
const MATCH_PAR_ORDER_THRESHOLD: u64 = 512;

type PersonIndexLookup = HashMap<u32, usize>;

#[derive(Debug, Clone, Copy)]
struct SymbolContext {
    company_id: u16,
    last_price: f64,
    min_price: f64,
    max_price: f64,
    trend: f64,
    value_gap: f64,
    day_sentiment: f64,
    max_order_by_float: u64,
    market_open: bool,
    retail_enabled: bool,
}

#[derive(Debug)]
struct CountryStockScratch {
    buy_orders: Vec<Vec<LimitOrder>>,
    sell_orders: Vec<Vec<LimitOrder>>,
    active_traders: HashSet<u64>,
    buy_orders_count: u64,
    sell_orders_count: u64,
}

impl CountryStockScratch {
    fn with_symbol_count(symbol_count: usize) -> Self {
        Self {
            buy_orders: vec![Vec::new(); symbol_count],
            sell_orders: vec![Vec::new(); symbol_count],
            active_traders: HashSet::new(),
            buy_orders_count: 0,
            sell_orders_count: 0,
        }
    }

    fn push_buy(&mut self, symbol_idx: usize, order: LimitOrder) {
        let trader_key = order.trader.stable_key();
        self.buy_orders[symbol_idx].push(order);
        self.buy_orders_count = self.buy_orders_count.saturating_add(1);
        self.active_traders.insert(trader_key);
    }

    fn push_sell(&mut self, symbol_idx: usize, order: LimitOrder) {
        let trader_key = order.trader.stable_key();
        self.sell_orders[symbol_idx].push(order);
        self.sell_orders_count = self.sell_orders_count.saturating_add(1);
        self.active_traders.insert(trader_key);
    }

    fn merge_from(&mut self, other: Self) {
        let Self {
            buy_orders,
            sell_orders,
            active_traders,
            buy_orders_count,
            sell_orders_count,
        } = other;

        for (target, mut source) in self.buy_orders.iter_mut().zip(buy_orders.into_iter()) {
            target.append(&mut source);
        }
        for (target, mut source) in self.sell_orders.iter_mut().zip(sell_orders.into_iter()) {
            target.append(&mut source);
        }
        self.active_traders.extend(active_traders);
        self.buy_orders_count = self.buy_orders_count.saturating_add(buy_orders_count);
        self.sell_orders_count = self.sell_orders_count.saturating_add(sell_orders_count);
    }
}

struct SymbolMatchResult {
    symbol_idx: usize,
    fills: Vec<TradeFill>,
}

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

fn build_symbol_contexts(
    country: &crate::sim::country::Country,
    day: u64,
    max_float_frac: f64,
    max_move: f64,
) -> Vec<SymbolContext> {
    let mut listed_ids: Vec<u16> = country.stock_market().listed.keys().copied().collect();
    listed_ids.sort_unstable();

    listed_ids
        .into_iter()
        .filter_map(|cid| {
            let equity = country.stock_market().listed.get(&cid)?;
            let last_price = equity.last_price;
            let float_shares = equity.free_float_shares;
            let market_open = last_price > 0.0 && float_shares > 0;
            let max_order_by_float = ((float_shares as f64 * max_float_frac).floor() as u64).max(1);

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

            let maybe_value_gap = country.companies().get(cid as usize).map(|company| {
                let debt = company.loan.as_ref().map_or(0.0, |loan| loan.debt_balance);
                let anchor = StockMarket::ipo_valuation_anchor(
                    company.average_monthly_revenue(),
                    company.last_month_profit,
                    company.cash_balance,
                    debt,
                    equity.shares_outstanding,
                );
                if last_price > 0.0 {
                    (anchor - last_price) / last_price
                } else {
                    0.0
                }
            });

            let retail_enabled = market_open && maybe_value_gap.is_some();
            let value_gap = maybe_value_gap.unwrap_or(0.0);
            let day_sentiment = (unit_interval(hash64(
                day ^ (u64::from(country.id()) << 20) ^ (u64::from(cid) << 4) ^ 0x1bad_b002,
            )) - 0.5)
                * 0.12;

            let base = if equity.prev_close > 0.0 {
                equity.prev_close
            } else {
                last_price
            };
            let max_price = round_cents(base * (1.0 + max_move));
            let min_price = round_cents((base * (1.0 - max_move)).max(0.01));

            Some(SymbolContext {
                company_id: cid,
                last_price,
                min_price,
                max_price,
                trend,
                value_gap,
                day_sentiment,
                max_order_by_float,
                market_open,
                retail_enabled,
            })
        })
        .collect()
}

fn append_retail_orders_for_person(
    country: &crate::sim::country::Country,
    day: u64,
    slot: usize,
    person_idx: usize,
    symbol_contexts: &[SymbolContext],
    params: &crate::sim::params::StockMarketParams,
    scratch: &mut CountryStockScratch,
) {
    let reserve_ratio = params.investor_cash_reserve_ratio;
    let noise_prob = params.retail_noise_trade_prob;

    let slot_seed = hash64(
        day.wrapping_mul(0xb5ad4eceda1ce2a9) ^ (u64::from(country.id()) << 32) ^ slot as u64,
    );
    let person = &country.people()[person_idx];
    let person_id = person.id;
    let person_cash = person.cash_balance;
    let portfolio = country.stock_market().person_portfolios.get(&person_id);

    for (symbol_idx, context) in symbol_contexts.iter().enumerate() {
        if !context.retail_enabled {
            continue;
        }

        let position = portfolio.and_then(|entry| entry.position(context.company_id));
        let has_position = position.is_some_and(|pos| pos.shares > 0);
        let free_shares = position.map_or(0, |pos| pos.shares.saturating_sub(pos.pledged_shares));

        let distress_sell = has_position
            && free_shares > 0
            && person_cash < params.retail_distress_sell_cash_threshold;

        let order_seed = hash64(slot_seed ^ u64::from(context.company_id) ^ 0xf00d);
        let noise = (unit_interval(hash64(order_seed ^ 0x1111)) - 0.5) * 2.0 * noise_prob;
        let trader_style_roll = unit_interval(hash64(order_seed ^ 0x7777));

        let value_gap = context.value_gap.clamp(-0.08, 0.08);
        let directional_core = if trader_style_roll < 0.45 {
            0.20 * value_gap - 0.80 * context.trend
        } else if trader_style_roll < 0.80 {
            -0.10 * value_gap - 0.90 * context.trend
        } else {
            -0.60 * context.trend
        };
        let directional = directional_core.clamp(-0.12, 0.12) + noise + context.day_sentiment;
        let buy_threshold = if has_position { 0.03 } else { 0.05 };
        let sell_threshold = 0.03;
        let mut want_buy = directional > buy_threshold;
        let mut want_sell = has_position && directional < -sell_threshold;

        let mut profit_taking = false;
        if has_position
            && free_shares > 0
            && !want_sell
            && !distress_sell
            && let Some(pos) = position
            && pos.avg_cost > 0.0
            && context.last_price > pos.avg_cost * (1.0 + params.retail_take_profit_gain_threshold)
        {
            profit_taking = true;
            want_sell = true;
        }
        if !want_buy && !want_sell {
            if has_position
                && unit_interval(hash64(order_seed ^ 0x8888)) < params.retail_random_sell_prob
            {
                want_sell = true;
            } else if !has_position && unit_interval(hash64(order_seed ^ 0x9999)) < 0.02 {
                want_buy = true;
            }
        }

        if distress_sell || want_sell {
            if free_shares > 0 {
                let (sell_frac, disc) = if distress_sell {
                    (
                        unit_interval(hash64(order_seed ^ 0x2222)) * 0.40 + 0.50,
                        unit_interval(hash64(order_seed ^ 0x3333)) * 0.08,
                    )
                } else if profit_taking {
                    (
                        unit_interval(hash64(order_seed ^ 0x2222)) * 0.30 + 0.30,
                        unit_interval(hash64(order_seed ^ 0x3333)) * 0.02,
                    )
                } else {
                    (
                        unit_interval(hash64(order_seed ^ 0x2222)) * 0.45 + 0.10,
                        unit_interval(hash64(order_seed ^ 0x3333)) * 0.03,
                    )
                };
                let shares = ((free_shares as f64 * sell_frac).ceil() as u64)
                    .min(free_shares)
                    .min(context.max_order_by_float)
                    .max(1);
                let limit_price = round_cents((context.last_price * (1.0 - disc)).max(0.01));
                scratch.push_sell(
                    symbol_idx,
                    LimitOrder {
                        trader: TraderId::Person(person_id),
                        company_id: context.company_id,
                        side: OrderSide::Sell,
                        limit_price,
                        remaining_shares: shares,
                        remaining_cash_budget: 0.0,
                    },
                );
            }
            continue;
        }

        if !want_buy {
            continue;
        }

        let investable = round_cents((person_cash - person_cash * reserve_ratio).max(0.0));
        if investable < context.last_price {
            continue;
        }
        let buy_frac = unit_interval(hash64(order_seed ^ 0x4444)) * 0.45 + 0.08;
        let budget = round_cents(investable * buy_frac);
        let shares = (budget / context.last_price) as u64;
        let shares = shares.min(context.max_order_by_float).max(1);
        if shares == 0 {
            continue;
        }
        let prem = unit_interval(hash64(order_seed ^ 0x5555)) * 0.001;
        let limit_price = round_cents(context.last_price * (1.0 + prem));
        let cash_budget = round_cents(shares as f64 * limit_price);
        scratch.push_buy(
            symbol_idx,
            LimitOrder {
                trader: TraderId::Person(person_id),
                company_id: context.company_id,
                side: OrderSide::Buy,
                limit_price,
                remaining_shares: shares,
                remaining_cash_budget: cash_budget,
            },
        );
    }
}

fn build_person_index_lookup(country: &crate::sim::country::Country) -> PersonIndexLookup {
    country
        .people()
        .iter()
        .enumerate()
        .map(|(idx, person)| (person.id, idx))
        .collect()
}

fn run_matching_for_symbol(
    context: SymbolContext,
    mut buys: Vec<LimitOrder>,
    mut sells: Vec<LimitOrder>,
) -> Vec<TradeFill> {
    if !context.market_open || buys.is_empty() || sells.is_empty() {
        return Vec::new();
    }

    buys.sort_by(|left, right| {
        right
            .limit_price
            .partial_cmp(&left.limit_price)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.trader.stable_key().cmp(&right.trader.stable_key()))
    });
    sells.sort_by(|left, right| {
        left.limit_price
            .partial_cmp(&right.limit_price)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.trader.stable_key().cmp(&right.trader.stable_key()))
    });

    match_books(
        &mut buys,
        &mut sells,
        context.min_price,
        context.max_price,
        context.company_id,
    )
}

// ---------------------------------------------------------------------------
// Main daily market function
// ---------------------------------------------------------------------------
