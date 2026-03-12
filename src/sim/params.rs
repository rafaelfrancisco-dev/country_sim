use super::person::EducationTrack;
pub use super::stock_market_params::StockMarketParams;
mod fiscal;
mod monetary_banking;
pub use fiscal::{
    EssentialEconomyParams, FiscalParams, GovernmentBackfillParams, HousingMarketParams,
};
pub use monetary_banking::{BankingSystemParams, MonetaryPolicyParams};

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EducationParams {
    pub basic_only_share: f64,
    pub high_school_share: f64,
    pub bachelor_share: f64,
    pub doctor_share: f64,
}

impl Default for EducationParams {
    fn default() -> Self {
        Self {
            basic_only_share: 0.38,
            high_school_share: 0.37,
            bachelor_share: 0.22,
            doctor_share: 0.03,
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
            EducationTrack::Doctor
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LaborDemandParams {
    /// Baseline monthly opening target as a fraction of current company headcount.
    pub replacement_opening_rate: f64,
    /// Minimum baseline openings for companies that currently employ workers.
    pub min_openings_per_active_company: u16,
}

impl Default for LaborDemandParams {
    fn default() -> Self {
        Self {
            replacement_opening_rate: 0.01,
            min_openings_per_active_company: 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
            startup_openings_min: 3,
            startup_openings_max: 8,
            startup_bank_loan_enabled: true,
            startup_bank_loan_max_fraction: 0.95,
            startup_max_payroll_to_revenue: 0.70,
            labor_slack_startup_trigger: 0.07,
            necessity_startup_sales_floor: 50_000.0,
            high_margin_threshold: 0.25,
            max_startups_per_month: Some(8),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
            max_monthly_layoff_rate: 0.03,
            loss_distress_month_threshold: 2,
            bankruptcy_min_loss_months: 18,
            countercyclical_layoff_unemployment_guard: 0.12,
            loss_trigger_wage_cut_rate: 0.06,
            loss_trigger_layoff_rate: 0.05,
            bankruptcy_missed_payment_threshold: 36,
            min_wage_floor_ratio_to_catalog: 0.70,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SwitchCadence {
    MonthlyOnce,
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LaborMobilityParams {
    pub switch_raise_threshold: f64,
    pub min_tenure_months_before_switch: u16,
    pub switch_cadence: SwitchCadence,
}

impl Default for LaborMobilityParams {
    fn default() -> Self {
        Self {
            switch_raise_threshold: 0.10,
            min_tenure_months_before_switch: 6,
            switch_cadence: SwitchCadence::MonthlyOnce,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AnnualWageRaiseParams {
    pub enabled: bool,
    pub cadence_months: u8,
    pub min_history_months: u8,
    pub min_cash_runway_months: f64,
    pub market_gap_convergence: f64,
    pub min_profit_to_payroll_ratio: f64,
    pub max_raise_rate: Option<f64>,
}

impl Default for AnnualWageRaiseParams {
    fn default() -> Self {
        Self {
            enabled: true,
            cadence_months: 12,
            min_history_months: 12,
            min_cash_runway_months: 2.0,
            market_gap_convergence: 0.35,
            min_profit_to_payroll_ratio: 0.05,
            max_raise_rate: Some(0.06),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MigrationParams {
    pub enabled: bool,
    pub long_term_unemployment_months: u32,
    pub benefit_exhaustion_bonus_months: u32,
    pub chronic_unmet_needs_months: u8,
    pub base_monthly_migration_rate: f64,
    pub destination_unemployment_guard: f64,
    pub irresistible_offer_salary_premium: f64,
    pub irresistible_offer_min_open_slots: u16,
    pub cash_buffer_required: f64,
    pub migration_cooldown_months: u32,
}

impl Default for MigrationParams {
    fn default() -> Self {
        Self {
            enabled: true,
            long_term_unemployment_months: 10,
            benefit_exhaustion_bonus_months: 2,
            chronic_unmet_needs_months: 4,
            base_monthly_migration_rate: 0.08,
            destination_unemployment_guard: 0.16,
            irresistible_offer_salary_premium: 0.30,
            irresistible_offer_min_open_slots: 3,
            cash_buffer_required: 100.0,
            migration_cooldown_months: 6,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
            min_margin_rate: 0.15,
            price_raise_rate: 0.02,
            price_cut_rate: 0.02,
            sell_through_raise_threshold: 0.90,
            sell_through_cut_threshold: 0.70,
            excess_inventory_months: 2.0,
            excess_inventory_cut_rate: 0.05,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TaxPolicyParams {
    pub bracket_1_max: f64,
    pub bracket_2_max: f64,
    pub rate_1: f64,
    pub rate_2: f64,
    pub rate_3: f64,
    // Corporate profit tax brackets (applied to last_month_profit)
    pub corp_bracket_1_max: f64,
    pub corp_bracket_2_max: f64,
    pub corp_rate_1: f64,
    pub corp_rate_2: f64,
    pub corp_rate_3: f64,
}

impl Default for TaxPolicyParams {
    fn default() -> Self {
        Self {
            bracket_1_max: 3_000.0,
            bracket_2_max: 7_000.0,
            rate_1: 0.10,
            rate_2: 0.20,
            rate_3: 0.30,
            corp_bracket_1_max: 50_000.0,
            corp_bracket_2_max: 200_000.0,
            corp_rate_1: 0.15,
            corp_rate_2: 0.25,
            corp_rate_3: 0.35,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct SimulationParams {
    pub demographic: DemographicParams,
    pub education: EducationParams,
    pub labor: LaborParams,
    pub migration: MigrationParams,
    pub essentials: EssentialEconomyParams,
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
    pub housing: HousingMarketParams,
}

#[cfg(test)]
mod tests;
