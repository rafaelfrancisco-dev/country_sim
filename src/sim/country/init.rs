use std::sync::Arc;

use super::Country;
#[cfg(test)]
use crate::sim::company::{build_default_companies, DEFAULT_COMPANIES_PER_COUNTRY};
use crate::sim::{
    bank::{Bank, BankKind},
    company::Company,
    currency::generated_currency,
    essentials::{estimated_household_count, Household},
    good::GOODS_COUNT,
    money::round_cents,
    params::SimulationParams,
    person::Person,
    stock_market::StockMarket,
};

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
        let mut import_tariffs_by_good = [0.0; GOODS_COUNT];
        import_tariffs_by_good.fill(sim_params.goods_market.tariff_rate.clamp(0.0, 1.0));
        let initial_policy_rate = sim_params.monetary.neutral_policy_rate;
        let name: Arc<str> = name.into();
        let currency_spec = generated_currency(id, 0);
        let public_bank_name: Arc<str> = format!("{name} Public Bank").into();

        let mut country = Self {
            id,
            name,
            currency_spec,
            currency_idx: 0,
            is_currency_issuer: true,
            people,
            companies,
            households: Vec::new(),
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
            monthly_import_tariff_collected: 0.0,
            monthly_goods_import_spending: 0.0,
            monthly_goods_import_spending_base: 0.0,
            monthly_goods_export_revenue: 0.0,
            monthly_goods_export_revenue_base: 0.0,
            monthly_unmet_need_events: 0,
            monthly_severe_unmet_need_events: 0,
            monthly_chronic_unmet_need_count: 0,
            monthly_essential_debt_spending: 0.0,
            monthly_hires_total: 0,
            monthly_immigrants: 0,
            monthly_emigrants: 0,
            monthly_migrants_for_work: 0,
            monthly_migrants_for_distress: 0,
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
            monthly_fx_financial_inflows_base: 0.0,
            monthly_fx_financial_outflows_base: 0.0,
            monthly_fx_net_flow_base: 0.0,
            monthly_money_minted: 0.0,
            cumulative_money_minted: 0.0,
            last_mint_inflation_shortfall: 0.0,
            last_mint_unemployment_gap: 0.0,
            last_mint_fraction: 0.0,
            last_completed_month_tax_collected: 0.0,
            last_completed_month_household_spending: 0.0,
            last_completed_month_tariff_collected: 0.0,
            last_completed_month_gov_spending: 0.0,
            last_completed_month_budget_balance: 0.0,
            last_completed_month_coupon_paid: 0.0,
            last_completed_month_bonds_issued: 0.0,
            last_completed_month_bonds_matured: 0.0,
            last_completed_month_savings_interest_paid: 0.0,
            last_completed_month_fx_net_flow_base: 0.0,
            last_completed_month_rent_collected: 0.0,
            last_completed_month_mortgage_payments: 0.0,
            last_completed_month_home_purchases: 0,
            last_completed_month_foreclosures: 0,
            last_completed_month_unmet_need_events: 0,
            last_completed_month_severe_unmet_need_events: 0,
            last_completed_month_chronic_unmet_need_count: 0,
            last_completed_month_essential_debt_spending: 0.0,
            last_completed_month_hires_total: 0,
            last_completed_month_money_minted: 0.0,
            policy_rate: initial_policy_rate,
            fx_rate_local_per_base: 1.0,
            fx_reserves_base: 0.0,
            fx_reserves_by_source: std::collections::HashMap::new(),
            fx_pressure: 0.0,
            cpi_index: 1.0,
            last_month_cpi_index: 1.0,
            monthly_inflation_rate: 0.0,
            smoothed_monthly_cpi_change: 0.0,
            stock_market: StockMarket::new(),
            export_tariffs_by_good,
            import_tariffs_by_good,
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
            use_inner_parallelism: true,
            runtime: super::runtime::CountryRuntime::default(),
        };
        for person in &mut country.people {
            person.birth_country_id = id;
            person.current_country_id = id;
        }
        country.ensure_entity_deposit_assignments();
        let initial_cpi = country.cpi_basket_index();
        country.cpi_index = initial_cpi;
        country.last_month_cpi_index = initial_cpi;
        country.monthly_inflation_rate = 0.0;
        country.smoothed_monthly_cpi_change = 0.0;
        country.recompute_company_headcounts();
        country.rebuild_households();
        country.recompute_aggregates();
        country.runtime.rebuild_from_people(&country.people);
        country
    }

    pub fn population(&self) -> usize {
        self.people.len()
    }

    pub fn rebuild_households(&mut self) {
        let population = self.people.len();
        if population == 0 {
            self.households.clear();
            return;
        }

        let housed_adults = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, person)| {
                person.age_years() >= 18
                    && !matches!(person.housing, crate::sim::person::HousingStatus::Unhoused)
            })
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        let desired_count = estimated_household_count(&self.sim_params.essentials, population);
        let household_count = if housed_adults.is_empty() {
            desired_count.min(population)
        } else {
            desired_count.min(housed_adults.len()).max(1)
        };

        let mut member_counts = vec![0_u16; household_count];
        for person in &mut self.people {
            person.household_id = u32::MAX;
        }

        for (offset, person_idx) in housed_adults.iter().copied().enumerate() {
            let household_idx = offset % household_count;
            self.people[person_idx].household_id = household_idx as u32;
            member_counts[household_idx] = member_counts[household_idx].saturating_add(1);
        }

        let mut next_household_idx = 0usize;
        for person in &mut self.people {
            if person.household_id != u32::MAX {
                continue;
            }
            let household_idx = next_household_idx % household_count;
            person.household_id = household_idx as u32;
            member_counts[household_idx] = member_counts[household_idx].saturating_add(1);
            next_household_idx = next_household_idx.saturating_add(1);
        }

        let mut sheltered = vec![false; household_count];
        for person in &self.people {
            let household_idx = person.household_id as usize;
            if household_idx < household_count
                && !matches!(person.housing, crate::sim::person::HousingStatus::Unhoused)
            {
                sheltered[household_idx] = true;
            }
        }

        self.households = (0..household_count)
            .map(|idx| Household {
                id: idx as u32,
                member_count: member_counts[idx].max(1),
                sheltered: sheltered[idx],
            })
            .collect();
    }

    pub fn household_size_for_person(&self, person_idx: usize) -> usize {
        self.people
            .get(person_idx)
            .and_then(|person| self.households.get(person.household_id as usize))
            .map_or(1, |household| household.member_count.max(1) as usize)
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
}
