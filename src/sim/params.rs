use super::person::EducationTrack;
pub use super::stock_market_params::StockMarketParams;

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
pub struct AnnualWageRaiseParams {
    pub enabled: bool,
    pub cadence_months: u8,
    pub min_history_months: u8,
    pub min_cash_runway_months: f64,
    pub max_raise_rate: Option<f64>,
}

impl Default for AnnualWageRaiseParams {
    fn default() -> Self {
        Self {
            enabled: true,
            cadence_months: 12,
            min_history_months: 12,
            min_cash_runway_months: 2.0,
            max_raise_rate: None,
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
    /// Discrete step size for policy rate moves (0.0025 = 25 bps, like real central banks).
    pub policy_rate_step: f64,
    /// Deposit rate = max(0, policy_rate - deposit_spread). Models bank interest margin.
    pub deposit_spread: f64,
    /// Minimum cash balance (exclusive) required to earn deposit interest.
    pub savings_min_balance: f64,
    /// Absolute annual-rate gap from neutral loan rate that maps to a full policy response.
    pub rate_response_band: f64,
    /// Slope for loan appetite response to the normalized rate gap.
    pub loan_appetite_strength: f64,
    /// Minimum loan appetite multiplier when rates are high.
    pub loan_appetite_min: f64,
    /// Maximum loan appetite multiplier when rates are low.
    pub loan_appetite_max: f64,
    /// Slope for discretionary spending response to the normalized rate gap.
    pub discretionary_spend_strength: f64,
    /// Minimum discretionary spending multiplier when rates are high.
    pub discretionary_spend_min: f64,
    /// Maximum discretionary spending multiplier when rates are low.
    pub discretionary_spend_max: f64,
    /// Slope for company expansion and startup response to the normalized rate gap.
    pub expansion_strength: f64,
    /// Minimum expansion multiplier when rates are high.
    pub expansion_min: f64,
    /// Maximum expansion multiplier when rates are low.
    pub expansion_max: f64,
    /// Slope for firm pricing pressure response to the normalized rate gap.
    pub price_pressure_strength: f64,
    /// Minimum pricing-pressure multiplier when rates are high.
    pub price_pressure_min: f64,
    /// Maximum pricing-pressure multiplier when rates are low.
    pub price_pressure_max: f64,
    /// Maximum absolute shift applied to sell-through pricing thresholds.
    pub sell_through_shift_max: f64,
    /// Slope for sell-through threshold shift response to the normalized rate gap.
    pub sell_through_shift_strength: f64,
    /// Exponential smoothing factor for annualized month-over-month inflation.
    /// 1.0 = no smoothing, 0.0 = fully static.
    pub inflation_ema_alpha: f64,
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
            policy_rate_step: 0.0025,
            deposit_spread: 0.01,
            savings_min_balance: 200.0,
            rate_response_band: 0.06,
            loan_appetite_strength: 0.55,
            loan_appetite_min: 0.55,
            loan_appetite_max: 1.35,
            discretionary_spend_strength: 0.45,
            discretionary_spend_min: 0.65,
            discretionary_spend_max: 1.25,
            expansion_strength: 0.60,
            expansion_min: 0.45,
            expansion_max: 1.45,
            price_pressure_strength: 0.70,
            price_pressure_min: 0.35,
            price_pressure_max: 1.20,
            sell_through_shift_max: 0.05,
            sell_through_shift_strength: 0.04,
            inflation_ema_alpha: 0.35,
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
pub struct BankingSystemParams {
    pub deposit_target_ratio: f64,
    pub wallet_buffer_months_person: f64,
    pub wallet_buffer_months_company: f64,
    pub deposit_insurance_cap: f64,
    pub uninsured_haircut_rate: f64,
    pub run_health_threshold: f64,
    pub run_withdraw_fraction: f64,
    pub run_switch_fraction: f64,
    pub public_bank_stability_bonus: f64,
    pub private_bank_failure_sensitivity: f64,
    pub min_private_bank_founders: usize,
    pub min_private_bank_seed_capital: f64,
    pub max_new_banks_per_month: u16,
    pub public_bank_creation_unemployment_trigger: f64,
    pub public_bank_creation_loss_trigger: f64,
}

impl Default for BankingSystemParams {
    fn default() -> Self {
        Self {
            deposit_target_ratio: 0.85,
            wallet_buffer_months_person: 0.20,
            wallet_buffer_months_company: 0.25,
            deposit_insurance_cap: 100_000.0,
            uninsured_haircut_rate: 0.35,
            run_health_threshold: 0.45,
            run_withdraw_fraction: 0.40,
            run_switch_fraction: 0.65,
            public_bank_stability_bonus: 0.20,
            private_bank_failure_sensitivity: 1.0,
            min_private_bank_founders: 8,
            min_private_bank_seed_capital: 500_000.0,
            max_new_banks_per_month: 2,
            public_bank_creation_unemployment_trigger: 0.15,
            public_bank_creation_loss_trigger: 0.25,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimulationParams {
    pub demographic: DemographicParams,
    pub education: EducationParams,
    pub labor: LaborParams,
    pub annual_wage_raise: AnnualWageRaiseParams,
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
    pub banking: BankingSystemParams,
}

#[cfg(test)]
mod tests;
