use std::collections::{HashSet, VecDeque};

use super::{BankMetricPoint, HISTORY_LIMIT, MetricPoint, World};
use crate::sim::bank::Bank;
use crate::sim::country::Country;

impl World {
    pub(super) fn capture_snapshot(&mut self) {
        for (idx, country) in self.countries.iter().enumerate() {
            let point = self.country_metric_point(country);
            if let Some(history) = self.country_history.get_mut(idx) {
                Self::push_metric_point(history, point);
            }

            for bank in country.all_banks() {
                let key = (idx, bank.id);
                let bank_point = Self::bank_metric_point(country.month(), bank);
                let history = self
                    .bank_history
                    .entry(key)
                    .or_insert_with(|| VecDeque::with_capacity(HISTORY_LIMIT));
                Self::push_bank_metric_point(history, bank_point);
            }
        }

        let aggregate = self.aggregate_metric_point();
        Self::push_metric_point(&mut self.world_history, aggregate);

        // Prune history for banks that no longer exist to prevent unbounded growth.
        let live_keys: HashSet<(usize, u16)> = self
            .countries
            .iter()
            .enumerate()
            .flat_map(|(idx, country)| country.all_banks().map(move |bank| (idx, bank.id)))
            .collect();
        self.bank_history.retain(|key, _| live_keys.contains(key));
    }

    fn aggregate_metric_point(&self) -> MetricPoint {
        let n = self.countries.len().max(1) as f64;

        // Accumulators for plain sums.
        let mut month = 0_u64;
        let mut population = 0.0_f64;
        let mut total_money_supply = 0.0_f64;
        let mut household_cash_total = 0.0_f64;
        let mut company_cash_total = 0.0_f64;
        let mut expected_money_supply = 0.0_f64;
        let mut money_supply_gap = 0.0_f64;
        let mut created_money_outstanding = 0.0_f64;
        let mut bank_reserve = 0.0_f64;
        let mut treasury_balance = 0.0_f64;
        let mut budget_balance = 0.0_f64;
        let mut monthly_tax_collected = 0.0_f64;
        let mut monthly_gov_spending = 0.0_f64;
        let mut gov_debt = 0.0_f64;
        let mut monthly_money_minted = 0.0_f64;
        let mut cumulative_money_minted = 0.0_f64;
        let mut monthly_mint_fraction_sum = 0.0_f64;
        let mut monthly_goods_import_spending = 0.0_f64;
        let mut monthly_goods_export_revenue = 0.0_f64;
        let mut monthly_unmet_need_events = 0.0_f64;
        let mut monthly_severe_unmet_need_events = 0.0_f64;
        let mut monthly_chronic_unmet_need_count = 0.0_f64;
        let mut monthly_essential_debt_spending = 0.0_f64;
        let mut monthly_savings_interest_paid = 0.0_f64;
        let mut bonds_outstanding = 0.0_f64;
        let mut stock_market_cap = 0.0_f64;
        let mut stock_bank_equity_exposure = 0.0_f64;
        let mut monthly_foreclosures = 0.0_f64;
        let mut monthly_home_purchases = 0.0_f64;
        let mut monthly_immigrants = 0.0_f64;
        let mut monthly_emigrants = 0.0_f64;
        let mut monthly_migrants_for_work = 0.0_f64;
        let mut monthly_migrants_for_distress = 0.0_f64;
        // Accumulators for averages.
        let mut cpi_index_sum = 0.0_f64;
        let mut inflation_rate_sum = 0.0_f64;
        let mut housing_price_index_sum = 0.0_f64;
        let mut monthly_rent_sum = 0.0_f64;
        let mut homeownership_rate_sum = 0.0_f64;
        let mut housing_vacancy_rate_sum = 0.0_f64;
        // Accumulators for deposit/loan rates weighted by population.
        let mut weighted_deposit_sum = 0.0_f64;
        let mut weighted_loan_sum = 0.0_f64;
        let mut weighted_policy_sum = 0.0_f64;
        let mut total_population_f64 = 0.0_f64;

        for (country_idx, country) in self.countries.iter().enumerate() {
            let pop = country.population() as f64;
            month = month.max(country.month());
            population += pop;
            total_money_supply += country.total_money_supply();
            household_cash_total += country.household_cash_total();
            company_cash_total += country.company_cash_total();
            expected_money_supply += country.expected_money_supply();
            money_supply_gap += country.money_supply_gap();
            created_money_outstanding += country.total_bank_created_money();
            bank_reserve += country.total_bank_reserves();
            treasury_balance += country.treasury_balance();
            budget_balance += country.monthly_budget_balance();
            monthly_tax_collected += country.monthly_tax_collected();
            monthly_gov_spending += country.monthly_gov_spending();
            gov_debt += country.gov_debt();
            monthly_money_minted += country.monthly_money_minted();
            cumulative_money_minted += country.cumulative_money_minted();
            monthly_mint_fraction_sum += country.last_mint_fraction();
            monthly_goods_import_spending += country.monthly_goods_import_spending();
            monthly_goods_export_revenue += country.monthly_goods_export_revenue();
            monthly_unmet_need_events += country.monthly_unmet_need_events() as f64;
            monthly_severe_unmet_need_events += country.monthly_severe_unmet_need_events() as f64;
            monthly_chronic_unmet_need_count += country.monthly_chronic_unmet_need_count() as f64;
            monthly_essential_debt_spending += country.monthly_essential_debt_spending();
            monthly_savings_interest_paid += country.monthly_savings_interest_paid();
            bonds_outstanding += country.bonds_outstanding();
            stock_market_cap += country.stock_market().total_market_cap();
            stock_bank_equity_exposure += self
                .export_cache_for_country(country_idx)
                .map(|cache| cache.bank_equity_exposure)
                .unwrap_or_else(|| self.market_bank_portfolio_value(country.id()));
            monthly_foreclosures += country.monthly_foreclosures() as f64;
            monthly_home_purchases += country.monthly_home_purchases() as f64;
            monthly_immigrants += country.monthly_immigrants() as f64;
            monthly_emigrants += country.monthly_emigrants() as f64;
            monthly_migrants_for_work += country.monthly_migrants_for_work() as f64;
            monthly_migrants_for_distress += country.monthly_migrants_for_distress() as f64;
            cpi_index_sum += country.cpi_index();
            inflation_rate_sum += country.monthly_inflation_rate();
            housing_price_index_sum += country.housing_price_index();
            monthly_rent_sum += country.monthly_rent();
            homeownership_rate_sum += country.homeownership_rate();
            housing_vacancy_rate_sum += country.housing_vacancy_rate();
            total_population_f64 += pop;
            weighted_policy_sum += country.policy_rate() * pop;
            weighted_deposit_sum += country.deposit_rate() * pop;
            weighted_loan_sum += country.bank_loan_rate() * pop;
        }

        let (policy_rate, deposit_rate, loan_rate) = if total_population_f64 > 0.0 {
            (
                weighted_policy_sum / total_population_f64,
                weighted_deposit_sum / total_population_f64,
                weighted_loan_sum / total_population_f64,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        MetricPoint {
            month,
            population,
            total_money_supply,
            household_cash_total,
            company_cash_total,
            expected_money_supply,
            money_supply_gap,
            created_money_outstanding,
            bank_reserve,
            treasury_balance,
            budget_balance,
            monthly_tax_collected,
            monthly_gov_spending,
            gov_debt,
            monthly_money_minted,
            cumulative_money_minted,
            monthly_mint_fraction: monthly_mint_fraction_sum / n,
            monthly_goods_import_spending,
            monthly_goods_export_revenue,
            monthly_unmet_need_events,
            monthly_severe_unmet_need_events,
            monthly_chronic_unmet_need_count,
            monthly_essential_debt_spending,
            cpi_index: cpi_index_sum / n,
            inflation_rate: inflation_rate_sum / n,
            policy_rate,
            deposit_rate,
            loan_rate,
            monthly_savings_interest_paid,
            bonds_outstanding,
            stock_market_cap,
            stock_bank_equity_exposure,
            housing_price_index: housing_price_index_sum / n,
            monthly_rent: monthly_rent_sum / n,
            homeownership_rate: homeownership_rate_sum / n,
            housing_vacancy_rate: housing_vacancy_rate_sum / n,
            monthly_foreclosures,
            monthly_home_purchases,
            monthly_immigrants,
            monthly_emigrants,
            monthly_migrants_for_work,
            monthly_migrants_for_distress,
        }
    }

    fn country_metric_point(&self, country: &Country) -> MetricPoint {
        let country_idx = self
            .countries
            .iter()
            .position(|candidate| candidate.id() == country.id());
        MetricPoint {
            month: country.month(),
            population: country.population() as f64,
            total_money_supply: country.total_money_supply(),
            household_cash_total: country.household_cash_total(),
            company_cash_total: country.company_cash_total(),
            expected_money_supply: country.expected_money_supply(),
            money_supply_gap: country.money_supply_gap(),
            created_money_outstanding: country.total_bank_created_money(),
            bank_reserve: country.total_bank_reserves(),
            treasury_balance: country.treasury_balance(),
            budget_balance: country.monthly_budget_balance(),
            monthly_tax_collected: country.monthly_tax_collected(),
            monthly_gov_spending: country.monthly_gov_spending(),
            gov_debt: country.gov_debt(),
            monthly_money_minted: country.monthly_money_minted(),
            cumulative_money_minted: country.cumulative_money_minted(),
            monthly_mint_fraction: country.last_mint_fraction(),
            monthly_goods_import_spending: country.monthly_goods_import_spending(),
            monthly_goods_export_revenue: country.monthly_goods_export_revenue(),
            monthly_unmet_need_events: country.monthly_unmet_need_events() as f64,
            monthly_severe_unmet_need_events: country.monthly_severe_unmet_need_events() as f64,
            monthly_chronic_unmet_need_count: country.monthly_chronic_unmet_need_count() as f64,
            monthly_essential_debt_spending: country.monthly_essential_debt_spending(),
            cpi_index: country.cpi_index(),
            inflation_rate: country.monthly_inflation_rate(),
            policy_rate: country.policy_rate(),
            deposit_rate: country.deposit_rate(),
            loan_rate: country.bank_loan_rate(),
            monthly_savings_interest_paid: country.monthly_savings_interest_paid(),
            bonds_outstanding: country.bonds_outstanding(),
            stock_market_cap: country.stock_market().total_market_cap(),
            stock_bank_equity_exposure: country_idx
                .and_then(|idx| self.export_cache_for_country(idx))
                .map(|cache| cache.bank_equity_exposure)
                .unwrap_or_else(|| self.market_bank_portfolio_value(country.id())),
            housing_price_index: country.housing_price_index(),
            monthly_rent: country.monthly_rent(),
            homeownership_rate: country.homeownership_rate(),
            housing_vacancy_rate: country.housing_vacancy_rate(),
            monthly_foreclosures: country.monthly_foreclosures() as f64,
            monthly_home_purchases: country.monthly_home_purchases() as f64,
            monthly_immigrants: country.monthly_immigrants() as f64,
            monthly_emigrants: country.monthly_emigrants() as f64,
            monthly_migrants_for_work: country.monthly_migrants_for_work() as f64,
            monthly_migrants_for_distress: country.monthly_migrants_for_distress() as f64,
        }
    }

    fn push_metric_point(history: &mut VecDeque<MetricPoint>, point: MetricPoint) {
        if history.len() >= HISTORY_LIMIT {
            history.pop_front();
        }
        history.push_back(point);
    }

    fn bank_metric_point(month: u64, bank: &Bank) -> BankMetricPoint {
        BankMetricPoint {
            month,
            reserve: bank.reserve_balance,
            deposits: bank.deposit_liabilities,
            created_money: bank.created_money_outstanding,
            equity: bank.reserve_balance + bank.created_money_outstanding
                - bank.deposit_liabilities,
            run_intensity: bank.run_intensity,
        }
    }

    fn push_bank_metric_point(history: &mut VecDeque<BankMetricPoint>, point: BankMetricPoint) {
        if history.len() >= HISTORY_LIMIT {
            history.pop_front();
        }
        history.push_back(point);
    }
}
