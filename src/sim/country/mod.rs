#[cfg(test)]
use super::company::{DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies};
use super::{
    bank::{Bank, DEFAULT_MAX_DEBT_TO_INCOME},
    company::{Company, JobOpening},
    good::GOODS_COUNT,
    job::{JobTypeId, job_is_eligible_for_habilitation},
    money::{from_cents, round_cents, to_cents},
    params::SimulationParams,
    person::{EmploymentStatus, HabilitationLevel, Person},
    tickable::Tickable,
};
use rayon::prelude::*;

mod daily;
mod metrics;
mod monthly;
mod tax;

pub const DAYS_PER_MONTH: u64 = 30;
pub const TAX_BRACKET_1_MAX: f64 = 3_000.0;
pub const TAX_BRACKET_2_MAX: f64 = 7_000.0;
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
    pub export_tariffs_by_good: [f64; GOODS_COUNT],
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

        let mut country = Self {
            id,
            name,
            people,
            companies,
            day: 0,
            month: 0,
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
            export_tariffs_by_good,
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
}

impl Tickable for Country {
    fn tick_day(&mut self) {
        self.tick_day_without_month();
    }
}

include!("job_market.rs");
include!("random.rs");
pub use tax::monthly_income_tax;

#[cfg(test)]
mod tests;
