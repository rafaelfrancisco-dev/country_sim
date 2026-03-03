mod export;
mod goods_market;
mod history;
mod monetary;
mod stock_market;

pub use export::{
    CountryExportData, MonthlyExportData, StockDailyCountryExportData, StockDailyEquityExportData,
};

use rayon::prelude::*;
#[cfg(feature = "bench-trace")]
use std::time::Instant;

use super::{
    country::Country,
    country::DAYS_PER_MONTH,
    event::{EVENT_LOG_LIMIT, SimEvent},
    tickable::Tickable,
};

pub const HISTORY_LIMIT: usize = 120;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MetricPoint {
    pub month: u64,
    pub population: f64,
    pub total_money_supply: f64,
    pub created_money_outstanding: f64,
    pub bank_reserve: f64,
    pub treasury_balance: f64,
    pub budget_balance: f64,
    pub cpi_index: f64,
    pub inflation_rate: f64,
    pub policy_rate: f64,
    pub bonds_outstanding: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub day: u64,
    pub generation_seed: u64,
    pub countries: Vec<Country>,
    pub paused: bool,
    pub policy_rate: f64,
    pub world_history: Vec<MetricPoint>,
    pub country_history: Vec<Vec<MetricPoint>>,
    pub event_log: Vec<SimEvent>,
    pub export_queue: Vec<MonthlyExportData>,
    pub export_queue_active: bool,
    pub stock_country_export_queue: Vec<StockDailyCountryExportData>,
    pub stock_equity_export_queue: Vec<StockDailyEquityExportData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    pub code: &'static str,
    pub message: String,
}

impl World {
    #[cfg(test)]
    pub fn new(countries: Vec<Country>) -> Self {
        Self::new_with_generation_seed(countries, 0)
    }

    pub fn new_with_generation_seed(countries: Vec<Country>, generation_seed: u64) -> Self {
        let country_count = countries.len();
        let initial_policy_rate = countries
            .first()
            .map(|c| c.sim_params.monetary.neutral_policy_rate)
            .unwrap_or(0.04);

        let mut world = Self {
            day: 0,
            generation_seed,
            countries,
            paused: false,
            policy_rate: initial_policy_rate,
            world_history: Vec::with_capacity(HISTORY_LIMIT),
            country_history: vec![Vec::with_capacity(HISTORY_LIMIT); country_count],
            event_log: Vec::with_capacity(EVENT_LOG_LIMIT),
            export_queue: Vec::new(),
            export_queue_active: false,
            stock_country_export_queue: Vec::new(),
            stock_equity_export_queue: Vec::new(),
        };
        world.capture_snapshot();
        world
    }

    pub fn total_population(&self) -> usize {
        self.countries.iter().map(Country::population).sum()
    }

    #[cfg(test)]
    pub fn completed_month_boundaries(&self) -> u64 {
        self.day / DAYS_PER_MONTH
    }

    fn drain_country_events(&mut self) {
        let pending: Vec<SimEvent> = self
            .countries
            .iter_mut()
            .flat_map(|c| c.pending_events.drain(..))
            .collect();
        self.event_log.extend(pending);
        if self.event_log.len() > EVENT_LOG_LIMIT {
            let evict = EVENT_LOG_LIMIT / 4;
            self.event_log.drain(0..evict);
        }
    }

    pub fn validate_invariants(&self) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        if self.country_history.len() != self.countries.len() {
            violations.push(InvariantViolation {
                code: "country_history_len",
                message: format!(
                    "country history length {} does not match countries {}",
                    self.country_history.len(),
                    self.countries.len()
                ),
            });
        }

        let expected_month_points = (self.day / DAYS_PER_MONTH + 1) as usize;
        if self.world_history.len() > HISTORY_LIMIT {
            violations.push(InvariantViolation {
                code: "world_history_limit",
                message: format!(
                    "world history length {} exceeds limit {}",
                    self.world_history.len(),
                    HISTORY_LIMIT
                ),
            });
        }
        if !self.world_history.is_empty() && self.world_history.len() > expected_month_points {
            violations.push(InvariantViolation {
                code: "world_history_count",
                message: format!(
                    "world history length {} exceeds expected points {}",
                    self.world_history.len(),
                    expected_month_points
                ),
            });
        }

        for country in &self.countries {
            for violation in country.validate_money_invariants() {
                violations.push(InvariantViolation {
                    code: violation.code,
                    message: format!("country {}: {}", country.id, violation.message),
                });
            }
        }

        violations
    }
}

impl Tickable for World {
    fn tick_day(&mut self) {
        self.day = self.day.saturating_add(1);

        self.countries.par_iter_mut().for_each(Country::tick_day);

        self.run_daily_goods_economy();
        self.run_daily_stock_market();

        if self.day.is_multiple_of(DAYS_PER_MONTH) {
            self.countries
                .par_iter_mut()
                .for_each(Country::process_month_end);
            self.adjust_dynamic_country_tariffs();
            self.update_policy_rate();
            let rate = self.policy_rate;
            for country in &mut self.countries {
                country.policy_rate = rate;
            }
            self.capture_snapshot();
            if self.export_queue_active {
                self.push_export_snapshot();
            }
            self.countries
                .par_iter_mut()
                .for_each(Country::reset_month_trackers);
        }
        self.drain_country_events();
    }
}

#[cfg(feature = "bench-trace")]
impl World {
    pub(crate) fn tick_day_with_trace(&mut self) -> crate::sim::trace::DayTraceRow {
        let tick_started = Instant::now();

        self.day = self.day.saturating_add(1);
        let day = self.day;

        let started = Instant::now();
        let daily_traces = self
            .countries
            .par_iter_mut()
            .map(Country::tick_day_without_month_traced)
            .collect::<Vec<_>>();
        let world_country_tick_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        self.run_daily_goods_economy();
        let world_goods_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        self.run_daily_stock_market();
        let world_stock_ms = crate::sim::trace::duration_ms(started.elapsed());

        let mut world_monthly_total_ms = 0.0;
        let mut world_monthly_process_countries_ms = 0.0;
        let mut world_monthly_tariff_adjust_ms = 0.0;
        let mut world_monthly_rate_update_ms = 0.0;
        let mut world_monthly_capture_snapshot_ms = 0.0;
        let mut world_monthly_reset_trackers_ms = 0.0;

        let mut monthly_traces = Vec::new();
        let is_month_boundary = self.day.is_multiple_of(DAYS_PER_MONTH);
        if is_month_boundary {
            let monthly_started = Instant::now();

            let started = Instant::now();
            monthly_traces = self
                .countries
                .par_iter_mut()
                .map(Country::process_month_end_traced)
                .collect::<Vec<_>>();
            world_monthly_process_countries_ms = crate::sim::trace::duration_ms(started.elapsed());

            let started = Instant::now();
            self.adjust_dynamic_country_tariffs();
            world_monthly_tariff_adjust_ms = crate::sim::trace::duration_ms(started.elapsed());

            let started = Instant::now();
            self.update_policy_rate();
            let rate = self.policy_rate;
            for country in &mut self.countries {
                country.policy_rate = rate;
            }
            world_monthly_rate_update_ms = crate::sim::trace::duration_ms(started.elapsed());

            let started = Instant::now();
            self.capture_snapshot();
            world_monthly_capture_snapshot_ms = crate::sim::trace::duration_ms(started.elapsed());

            if self.export_queue_active {
                self.push_export_snapshot();
            }

            let started = Instant::now();
            self.countries
                .par_iter_mut()
                .for_each(Country::reset_month_trackers);
            world_monthly_reset_trackers_ms = crate::sim::trace::duration_ms(started.elapsed());

            world_monthly_total_ms = crate::sim::trace::duration_ms(monthly_started.elapsed());
        }

        let started = Instant::now();
        self.drain_country_events();
        let world_drain_events_ms = crate::sim::trace::duration_ms(started.elapsed());

        let (
            country_daily_total_ms_sum,
            country_daily_age_ms_sum,
            country_daily_deaths_ms_sum,
            country_daily_births_ms_sum,
            country_daily_job_market_snapshot_ms_sum,
            country_daily_job_market_plan_ms_sum,
            country_daily_job_market_apply_ms_sum,
            country_daily_recompute_headcounts_ms_sum,
            country_daily_recompute_aggregates_ms_sum,
        ) = daily_traces.iter().fold(
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            |acc, trace| {
                (
                    acc.0 + trace.total_ms,
                    acc.1 + trace.age_ms,
                    acc.2 + trace.deaths_ms,
                    acc.3 + trace.births_ms,
                    acc.4 + trace.job_market_snapshot_ms,
                    acc.5 + trace.job_market_plan_ms,
                    acc.6 + trace.job_market_apply_ms,
                    acc.7 + trace.recompute_headcounts_ms,
                    acc.8 + trace.recompute_aggregates_ms,
                )
            },
        );

        let (
            country_monthly_total_ms_sum,
            country_monthly_payroll_liquidity_ms_sum,
            country_monthly_payroll_taxes_ms_sum,
            country_monthly_gov_spending_ms_sum,
            country_monthly_bond_coupon_ms_sum,
            country_monthly_stock_collateral_ms_sum,
            country_monthly_loan_repayments_ms_sum,
            country_monthly_profitability_ms_sum,
            country_monthly_company_distress_ms_sum,
            country_monthly_loan_requests_ms_sum,
            country_monthly_growth_openings_ms_sum,
            country_monthly_startup_formation_ms_sum,
            country_monthly_essential_backfill_ms_sum,
            country_monthly_bond_issuance_ms_sum,
            country_monthly_public_wage_adjust_ms_sum,
            country_monthly_job_switches_ms_sum,
            country_monthly_close_company_months_ms_sum,
            country_monthly_dividends_ms_sum,
            country_monthly_cpi_ms_sum,
            country_monthly_recompute_aggregates_ms_sum,
        ) = monthly_traces.iter().fold(
            (
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0,
            ),
            |acc, trace| {
                (
                    acc.0 + trace.total_ms,
                    acc.1 + trace.payroll_liquidity_ms,
                    acc.2 + trace.payroll_taxes_ms,
                    acc.3 + trace.gov_spending_ms,
                    acc.4 + trace.bond_coupon_ms,
                    acc.5 + trace.stock_collateral_ms,
                    acc.6 + trace.loan_repayments_ms,
                    acc.7 + trace.profitability_ms,
                    acc.8 + trace.company_distress_ms,
                    acc.9 + trace.loan_requests_ms,
                    acc.10 + trace.growth_openings_ms,
                    acc.11 + trace.startup_formation_ms,
                    acc.12 + trace.essential_backfill_ms,
                    acc.13 + trace.bond_issuance_ms,
                    acc.14 + trace.public_wage_adjust_ms,
                    acc.15 + trace.job_switches_ms,
                    acc.16 + trace.close_company_months_ms,
                    acc.17 + trace.dividends_ms,
                    acc.18 + trace.cpi_ms,
                    acc.19 + trace.recompute_aggregates_ms,
                )
            },
        );

        let country_count = self.countries.len();
        let people_count = self.countries.iter().map(|c| c.people.len()).sum();
        let company_count = self.countries.iter().map(|c| c.companies.len()).sum();
        let open_positions_count = self
            .countries
            .iter()
            .flat_map(|c| c.companies.iter())
            .map(|company| company.open_positions.len())
            .sum();
        let listed_equities_count = self
            .countries
            .iter()
            .map(|c| c.stock_market.listed.len())
            .sum();

        crate::sim::trace::DayTraceRow {
            day,
            is_month_boundary,
            tick_total_ms: crate::sim::trace::duration_ms(tick_started.elapsed()),
            world_country_tick_ms,
            world_goods_ms,
            world_stock_ms,
            world_monthly_total_ms,
            world_monthly_process_countries_ms,
            world_monthly_tariff_adjust_ms,
            world_monthly_rate_update_ms,
            world_monthly_capture_snapshot_ms,
            world_monthly_reset_trackers_ms,
            world_drain_events_ms,
            country_daily_total_ms_sum,
            country_daily_age_ms_sum,
            country_daily_deaths_ms_sum,
            country_daily_births_ms_sum,
            country_daily_job_market_snapshot_ms_sum,
            country_daily_job_market_plan_ms_sum,
            country_daily_job_market_apply_ms_sum,
            country_daily_recompute_headcounts_ms_sum,
            country_daily_recompute_aggregates_ms_sum,
            country_monthly_total_ms_sum,
            country_monthly_payroll_liquidity_ms_sum,
            country_monthly_payroll_taxes_ms_sum,
            country_monthly_gov_spending_ms_sum,
            country_monthly_bond_coupon_ms_sum,
            country_monthly_stock_collateral_ms_sum,
            country_monthly_loan_repayments_ms_sum,
            country_monthly_profitability_ms_sum,
            country_monthly_company_distress_ms_sum,
            country_monthly_loan_requests_ms_sum,
            country_monthly_growth_openings_ms_sum,
            country_monthly_startup_formation_ms_sum,
            country_monthly_essential_backfill_ms_sum,
            country_monthly_bond_issuance_ms_sum,
            country_monthly_public_wage_adjust_ms_sum,
            country_monthly_job_switches_ms_sum,
            country_monthly_close_company_months_ms_sum,
            country_monthly_dividends_ms_sum,
            country_monthly_cpi_ms_sum,
            country_monthly_recompute_aggregates_ms_sum,
            country_count,
            people_count,
            company_count,
            open_positions_count,
            listed_equities_count,
        }
    }
}

#[cfg(test)]
mod tests;
