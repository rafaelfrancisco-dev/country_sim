#[cfg(test)]
use super::company::{DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies};
use super::{
    bank::{Bank, BankId, BankKind, DEFAULT_MAX_DEBT_TO_INCOME},
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

mod banking;
mod daily;
#[cfg(feature = "bench-trace")]
mod daily_bench_trace;
mod metrics;
mod monthly;
mod tax;

pub const DAYS_PER_MONTH: u64 = 30;
const PEOPLE_CHUNK_SIZE: usize = 2_048;
const COMPANY_CHUNK_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Country {
    pub(crate) id: u16,
    pub(crate) name: String,
    pub(crate) people: Vec<Person>,
    pub(crate) companies: Vec<Company>,
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
    pub(crate) monthly_goods_import_spending: f64,
    pub(crate) monthly_goods_export_revenue: f64,
    pub(crate) monthly_unmet_need_events: u64,
    pub(crate) monthly_essential_debt_spending: f64,
    pub(crate) monthly_hires_total: u64,
    pub(crate) monthly_private_banks_founded: u64,
    pub(crate) cumulative_private_banks_founded: u64,
    pub(crate) monthly_private_banks_closed: u64,
    pub(crate) cumulative_private_banks_closed: u64,
    pub(crate) monthly_gov_spending: f64,
    pub(crate) monthly_budget_balance: f64,
    pub(crate) monthly_gdp: f64,
    pub(crate) austerity_active: bool,
    pub(crate) gov_debt: f64,
    // Sovereign bond state
    pub(crate) bonds_outstanding: f64,
    pub(crate) bond_coupon_rate: f64,
    pub(crate) bond_months_to_maturity: u8,
    // Monthly bond trackers (reset each month)
    pub(crate) monthly_coupon_paid: f64,
    pub(crate) monthly_bonds_issued: f64,
    pub(crate) monthly_bonds_matured: f64,
    pub(crate) monthly_savings_interest_paid: f64,
    pub(crate) monthly_money_minted: f64,
    pub(crate) cumulative_money_minted: f64,
    pub(crate) last_mint_inflation_shortfall: f64,
    pub(crate) last_mint_unemployment_gap: f64,
    pub(crate) last_mint_fraction: f64,
    // Monetary policy (propagated from World each month)
    pub(crate) policy_rate: f64,
    // CPI / inflation tracking (persistent; not reset monthly)
    pub(crate) cpi_index: f64,
    pub(crate) last_month_cpi_index: f64,
    pub(crate) monthly_inflation_rate: f64,
    pub(crate) stock_market: StockMarket,
    pub(crate) export_tariffs_by_good: [f64; GOODS_COUNT],
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
    // Housing market
    pub(crate) housing_stock_total: u32,
    pub(crate) housing_stock_occupied_owned: u32,
    pub(crate) housing_stock_occupied_rented: u32,
    pub(crate) housing_stock_vacant: u32,
    pub(crate) housing_price_index: f64,
    pub(crate) monthly_rent: f64,
    // Monthly housing trackers (reset each month)
    pub(crate) monthly_rent_collected: f64,
    pub(crate) monthly_mortgage_payments: f64,
    pub(crate) monthly_home_purchases: u32,
    pub(crate) monthly_foreclosures: u32,
    pub(crate) monthly_housing_units_built: u32,
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
        let public_bank_name = format!("{name} Public Bank");

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
            treasury_bank_id: None,
            monthly_tax_collected: 0.0,
            monthly_corporate_tax_collected: 0.0,
            monthly_household_spending: 0.0,
            monthly_tariff_collected: 0.0,
            monthly_goods_import_spending: 0.0,
            monthly_goods_export_revenue: 0.0,
            monthly_unmet_need_events: 0,
            monthly_essential_debt_spending: 0.0,
            monthly_hires_total: 0,
            monthly_private_banks_founded: 0,
            cumulative_private_banks_founded: 0,
            monthly_private_banks_closed: 0,
            cumulative_private_banks_closed: 0,
            monthly_gov_spending: 0.0,
            monthly_budget_balance: 0.0,
            monthly_gdp: 0.0,
            austerity_active: false,
            gov_debt: 0.0,
            bonds_outstanding: 0.0,
            bond_coupon_rate: 0.0,
            bond_months_to_maturity: 0,
            monthly_coupon_paid: 0.0,
            monthly_bonds_issued: 0.0,
            monthly_bonds_matured: 0.0,
            monthly_savings_interest_paid: 0.0,
            monthly_money_minted: 0.0,
            cumulative_money_minted: 0.0,
            last_mint_inflation_shortfall: 0.0,
            last_mint_unemployment_gap: 0.0,
            last_mint_fraction: 0.0,
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
            banks: vec![Bank::with_identity(0, public_bank_name, BankKind::Public)],
            next_bank_id: 1,
            next_bank_serial: 1,
            public_bank_id: Some(0),
            housing_stock_total: 0,
            housing_stock_occupied_owned: 0,
            housing_stock_occupied_rented: 0,
            housing_stock_vacant: 0,
            housing_price_index: 0.0,
            monthly_rent: 0.0,
            monthly_rent_collected: 0.0,
            monthly_mortgage_payments: 0.0,
            monthly_home_purchases: 0,
            monthly_foreclosures: 0,
            monthly_housing_units_built: 0,
        };
        country.ensure_entity_deposit_assignments();
        let initial_cpi = country.cpi_basket_index();
        country.cpi_index = initial_cpi;
        country.last_month_cpi_index = initial_cpi;
        country.monthly_inflation_rate = 0.0;
        country.recompute_company_headcounts();
        country.recompute_aggregates();
        country
    }

    pub fn population(&self) -> usize {
        self.people.len()
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn people(&self) -> &[Person] {
        &self.people
    }

    pub(crate) fn people_mut(&mut self) -> &mut Vec<Person> {
        &mut self.people
    }

    pub fn companies(&self) -> &[Company] {
        &self.companies
    }

    pub(crate) fn companies_mut(&mut self) -> &mut Vec<Company> {
        &mut self.companies
    }

    pub fn day(&self) -> u64 {
        self.day
    }

    pub fn month(&self) -> u64 {
        self.month
    }

    pub fn sim_params(&self) -> &SimulationParams {
        &self.sim_params
    }

    pub fn policy_rate(&self) -> f64 {
        self.policy_rate
    }

    pub(crate) fn set_policy_rate(&mut self, rate: f64) {
        self.policy_rate = rate;
    }

    pub fn stock_market(&self) -> &StockMarket {
        &self.stock_market
    }

    pub(crate) fn stock_market_mut(&mut self) -> &mut StockMarket {
        &mut self.stock_market
    }

    pub fn treasury_balance(&self) -> f64 {
        self.treasury_balance
    }

    pub fn treasury_bank_id(&self) -> Option<BankId> {
        self.treasury_bank_id
    }

    pub(crate) fn treasury_balance_mut(&mut self) -> &mut f64 {
        &mut self.treasury_balance
    }

    pub fn total_money_supply(&self) -> f64 {
        self.total_money_supply
    }

    pub fn household_cash_total(&self) -> f64 {
        self.household_cash_total
    }

    pub fn company_cash_total(&self) -> f64 {
        self.company_cash_total
    }

    pub fn expected_money_supply(&self) -> f64 {
        self.expected_money_supply
    }

    pub fn money_supply_gap(&self) -> f64 {
        self.money_supply_gap
    }

    pub fn monthly_budget_balance(&self) -> f64 {
        self.monthly_budget_balance
    }

    pub fn monthly_household_spending(&self) -> f64 {
        self.monthly_household_spending
    }

    pub fn monthly_tariff_collected(&self) -> f64 {
        self.monthly_tariff_collected
    }

    pub fn monthly_tax_collected(&self) -> f64 {
        self.monthly_tax_collected
    }

    pub fn monthly_gov_spending(&self) -> f64 {
        self.monthly_gov_spending
    }

    pub fn gov_debt(&self) -> f64 {
        self.gov_debt
    }

    pub fn monthly_money_minted(&self) -> f64 {
        self.monthly_money_minted
    }

    pub fn cumulative_money_minted(&self) -> f64 {
        self.cumulative_money_minted
    }

    pub fn last_mint_fraction(&self) -> f64 {
        self.last_mint_fraction
    }

    pub fn last_mint_inflation_shortfall(&self) -> f64 {
        self.last_mint_inflation_shortfall
    }

    pub fn last_mint_unemployment_gap(&self) -> f64 {
        self.last_mint_unemployment_gap
    }

    pub fn monthly_goods_import_spending(&self) -> f64 {
        self.monthly_goods_import_spending
    }

    pub fn monthly_goods_export_revenue(&self) -> f64 {
        self.monthly_goods_export_revenue
    }

    pub fn monthly_unmet_need_events(&self) -> u64 {
        self.monthly_unmet_need_events
    }

    pub fn monthly_essential_debt_spending(&self) -> f64 {
        self.monthly_essential_debt_spending
    }

    pub fn cpi_index(&self) -> f64 {
        self.cpi_index
    }

    pub fn monthly_inflation_rate(&self) -> f64 {
        self.monthly_inflation_rate
    }

    pub fn monthly_savings_interest_paid(&self) -> f64 {
        self.monthly_savings_interest_paid
    }

    pub fn bonds_outstanding(&self) -> f64 {
        self.bonds_outstanding
    }

    pub fn bond_coupon_rate(&self) -> f64 {
        self.bond_coupon_rate
    }

    pub fn bond_months_to_maturity(&self) -> u8 {
        self.bond_months_to_maturity
    }

    pub fn monthly_coupon_paid(&self) -> f64 {
        self.monthly_coupon_paid
    }

    pub fn monthly_bonds_issued(&self) -> f64 {
        self.monthly_bonds_issued
    }

    pub fn monthly_bonds_matured(&self) -> f64 {
        self.monthly_bonds_matured
    }

    pub fn monthly_private_banks_founded(&self) -> u64 {
        self.monthly_private_banks_founded
    }

    pub fn cumulative_private_banks_founded(&self) -> u64 {
        self.cumulative_private_banks_founded
    }

    pub fn monthly_private_banks_closed(&self) -> u64 {
        self.monthly_private_banks_closed
    }

    pub fn cumulative_private_banks_closed(&self) -> u64 {
        self.cumulative_private_banks_closed
    }

    pub fn monthly_hires_total(&self) -> u64 {
        self.monthly_hires_total
    }

    pub fn export_tariffs_by_good(&self) -> &[f64; GOODS_COUNT] {
        &self.export_tariffs_by_good
    }

    pub fn pending_events(&self) -> &[SimEvent] {
        &self.pending_events
    }

    pub(crate) fn push_pending_event(&mut self, event: SimEvent) {
        self.pending_events.push(event);
    }

    pub(crate) fn drain_pending_events(&mut self) -> impl Iterator<Item = SimEvent> + '_ {
        self.pending_events.drain(..)
    }

    pub fn banks(&self) -> &[Bank] {
        &self.banks
    }

    pub fn public_bank(&self) -> Option<&Bank> {
        self.public_bank_id
            .and_then(|bank_id| self.bank_by_id(bank_id))
    }

    pub(crate) fn primary_bank(&self) -> &Bank {
        self.public_bank()
            .or_else(|| self.banks.first())
            .expect("country should always have at least one bank")
    }

    pub(crate) fn primary_bank_mut(&mut self) -> &mut Bank {
        let bank_id = self.public_bank_id.or_else(|| self.first_bank_id());
        let Some(bank_id) = bank_id else {
            panic!("country should always have at least one bank");
        };
        self.bank_by_id_mut(bank_id)
            .expect("country should always have a resolvable primary bank")
    }

    pub fn private_banks(&self) -> impl Iterator<Item = &Bank> {
        self.banks
            .iter()
            .filter(|bank| Some(bank.id) != self.public_bank_id)
    }

    pub fn all_banks(&self) -> impl Iterator<Item = &Bank> {
        self.banks.iter()
    }

    pub(crate) fn all_banks_mut(&mut self) -> impl Iterator<Item = &mut Bank> {
        self.banks.iter_mut()
    }

    pub fn total_bank_reserves(&self) -> f64 {
        round_cents(
            self.banks
                .iter()
                .map(|bank| bank.reserve_balance)
                .sum::<f64>(),
        )
    }

    pub fn total_bank_created_money(&self) -> f64 {
        round_cents(
            self.banks
                .iter()
                .map(|bank| bank.created_money_outstanding)
                .sum::<f64>(),
        )
    }

    pub fn total_bank_deposit_liabilities(&self) -> f64 {
        round_cents(
            self.banks
                .iter()
                .map(|bank| bank.deposit_liabilities)
                .sum::<f64>(),
        )
    }

    pub fn first_bank_id(&self) -> Option<BankId> {
        self.banks.first().map(|bank| bank.id)
    }

    pub fn public_bank_id_or_first(&self) -> Option<BankId> {
        self.public_bank_id.or_else(|| self.first_bank_id())
    }

    pub fn bank_by_id(&self, bank_id: BankId) -> Option<&Bank> {
        self.banks.iter().find(|bank| bank.id == bank_id)
    }

    pub fn bank_by_id_mut(&mut self, bank_id: BankId) -> Option<&mut Bank> {
        self.banks.iter_mut().find(|bank| bank.id == bank_id)
    }

    pub fn stock_market_bank_id(&self) -> Option<BankId> {
        self.public_bank_id_or_first()
    }

    pub fn bank_loan_rate_for(&self, bank_id: BankId) -> f64 {
        let base = self.bank_loan_rate();
        let Some(bank) = self.bank_by_id(bank_id) else {
            return base;
        };
        match bank.kind {
            BankKind::Private => (base - 0.005).max(self.sim_params.monetary.rate_floor),
            BankKind::Public => base,
        }
    }

    pub fn deposit_rate_for(&self, bank_id: BankId) -> f64 {
        let base = self.deposit_rate();
        let Some(bank) = self.bank_by_id(bank_id) else {
            return base;
        };
        match bank.kind {
            BankKind::Public => base.max(0.0),
            BankKind::Private => {
                (base + self.sim_params.banking.private_bank_deposit_rate_bonus).max(0.0)
            }
        }
    }

    pub(crate) fn best_deposit_bank_id(&self) -> Option<BankId> {
        self.banks
            .iter()
            .filter(|bank| !bank.is_in_resolution)
            .max_by(|a, b| {
                self.deposit_rate_for(a.id)
                    .partial_cmp(&self.deposit_rate_for(b.id))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bank| bank.id)
    }

    pub fn bank_cross_country_equity_exposure_cost(&self) -> f64 {
        let total_exposure = self
            .banks
            .iter()
            .flat_map(|bank| bank.stock_positions.values())
            .map(|position| position.shares as f64 * position.avg_cost)
            .sum::<f64>();
        round_cents(total_exposure)
    }

    pub fn bank_health_score(&self, bank_id: BankId) -> f64 {
        let Some(bank) = self.bank_by_id(bank_id) else {
            return 0.0;
        };
        let params = self.sim_params.banking;
        match bank.kind {
            BankKind::Private => bank.health_score_private(),
            BankKind::Public => (bank.health_score_public(
                self.treasury_balance,
                params.public_health_floor,
                params.public_health_liquidity_weight,
                params.public_health_capital_weight,
                params.public_health_sovereign_support_weight,
            ) + params.public_bank_stability_bonus * 0.25)
                .clamp(0.0, 1.0),
        }
    }

    pub fn healthiest_bank_id(&self) -> Option<BankId> {
        self.banks
            .iter()
            .max_by(|a, b| {
                self.bank_health_score(a.id)
                    .partial_cmp(&self.bank_health_score(b.id))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bank| bank.id)
    }

    pub fn preferred_private_bank_id(&self) -> Option<BankId> {
        self.banks
            .iter()
            .filter(|bank| bank.kind == BankKind::Private && !bank.is_in_resolution)
            .max_by(|a, b| {
                self.bank_health_score(a.id)
                    .partial_cmp(&self.bank_health_score(b.id))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bank| bank.id)
    }

    pub fn preferred_lender_bank_id(&self) -> Option<BankId> {
        self.preferred_private_bank_id()
            .or_else(|| self.public_bank_id_or_first())
            .or_else(|| self.healthiest_bank_id())
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

    /// Loan-rate anchor used for policy transmission.
    pub fn neutral_loan_rate(&self) -> f64 {
        self.sim_params.monetary.neutral_policy_rate + self.sim_params.monetary.credit_spread
    }

    /// Normalized and clamped policy stance vs neutral loan rate.
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

    pub fn housing_price_index(&self) -> f64 {
        self.housing_price_index
    }

    pub fn monthly_rent(&self) -> f64 {
        self.monthly_rent
    }

    pub fn housing_stock_total(&self) -> u32 {
        self.housing_stock_total
    }

    pub fn housing_stock_vacant(&self) -> u32 {
        self.housing_stock_vacant
    }

    pub fn housing_vacancy_rate(&self) -> f64 {
        if self.housing_stock_total == 0 {
            return 0.0;
        }
        self.housing_stock_vacant as f64 / self.housing_stock_total as f64
    }

    pub fn homeownership_rate(&self) -> f64 {
        let pop = self.people.len();
        if pop == 0 {
            return 0.0;
        }
        self.housing_stock_occupied_owned as f64 / pop as f64
    }

    pub fn monthly_home_purchases(&self) -> u32 {
        self.monthly_home_purchases
    }

    pub fn monthly_foreclosures(&self) -> u32 {
        self.monthly_foreclosures
    }

    pub fn monthly_rent_collected(&self) -> f64 {
        self.monthly_rent_collected
    }

    pub fn monthly_mortgage_payments(&self) -> f64 {
        self.monthly_mortgage_payments
    }

    pub fn monthly_housing_units_built(&self) -> u32 {
        self.monthly_housing_units_built
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
