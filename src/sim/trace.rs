#![cfg(feature = "bench-trace")]

use serde::Serialize;
use std::{fs, io, path::Path, time::Duration};

#[derive(Debug, Clone, Copy, Default)]
pub struct CountryDailyTrace {
    pub total_ms: f64,
    pub age_ms: f64,
    pub deaths_ms: f64,
    pub births_ms: f64,
    pub job_market_snapshot_ms: f64,
    pub job_market_plan_ms: f64,
    pub job_market_apply_ms: f64,
    pub recompute_headcounts_ms: f64,
    pub recompute_aggregates_ms: f64,
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct CountryMonthlyTrace {
    pub total_ms: f64,
    pub recompute_headcounts_before_ms: f64,
    pub payroll_liquidity_ms: f64,
    pub payroll_taxes_ms: f64,
    pub gov_spending_ms: f64,
    pub bond_coupon_ms: f64,
    pub stock_collateral_ms: f64,
    pub loan_repayments_ms: f64,
    pub profitability_ms: f64,
    pub company_distress_ms: f64,
    pub loan_requests_ms: f64,
    pub growth_openings_ms: f64,
    pub startup_formation_ms: f64,
    pub essential_backfill_ms: f64,
    pub bond_issuance_ms: f64,
    pub public_wage_adjust_ms: f64,
    pub job_switches_ms: f64,
    pub recompute_headcounts_after_ms: f64,
    pub close_company_months_ms: f64,
    pub dividends_ms: f64,
    pub cpi_ms: f64,
    pub recompute_aggregates_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayTraceRow {
    pub day: u64,
    pub is_month_boundary: bool,
    pub tick_total_ms: f64,
    pub world_country_tick_ms: f64,
    pub world_goods_ms: f64,
    pub world_stock_ms: f64,
    pub world_monthly_total_ms: f64,
    pub world_monthly_process_countries_ms: f64,
    pub world_monthly_tariff_adjust_ms: f64,
    pub world_monthly_rate_update_ms: f64,
    pub world_monthly_capture_snapshot_ms: f64,
    pub world_monthly_reset_trackers_ms: f64,
    pub world_drain_events_ms: f64,
    pub country_daily_total_ms_sum: f64,
    pub country_daily_age_ms_sum: f64,
    pub country_daily_deaths_ms_sum: f64,
    pub country_daily_births_ms_sum: f64,
    pub country_daily_job_market_snapshot_ms_sum: f64,
    pub country_daily_job_market_plan_ms_sum: f64,
    pub country_daily_job_market_apply_ms_sum: f64,
    pub country_daily_recompute_headcounts_ms_sum: f64,
    pub country_daily_recompute_aggregates_ms_sum: f64,
    pub country_monthly_total_ms_sum: f64,
    pub country_monthly_payroll_liquidity_ms_sum: f64,
    pub country_monthly_payroll_taxes_ms_sum: f64,
    pub country_monthly_gov_spending_ms_sum: f64,
    pub country_monthly_bond_coupon_ms_sum: f64,
    pub country_monthly_stock_collateral_ms_sum: f64,
    pub country_monthly_loan_repayments_ms_sum: f64,
    pub country_monthly_profitability_ms_sum: f64,
    pub country_monthly_company_distress_ms_sum: f64,
    pub country_monthly_loan_requests_ms_sum: f64,
    pub country_monthly_growth_openings_ms_sum: f64,
    pub country_monthly_startup_formation_ms_sum: f64,
    pub country_monthly_essential_backfill_ms_sum: f64,
    pub country_monthly_bond_issuance_ms_sum: f64,
    pub country_monthly_public_wage_adjust_ms_sum: f64,
    pub country_monthly_job_switches_ms_sum: f64,
    pub country_monthly_close_company_months_ms_sum: f64,
    pub country_monthly_dividends_ms_sum: f64,
    pub country_monthly_cpi_ms_sum: f64,
    pub country_monthly_recompute_aggregates_ms_sum: f64,
    pub country_count: usize,
    pub people_count: usize,
    pub company_count: usize,
    pub open_positions_count: usize,
    pub listed_equities_count: usize,
}

impl DayTraceRow {
    fn csv_header() -> &'static str {
        "day,is_month_boundary,tick_total_ms,world_country_tick_ms,world_goods_ms,world_stock_ms,world_monthly_total_ms,world_monthly_process_countries_ms,world_monthly_tariff_adjust_ms,world_monthly_rate_update_ms,world_monthly_capture_snapshot_ms,world_monthly_reset_trackers_ms,world_drain_events_ms,country_daily_total_ms_sum,country_daily_age_ms_sum,country_daily_deaths_ms_sum,country_daily_births_ms_sum,country_daily_job_market_snapshot_ms_sum,country_daily_job_market_plan_ms_sum,country_daily_job_market_apply_ms_sum,country_daily_recompute_headcounts_ms_sum,country_daily_recompute_aggregates_ms_sum,country_monthly_total_ms_sum,country_monthly_payroll_liquidity_ms_sum,country_monthly_payroll_taxes_ms_sum,country_monthly_gov_spending_ms_sum,country_monthly_bond_coupon_ms_sum,country_monthly_stock_collateral_ms_sum,country_monthly_loan_repayments_ms_sum,country_monthly_profitability_ms_sum,country_monthly_company_distress_ms_sum,country_monthly_loan_requests_ms_sum,country_monthly_growth_openings_ms_sum,country_monthly_startup_formation_ms_sum,country_monthly_essential_backfill_ms_sum,country_monthly_bond_issuance_ms_sum,country_monthly_public_wage_adjust_ms_sum,country_monthly_job_switches_ms_sum,country_monthly_close_company_months_ms_sum,country_monthly_dividends_ms_sum,country_monthly_cpi_ms_sum,country_monthly_recompute_aggregates_ms_sum,country_count,people_count,company_count,open_positions_count,listed_equities_count"
    }

    fn to_csv_row(&self) -> String {
        format!(
            "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{}",
            self.day,
            self.is_month_boundary,
            self.tick_total_ms,
            self.world_country_tick_ms,
            self.world_goods_ms,
            self.world_stock_ms,
            self.world_monthly_total_ms,
            self.world_monthly_process_countries_ms,
            self.world_monthly_tariff_adjust_ms,
            self.world_monthly_rate_update_ms,
            self.world_monthly_capture_snapshot_ms,
            self.world_monthly_reset_trackers_ms,
            self.world_drain_events_ms,
            self.country_daily_total_ms_sum,
            self.country_daily_age_ms_sum,
            self.country_daily_deaths_ms_sum,
            self.country_daily_births_ms_sum,
            self.country_daily_job_market_snapshot_ms_sum,
            self.country_daily_job_market_plan_ms_sum,
            self.country_daily_job_market_apply_ms_sum,
            self.country_daily_recompute_headcounts_ms_sum,
            self.country_daily_recompute_aggregates_ms_sum,
            self.country_monthly_total_ms_sum,
            self.country_monthly_payroll_liquidity_ms_sum,
            self.country_monthly_payroll_taxes_ms_sum,
            self.country_monthly_gov_spending_ms_sum,
            self.country_monthly_bond_coupon_ms_sum,
            self.country_monthly_stock_collateral_ms_sum,
            self.country_monthly_loan_repayments_ms_sum,
            self.country_monthly_profitability_ms_sum,
            self.country_monthly_company_distress_ms_sum,
            self.country_monthly_loan_requests_ms_sum,
            self.country_monthly_growth_openings_ms_sum,
            self.country_monthly_startup_formation_ms_sum,
            self.country_monthly_essential_backfill_ms_sum,
            self.country_monthly_bond_issuance_ms_sum,
            self.country_monthly_public_wage_adjust_ms_sum,
            self.country_monthly_job_switches_ms_sum,
            self.country_monthly_close_company_months_ms_sum,
            self.country_monthly_dividends_ms_sum,
            self.country_monthly_cpi_ms_sum,
            self.country_monthly_recompute_aggregates_ms_sum,
            self.country_count,
            self.people_count,
            self.company_count,
            self.open_positions_count,
            self.listed_equities_count
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PhaseRegression {
    pub phase: String,
    pub first_window_avg_ms: f64,
    pub last_window_avg_ms: f64,
    pub delta_ms: f64,
    pub ratio: f64,
    pub slope_ms_per_day: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlowdownSummary {
    pub days: usize,
    pub window_size: usize,
    pub tick_first_window_avg_ms: f64,
    pub tick_last_window_avg_ms: f64,
    pub tick_ratio: f64,
    pub inconclusive: bool,
    pub top_regressions: Vec<PhaseRegression>,
}

/// Time a single expression and return its wall-clock duration in milliseconds.
///
/// Only available with `--features bench-trace`. The macro uses full paths so
/// callers do not need to import `std::time::Instant` themselves.
///
/// ```ignore
/// let elapsed_ms = time_op!(self.some_expensive_fn());
/// ```
#[macro_export]
macro_rules! time_op {
    ($expr:expr) => {{
        let _started = ::std::time::Instant::now();
        $expr;
        $crate::sim::trace::duration_ms(_started.elapsed())
    }};
}

pub fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

pub fn write_day_traces_csv(path: &Path, rows: &[DayTraceRow]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut output = String::new();
    output.push_str(DayTraceRow::csv_header());
    output.push('\n');
    for row in rows {
        output.push_str(&row.to_csv_row());
        output.push('\n');
    }
    fs::write(path, output)
}

pub fn summarize_slowdown(rows: &[DayTraceRow]) -> SlowdownSummary {
    if rows.is_empty() {
        return SlowdownSummary {
            days: 0,
            window_size: 0,
            tick_first_window_avg_ms: 0.0,
            tick_last_window_avg_ms: 0.0,
            tick_ratio: 0.0,
            inconclusive: true,
            top_regressions: Vec::new(),
        };
    }

    let days = rows.len();
    let window_size = (days / 5).max(10).min(days);
    let first = &rows[..window_size];
    let last = &rows[days - window_size..];

    let tick_first = avg(first, |r| r.tick_total_ms);
    let tick_last = avg(last, |r| r.tick_total_ms);
    let tick_ratio = safe_ratio(tick_last, tick_first);

    let phases: [(&str, fn(&DayTraceRow) -> f64); 15] = [
        ("world_country_tick_ms", |r| r.world_country_tick_ms),
        ("world_goods_ms", |r| r.world_goods_ms),
        ("world_stock_ms", |r| r.world_stock_ms),
        ("world_monthly_total_ms", |r| r.world_monthly_total_ms),
        ("world_drain_events_ms", |r| r.world_drain_events_ms),
        ("country_daily_age_ms_sum", |r| r.country_daily_age_ms_sum),
        ("country_daily_deaths_ms_sum", |r| {
            r.country_daily_deaths_ms_sum
        }),
        ("country_daily_births_ms_sum", |r| {
            r.country_daily_births_ms_sum
        }),
        ("country_daily_job_market_snapshot_ms_sum", |r| {
            r.country_daily_job_market_snapshot_ms_sum
        }),
        ("country_daily_job_market_plan_ms_sum", |r| {
            r.country_daily_job_market_plan_ms_sum
        }),
        ("country_daily_job_market_apply_ms_sum", |r| {
            r.country_daily_job_market_apply_ms_sum
        }),
        ("country_daily_recompute_headcounts_ms_sum", |r| {
            r.country_daily_recompute_headcounts_ms_sum
        }),
        ("country_daily_recompute_aggregates_ms_sum", |r| {
            r.country_daily_recompute_aggregates_ms_sum
        }),
        ("country_monthly_total_ms_sum", |r| {
            r.country_monthly_total_ms_sum
        }),
        ("country_monthly_startup_formation_ms_sum", |r| {
            r.country_monthly_startup_formation_ms_sum
        }),
    ];

    let first_mid_day = midpoint_day(first);
    let last_mid_day = midpoint_day(last);
    let day_span = (last_mid_day - first_mid_day).abs().max(1.0);

    let mut regressions = phases
        .iter()
        .map(|(phase, accessor)| {
            let first_avg = avg(first, *accessor);
            let last_avg = avg(last, *accessor);
            let delta = last_avg - first_avg;
            let ratio = safe_ratio(last_avg, first_avg);
            PhaseRegression {
                phase: (*phase).to_string(),
                first_window_avg_ms: first_avg,
                last_window_avg_ms: last_avg,
                delta_ms: delta,
                ratio,
                slope_ms_per_day: delta / day_span,
            }
        })
        .filter(|r| r.delta_ms > 0.0)
        .collect::<Vec<_>>();
    regressions.sort_by(|a, b| b.delta_ms.total_cmp(&a.delta_ms));
    regressions.truncate(5);

    let has_clear_phase = regressions
        .iter()
        .any(|r| r.ratio >= 1.15 && r.delta_ms >= 15.0);
    let inconclusive = tick_ratio < 1.10 && !has_clear_phase;

    SlowdownSummary {
        days,
        window_size,
        tick_first_window_avg_ms: tick_first,
        tick_last_window_avg_ms: tick_last,
        tick_ratio,
        inconclusive,
        top_regressions: regressions,
    }
}

pub fn write_summary_json(path: &Path, summary: &SlowdownSummary) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(summary)
        .map_err(|err| io::Error::other(format!("summary json serialization failed: {err}")))?;
    fs::write(path, body)
}

fn avg(rows: &[DayTraceRow], accessor: fn(&DayTraceRow) -> f64) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(accessor).sum::<f64>() / rows.len() as f64
}

fn safe_ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator <= f64::EPSILON {
        if numerator <= f64::EPSILON {
            1.0
        } else {
            f64::INFINITY
        }
    } else {
        numerator / denominator
    }
}

fn midpoint_day(rows: &[DayTraceRow]) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    let first = rows.first().map_or(0_u64, |r| r.day);
    let last = rows.last().map_or(first, |r| r.day);
    (first as f64 + last as f64) / 2.0
}
