use super::{BankMetricPoint, HISTORY_LIMIT, MetricPoint, World};
use crate::sim::bank::Bank;
use crate::sim::country::Country;

impl World {
    pub(super) fn capture_snapshot(&mut self) {
        for (idx, country) in self.countries.iter().enumerate() {
            let point = Self::country_metric_point(country);
            if let Some(history) = self.country_history.get_mut(idx) {
                Self::push_metric_point(history, point);
            }

            for bank in std::iter::once(&country.bank).chain(country.banks.iter()) {
                let key = (idx, bank.id);
                let bank_point = Self::bank_metric_point(country.month, bank);
                let history = self
                    .bank_history
                    .entry(key)
                    .or_insert_with(|| Vec::with_capacity(HISTORY_LIMIT));
                Self::push_bank_metric_point(history, bank_point);
            }
        }

        let aggregate = self.aggregate_metric_point();
        Self::push_metric_point(&mut self.world_history, aggregate);
    }

    fn aggregate_metric_point(&self) -> MetricPoint {
        let count = self.countries.len().max(1) as f64;
        let month = self
            .countries
            .iter()
            .map(|country| country.month)
            .max()
            .unwrap_or(0);
        let population = self
            .countries
            .iter()
            .map(|country| country.population() as f64)
            .sum();
        let total_money_supply = self
            .countries
            .iter()
            .map(|country| country.total_money_supply)
            .sum();
        let household_cash_total = self
            .countries
            .iter()
            .map(|country| country.household_cash_total)
            .sum();
        let company_cash_total = self
            .countries
            .iter()
            .map(|country| country.company_cash_total)
            .sum();
        let expected_money_supply = self
            .countries
            .iter()
            .map(|country| country.expected_money_supply)
            .sum();
        let money_supply_gap = self
            .countries
            .iter()
            .map(|country| country.money_supply_gap)
            .sum();
        let created_money_outstanding = self
            .countries
            .iter()
            .map(|country| country.total_bank_created_money())
            .sum();
        let bank_reserve = self
            .countries
            .iter()
            .map(|country| country.total_bank_reserves())
            .sum();
        let treasury_balance = self
            .countries
            .iter()
            .map(|country| country.treasury_balance)
            .sum();
        let budget_balance = self
            .countries
            .iter()
            .map(|country| country.monthly_budget_balance)
            .sum();
        let monthly_tax_collected = self
            .countries
            .iter()
            .map(|country| country.monthly_tax_collected)
            .sum();
        let monthly_gov_spending = self
            .countries
            .iter()
            .map(|country| country.monthly_gov_spending)
            .sum();
        let gov_debt = self.countries.iter().map(|country| country.gov_debt).sum();
        let monthly_money_minted = self
            .countries
            .iter()
            .map(|country| country.monthly_money_minted)
            .sum();
        let cumulative_money_minted = self
            .countries
            .iter()
            .map(|country| country.cumulative_money_minted)
            .sum();
        let monthly_goods_import_spending = self
            .countries
            .iter()
            .map(|country| country.monthly_goods_import_spending)
            .sum();
        let monthly_goods_export_revenue = self
            .countries
            .iter()
            .map(|country| country.monthly_goods_export_revenue)
            .sum();
        let monthly_unmet_need_events = self
            .countries
            .iter()
            .map(|country| country.monthly_unmet_need_events as f64)
            .sum();
        let monthly_essential_debt_spending = self
            .countries
            .iter()
            .map(|country| country.monthly_essential_debt_spending)
            .sum();
        let cpi_index = self
            .countries
            .iter()
            .map(|country| country.cpi_index)
            .sum::<f64>()
            / count;
        let inflation_rate = self
            .countries
            .iter()
            .map(|country| country.monthly_inflation_rate)
            .sum::<f64>()
            / count;
        let total_population = self
            .countries
            .iter()
            .map(|country| country.population() as f64)
            .sum::<f64>();
        let (deposit_rate, loan_rate) = if total_population > 0.0 {
            let weighted_deposit = self
                .countries
                .iter()
                .map(|country| country.deposit_rate() * country.population() as f64)
                .sum::<f64>()
                / total_population;
            let weighted_loan = self
                .countries
                .iter()
                .map(|country| country.bank_loan_rate() * country.population() as f64)
                .sum::<f64>()
                / total_population;
            (weighted_deposit, weighted_loan)
        } else {
            (0.0, 0.0)
        };
        let monthly_savings_interest_paid = self
            .countries
            .iter()
            .map(|country| country.monthly_savings_interest_paid)
            .sum();
        let bonds_outstanding = self
            .countries
            .iter()
            .map(|country| country.bonds_outstanding)
            .sum();
        let stock_market_cap = self
            .countries
            .iter()
            .map(|country| country.stock_market.total_market_cap())
            .sum();
        let stock_bank_equity_exposure = self
            .countries
            .iter()
            .map(|country| {
                country.stock_market.bank_equity_exposure()
                    + country.bank_cross_country_equity_exposure_cost()
            })
            .sum();

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
            monthly_goods_import_spending,
            monthly_goods_export_revenue,
            monthly_unmet_need_events,
            monthly_essential_debt_spending,
            cpi_index,
            inflation_rate,
            policy_rate: self.policy_rate,
            deposit_rate,
            loan_rate,
            monthly_savings_interest_paid,
            bonds_outstanding,
            stock_market_cap,
            stock_bank_equity_exposure,
        }
    }

    fn country_metric_point(country: &Country) -> MetricPoint {
        MetricPoint {
            month: country.month,
            population: country.population() as f64,
            total_money_supply: country.total_money_supply,
            household_cash_total: country.household_cash_total,
            company_cash_total: country.company_cash_total,
            expected_money_supply: country.expected_money_supply,
            money_supply_gap: country.money_supply_gap,
            created_money_outstanding: country.total_bank_created_money(),
            bank_reserve: country.total_bank_reserves(),
            treasury_balance: country.treasury_balance,
            budget_balance: country.monthly_budget_balance,
            monthly_tax_collected: country.monthly_tax_collected,
            monthly_gov_spending: country.monthly_gov_spending,
            gov_debt: country.gov_debt,
            monthly_money_minted: country.monthly_money_minted,
            cumulative_money_minted: country.cumulative_money_minted,
            monthly_goods_import_spending: country.monthly_goods_import_spending,
            monthly_goods_export_revenue: country.monthly_goods_export_revenue,
            monthly_unmet_need_events: country.monthly_unmet_need_events as f64,
            monthly_essential_debt_spending: country.monthly_essential_debt_spending,
            cpi_index: country.cpi_index,
            inflation_rate: country.monthly_inflation_rate,
            policy_rate: country.policy_rate,
            deposit_rate: country.deposit_rate(),
            loan_rate: country.bank_loan_rate(),
            monthly_savings_interest_paid: country.monthly_savings_interest_paid,
            bonds_outstanding: country.bonds_outstanding,
            stock_market_cap: country.stock_market.total_market_cap(),
            stock_bank_equity_exposure: country.stock_market.bank_equity_exposure()
                + country.bank_cross_country_equity_exposure_cost(),
        }
    }

    fn push_metric_point(history: &mut Vec<MetricPoint>, point: MetricPoint) {
        history.push(point);
        if history.len() > HISTORY_LIMIT {
            history.remove(0);
        }
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

    fn push_bank_metric_point(history: &mut Vec<BankMetricPoint>, point: BankMetricPoint) {
        history.push(point);
        if history.len() > HISTORY_LIMIT {
            history.remove(0);
        }
    }
}
