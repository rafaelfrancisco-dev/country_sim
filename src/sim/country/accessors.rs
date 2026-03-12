use super::{Bank, BankId, BankKind, Country, GOODS_COUNT, SimEvent, StockMarket, round_cents};
use crate::sim::params::SimulationParams;

impl Country {
    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn people(&self) -> &[crate::sim::person::Person] {
        &self.people
    }

    pub(crate) fn people_mut(&mut self) -> &mut Vec<crate::sim::person::Person> {
        &mut self.people
    }

    pub fn companies(&self) -> &[crate::sim::company::Company] {
        &self.companies
    }

    pub(crate) fn companies_mut(&mut self) -> &mut Vec<crate::sim::company::Company> {
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
    pub fn monthly_import_tariff_collected(&self) -> f64 {
        self.monthly_import_tariff_collected
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

    pub fn monthly_severe_unmet_need_events(&self) -> u64 {
        self.monthly_severe_unmet_need_events
    }

    pub fn monthly_chronic_unmet_need_count(&self) -> u64 {
        self.monthly_chronic_unmet_need_count
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
    pub fn monthly_immigrants(&self) -> u64 {
        self.monthly_immigrants
    }
    pub fn monthly_emigrants(&self) -> u64 {
        self.monthly_emigrants
    }
    pub fn monthly_migrants_for_work(&self) -> u64 {
        self.monthly_migrants_for_work
    }
    pub fn monthly_migrants_for_distress(&self) -> u64 {
        self.monthly_migrants_for_distress
    }
    pub fn export_tariffs_by_good(&self) -> &[f64; GOODS_COUNT] {
        &self.export_tariffs_by_good
    }
    pub fn import_tariffs_by_good(&self) -> &[f64; GOODS_COUNT] {
        &self.import_tariffs_by_good
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

    pub(crate) fn set_public_bank_fx_reserves(
        &mut self,
        total_base: f64,
        by_source: std::collections::HashMap<usize, f64>,
        counterpart_local: f64,
    ) {
        self.primary_bank_mut()
            .set_fx_reserve_assets(total_base, by_source, counterpart_local);
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
            .flat_map(|bank| bank.stock_positions.iter())
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
            .map(|company| company.name.as_ref())
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
}
