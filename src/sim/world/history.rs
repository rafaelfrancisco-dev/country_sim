use super::{HISTORY_LIMIT, MetricPoint, World};
use crate::sim::country::Country;

impl World {
    pub(super) fn capture_snapshot(&mut self) {
        for (idx, country) in self.countries.iter().enumerate() {
            let point = Self::country_metric_point(country);
            if let Some(history) = self.country_history.get_mut(idx) {
                Self::push_metric_point(history, point);
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
        let created_money_outstanding = self
            .countries
            .iter()
            .map(|country| country.bank.created_money_outstanding)
            .sum();
        let bank_reserve = self
            .countries
            .iter()
            .map(|country| country.bank.reserve_balance)
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
        let bonds_outstanding = self
            .countries
            .iter()
            .map(|country| country.bonds_outstanding)
            .sum();

        MetricPoint {
            month,
            population,
            total_money_supply,
            created_money_outstanding,
            bank_reserve,
            treasury_balance,
            budget_balance,
            cpi_index,
            inflation_rate,
            policy_rate: self.policy_rate,
            bonds_outstanding,
        }
    }

    fn country_metric_point(country: &Country) -> MetricPoint {
        MetricPoint {
            month: country.month,
            population: country.population() as f64,
            total_money_supply: country.total_money_supply,
            created_money_outstanding: country.bank.created_money_outstanding,
            bank_reserve: country.bank.reserve_balance,
            treasury_balance: country.treasury_balance,
            budget_balance: country.monthly_budget_balance,
            cpi_index: country.cpi_index,
            inflation_rate: country.monthly_inflation_rate,
            policy_rate: country.policy_rate,
            bonds_outstanding: country.bonds_outstanding,
        }
    }

    fn push_metric_point(history: &mut Vec<MetricPoint>, point: MetricPoint) {
        history.push(point);
        if history.len() > HISTORY_LIMIT {
            history.remove(0);
        }
    }
}
