#[cfg(test)]
use super::company::{DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies};
use super::{
    bank::{Bank, DEFAULT_MAX_DEBT_TO_INCOME},
    company::{Company, FounderStake, JobOpening},
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

mod daily;
mod metrics;
mod monthly;
mod tax;

pub const DAYS_PER_MONTH: u64 = 30;
const PEOPLE_CHUNK_SIZE: usize = 2_048;
const COMPANY_CHUNK_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Country {
    pub id: u16,
    pub name: String,
    pub people: Vec<Person>,
    pub companies: Vec<Company>,
    pub day: u64,
    pub month: u64,
    pub next_startup_serial: u32,
    pub next_person_id: u32,
    pub birth_carry: f64,
    pub last_day_births: u32,
    pub last_day_deaths: u32,
    pub sim_params: SimulationParams,
    pub treasury_balance: f64,
    pub monthly_tax_collected: f64,
    pub monthly_household_spending: f64,
    pub monthly_tariff_collected: f64,
    pub monthly_goods_import_spending: f64,
    pub monthly_goods_export_revenue: f64,
    pub monthly_unmet_need_events: u64,
    pub monthly_essential_debt_spending: f64,
    pub monthly_hires_total: u64,
    pub monthly_gov_spending: f64,
    pub monthly_budget_balance: f64,
    pub gov_debt: f64,
    // Sovereign bond state
    pub bonds_outstanding: f64,
    pub bond_coupon_rate: f64,
    pub bond_months_to_maturity: u8,
    // Monthly bond trackers (reset each month)
    pub monthly_coupon_paid: f64,
    pub monthly_bonds_issued: f64,
    pub monthly_bonds_matured: f64,
    pub monthly_savings_interest_paid: f64,
    // Monetary policy (propagated from World each month)
    pub policy_rate: f64,
    // CPI / inflation tracking (persistent; not reset monthly)
    pub cpi_index: f64,
    pub last_month_cpi_index: f64,
    pub monthly_inflation_rate: f64,
    pub stock_market: StockMarket,
    pub export_tariffs_by_good: [f64; GOODS_COUNT],
    pub pending_events: Vec<SimEvent>,
    pub initial_money_supply: f64,
    pub household_cash_total: f64,
    pub company_cash_total: f64,
    pub total_money_supply: f64,
    pub expected_money_supply: f64,
    pub money_supply_gap: f64,
    pub bank: Bank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    pub code: &'static str,
    pub message: String,
}

impl Country {
    #[cfg(test)]
    pub fn new(id: u16, name: String, people: Vec<Person>) -> Self {
        let companies = build_default_companies(&name, DEFAULT_COMPANIES_PER_COUNTRY);
        Self::with_economy(id, name, people, companies, 0.0)
    }

    #[cfg(test)]
    pub fn with_economy(
        id: u16,
        name: String,
        people: Vec<Person>,
        companies: Vec<Company>,
        initial_money_supply: f64,
    ) -> Self {
        Self::with_economy_and_params(
            id,
            name,
            people,
            companies,
            initial_money_supply,
            SimulationParams::default(),
        )
    }

    pub fn with_economy_and_params(
        id: u16,
        name: String,
        people: Vec<Person>,
        companies: Vec<Company>,
        initial_money_supply: f64,
        sim_params: SimulationParams,
    ) -> Self {
        let next_person_id = people
            .iter()
            .map(|person| person.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut export_tariffs_by_good = [0.0; GOODS_COUNT];
        export_tariffs_by_good.fill(sim_params.goods_market.tariff_rate.clamp(0.0, 1.0));
        let initial_policy_rate = sim_params.monetary.neutral_policy_rate;

        let mut country = Self {
            id,
            name,
            people,
            companies,
            day: 0,
            month: 0,
            next_startup_serial: 1,
            next_person_id,
            birth_carry: 0.0,
            last_day_births: 0,
            last_day_deaths: 0,
            sim_params,
            treasury_balance: 0.0,
            monthly_tax_collected: 0.0,
            monthly_household_spending: 0.0,
            monthly_tariff_collected: 0.0,
            monthly_goods_import_spending: 0.0,
            monthly_goods_export_revenue: 0.0,
            monthly_unmet_need_events: 0,
            monthly_essential_debt_spending: 0.0,
            monthly_hires_total: 0,
            monthly_gov_spending: 0.0,
            monthly_budget_balance: 0.0,
            gov_debt: 0.0,
            bonds_outstanding: 0.0,
            bond_coupon_rate: 0.0,
            bond_months_to_maturity: 0,
            monthly_coupon_paid: 0.0,
            monthly_bonds_issued: 0.0,
            monthly_bonds_matured: 0.0,
            monthly_savings_interest_paid: 0.0,
            policy_rate: initial_policy_rate,
            cpi_index: 1.0,
            last_month_cpi_index: 1.0,
            monthly_inflation_rate: 0.0,
            stock_market: StockMarket::new(),
            export_tariffs_by_good,
            pending_events: Vec::new(),
            initial_money_supply: round_cents(initial_money_supply),
            household_cash_total: 0.0,
            company_cash_total: 0.0,
            total_money_supply: 0.0,
            expected_money_supply: 0.0,
            money_supply_gap: 0.0,
            bank: Bank::new(),
        };
        country.recompute_company_headcounts();
        country.recompute_aggregates();
        country
    }

    pub fn population(&self) -> usize {
        self.people.len()
    }

    /// Current bank loan rate: policy rate + credit spread, clamped to [floor, ceiling].
    pub fn bank_loan_rate(&self) -> f64 {
        (self.policy_rate + self.sim_params.monetary.credit_spread).clamp(
            self.sim_params.monetary.rate_floor,
            self.sim_params.monetary.rate_ceiling,
        )
    }

    /// Current deposit rate: policy rate minus deposit spread, floored at zero.
    pub fn deposit_rate(&self) -> f64 {
        (self.policy_rate - self.sim_params.monetary.deposit_spread).max(0.0)
    }

    pub(super) fn public_utility_dynamic_base_index(&self) -> f64 {
        let inflation_index = if self.cpi_index.is_finite() {
            self.cpi_index.max(1.0)
        } else {
            1.0
        };

        let money_supply_index = if self.initial_money_supply > 0.0 {
            if self.expected_money_supply.is_finite() {
                (self.expected_money_supply / self.initial_money_supply).max(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        let blended = (inflation_index * money_supply_index).sqrt();
        if blended.is_finite() {
            blended.max(1.0)
        } else {
            1.0
        }
    }

    pub(super) fn public_utility_price_bounds_for_good(&self, good_id: u8) -> Option<(f64, f64)> {
        let good = crate::sim::good::good_by_id(good_id)?;
        let min_price =
            round_cents(good.base_unit_cost * (1.0 + self.sim_params.goods_market.min_margin_rate));
        let dynamic_base_cost =
            round_cents(good.base_unit_cost * self.public_utility_dynamic_base_index());
        let cap_multiplier = self
            .sim_params
            .government_backfill
            .public_utility_price_cap_multiplier
            .max(1.0);
        let max_price = round_cents((dynamic_base_cost * cap_multiplier).max(min_price));
        Some((min_price, max_price))
    }

    pub fn company_name(&self, company_id: u16) -> Option<&str> {
        self.companies
            .get(company_id as usize)
            .map(|company| company.name.as_str())
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

mod job_market;
mod random;
use job_market::*;
use random::*;
pub use tax::{monthly_income_tax, monthly_income_tax_with_params};

#[cfg(test)]
mod tests;
