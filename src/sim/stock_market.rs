use std::collections::HashMap;

use super::money::round_cents;

pub const STOCK_PRICE_HISTORY_WINDOW: usize = 120;
pub const STOCK_INDEX_HISTORY_WINDOW: usize = 120;

// ---------------------------------------------------------------------------
// Position / Portfolio
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub company_id: u16,
    pub shares: u64,
    pub avg_cost: f64,
    pub pledged_shares: u64,
}

impl Position {
    pub fn market_value(&self, last_price: f64) -> f64 {
        self.shares as f64 * last_price
    }

    pub fn unrealized_pnl(&self, last_price: f64) -> f64 {
        self.market_value(last_price) - (self.shares as f64 * self.avg_cost)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Portfolio {
    pub positions: Vec<Position>,
    pub realized_pnl_mtd: f64,
    pub dividends_mtd: f64,
    pub last_completed_month_dividends: f64,
}

impl Portfolio {
    pub fn position(&self, company_id: u16) -> Option<&Position> {
        self.positions.iter().find(|p| p.company_id == company_id)
    }

    /// Add `shares` bought at `cost_per_share` to the position; updates avg cost.
    pub fn add_shares(&mut self, company_id: u16, shares: u64, cost_per_share: f64) {
        if shares == 0 {
            return;
        }
        if let Some(pos) = self
            .positions
            .iter_mut()
            .find(|p| p.company_id == company_id)
        {
            let total_cost = pos.shares as f64 * pos.avg_cost + shares as f64 * cost_per_share;
            pos.shares += shares;
            pos.avg_cost = if pos.shares == 0 {
                0.0
            } else {
                total_cost / pos.shares as f64
            };
        } else {
            self.positions.push(Position {
                company_id,
                shares,
                avg_cost: cost_per_share,
                pledged_shares: 0,
            });
        }
    }

    /// Remove up to `shares` from position at `sale_price`; returns realized PnL.
    pub fn remove_shares(&mut self, company_id: u16, shares: u64, sale_price: f64) -> f64 {
        let Some(pos) = self
            .positions
            .iter_mut()
            .find(|p| p.company_id == company_id)
        else {
            return 0.0;
        };
        let removed = shares.min(pos.shares);
        let pnl = round_cents((sale_price - pos.avg_cost) * removed as f64);
        self.realized_pnl_mtd = round_cents(self.realized_pnl_mtd + pnl);
        pos.shares -= removed;
        pos.pledged_shares = pos.pledged_shares.min(pos.shares);
        if pos.shares == 0 {
            let cid = company_id;
            self.positions.retain(|p| p.company_id != cid);
        }
        pnl
    }

    /// Write all shares to zero (bankruptcy); books realized loss at avg cost.
    /// Returns the number of shares wiped.
    pub fn wipe_company(&mut self, company_id: u16) -> u64 {
        let Some(idx) = self
            .positions
            .iter()
            .position(|p| p.company_id == company_id)
        else {
            return 0;
        };
        let pos = &self.positions[idx];
        let shares = pos.shares;
        let loss = round_cents(-(pos.shares as f64 * pos.avg_cost));
        self.realized_pnl_mtd = round_cents(self.realized_pnl_mtd + loss);
        self.positions.swap_remove(idx);
        shares
    }

    /// Sum of market values across all positions.
    pub fn total_equity_value(&self, market: &StockMarket) -> f64 {
        self.positions
            .iter()
            .map(|pos| {
                let price = market
                    .listed
                    .get(&pos.company_id)
                    .map_or(0.0, |e| e.last_price);
                pos.shares as f64 * price
            })
            .sum()
    }

    /// Mark all shares as pledged (for borrowers).
    pub fn pledge_all(&mut self) {
        for pos in &mut self.positions {
            pos.pledged_shares = pos.shares;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Remap position company IDs after compaction.
    pub fn remap_company_ids(&mut self, old_to_new: &HashMap<u16, u16>) {
        for pos in &mut self.positions {
            if let Some(&new_id) = old_to_new.get(&pos.company_id) {
                pos.company_id = new_id;
            }
        }
        // positions whose company was removed (not in map) are already wiped
        // before this call
    }
}

// ---------------------------------------------------------------------------
// Listed equity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ListedEquity {
    pub company_id: u16,
    pub ticker: String,
    pub shares_outstanding: u64,
    pub free_float_shares: u64,
    pub last_price: f64,
    pub prev_close: f64,
    pub last_closed_day_pct_change: f64,
    pub day_volume: u64,
    pub day_turnover: f64,
    pub day_trades_count: u64,
    pub market_cap: f64,
    pub price_history: Vec<f64>,
    pub volume_history: Vec<u64>,
    pub market_cap_history: Vec<f64>,
    /// Per-share dividends paid per month (ring buffer, trailing 12 months).
    pub monthly_dps: [f64; 12],
    pub dps_cursor: u8,
}

impl ListedEquity {
    pub fn new(
        company_id: u16,
        ticker: String,
        initial_price: f64,
        shares_outstanding: u64,
        free_float_shares: u64,
    ) -> Self {
        let market_cap = initial_price * shares_outstanding as f64;
        Self {
            company_id,
            ticker,
            shares_outstanding,
            free_float_shares,
            last_price: initial_price,
            prev_close: initial_price,
            last_closed_day_pct_change: 0.0,
            day_volume: 0,
            day_turnover: 0.0,
            day_trades_count: 0,
            market_cap,
            price_history: Vec::with_capacity(STOCK_PRICE_HISTORY_WINDOW),
            volume_history: Vec::with_capacity(STOCK_PRICE_HISTORY_WINDOW),
            market_cap_history: Vec::with_capacity(STOCK_PRICE_HISTORY_WINDOW),
            monthly_dps: [0.0; 12],
            dps_cursor: 0,
        }
    }

    /// Sum of trailing 12-month per-share dividends.
    pub fn trailing_annual_dps(&self) -> f64 { self.monthly_dps.iter().sum() }

    pub fn record_monthly_dps(&mut self, per_share: f64) {
        self.monthly_dps[self.dps_cursor as usize] += per_share;
    }

    pub fn advance_dps_cursor(&mut self) {
        let next = (self.dps_cursor as usize + 1) % 12;
        self.dps_cursor = next as u8;
        self.monthly_dps[next] = 0.0;
    }

    pub fn day_pct_change(&self) -> f64 {
        if self.prev_close <= 0.0 {
            return 0.0;
        }
        (self.last_price - self.prev_close) / self.prev_close
    }

    pub fn closed_day_pct_change(&self) -> f64 {
        self.last_closed_day_pct_change
    }

    /// Closes the day: archives history, resets daily accumulators.
    pub fn record_day_close(&mut self) {
        self.last_closed_day_pct_change = self.day_pct_change();
        self.prev_close = self.last_price;
        self.market_cap = self.last_price * self.shares_outstanding as f64;

        push_capped(
            &mut self.price_history,
            self.last_price,
            STOCK_PRICE_HISTORY_WINDOW,
        );
        push_capped(
            &mut self.volume_history,
            self.day_volume,
            STOCK_PRICE_HISTORY_WINDOW,
        );
        push_capped(
            &mut self.market_cap_history,
            self.market_cap,
            STOCK_PRICE_HISTORY_WINDOW,
        );

        self.day_volume = 0;
        self.day_turnover = 0.0;
        self.day_trades_count = 0;
    }

    /// Apply a trade: update price, volume, turnover, trade count.
    pub fn record_trade(&mut self, shares: u64, price: f64) {
        self.day_volume = self.day_volume.saturating_add(shares);
        self.day_turnover = round_cents(self.day_turnover + price * shares as f64);
        self.day_trades_count = self.day_trades_count.saturating_add(1);
        self.last_price = price;
        self.market_cap = price * self.shares_outstanding as f64;
    }
}

// ---------------------------------------------------------------------------
// Daily index point
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct DailyIndexPoint {
    pub day: u64,
    pub index_value: f64,
    pub total_turnover: f64,
    pub bank_equity_exposure: f64,
}

// ---------------------------------------------------------------------------
// StockMarket
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct StockMarket {
    /// Sparse portfolios keyed by person id.
    pub person_portfolios: HashMap<u32, Portfolio>,
    /// Bank portfolio (institutional investor).
    pub bank_portfolio: Portfolio,
    /// Listed companies keyed by company_id.
    pub listed: HashMap<u16, ListedEquity>,
    /// Country-level daily index history.
    pub daily_index_history: Vec<DailyIndexPoint>,
    /// Index base value (market cap at first listing).
    pub index_base: f64,
    // MTD trackers
    pub ipo_raised_mtd: f64,
    pub dividends_paid_mtd: f64,
    pub last_completed_month_dividends_paid: f64,
    pub forced_liquidations_mtd: f64,
    pub day_active_traders_count: usize,
    pub day_buy_orders_count: u64,
    pub day_sell_orders_count: u64,
}

impl Default for StockMarket {
    fn default() -> Self {
        Self {
            person_portfolios: HashMap::new(),
            bank_portfolio: Portfolio::default(),
            listed: HashMap::new(),
            daily_index_history: Vec::new(),
            index_base: 0.0,
            ipo_raised_mtd: 0.0,
            dividends_paid_mtd: 0.0,
            last_completed_month_dividends_paid: 0.0,
            forced_liquidations_mtd: 0.0,
            day_active_traders_count: 0,
            day_buy_orders_count: 0,
            day_sell_orders_count: 0,
        }
    }
}

impl StockMarket {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_market_cap(&self) -> f64 {
        self.listed.values().map(|e| e.market_cap).sum()
    }

    pub fn total_daily_turnover(&self) -> f64 {
        self.listed.values().map(|e| e.day_turnover).sum()
    }

    pub fn listed_count(&self) -> usize {
        self.listed.len()
    }

    pub fn bank_equity_exposure(&self) -> f64 {
        self.bank_portfolio.total_equity_value(self)
    }

    pub fn is_listed(&self, company_id: u16) -> bool {
        self.listed.contains_key(&company_id)
    }

    /// Compute the composite index value (base 1000 at first listing).
    pub fn index_value(&self) -> f64 {
        let total_cap = self.total_market_cap();
        if self.index_base <= 0.0 || total_cap <= 0.0 {
            return 0.0;
        }
        1_000.0 * total_cap / self.index_base
    }

    /// Record today's index snapshot and close all listed equities.
    pub fn record_day_close(&mut self, day: u64) {
        let bank_exposure = self.bank_equity_exposure();
        let total_turnover = self.total_daily_turnover();
        let index_value = self.index_value();

        push_capped(
            &mut self.daily_index_history,
            DailyIndexPoint {
                day,
                index_value,
                total_turnover,
                bank_equity_exposure: bank_exposure,
            },
            STOCK_INDEX_HISTORY_WINDOW,
        );

        for equity in self.listed.values_mut() {
            equity.record_day_close();
        }
        self.day_active_traders_count = 0;
        self.day_buy_orders_count = 0;
        self.day_sell_orders_count = 0;
    }

    /// Reset MTD accumulators (called at month end after capturing history).
    pub fn reset_mtd_trackers(&mut self) {
        self.ipo_raised_mtd = 0.0;
        self.last_completed_month_dividends_paid = self.dividends_paid_mtd;
        self.dividends_paid_mtd = 0.0;
        self.forced_liquidations_mtd = 0.0;
        for portfolio in self.person_portfolios.values_mut() {
            portfolio.realized_pnl_mtd = 0.0;
            portfolio.last_completed_month_dividends = portfolio.dividends_mtd;
            portfolio.dividends_mtd = 0.0;
        }
        self.bank_portfolio.realized_pnl_mtd = 0.0;
        self.bank_portfolio.last_completed_month_dividends = self.bank_portfolio.dividends_mtd;
        self.bank_portfolio.dividends_mtd = 0.0;
        for equity in self.listed.values_mut() {
            equity.advance_dps_cursor();
        }
    }

    fn ticker_in_use_by_other(&self, ticker: &str, company_id: u16) -> bool {
        self.listed
            .iter()
            .any(|(listed_company_id, listed_equity)| {
                *listed_company_id != company_id
                    && listed_equity.ticker.eq_ignore_ascii_case(ticker)
            })
    }

    fn distinct_ticker_for_listing(&self, company_id: u16, preferred: &str) -> String {
        let preferred = preferred.trim().to_ascii_uppercase();
        let base = if preferred.is_empty() {
            format!("S{:03}", company_id % 1000)
        } else {
            preferred
        };

        if !self.ticker_in_use_by_other(&base, company_id) {
            return base;
        }

        let mut suffix = 2_u32;
        loop {
            let candidate = format!("{base}{suffix}");
            if !self.ticker_in_use_by_other(&candidate, company_id) {
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }

    /// Add a listing while preserving index continuity.
    /// Returns the ticker that was ultimately assigned to the listing.
    pub fn insert_listing_preserve_index(
        &mut self,
        company_id: u16,
        mut equity: ListedEquity,
    ) -> String {
        let before_cap = self.total_market_cap();
        equity.ticker = self.distinct_ticker_for_listing(company_id, &equity.ticker);
        let assigned_ticker = equity.ticker.clone();
        let added_cap = equity.market_cap.max(0.0);

        self.listed.insert(company_id, equity);

        let after_cap = self.total_market_cap();
        if self.index_base <= 0.0 {
            self.index_base = after_cap.max(0.0);
            return assigned_ticker;
        }

        if before_cap > 0.0 && after_cap > 0.0 && added_cap > 0.0 {
            self.index_base *= after_cap / before_cap;
        }

        assigned_ticker
    }

    /// Called on bankruptcy: wipe all portfolio positions for `company_id`,
    /// remove from listings. Returns total shares wiped across all holders.
    pub fn delist_company(&mut self, company_id: u16) -> u64 {
        let before_cap = self.total_market_cap();
        let removed_cap = self
            .listed
            .remove(&company_id)
            .map_or(0.0, |equity| equity.market_cap.max(0.0));
        let after_cap = self.total_market_cap();
        if self.index_base > 0.0 && before_cap > 0.0 && removed_cap > 0.0 {
            if after_cap > 0.0 {
                self.index_base *= after_cap / before_cap;
            } else {
                self.index_base = 0.0;
            }
        }
        let mut total_wiped = 0_u64;
        for portfolio in self.person_portfolios.values_mut() {
            total_wiped = total_wiped.saturating_add(portfolio.wipe_company(company_id));
        }
        total_wiped = total_wiped.saturating_add(self.bank_portfolio.wipe_company(company_id));
        total_wiped
    }

    /// After compaction, remap all company_id references.
    /// `old_to_new` maps old company_id → new company_id for surviving companies.
    pub fn remap_company_ids(&mut self, old_to_new: &HashMap<u16, u16>) {
        // Remap listed equities
        let old_listed: HashMap<u16, ListedEquity> = std::mem::take(&mut self.listed);
        for (old_id, mut equity) in old_listed {
            if let Some(&new_id) = old_to_new.get(&old_id) {
                equity.company_id = new_id;
                self.listed.insert(new_id, equity);
            }
            // else: already delisted by delist_company
        }
        // Remap portfolios
        for portfolio in self.person_portfolios.values_mut() {
            portfolio.remap_company_ids(old_to_new);
        }
        self.bank_portfolio.remap_company_ids(old_to_new);
    }

    /// Get or create a person portfolio.
    pub fn person_portfolio_mut(&mut self, person_id: u32) -> &mut Portfolio {
        self.person_portfolios.entry(person_id).or_default()
    }

    /// Remove empty portfolios to save memory.
    pub fn gc_portfolios(&mut self) {
        self.person_portfolios
            .retain(|_, portfolio| !portfolio.is_empty());
    }

    /// Generate a ticker symbol from company name.
    pub fn generate_ticker(company_id: u16, name: &str) -> String {
        let ticker: String = name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(4)
            .map(|c| c.to_ascii_uppercase())
            .collect();
        if ticker.is_empty() {
            format!("S{:03}", company_id % 1000)
        } else {
            ticker
        }
    }

    /// Compute a deterministic valuation anchor for IPO pricing.
    /// Uses a P/S multiple on trailing revenue with debt/cash adjustments.
    pub fn ipo_valuation_anchor(
        monthly_revenue: f64,
        last_month_profit: f64,
        cash_balance: f64,
        debt_balance: f64,
        initial_shares: u64,
    ) -> f64 {
        if initial_shares == 0 {
            return 0.01;
        }
        // Annualized revenue x 0.5 P/S, adjusted for profitability
        let ann_rev = monthly_revenue * 12.0;
        let ps_multiple = if last_month_profit > 0.0 { 0.5 } else { 0.2 };
        let enterprise_value = ann_rev * ps_multiple;
        let equity_value = (enterprise_value + cash_balance * 0.35 - debt_balance).max(0.0);
        let price_per_share = equity_value / initial_shares as f64;
        price_per_share.max(0.01)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Push to a capped ring-like Vec (remove oldest when over capacity).
fn push_capped<T>(v: &mut Vec<T>, item: T, limit: usize) {
    if v.len() >= limit {
        v.remove(0);
    }
    v.push(item);
}

// ---------------------------------------------------------------------------
// A lightweight snapshot for the UI (built in snapshot.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct StockRowSnapshot {
    pub ticker: String,
    pub company_name: String,
    pub last_price: String,
    pub day_pct: String,
    pub market_cap: String,
    pub volume: String,
    pub float_shares: String,
    pub bank_pct: String,
    pub is_selected: bool,
    pub is_even: bool,
}

#[cfg(test)]
#[path = "stock_market_tests.rs"]
mod tests;
