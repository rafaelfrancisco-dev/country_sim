use super::stock_daily::EssentialGoodExportData;
use super::*;
use crate::sim::{
    bank::BankKind,
    essentials::{ESSENTIAL_GOOD_IDS, ESSENTIAL_NEEDS_MASK},
    person::EmploymentStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::sim::world) struct CountryMonthlyExportCache {
    pub(in crate::sim::world) population: usize,
    pub(in crate::sim::world) employed_count: usize,
    pub(in crate::sim::world) unemployed_count: usize,
    pub(in crate::sim::world) labor_force_count: usize,
    pub(in crate::sim::world) non_participating_count: usize,
    pub(in crate::sim::world) headline_non_employed_rate_pct: f64,
    pub(in crate::sim::world) avg_age_years: f64,
    pub(in crate::sim::world) avg_cash_balance: f64,
    pub(in crate::sim::world) total_gross_pay: f64,
    pub(in crate::sim::world) total_tax_paid: f64,
    pub(in crate::sim::world) total_goods_spending: f64,
    pub(in crate::sim::world) count_with_active_loans: usize,
    pub(in crate::sim::world) avg_loan_balance: f64,
    pub(in crate::sim::world) count_with_unmet_needs: usize,
    pub(in crate::sim::world) count_with_severe_unmet_needs: usize,
    pub(in crate::sim::world) count_with_chronic_unmet_needs: usize,
    pub(in crate::sim::world) essential_goods: [EssentialGoodExportData; ESSENTIAL_GOOD_IDS.len()],
    pub(in crate::sim::world) private_people_depositor_share_pct: f64,
    pub(in crate::sim::world) private_company_depositor_share_pct: f64,
    pub(in crate::sim::world) total_employees: u32,
    pub(in crate::sim::world) total_open_positions: usize,
    pub(in crate::sim::world) total_company_cash: f64,
    pub(in crate::sim::world) total_revenue: f64,
    pub(in crate::sim::world) total_payroll: f64,
    pub(in crate::sim::world) total_profit: f64,
    pub(in crate::sim::world) count_profitable: usize,
    pub(in crate::sim::world) count_unprofitable: usize,
    pub(in crate::sim::world) total_units_produced: u32,
    pub(in crate::sim::world) total_units_sold: u32,
    pub(in crate::sim::world) total_units_exported: u32,
    pub(in crate::sim::world) count_public_utility: usize,
    pub(in crate::sim::world) count_backfill: usize,
    pub(in crate::sim::world) count_companies_with_loans: usize,
    pub(in crate::sim::world) bank_count_total: usize,
    pub(in crate::sim::world) bank_count_public: usize,
    pub(in crate::sim::world) bank_count_private: usize,
    pub(in crate::sim::world) bank_health_avg_pct: f64,
    pub(in crate::sim::world) bank_health_min_pct: f64,
    pub(in crate::sim::world) bank_health_max_pct: f64,
    pub(in crate::sim::world) bank_health_public_pct: f64,
    pub(in crate::sim::world) bank_health_private_avg_pct: f64,
    pub(in crate::sim::world) bank_health_private_min_pct: f64,
    pub(in crate::sim::world) bank_health_private_max_pct: f64,
    pub(in crate::sim::world) bank_health_below_50_count: usize,
    pub(in crate::sim::world) bank_health_private_below_50_count: usize,
    pub(in crate::sim::world) private_bank_deposit_liabilities: f64,
    pub(in crate::sim::world) private_bank_deposit_share_pct: f64,
    pub(in crate::sim::world) bank_equity_exposure: f64,
}

impl Default for CountryMonthlyExportCache {
    fn default() -> Self {
        Self {
            population: 0,
            employed_count: 0,
            unemployed_count: 0,
            labor_force_count: 0,
            non_participating_count: 0,
            headline_non_employed_rate_pct: 0.0,
            avg_age_years: 0.0,
            avg_cash_balance: 0.0,
            total_gross_pay: 0.0,
            total_tax_paid: 0.0,
            total_goods_spending: 0.0,
            count_with_active_loans: 0,
            avg_loan_balance: 0.0,
            count_with_unmet_needs: 0,
            count_with_severe_unmet_needs: 0,
            count_with_chronic_unmet_needs: 0,
            essential_goods: [EssentialGoodExportData {
                unmet_share_pct: 0.0,
                severe_unmet_share_pct: 0.0,
                producer_count: 0,
                public_utility_count: 0,
                inventory_cover_months: 0.0,
            }; ESSENTIAL_GOOD_IDS.len()],
            private_people_depositor_share_pct: 0.0,
            private_company_depositor_share_pct: 0.0,
            total_employees: 0,
            total_open_positions: 0,
            total_company_cash: 0.0,
            total_revenue: 0.0,
            total_payroll: 0.0,
            total_profit: 0.0,
            count_profitable: 0,
            count_unprofitable: 0,
            total_units_produced: 0,
            total_units_sold: 0,
            total_units_exported: 0,
            count_public_utility: 0,
            count_backfill: 0,
            count_companies_with_loans: 0,
            bank_count_total: 0,
            bank_count_public: 0,
            bank_count_private: 0,
            bank_health_avg_pct: 0.0,
            bank_health_min_pct: 0.0,
            bank_health_max_pct: 0.0,
            bank_health_public_pct: 0.0,
            bank_health_private_avg_pct: 0.0,
            bank_health_private_min_pct: 0.0,
            bank_health_private_max_pct: 0.0,
            bank_health_below_50_count: 0,
            bank_health_private_below_50_count: 0,
            private_bank_deposit_liabilities: 0.0,
            private_bank_deposit_share_pct: 0.0,
            bank_equity_exposure: 0.0,
        }
    }
}

impl World {
    pub(crate) fn refresh_monthly_export_caches(&mut self) {
        let mut caches = Vec::with_capacity(self.countries.len());
        for country in &self.countries {
            caches.push(build_country_export_cache(self, country));
        }
        self.monthly_export_caches = caches;
    }

    pub(in crate::sim::world) fn export_cache_for_country(
        &self,
        country_idx: usize,
    ) -> Option<&CountryMonthlyExportCache> {
        self.monthly_export_caches.get(country_idx)
    }
}

fn build_country_export_cache(world: &World, country: &Country) -> CountryMonthlyExportCache {
    let mut cache = CountryMonthlyExportCache::default();
    let mut total_employed_population = 0usize;
    let mut total_age_days = 0u64;
    let mut total_cash_balance = 0.0;
    let mut total_loan_balance = 0.0;
    let mut private_people_depositors = 0usize;

    for person in country.people() {
        if person.is_employed() {
            total_employed_population += 1;
        }
        if person.is_labor_force_participant() {
            match person.employment {
                EmploymentStatus::Employed { .. } | EmploymentStatus::EmployedAtBank { .. } => {
                    cache.employed_count += 1
                }
                EmploymentStatus::Unemployed => cache.unemployed_count += 1,
            }
            cache.labor_force_count += 1;
        } else {
            cache.non_participating_count += 1;
        }
        total_age_days += u64::from(person.age_days);
        total_cash_balance += person.cash_balance;
        cache.total_gross_pay += person.last_month_gross_pay;
        cache.total_tax_paid += person.last_month_tax_paid;
        cache.total_goods_spending += person.last_month_goods_spending;
        if let Some(loan) = &person.loan {
            cache.count_with_active_loans += 1;
            total_loan_balance += loan.debt_balance;
        }
        if (person.last_month_unmet_needs_mask & ESSENTIAL_NEEDS_MASK) != 0 {
            cache.count_with_unmet_needs += 1;
        }
        if person.last_month_severe_unmet_needs_mask != 0 {
            cache.count_with_severe_unmet_needs += 1;
        }
        if person.last_month_chronic_unmet {
            cache.count_with_chronic_unmet_needs += 1;
        }
        if person
            .deposit_bank_id
            .and_then(|bank_id| country.bank_by_id(bank_id))
            .is_some_and(|bank| bank.kind == BankKind::Private)
        {
            private_people_depositors += 1;
        }
    }

    cache.population = country.people().len();
    if cache.population > 0 {
        cache.avg_age_years = total_age_days as f64 / cache.population as f64 / 365.0;
        cache.avg_cash_balance = total_cash_balance / cache.population as f64;
        cache.headline_non_employed_rate_pct =
            (cache.population.saturating_sub(total_employed_population)) as f64
                / cache.population as f64
                * 100.0;
        cache.private_people_depositor_share_pct =
            private_people_depositors as f64 / cache.population as f64 * 100.0;
    }
    if cache.count_with_active_loans > 0 {
        cache.avg_loan_balance = total_loan_balance / cache.count_with_active_loans as f64;
    }

    cache.essential_goods = std::array::from_fn(|idx| {
        let good_id = ESSENTIAL_GOOD_IDS[idx];
        let (unmet_count, severe_count) = country.essential_people_counts_for_good(good_id);
        let (producer_count, public_utility_count) =
            country.essential_company_counts_for_good(good_id);
        let denominator = cache.population.max(1) as f64;

        EssentialGoodExportData {
            unmet_share_pct: unmet_count as f64 / denominator * 100.0,
            severe_unmet_share_pct: severe_count as f64 / denominator * 100.0,
            producer_count,
            public_utility_count,
            inventory_cover_months: country.essential_inventory_cover_months_for_good(good_id),
        }
    });

    let mut private_company_depositors = 0usize;
    for company in country.companies() {
        cache.total_employees += company.employee_count;
        cache.total_open_positions += company
            .open_positions
            .iter()
            .map(|opening| usize::from(opening.slots))
            .sum::<usize>();
        cache.total_company_cash += company.cash_balance;
        cache.total_revenue += company.last_month_sales;
        cache.total_payroll += company.last_month_payroll_paid;
        cache.total_profit += company.last_month_profit;
        if company.last_month_profit > 0.0 {
            cache.count_profitable += 1;
        } else {
            cache.count_unprofitable += 1;
        }
        cache.total_units_produced += company.last_month_units_produced;
        cache.total_units_sold += company.last_month_units_sold;
        cache.total_units_exported += company.last_month_export_units_sold;
        if company.is_public_utility {
            cache.count_public_utility += 1;
        }
        if company.is_essential_backfill {
            cache.count_backfill += 1;
        }
        if company.loan.is_some() {
            cache.count_companies_with_loans += 1;
        }
        if company
            .deposit_bank_id
            .and_then(|bank_id| country.bank_by_id(bank_id))
            .is_some_and(|bank| bank.kind == BankKind::Private)
        {
            private_company_depositors += 1;
        }
    }
    if !country.companies().is_empty() {
        cache.private_company_depositor_share_pct =
            private_company_depositors as f64 / country.companies().len() as f64 * 100.0;
    }

    let mut bank_health_sum = 0.0_f64;
    let mut bank_health_min = 1.0_f64;
    let mut bank_health_max = 0.0_f64;
    let mut bank_health_public = 0.0_f64;
    let mut bank_health_private_sum = 0.0_f64;
    let mut bank_health_private_min = 1.0_f64;
    let mut bank_health_private_max = 0.0_f64;
    let mut total_deposit_liabilities = 0.0_f64;

    for bank in country.all_banks() {
        let health = country.bank_health_score(bank.id).clamp(0.0, 1.0);
        cache.bank_count_total += 1;
        bank_health_sum += health;
        bank_health_min = bank_health_min.min(health);
        bank_health_max = bank_health_max.max(health);
        if health < 0.5 {
            cache.bank_health_below_50_count += 1;
        }

        let deposits = bank.deposit_liabilities.max(0.0);
        total_deposit_liabilities += deposits;

        if bank.kind == BankKind::Private {
            cache.bank_count_private += 1;
            bank_health_private_sum += health;
            bank_health_private_min = bank_health_private_min.min(health);
            bank_health_private_max = bank_health_private_max.max(health);
            cache.private_bank_deposit_liabilities += deposits;
            if health < 0.5 {
                cache.bank_health_private_below_50_count += 1;
            }
        } else {
            cache.bank_count_public += 1;
            bank_health_public = health;
        }
    }

    if cache.bank_count_total > 0 {
        cache.bank_health_avg_pct = bank_health_sum / cache.bank_count_total as f64 * 100.0;
        cache.bank_health_min_pct = bank_health_min * 100.0;
        cache.bank_health_max_pct = bank_health_max * 100.0;
    }
    cache.bank_health_public_pct = bank_health_public * 100.0;
    if cache.bank_count_private > 0 {
        cache.bank_health_private_avg_pct =
            bank_health_private_sum / cache.bank_count_private as f64 * 100.0;
        cache.bank_health_private_min_pct = bank_health_private_min * 100.0;
        cache.bank_health_private_max_pct = bank_health_private_max * 100.0;
    }
    if total_deposit_liabilities > 0.0 {
        cache.private_bank_deposit_share_pct =
            cache.private_bank_deposit_liabilities / total_deposit_liabilities * 100.0;
    }

    cache.bank_equity_exposure = world.market_bank_portfolio_value(country.id());
    cache
}
