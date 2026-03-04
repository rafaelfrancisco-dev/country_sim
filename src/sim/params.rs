use super::person::EducationTrack;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DemographicParams {
    pub births_per_1000_per_year: f64,
}

impl Default for DemographicParams {
    fn default() -> Self {
        Self {
            births_per_1000_per_year: 17.0,
        }
    }
}

impl DemographicParams {
    pub fn births_per_person_per_day(&self) -> f64 {
        self.births_per_1000_per_year.max(0.0) / 1000.0 / 365.0
    }

    pub fn annual_death_probability_for_age_years(&self, age_years: u8) -> f64 {
        match age_years {
            0 => 0.005,
            1..=14 => 0.0002,
            15..=24 => 0.0007,
            25..=34 => 0.0013,
            35..=44 => 0.0020,
            45..=54 => 0.0045,
            55..=64 => 0.0095,
            65..=74 => 0.0220,
            75..=84 => 0.0570,
            85..=94 => 0.1400,
            _ => 0.3000,
        }
    }

    pub fn daily_death_probability_for_age_years(&self, age_years: u8) -> f64 {
        let annual = self
            .annual_death_probability_for_age_years(age_years)
            .clamp(0.0, 1.0);
        1.0 - (1.0 - annual).powf(1.0 / 365.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EducationParams {
    pub basic_only_share: f64,
    pub high_school_share: f64,
    pub bachelor_share: f64,
    pub medical_doctor_share: f64,
}

impl Default for EducationParams {
    fn default() -> Self {
        Self {
            basic_only_share: 0.38,
            high_school_share: 0.37,
            bachelor_share: 0.22,
            medical_doctor_share: 0.03,
        }
    }
}

impl EducationParams {
    pub fn pick_track(&self, roll: f64) -> EducationTrack {
        let roll = roll.clamp(0.0, 1.0);
        let threshold_basic = self.basic_only_share;
        let threshold_high = threshold_basic + self.high_school_share;
        let threshold_bachelor = threshold_high + self.bachelor_share;

        if roll < threshold_basic {
            EducationTrack::BasicOnly
        } else if roll < threshold_high {
            EducationTrack::HighSchool
        } else if roll < threshold_bachelor {
            EducationTrack::Bachelor
        } else {
            EducationTrack::MedicalDoctor
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LaborParams {
    pub baseline_unemployment_rate: f64,
    pub seek_base: f64,
    pub seek_age_slope: f64,
    pub seek_cash_pressure_scale: f64,
    pub seek_cash_reference: f64,
    pub seek_probability_cap: f64,
}

impl Default for LaborParams {
    fn default() -> Self {
        Self {
            baseline_unemployment_rate: 0.06,
            seek_base: 0.010,
            seek_age_slope: 0.0003,
            seek_cash_pressure_scale: 0.05,
            seek_cash_reference: 3_000.0,
            seek_probability_cap: 0.15,
        }
    }
}

impl LaborParams {
    pub fn daily_seek_probability(&self, age_years: u8, cash_balance: f64) -> f64 {
        let age_component = (i32::from(age_years) - 18).max(0) as f64 * self.seek_age_slope;
        let cash_pressure = ((self.seek_cash_reference - cash_balance) / self.seek_cash_reference)
            .clamp(0.0, 1.0)
            * self.seek_cash_pressure_scale;

        (self.seek_base + age_component + cash_pressure).min(self.seek_probability_cap)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LaborDemandParams {
    /// Baseline monthly opening target as a fraction of current company headcount.
    pub replacement_opening_rate: f64,
    /// Minimum baseline openings for companies that currently employ workers.
    pub min_openings_per_active_company: u16,
}

impl Default for LaborDemandParams {
    fn default() -> Self {
        Self {
            replacement_opening_rate: 0.03,
            min_openings_per_active_company: 3,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CompanyGrowthParams {
    pub min_profitable_months: u8,
    pub min_cash_reserve: f64,
    pub payroll_reserve_multiplier: f64,
    pub profit_per_opening: f64,
    pub max_monthly_openings: u16,
}

impl Default for CompanyGrowthParams {
    fn default() -> Self {
        Self {
            min_profitable_months: 3,
            min_cash_reserve: 100_000.0,
            payroll_reserve_multiplier: 1.5,
            profit_per_opening: 50_000.0,
            max_monthly_openings: 8,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CompanyFormationParams {
    pub min_founders_same_profession: usize,
    pub min_founders: usize,
    pub max_founders: usize,
    pub allow_unemployed_founders: bool,
    pub founder_personal_reserve_months: f64,
    pub startup_runway_months: f64,
    pub initial_openings_total: u16,
    pub initial_openings_founder_profession: u16,
    pub startup_openings_min: u16,
    pub startup_openings_max: u16,
    pub startup_bank_loan_enabled: bool,
    pub startup_bank_loan_max_fraction: f64,
    pub startup_max_payroll_to_revenue: f64,
    pub labor_slack_startup_trigger: f64,
    pub necessity_startup_sales_floor: f64,
    pub high_margin_threshold: f64,
    pub max_startups_per_month: Option<u16>,
}

impl Default for CompanyFormationParams {
    fn default() -> Self {
        Self {
            min_founders_same_profession: 5,
            min_founders: 3,
            max_founders: 6,
            allow_unemployed_founders: true,
            founder_personal_reserve_months: 0.5,
            startup_runway_months: 2.0,
            initial_openings_total: 10,
            initial_openings_founder_profession: 7,
            startup_openings_min: 6,
            startup_openings_max: 24,
            startup_bank_loan_enabled: true,
            startup_bank_loan_max_fraction: 0.95,
            startup_max_payroll_to_revenue: 1.00,
            labor_slack_startup_trigger: 0.07,
            necessity_startup_sales_floor: 50_000.0,
            high_margin_threshold: 0.25,
            max_startups_per_month: Some(36),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CompanyDistressParams {
    pub wage_cut_rate: f64,
    pub layoff_rate: f64,
    pub max_monthly_layoff_rate: f64,
    pub loss_distress_month_threshold: u8,
    pub bankruptcy_min_loss_months: u8,
    pub countercyclical_layoff_unemployment_guard: f64,
    pub loss_trigger_wage_cut_rate: f64,
    pub loss_trigger_layoff_rate: f64,
    pub bankruptcy_missed_payment_threshold: u8,
    pub min_wage_floor_ratio_to_catalog: f64,
}

impl Default for CompanyDistressParams {
    fn default() -> Self {
        Self {
            wage_cut_rate: 0.10,
            layoff_rate: 0.20,
            max_monthly_layoff_rate: 0.01,
            loss_distress_month_threshold: 3,
            bankruptcy_min_loss_months: 18,
            countercyclical_layoff_unemployment_guard: 0.08,
            loss_trigger_wage_cut_rate: 0.04,
            loss_trigger_layoff_rate: 0.03,
            bankruptcy_missed_payment_threshold: 36,
            min_wage_floor_ratio_to_catalog: 0.70,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SwitchCadence {
    MonthlyOnce,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LaborMobilityParams {
    pub switch_raise_threshold: f64,
    pub switch_cadence: SwitchCadence,
}

impl Default for LaborMobilityParams {
    fn default() -> Self {
        Self {
            switch_raise_threshold: 0.10,
            switch_cadence: SwitchCadence::MonthlyOnce,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LaborChurnParams {
    /// Monthly quit probability applied to employed workers.
    pub quit_rate_monthly: f64,
    /// Monthly retirement probability applied to employed workers age 65+.
    pub retirement_rate_monthly: f64,
    /// Monthly layoff probability applied to employed workers, independent of distress/bankruptcy.
    pub layoff_rate_monthly: f64,
}

impl Default for LaborChurnParams {
    fn default() -> Self {
        Self {
            quit_rate_monthly: 0.01,
            retirement_rate_monthly: 0.03,
            layoff_rate_monthly: 0.005,
        }
    }
}

impl LaborChurnParams {
    pub fn quit_probability(&self) -> f64 {
        self.quit_rate_monthly.clamp(0.0, 1.0)
    }

    pub fn retirement_probability(&self) -> f64 {
        self.retirement_rate_monthly.clamp(0.0, 1.0)
    }

    pub fn layoff_probability(&self) -> f64 {
        self.layoff_rate_monthly.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GoodsMarketParams {
    pub tariff_rate: f64,
    pub worker_effort_per_month: f64,
    pub min_margin_rate: f64,
    pub price_raise_rate: f64,
    pub price_cut_rate: f64,
    pub sell_through_raise_threshold: f64,
    pub sell_through_cut_threshold: f64,
    pub excess_inventory_months: f64,
    pub excess_inventory_cut_rate: f64,
}

impl Default for GoodsMarketParams {
    fn default() -> Self {
        Self {
            tariff_rate: 0.15,
            worker_effort_per_month: 300.0,
            min_margin_rate: 0.0,
            price_raise_rate: 0.02,
            price_cut_rate: 0.02,
            sell_through_raise_threshold: 0.90,
            sell_through_cut_threshold: 0.70,
            excess_inventory_months: 2.0,
            excess_inventory_cut_rate: 0.05,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TaxPolicyParams {
    pub bracket_1_max: f64,
    pub bracket_2_max: f64,
    pub rate_1: f64,
    pub rate_2: f64,
    pub rate_3: f64,
}

impl Default for TaxPolicyParams {
    fn default() -> Self {
        Self {
            bracket_1_max: 3_000.0,
            bracket_2_max: 7_000.0,
            rate_1: 0.10,
            rate_2: 0.20,
            rate_3: 0.30,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FiscalParams {
    /// Monthly benefit paid to each unemployed working-age (18–65) adult,
    /// expressed as a fraction of `median_income_reference`.
    pub unemployment_benefit_rate: f64,
    /// Reference monthly income used to compute the unemployment benefit amount.
    pub median_income_reference: f64,
    /// If an unemployed person's cash is below this threshold AND they have
    /// unmet essential needs, the government tops up their cash to this amount.
    pub basic_needs_cash_threshold: f64,
}

impl Default for FiscalParams {
    fn default() -> Self {
        Self {
            unemployment_benefit_rate: 0.50,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
        }
    }
}

impl FiscalParams {
    pub fn monthly_unemployment_benefit(&self) -> f64 {
        self.unemployment_benefit_rate * self.median_income_reference
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MonetaryPolicyParams {
    /// Annual CPI inflation target (2% = 0.02).
    pub target_inflation_rate: f64,
    /// Equilibrium policy rate when inflation is on target and unemployment is at NAIRU.
    pub neutral_policy_rate: f64,
    /// Taylor Rule inflation coefficient (α). Typically > 1 (Taylor principle).
    pub inflation_coefficient: f64,
    /// Taylor Rule unemployment gap coefficient (β).
    pub unemployment_coefficient: f64,
    /// Natural rate of unemployment (NAIRU).
    pub natural_unemployment_rate: f64,
    /// Zero lower bound — policy rate cannot fall below this.
    pub rate_floor: f64,
    /// Hard cap on policy rate.
    pub rate_ceiling: f64,
    /// Spread added to policy rate for private bank loans (credit spread).
    pub credit_spread: f64,
    /// Spread added to policy rate for sovereign bond coupon.
    pub sovereign_spread: f64,
    /// Government issues bonds when treasury_balance falls at or below this level.
    pub bond_issuance_treasury_threshold: f64,
    /// Fraction of each bond issuance absorbed by the bank as primary dealer.
    pub bank_primary_dealer_fraction: f64,
    /// Bond term in months.
    pub bond_maturity_months: u8,
    /// Top-N wealthiest people who absorb the retail portion of bond issuances/coupons.
    pub bond_retail_top_n: usize,
    /// Minimum absolute rate change needed to emit a RatePolicyChange event.
    pub rate_change_event_threshold: f64,
}

impl Default for MonetaryPolicyParams {
    fn default() -> Self {
        Self {
            target_inflation_rate: 0.02,
            neutral_policy_rate: 0.04,
            inflation_coefficient: 1.5,
            unemployment_coefficient: 0.5,
            natural_unemployment_rate: 0.03,
            rate_floor: 0.0,
            rate_ceiling: 0.20,
            credit_spread: 0.03,
            sovereign_spread: 0.01,
            bond_issuance_treasury_threshold: 0.0,
            bank_primary_dealer_fraction: 0.50,
            bond_maturity_months: 12,
            bond_retail_top_n: 200,
            rate_change_event_threshold: 0.005,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GovernmentBackfillParams {
    pub enabled: bool,
    pub essential_good_max_id_inclusive: u8,
    pub openings_total: u16,
    pub manufacturing_openings: u16,
    pub runway_months: f64,
    pub base_wage_multiplier: f64,
    pub vacancy_escalation_rate: f64,
    pub max_wage_multiplier: f64,
    pub switch_incentive_buffer: f64,
    pub public_utility_price_cap_multiplier: f64,
}

impl Default for GovernmentBackfillParams {
    fn default() -> Self {
        Self {
            enabled: true,
            essential_good_max_id_inclusive: 4,
            openings_total: 3,
            manufacturing_openings: 2,
            runway_months: 1.5,
            base_wage_multiplier: 1.35,
            vacancy_escalation_rate: 0.12,
            max_wage_multiplier: 1.8,
            switch_incentive_buffer: 0.05,
            public_utility_price_cap_multiplier: 1.5,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
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
    /// Minimum sampled participants per listed symbol.
    pub min_participants_per_listed: usize,
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
            min_participants_per_listed: 50,
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
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimulationParams {
    pub demographic: DemographicParams,
    pub education: EducationParams,
    pub labor: LaborParams,
    pub labor_churn: LaborChurnParams,
    pub labor_demand: LaborDemandParams,
    pub company_growth: CompanyGrowthParams,
    pub company_formation: CompanyFormationParams,
    pub company_distress: CompanyDistressParams,
    pub labor_mobility: LaborMobilityParams,
    pub goods_market: GoodsMarketParams,
    pub tax: TaxPolicyParams,
    pub fiscal: FiscalParams,
    pub government_backfill: GovernmentBackfillParams,
    pub stock_market: StockMarketParams,
    pub monetary: MonetaryPolicyParams,
}

#[cfg(test)]
mod tests;
