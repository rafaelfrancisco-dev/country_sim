use std::sync::Arc;

use super::{
    bank::{Bank, BankId, BankKind, DEFAULT_MAX_DEBT_TO_INCOME},
    company::{Company, FounderStake},
    currency::CurrencySpec,
    essentials::Household,
    event::{EntityKind, IpoReason, SimEvent},
    good::GOODS_COUNT,
    job::{JobTypeId, job_is_eligible_for_habilitation},
    money::{from_cents, round_cents, to_cents},
    params::SimulationParams,
    person::{EmploymentStatus, HabilitationLevel, Person},
    stock_market::StockMarket,
    tickable::Tickable,
};
use rayon::prelude::*;

mod accessors;
mod accessors_fx;
mod accessors_last_month;
mod banking;
mod daily;
mod essential_economy;
mod init;
mod metrics;
mod monthly;
mod runtime;
mod tax;
use runtime::CountryRuntime;

pub const DAYS_PER_MONTH: u64 = 30;
const PEOPLE_CHUNK_SIZE: usize = 2_048;
const COMPANY_CHUNK_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Country {
    pub(crate) id: u16,
    pub(crate) name: Arc<str>,
    pub(crate) currency_spec: CurrencySpec,
    pub(crate) currency_idx: usize,
    pub(crate) is_currency_issuer: bool,
    pub(crate) people: Vec<Person>,
    pub(crate) companies: Vec<Company>,
    pub(crate) households: Vec<Household>,
    pub(crate) day: u64,
    pub(crate) month: u64,
    pub(crate) next_startup_serial: u32,
    pub(crate) next_person_id: u32,
    pub(crate) birth_carry: f64,
    pub(crate) last_day_births: u32,
    pub(crate) last_day_deaths: u32,
    pub(crate) sim_params: SimulationParams,
    pub(crate) treasury_balance: f64,
    pub(crate) treasury_bank_id: Option<BankId>,
    pub(crate) monthly_tax_collected: f64,
    pub(crate) monthly_corporate_tax_collected: f64,
    pub(crate) monthly_household_spending: f64,
    pub(crate) monthly_tariff_collected: f64,
    pub(crate) monthly_import_tariff_collected: f64,
    pub(crate) monthly_goods_import_spending: f64,
    pub(crate) monthly_goods_import_spending_base: f64,
    pub(crate) monthly_goods_export_revenue: f64,
    pub(crate) monthly_goods_export_revenue_base: f64,
    pub(crate) monthly_unmet_need_events: u64,
    pub(crate) monthly_severe_unmet_need_events: u64,
    pub(crate) monthly_chronic_unmet_need_count: u64,
    pub(crate) monthly_essential_debt_spending: f64,
    pub(crate) monthly_hires_total: u64,
    pub(crate) monthly_immigrants: u64,
    pub(crate) monthly_emigrants: u64,
    pub(crate) monthly_migrants_for_work: u64,
    pub(crate) monthly_migrants_for_distress: u64,
    pub(crate) monthly_private_banks_founded: u64,
    pub(crate) cumulative_private_banks_founded: u64,
    pub(crate) monthly_private_banks_closed: u64,
    pub(crate) cumulative_private_banks_closed: u64,
    pub(crate) monthly_gov_spending: f64,
    pub(crate) monthly_budget_balance: f64,
    pub(crate) monthly_gdp: f64,
    pub(crate) austerity_active: bool,
    pub(crate) gov_debt: f64,
    pub(crate) bonds_outstanding: f64,
    pub(crate) bond_coupon_rate: f64,
    pub(crate) bond_months_to_maturity: u8,
    pub(crate) monthly_coupon_paid: f64,
    pub(crate) monthly_bonds_issued: f64,
    pub(crate) monthly_bonds_matured: f64,
    pub(crate) monthly_savings_interest_paid: f64,
    pub(crate) monthly_fx_financial_inflows_base: f64,
    pub(crate) monthly_fx_financial_outflows_base: f64,
    pub(crate) monthly_fx_net_flow_base: f64,
    pub(crate) monthly_money_minted: f64,
    pub(crate) cumulative_money_minted: f64,
    pub(crate) last_mint_inflation_shortfall: f64,
    pub(crate) last_mint_unemployment_gap: f64,
    pub(crate) last_mint_fraction: f64,
    pub(crate) last_completed_month_tax_collected: f64,
    pub(crate) last_completed_month_household_spending: f64,
    pub(crate) last_completed_month_tariff_collected: f64,
    pub(crate) last_completed_month_gov_spending: f64,
    pub(crate) last_completed_month_budget_balance: f64,
    pub(crate) last_completed_month_coupon_paid: f64,
    pub(crate) last_completed_month_bonds_issued: f64,
    pub(crate) last_completed_month_bonds_matured: f64,
    pub(crate) last_completed_month_savings_interest_paid: f64,
    pub(crate) last_completed_month_fx_net_flow_base: f64,
    pub(crate) last_completed_month_rent_collected: f64,
    pub(crate) last_completed_month_mortgage_payments: f64,
    pub(crate) last_completed_month_home_purchases: u32,
    pub(crate) last_completed_month_foreclosures: u32,
    pub(crate) last_completed_month_unmet_need_events: u64,
    pub(crate) last_completed_month_severe_unmet_need_events: u64,
    pub(crate) last_completed_month_chronic_unmet_need_count: u64,
    pub(crate) last_completed_month_essential_debt_spending: f64,
    pub(crate) last_completed_month_hires_total: u64,
    pub(crate) last_completed_month_money_minted: f64,
    pub(crate) policy_rate: f64,
    pub(crate) fx_rate_local_per_base: f64,
    pub(crate) fx_pressure: f64,
    pub(crate) cpi_index: f64,
    pub(crate) last_month_cpi_index: f64,
    pub(crate) monthly_inflation_rate: f64,
    pub(crate) smoothed_monthly_cpi_change: f64,
    pub(crate) stock_market: StockMarket,
    pub(crate) export_tariffs_by_good: [f64; GOODS_COUNT],
    pub(crate) import_tariffs_by_good: [f64; GOODS_COUNT],
    pub(crate) pending_events: Vec<SimEvent>,
    pub(crate) initial_money_supply: f64,
    pub(crate) household_cash_total: f64,
    pub(crate) company_cash_total: f64,
    pub(crate) total_money_supply: f64,
    pub(crate) expected_money_supply: f64,
    pub(crate) money_supply_gap: f64,
    pub(crate) banks: Vec<Bank>,
    pub(crate) next_bank_id: BankId,
    pub(crate) next_bank_serial: u32,
    pub(crate) public_bank_id: Option<BankId>,
    pub(crate) housing_stock_total: u32,
    pub(crate) housing_stock_occupied_owned: u32,
    pub(crate) housing_stock_occupied_rented: u32,
    pub(crate) housing_stock_company_facility: u32,
    pub(crate) housing_stock_vacant: u32,
    pub(crate) housing_price_index: f64,
    pub(crate) monthly_rent: f64,
    pub(crate) monthly_rent_collected: f64,
    pub(crate) monthly_mortgage_payments: f64,
    pub(crate) monthly_home_purchases: u32,
    pub(crate) monthly_foreclosures: u32,
    pub(crate) monthly_housing_units_built: u32,
    pub(crate) use_inner_parallelism: bool,
    pub(crate) runtime: CountryRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    pub code: &'static str,
    pub message: String,
}

impl Country {
    pub fn bank_loan_rate(&self) -> f64 {
        (self.policy_rate + self.sim_params.monetary.credit_spread).clamp(
            self.sim_params.monetary.rate_floor,
            self.sim_params.monetary.rate_ceiling,
        )
    }

    pub fn deposit_rate(&self) -> f64 {
        (self.policy_rate - self.sim_params.monetary.deposit_spread).max(0.0)
    }

    pub fn neutral_loan_rate(&self) -> f64 {
        self.sim_params.monetary.neutral_policy_rate + self.sim_params.monetary.credit_spread
    }

    pub fn rate_response_normalized_gap(&self) -> f64 {
        let band = self.sim_params.monetary.rate_response_band.max(0.0001);
        ((self.bank_loan_rate() - self.neutral_loan_rate()) / band).clamp(-1.0, 1.0)
    }

    fn response_multiplier(&self, strength: f64, min_value: f64, max_value: f64) -> f64 {
        let normalized_gap = self.rate_response_normalized_gap();
        let raw = 1.0 - normalized_gap * strength.max(0.0);
        let floor = min_value.max(0.0);
        let ceiling = max_value.max(floor);
        raw.clamp(floor, ceiling)
    }

    pub fn loan_appetite_multiplier(&self) -> f64 {
        let monetary = self.sim_params.monetary;
        self.response_multiplier(
            monetary.loan_appetite_strength,
            monetary.loan_appetite_min,
            monetary.loan_appetite_max,
        )
    }

    pub fn discretionary_spend_multiplier(&self) -> f64 {
        let monetary = self.sim_params.monetary;
        self.response_multiplier(
            monetary.discretionary_spend_strength,
            monetary.discretionary_spend_min,
            monetary.discretionary_spend_max,
        )
    }

    pub fn expansion_multiplier(&self) -> f64 {
        let monetary = self.sim_params.monetary;
        self.response_multiplier(
            monetary.expansion_strength,
            monetary.expansion_min,
            monetary.expansion_max,
        )
    }

    pub fn price_pressure_multiplier(&self) -> f64 {
        let monetary = self.sim_params.monetary;
        self.response_multiplier(
            monetary.price_pressure_strength,
            monetary.price_pressure_min,
            monetary.price_pressure_max,
        )
    }

    pub fn sell_through_threshold_shift(&self) -> f64 {
        let monetary = self.sim_params.monetary;
        let raw_shift =
            self.rate_response_normalized_gap() * monetary.sell_through_shift_strength.max(0.0);
        let max_abs = monetary.sell_through_shift_max.max(0.0);
        raw_shift.clamp(-max_abs, max_abs)
    }

    pub fn effective_debt_to_income_cap(&self, base_cap: f64) -> f64 {
        (base_cap.max(0.0) * self.loan_appetite_multiplier()).clamp(0.01, 0.95)
    }

    pub fn tick_day_without_month(&mut self) {
        self.day = self.day.saturating_add(1);
        self.last_day_births = 0;
        self.last_day_deaths = 0;
        self.run_daily_lifecycle();
    }

    #[cfg(feature = "bench-trace")]
    pub(crate) fn tick_day_without_month_traced(&mut self) -> crate::sim::trace::CountryDailyTrace {
        self.day = self.day.saturating_add(1);
        self.last_day_births = 0;
        self.last_day_deaths = 0;
        self.run_daily_lifecycle_traced()
    }
}

impl Tickable for Country {
    fn tick_day(&mut self) {
        self.tick_day_without_month();
    }
}

mod random;
use random::*;
pub use tax::{monthly_income_tax, monthly_income_tax_with_params};

#[cfg(test)]
mod tests;
