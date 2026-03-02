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
            seek_base: 0.006,
            seek_age_slope: 0.0003,
            seek_cash_pressure_scale: 0.03,
            seek_cash_reference: 3_000.0,
            seek_probability_cap: 0.08,
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
            replacement_opening_rate: 0.018,
            min_openings_per_active_company: 2,
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
    pub founder_personal_reserve_months: f64,
    pub startup_runway_months: f64,
    pub initial_openings_total: u16,
    pub initial_openings_founder_profession: u16,
    pub high_margin_threshold: f64,
    pub max_startups_per_month: Option<u16>,
}

impl Default for CompanyFormationParams {
    fn default() -> Self {
        Self {
            min_founders_same_profession: 5,
            founder_personal_reserve_months: 3.0,
            startup_runway_months: 6.0,
            initial_openings_total: 10,
            initial_openings_founder_profession: 7,
            high_margin_threshold: 0.25,
            max_startups_per_month: None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CompanyDistressParams {
    pub wage_cut_rate: f64,
    pub layoff_rate: f64,
    pub loss_distress_month_threshold: u8,
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
            loss_distress_month_threshold: 3,
            loss_trigger_wage_cut_rate: 0.04,
            loss_trigger_layoff_rate: 0.03,
            bankruptcy_missed_payment_threshold: 3,
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
            worker_effort_per_month: 24.0,
            min_margin_rate: 0.0,
            price_raise_rate: 0.05,
            price_cut_rate: 0.05,
            sell_through_raise_threshold: 0.90,
            sell_through_cut_threshold: 0.70,
            excess_inventory_months: 2.0,
            excess_inventory_cut_rate: 0.05,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimulationParams {
    pub demographic: DemographicParams,
    pub education: EducationParams,
    pub labor: LaborParams,
    pub labor_demand: LaborDemandParams,
    pub company_growth: CompanyGrowthParams,
    pub company_formation: CompanyFormationParams,
    pub company_distress: CompanyDistressParams,
    pub labor_mobility: LaborMobilityParams,
    pub goods_market: GoodsMarketParams,
    pub fiscal: FiscalParams,
    pub government_backfill: GovernmentBackfillParams,
    pub stock_market: StockMarketParams,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiscal_benefit_is_rate_times_reference_income() {
        let params = FiscalParams {
            unemployment_benefit_rate: 0.5,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
        };
        assert!((params.monthly_unemployment_benefit() - 1_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fiscal_benefit_is_zero_when_rate_is_zero() {
        let params = FiscalParams {
            unemployment_benefit_rate: 0.0,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
        };
        assert!((params.monthly_unemployment_benefit()).abs() < f64::EPSILON);
    }

    #[test]
    fn fiscal_benefit_scales_with_reference_income() {
        let params = FiscalParams {
            unemployment_benefit_rate: 0.4,
            median_income_reference: 3_000.0,
            basic_needs_cash_threshold: 500.0,
        };
        assert!((params.monthly_unemployment_benefit() - 1_200.0).abs() < f64::EPSILON);
    }
}
