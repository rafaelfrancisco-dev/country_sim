use super::{Country, World};
use crate::sim::country::DAYS_PER_MONTH;
use crate::sim::trace::{CountryDailyTrace, CountryMonthlyTrace, DayTraceRow, duration_ms};
use rayon::prelude::*;
use std::time::Instant;

use super::tick::WorldTickHooks;

#[derive(Default)]
struct TraceWorldTickHooks {
    world_country_tick_ms: f64,
    world_goods_ms: f64,
    world_stock_ms: f64,
    world_monthly_total_ms: f64,
    world_monthly_process_countries_ms: f64,
    world_monthly_tariff_adjust_ms: f64,
    world_monthly_rate_update_ms: f64,
    world_monthly_capture_snapshot_ms: f64,
    world_monthly_reset_trackers_ms: f64,
    world_drain_events_ms: f64,
    daily_traces: Vec<CountryDailyTrace>,
    monthly_traces: Vec<CountryMonthlyTrace>,
    monthly_started: Option<Instant>,
}

impl TraceWorldTickHooks {
    fn timed<F: FnOnce()>(f: F) -> f64 {
        let started = Instant::now();
        f();
        duration_ms(started.elapsed())
    }
}

impl WorldTickHooks for TraceWorldTickHooks {
    fn run_country_daily(&mut self, world: &mut World) {
        let started = Instant::now();
        self.daily_traces = if world.countries.len() >= 2 {
            world.countries.par_iter_mut().map(|c| {
                c.use_inner_parallelism = false;
                Country::tick_day_without_month_traced(c)
            }).collect()
        } else {
            world.countries.iter_mut().map(|c| {
                c.use_inner_parallelism = true;
                Country::tick_day_without_month_traced(c)
            }).collect()
        };
        self.world_country_tick_ms = duration_ms(started.elapsed());
    }

    fn run_daily_goods(&mut self, world: &mut World) {
        self.world_goods_ms = Self::timed(|| world.run_daily_goods_economy());
    }

    fn run_daily_stock(&mut self, world: &mut World) {
        self.world_stock_ms = Self::timed(|| world.run_daily_stock_market());
    }

    fn on_monthly_start(&mut self) {
        self.monthly_started = Some(Instant::now());
    }

    fn run_monthly_country_processing(&mut self, world: &mut World) {
        let started = Instant::now();
        self.monthly_traces = if world.countries.len() >= 2 {
            world.countries.par_iter_mut().map(|c| {
                c.use_inner_parallelism = false;
                Country::process_month_end_traced(c)
            }).collect()
        } else {
            world.countries.iter_mut().map(|c| {
                c.use_inner_parallelism = true;
                Country::process_month_end_traced(c)
            }).collect()
        };
        self.world_monthly_process_countries_ms = duration_ms(started.elapsed());
    }

    fn run_monthly_tariff_adjust(&mut self, world: &mut World) {
        self.world_monthly_tariff_adjust_ms =
            Self::timed(|| world.adjust_dynamic_country_tariffs());
    }

    fn run_monthly_rate_update(&mut self, world: &mut World) {
        self.world_monthly_rate_update_ms =
            Self::timed(|| world.update_policy_rate_and_propagate());
    }

    fn run_monthly_capture_snapshot(&mut self, world: &mut World) {
        self.world_monthly_capture_snapshot_ms = Self::timed(|| world.capture_snapshot());
    }

    fn run_monthly_reset_trackers(&mut self, world: &mut World) {
        self.world_monthly_reset_trackers_ms = Self::timed(|| {
            if world.countries.len() >= 2 {
                world.countries.par_iter_mut().for_each(|c| {
                    c.use_inner_parallelism = false;
                    Country::reset_month_trackers(c);
                });
            } else {
                for c in &mut world.countries {
                    c.use_inner_parallelism = true;
                    Country::reset_month_trackers(c);
                }
            }
        });
    }

    fn on_monthly_end(&mut self) {
        self.world_monthly_total_ms = self
            .monthly_started
            .take()
            .map_or(0.0, |started| duration_ms(started.elapsed()));
    }

    fn run_drain_country_events(&mut self, world: &mut World) {
        self.world_drain_events_ms = Self::timed(|| world.drain_country_events());
    }
}

impl World {
    pub(crate) fn tick_day_with_trace(&mut self) -> DayTraceRow {
        let tick_started = Instant::now();
        let mut hooks = TraceWorldTickHooks::default();
        self.tick_day_with_hooks(&mut hooks);

        let TraceWorldTickHooks {
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
            daily_traces,
            monthly_traces,
            ..
        } = hooks;

        let day = self.day;
        let is_month_boundary = self.day.is_multiple_of(DAYS_PER_MONTH);

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
            country_monthly_savings_interest_ms_sum,
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
                0.0, 0.0, 0.0, 0.0, 0.0,
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
                    acc.7 + trace.savings_interest_ms,
                    acc.8 + trace.profitability_ms,
                    acc.9 + trace.company_distress_ms,
                    acc.10 + trace.loan_requests_ms,
                    acc.11 + trace.growth_openings_ms,
                    acc.12 + trace.startup_formation_ms,
                    acc.13 + trace.essential_backfill_ms,
                    acc.14 + trace.bond_issuance_ms,
                    acc.15 + trace.public_wage_adjust_ms,
                    acc.16 + trace.job_switches_ms,
                    acc.17 + trace.close_company_months_ms,
                    acc.18 + trace.dividends_ms,
                    acc.19 + trace.cpi_ms,
                    acc.20 + trace.recompute_aggregates_ms,
                )
            },
        );

        let country_count = self.countries.len();
        let people_count = self
            .countries
            .iter()
            .map(|country| country.people.len())
            .sum();
        let company_count = self
            .countries
            .iter()
            .map(|country| country.companies.len())
            .sum();
        let open_positions_count = self
            .countries
            .iter()
            .flat_map(|country| country.companies.iter())
            .map(|company| company.open_positions.len())
            .sum();
        let listed_equities_count = self
            .countries
            .iter()
            .map(|country| country.stock_market.listed.len())
            .sum();

        DayTraceRow {
            day,
            is_month_boundary,
            tick_total_ms: duration_ms(tick_started.elapsed()),
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
            country_monthly_savings_interest_ms_sum,
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
