use super::*;

// ---------------------------------------------------------------------------
// Daily stock export types
// ---------------------------------------------------------------------------

/// One row per country per day in `stock_daily_country_*.csv`.
#[derive(Debug, Clone, PartialEq)]
pub struct StockDailyCountryExportData {
    pub day: u64,
    pub country_id: u16,
    pub country_name: String,
    pub listed_count: usize,
    pub trades_count: u64,
    pub total_volume: u64,
    pub total_turnover: f64,
    pub index_value: f64,
    pub bank_equity_exposure: f64,
    pub bank_reserve_balance: f64,
    pub active_traders_count: usize,
    pub buy_orders_count: u64,
    pub sell_orders_count: u64,
}

/// One row per listed equity per day in `stock_daily_equity_*.csv`.
#[derive(Debug, Clone, PartialEq)]
pub struct StockDailyEquityExportData {
    pub day: u64,
    pub country_id: u16,
    pub country_name: String,
    pub company_id: u16,
    pub ticker: String,
    pub close_price: f64,
    pub day_return_pct: f64,
    pub volume: u64,
    pub turnover: f64,
    pub trades_count: u64,
    pub bank_position_shares: u64,
    pub free_float_shares: u64,
}

impl World {
    /// Push daily stock snapshot after all countries finish trading.
    /// Called from `run_daily_stock_market` when export is active.
    pub(in crate::sim::world) fn push_daily_stock_snapshot(&mut self) {
        let day = self.day;
        for country in &self.countries {
            if !country.sim_params.stock_market.enabled {
                continue;
            }
            if country.stock_market.listed.is_empty() {
                continue;
            }

            let trades_count: u64 = country
                .stock_market
                .listed
                .values()
                .map(|e| e.day_trades_count)
                .sum();
            let total_volume: u64 = country
                .stock_market
                .listed
                .values()
                .map(|e| e.day_volume)
                .sum();
            let total_turnover: f64 = country
                .stock_market
                .listed
                .values()
                .map(|e| e.day_turnover)
                .sum();

            self.stock_country_export_queue
                .push(StockDailyCountryExportData {
                    day,
                    country_id: country.id,
                    country_name: country.name.clone(),
                    listed_count: country.stock_market.listed_count(),
                    trades_count,
                    total_volume,
                    total_turnover,
                    index_value: country.stock_market.index_value(),
                    bank_equity_exposure: country.stock_market.bank_equity_exposure(),
                    bank_reserve_balance: country.bank.reserve_balance,
                    active_traders_count: country.stock_market.day_active_traders_count,
                    buy_orders_count: country.stock_market.day_buy_orders_count,
                    sell_orders_count: country.stock_market.day_sell_orders_count,
                });

            for (cid, equity) in &country.stock_market.listed {
                let bank_shares = country
                    .stock_market
                    .bank_portfolio
                    .position(*cid)
                    .map_or(0, |p| p.shares);
                self.stock_equity_export_queue
                    .push(StockDailyEquityExportData {
                        day,
                        country_id: country.id,
                        country_name: country.name.clone(),
                        company_id: *cid,
                        ticker: equity.ticker.clone(),
                        close_price: equity.last_price,
                        day_return_pct: equity.day_pct_change() * 100.0,
                        volume: equity.day_volume,
                        turnover: equity.day_turnover,
                        trades_count: equity.day_trades_count,
                        bank_position_shares: bank_shares,
                        free_float_shares: equity.free_float_shares,
                    });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonthlyExportData {
    pub month: u64,
    pub generation_seed: u64,
    pub countries: Vec<CountryExportData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CountryExportData {
    pub month: u64,
    pub id: u16,
    pub name: String,

    // country_metrics
    pub population: usize,
    pub treasury_balance: f64,
    pub monthly_tax_collected: f64,
    pub monthly_tariff_collected: f64,
    pub monthly_gov_spending: f64,
    pub monthly_budget_balance: f64,
    pub gov_debt: f64,
    pub monthly_household_spending: f64,
    pub monthly_goods_import_spending: f64,
    pub monthly_goods_export_revenue: f64,
    pub monthly_unmet_need_events: u64,
    pub monthly_essential_debt_spending: f64,
    pub total_money_supply: f64,
    pub household_cash_total: f64,
    pub company_cash_total: f64,
    pub bank_reserve_balance: f64,
    pub bank_created_money_outstanding: f64,
    pub expected_money_supply: f64,
    pub money_supply_gap: f64,

    // people_agg
    pub employed_count: usize,
    pub unemployed_count: usize,
    pub labor_force_count: usize,
    pub non_participating_count: usize,
    pub headline_non_employed_rate_pct: f64,
    pub avg_age_years: f64,
    pub avg_cash_balance: f64,
    pub total_gross_pay: f64,
    pub total_tax_paid: f64,
    pub total_goods_spending: f64,
    pub count_with_active_loans: usize,
    pub avg_loan_balance: f64,
    pub count_with_unmet_needs: usize,

    // company_agg
    pub total_companies: usize,
    pub total_employees: u32,
    pub total_open_positions: usize,
    pub total_company_cash: f64,
    pub total_revenue: f64,
    pub total_payroll: f64,
    pub total_profit: f64,
    pub count_profitable: usize,
    pub count_unprofitable: usize,
    pub total_units_produced: u32,
    pub total_units_sold: u32,
    pub total_units_exported: u32,
    pub count_public_utility: usize,
    pub count_backfill: usize,
    pub count_companies_with_loans: usize,

    // events
    pub births: u64,
    pub deaths: u64,
    pub job_switches: u64,
    pub layoffs_total: u64,
    pub separations_quits: u64,
    pub separations_retirements: u64,
    pub separations_layoffs_non_distress: u64,
    pub separations_total: u64,
    pub companies_founded: u64,
    pub bankruptcies: u64,
    pub wage_cuts: u64,
    pub loans_issued: u64,
    pub loans_ended: u64,
    pub loans_defaulted: u64,
    pub debt_distress_events: u64,
    pub gov_transfers: u64,
    pub hires_total: u64,
    pub stock_ipos: u64,
    pub stock_delistings: u64,
    pub stock_margin_calls: u64,

    // monetary policy + bonds + CPI
    pub bonds_outstanding: f64,
    pub bond_coupon_rate: f64,
    pub monthly_coupon_paid: f64,
    pub monthly_bonds_issued: f64,
    pub monthly_bonds_matured: f64,
    pub cpi_index: f64,
    pub inflation_rate: f64,
    pub policy_rate: f64,
}
