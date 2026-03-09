mod accessors;
mod bank_history;
mod cross_country_banking;
mod cross_country_stock_investing;
mod cross_market_stock;
mod export;
mod goods_market;
mod history;
mod invariants;
mod monetary;
mod stock_market;
mod tick;
#[cfg(feature = "bench-trace")]
mod trace_tick;

pub use bank_history::BankMetricPoint;
pub use export::{
    CountryExportData, MonthlyExportData, StockDailyCountryExportData, StockDailyEquityExportData,
};
pub use invariants::InvariantViolation;

use std::collections::VecDeque;

use super::{country::Country, event::SimEvent};

pub const HISTORY_LIMIT: usize = 120;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MetricPoint {
    pub month: u64,
    pub population: f64,
    pub total_money_supply: f64,
    pub household_cash_total: f64,
    pub company_cash_total: f64,
    pub expected_money_supply: f64,
    pub money_supply_gap: f64,
    pub created_money_outstanding: f64,
    pub bank_reserve: f64,
    pub treasury_balance: f64,
    pub budget_balance: f64,
    pub monthly_tax_collected: f64,
    pub monthly_gov_spending: f64,
    pub gov_debt: f64,
    pub monthly_money_minted: f64,
    pub cumulative_money_minted: f64,
    pub monthly_mint_fraction: f64,
    pub monthly_goods_import_spending: f64,
    pub monthly_goods_export_revenue: f64,
    pub monthly_unmet_need_events: f64,
    pub monthly_severe_unmet_need_events: f64,
    pub monthly_chronic_unmet_need_count: f64,
    pub monthly_essential_debt_spending: f64,
    pub cpi_index: f64,
    pub inflation_rate: f64,
    pub policy_rate: f64,
    pub deposit_rate: f64,
    pub loan_rate: f64,
    pub monthly_savings_interest_paid: f64,
    pub bonds_outstanding: f64,
    pub stock_market_cap: f64,
    pub stock_bank_equity_exposure: f64,
    pub housing_price_index: f64,
    pub monthly_rent: f64,
    pub homeownership_rate: f64,
    pub housing_vacancy_rate: f64,
    pub monthly_foreclosures: f64,
    pub monthly_home_purchases: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub(crate) day: u64,
    pub(crate) generation_seed: u64,
    pub(crate) countries: Vec<Country>,
    pub(crate) paused: bool,
    pub(crate) policy_rate: f64,
    pub(crate) world_history: VecDeque<MetricPoint>,
    pub(crate) country_history: Vec<VecDeque<MetricPoint>>,
    pub(crate) bank_history: std::collections::HashMap<(usize, u16), VecDeque<BankMetricPoint>>,
    pub(crate) event_log: VecDeque<SimEvent>,
    pub(crate) export_queue: Vec<MonthlyExportData>,
    pub(crate) export_queue_active: bool,
    pub(crate) stock_country_export_queue: Vec<StockDailyCountryExportData>,
    pub(crate) stock_equity_export_queue: Vec<StockDailyEquityExportData>,
    /// Tariff rate lookup table: `tariff_rates_by_good[good_idx][seller_country_idx]`.
    /// Rebuilt in-place by `rebuild_tariff_matrix_in_place()` after tariff changes.
    pub(crate) tariff_rates_by_good: Vec<Vec<f64>>,
}

#[cfg(test)]
mod tests;
