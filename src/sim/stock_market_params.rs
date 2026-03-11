#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StockMarketParams {
    pub enabled: bool,
    /// Max participants sampled per day per country.
    pub daily_participant_cap: usize,
    /// Fraction of cash a person must keep as reserve before buying stocks.
    pub investor_cash_reserve_ratio: f64,
    /// Max fraction of new shares (dilution) allowed in an IPO.
    pub ipo_max_dilution: f64,
    /// Initial shares outstanding issued at IPO.
    pub ipo_initial_shares: u64,
    /// Fraction of last-month profit paid as dividends.
    pub dividend_payout_ratio: f64,
    /// Company must have at least this many months of cash runway to pay dividends.
    pub dividend_min_runway_months: f64,
    /// Discount applied to last price when bank force-liquidates pledged shares.
    pub margin_liquidation_discount: f64,
    /// Max fraction of bank reserves the bank may deploy in equity.
    pub bank_max_equity_reserve_fraction: f64,
    /// Number of the largest companies to list at simulation start (0 = none).
    pub initial_listing_count: usize,
    /// Free-float fraction of initial_shares sold to investors at seed IPO.
    pub initial_float_fraction: f64,
    /// Market-maker spread in basis points (50 bps = 0.50%). Bank quotes bid/ask this wide.
    pub maker_spread_bps: u16,
    /// Maximum allowed single-day price move as a fraction (0.08 = ±8%).
    pub max_daily_price_move_pct: f64,
    /// Probability that a sampled retail trader places a random noise order regardless of signal.
    pub retail_noise_trade_prob: f64,
    /// Maximum fraction of free-float shares that any single order may represent.
    pub max_order_fraction_of_float: f64,
    /// Share of population sampled daily as participants.
    pub participant_population_fraction: f64,
    /// Initial bank reserve injected (and removed from company cash) to enable maker liquidity.
    pub maker_bootstrap_reserve: f64,
    /// Minimum private-company age in months before growth IPO eligibility.
    pub ipo_min_company_age_months: u16,
    /// Minimum profitable streak for growth IPO path.
    pub ipo_growth_min_profitable_months: u8,
    /// Minimum average monthly revenue for growth IPO path.
    pub ipo_growth_min_monthly_revenue: f64,
    /// Lower cash-runway bound for growth IPO candidates.
    pub ipo_growth_min_cash_runway_months: f64,
    /// Upper cash-runway bound for growth IPO candidates.
    pub ipo_growth_max_cash_runway_months: f64,
    /// Target payroll runway funded by primary raise in growth IPOs.
    pub ipo_primary_raise_target_runway_months: f64,
    /// Maximum fraction of founder holdings allowed for secondary sale.
    pub ipo_founder_secondary_cap_fraction: f64,
    /// Minimum subscription fill ratio for growth IPO success.
    pub ipo_min_fill_ratio_growth: f64,
    /// Minimum subscription fill ratio for distress IPO success.
    pub ipo_min_fill_ratio_distress: f64,
    /// Maximum number of new non-seed listings per month.
    pub ipo_max_new_listings_per_month: u16,
    /// Cooldown months after failed IPO attempts.
    pub ipo_retry_cooldown_months: u8,
    pub retail_distress_sell_cash_threshold: f64,
    pub retail_take_profit_gain_threshold: f64,
    pub retail_random_sell_prob: f64,
    pub secondary_offering_missed_payment_trigger: u8,
    pub secondary_offering_discount: f64,
    pub secondary_offering_max_dilution: f64,
    /// Enable bank investment into listed equities from all countries.
    pub cross_country_bank_investing_enabled: bool,
    /// Fraction of each bank's reserves deployed monthly into cross-country equities.
    pub cross_country_bank_invest_fraction_monthly: f64,
    /// Maximum number of companies that may be listed simultaneously per country.
    /// New growth IPOs are blocked once the market reaches this cap.
    pub max_total_listings: usize,
}

impl Default for StockMarketParams {
    fn default() -> Self {
        Self {
            enabled: true,
            daily_participant_cap: 400,
            investor_cash_reserve_ratio: 0.30,
            ipo_max_dilution: 0.30,
            ipo_initial_shares: 1_000_000,
            dividend_payout_ratio: 0.40,
            dividend_min_runway_months: 3.0,
            margin_liquidation_discount: 0.05,
            bank_max_equity_reserve_fraction: 0.15,
            initial_listing_count: 5,
            initial_float_fraction: 0.20,
            maker_spread_bps: 20,
            max_daily_price_move_pct: 0.08,
            retail_noise_trade_prob: 0.12,
            max_order_fraction_of_float: 0.05,
            participant_population_fraction: 0.004,
            maker_bootstrap_reserve: 2_000_000.0,
            ipo_min_company_age_months: 3,
            ipo_growth_min_profitable_months: 1,
            ipo_growth_min_monthly_revenue: 20_000.0,
            ipo_growth_min_cash_runway_months: 0.0,
            ipo_growth_max_cash_runway_months: 120.0,
            ipo_primary_raise_target_runway_months: 4.0,
            ipo_founder_secondary_cap_fraction: 0.15,
            ipo_min_fill_ratio_growth: 0.50,
            ipo_min_fill_ratio_distress: 0.80,
            ipo_max_new_listings_per_month: 4,
            ipo_retry_cooldown_months: 2,
            retail_distress_sell_cash_threshold: 500.0,
            retail_take_profit_gain_threshold: 0.40,
            retail_random_sell_prob: 0.22,
            secondary_offering_missed_payment_trigger: 1,
            secondary_offering_discount: 0.10,
            secondary_offering_max_dilution: 0.15,
            cross_country_bank_investing_enabled: true,
            cross_country_bank_invest_fraction_monthly: 0.02,
            max_total_listings: 50,
        }
    }
}
