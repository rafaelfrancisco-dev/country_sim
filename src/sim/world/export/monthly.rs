use super::monthly_events::{EventCounts, event_country_and_day};
use super::*;

impl World {
    pub(in crate::sim::world) fn push_export_snapshot(&mut self) {
        let month = self.day() / DAYS_PER_MONTH;
        let month_start_day = self.day().saturating_sub(DAYS_PER_MONTH - 1);
        let generation_seed = self.generation_seed();

        let countries_data: Vec<CountryExportData> = {
            let event_log = self.event_log();
            self.countries
                .iter()
                .enumerate()
                .map(|(country_idx, country)| {
                    collect_country_data(
                        self,
                        country_idx,
                        country,
                        event_log,
                        month_start_day,
                        month,
                    )
                })
                .collect()
        };

        self.push_export_row(MonthlyExportData {
            month,
            generation_seed,
            countries: countries_data,
        });
    }
}

fn collect_country_data(
    world: &World,
    country_idx: usize,
    country: &Country,
    event_log: &std::collections::VecDeque<SimEvent>,
    month_start_day: u64,
    month: u64,
) -> CountryExportData {
    let cache = world
        .export_cache_for_country(country_idx)
        .cloned()
        .unwrap_or_default();

    // Event counting
    let mut events = EventCounts::default();

    // Events from event_log (days 1 to month_end-1, already drained from previous days)
    for event in event_log {
        let (ev_country, ev_day) = event_country_and_day(event);
        if ev_day >= month_start_day && ev_country == country.name() {
            events.accumulate(event);
        }
    }

    // Events from pending_events (month-end day, not yet drained)
    for event in country.pending_events() {
        events.accumulate(event);
    }

    CountryExportData {
        month,
        id: country.id(),
        name: country.name().to_string(),
        population: cache.population,
        treasury_balance: country.treasury_balance(),
        monthly_tax_collected: country.monthly_tax_collected(),
        monthly_tariff_collected: country.monthly_tariff_collected(),
        monthly_gov_spending: country.monthly_gov_spending(),
        monthly_budget_balance: country.monthly_budget_balance(),
        gov_debt: country.gov_debt(),
        monthly_household_spending: country.monthly_household_spending(),
        monthly_goods_import_spending: country.monthly_goods_import_spending(),
        monthly_goods_export_revenue: country.monthly_goods_export_revenue(),
        monthly_unmet_need_events: country.monthly_unmet_need_events(),
        monthly_severe_unmet_need_events: country.monthly_severe_unmet_need_events(),
        monthly_chronic_unmet_need_count: country.monthly_chronic_unmet_need_count(),
        monthly_essential_debt_spending: country.monthly_essential_debt_spending(),
        total_money_supply: country.total_money_supply(),
        household_cash_total: country.household_cash_total(),
        company_cash_total: country.company_cash_total(),
        bank_reserve_balance: country.total_bank_reserves(),
        bank_created_money_outstanding: country.total_bank_created_money(),
        expected_money_supply: country.expected_money_supply(),
        money_supply_gap: country.money_supply_gap(),
        monthly_money_minted: country.monthly_money_minted(),
        cumulative_money_minted: country.cumulative_money_minted(),
        bank_count_total: cache.bank_count_total,
        bank_count_public: cache.bank_count_public,
        bank_count_private: cache.bank_count_private,
        bank_health_avg_pct: cache.bank_health_avg_pct,
        bank_health_min_pct: cache.bank_health_min_pct,
        bank_health_max_pct: cache.bank_health_max_pct,
        bank_health_public_pct: cache.bank_health_public_pct,
        bank_health_private_avg_pct: cache.bank_health_private_avg_pct,
        bank_health_private_min_pct: cache.bank_health_private_min_pct,
        bank_health_private_max_pct: cache.bank_health_private_max_pct,
        bank_health_below_50_count: cache.bank_health_below_50_count,
        bank_health_private_below_50_count: cache.bank_health_private_below_50_count,
        private_bank_deposit_liabilities: cache.private_bank_deposit_liabilities,
        private_bank_deposit_share_pct: cache.private_bank_deposit_share_pct,
        private_people_depositor_share_pct: cache.private_people_depositor_share_pct,
        private_company_depositor_share_pct: cache.private_company_depositor_share_pct,
        monthly_private_banks_founded: country.monthly_private_banks_founded(),
        cumulative_private_banks_founded: country.cumulative_private_banks_founded(),
        monthly_private_banks_closed: country.monthly_private_banks_closed(),
        cumulative_private_banks_closed: country.cumulative_private_banks_closed(),
        employed_count: cache.employed_count,
        unemployed_count: cache.unemployed_count,
        labor_force_count: cache.labor_force_count,
        non_participating_count: cache.non_participating_count,
        headline_non_employed_rate_pct: cache.headline_non_employed_rate_pct,
        avg_age_years: cache.avg_age_years,
        avg_cash_balance: cache.avg_cash_balance,
        total_gross_pay: cache.total_gross_pay,
        total_tax_paid: cache.total_tax_paid,
        total_goods_spending: cache.total_goods_spending,
        count_with_active_loans: cache.count_with_active_loans,
        avg_loan_balance: cache.avg_loan_balance,
        count_with_unmet_needs: cache.count_with_unmet_needs,
        count_with_severe_unmet_needs: cache.count_with_severe_unmet_needs,
        count_with_chronic_unmet_needs: cache.count_with_chronic_unmet_needs,
        essential_goods: cache.essential_goods,
        total_companies: country.companies().len(),
        total_employees: cache.total_employees,
        total_open_positions: cache.total_open_positions,
        total_company_cash: cache.total_company_cash,
        total_revenue: cache.total_revenue,
        total_payroll: cache.total_payroll,
        total_profit: cache.total_profit,
        count_profitable: cache.count_profitable,
        count_unprofitable: cache.count_unprofitable,
        total_units_produced: cache.total_units_produced,
        total_units_sold: cache.total_units_sold,
        total_units_exported: cache.total_units_exported,
        count_public_utility: cache.count_public_utility,
        count_backfill: cache.count_backfill,
        count_companies_with_loans: cache.count_companies_with_loans,
        births: events.births,
        deaths: events.deaths,
        job_switches: events.job_switches,
        layoffs_total: events.layoffs_total,
        separations_quits: events.separations_quits,
        separations_retirements: events.separations_retirements,
        separations_layoffs_non_distress: events.separations_layoffs_non_distress,
        separations_total: events.separations_total,
        companies_founded: events.companies_founded,
        banks_founded: events.banks_founded,
        banks_closed: events.banks_closed,
        bank_runs_started: events.bank_runs_started,
        bank_resolutions_started: events.bank_resolutions_started,
        bankruptcies: events.bankruptcies,
        wage_cuts: events.wage_cuts,
        loans_issued: events.loans_issued,
        loans_ended: events.loans_ended,
        loans_defaulted: events.loans_defaulted,
        debt_distress_events: events.debt_distress_events,
        gov_transfers: events.gov_transfers,
        hires_total: country.monthly_hires_total(),
        stock_ipos: events.stock_ipos,
        stock_delistings: events.stock_delistings,
        stock_margin_calls: events.stock_margin_calls,
        bonds_outstanding: country.bonds_outstanding(),
        bond_coupon_rate: country.bond_coupon_rate(),
        monthly_coupon_paid: country.monthly_coupon_paid(),
        monthly_bonds_issued: country.monthly_bonds_issued(),
        monthly_bonds_matured: country.monthly_bonds_matured(),
        cpi_index: country.cpi_index(),
        inflation_rate: country.monthly_inflation_rate(),
        policy_rate: country.policy_rate(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{
        company::build_default_companies,
        person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
    };

    #[test]
    fn collect_country_data_populates_per_good_essential_export_metrics() {
        let mut person = Person::new_with_profile(
            1,
            EmploymentStatus::Unemployed,
            7,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.last_month_unmet_needs_mask = 1;
        person.last_month_severe_unmet_needs_mask = 1;

        let mut companies = build_default_companies("A", 1);
        companies[0].inventory_units = 9;
        companies[0].is_public_utility = true;

        let country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);
        let mut world = World::new(vec![country]);
        world.refresh_monthly_export_caches();
        let empty_log = std::collections::VecDeque::new();
        let data = collect_country_data(&world, 0, &world.countries[0], &empty_log, 1, 1);

        assert_eq!(data.essential_goods[0].unmet_share_pct, 100.0);
        assert_eq!(data.essential_goods[0].severe_unmet_share_pct, 100.0);
        assert_eq!(data.essential_goods[0].producer_count, 1);
        assert_eq!(data.essential_goods[0].public_utility_count, 1);
        assert!(data.essential_goods[0].inventory_cover_months > 0.0);
    }
}
