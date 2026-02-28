mod export;
mod goods_market;
mod history;
mod stock_market;

pub use export::{
    CountryExportData, MonthlyExportData, StockDailyCountryExportData,
    StockDailyEquityExportData,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub day: u64,
    pub generation_seed: u64,
    pub countries: Vec<Country>,
    pub paused: bool,
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
        let mut world = Self {
            day: 0,
            generation_seed,
            countries,
            paused: false,
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
            country_monthly_stock_collateral_ms_sum,
            country_monthly_loan_repayments_ms_sum,
            country_monthly_profitability_ms_sum,
            country_monthly_company_distress_ms_sum,
            country_monthly_loan_requests_ms_sum,
            country_monthly_growth_openings_ms_sum,
            country_monthly_startup_formation_ms_sum,
            country_monthly_essential_backfill_ms_sum,
            country_monthly_public_wage_adjust_ms_sum,
            country_monthly_job_switches_ms_sum,
            country_monthly_close_company_months_ms_sum,
            country_monthly_dividends_ms_sum,
            country_monthly_recompute_aggregates_ms_sum,
        ) = monthly_traces.iter().fold(
            (
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ),
            |acc, trace| {
                (
                    acc.0 + trace.total_ms,
                    acc.1 + trace.payroll_liquidity_ms,
                    acc.2 + trace.payroll_taxes_ms,
                    acc.3 + trace.gov_spending_ms,
                    acc.4 + trace.stock_collateral_ms,
                    acc.5 + trace.loan_repayments_ms,
                    acc.6 + trace.profitability_ms,
                    acc.7 + trace.company_distress_ms,
                    acc.8 + trace.loan_requests_ms,
                    acc.9 + trace.growth_openings_ms,
                    acc.10 + trace.startup_formation_ms,
                    acc.11 + trace.essential_backfill_ms,
                    acc.12 + trace.public_wage_adjust_ms,
                    acc.13 + trace.job_switches_ms,
                    acc.14 + trace.close_company_months_ms,
                    acc.15 + trace.dividends_ms,
                    acc.16 + trace.recompute_aggregates_ms,
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
            country_monthly_stock_collateral_ms_sum,
            country_monthly_loan_repayments_ms_sum,
            country_monthly_profitability_ms_sum,
            country_monthly_company_distress_ms_sum,
            country_monthly_loan_requests_ms_sum,
            country_monthly_growth_openings_ms_sum,
            country_monthly_startup_formation_ms_sum,
            country_monthly_essential_backfill_ms_sum,
            country_monthly_public_wage_adjust_ms_sum,
            country_monthly_job_switches_ms_sum,
            country_monthly_close_company_months_ms_sum,
            country_monthly_dividends_ms_sum,
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
mod tests {
    use super::*;
    use crate::sim::{
        company::Company,
        country::Country,
        generator::{GenerationConfig, generate_countries},
        person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
    };
    use rayon::ThreadPoolBuilder;
    use serde::Serialize;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    const DEFAULT_BENCH_SEED: u64 = 0x5eed_c0de;
    const DEFAULT_BENCH_COUNTRIES: u16 = 5;
    const DEFAULT_BENCH_POP_PER_COUNTRY: u32 = 1_000_000;
    const DEFAULT_BENCH_DAYS: u64 = 180;
    const DEFAULT_BENCH_THREADS: usize = 8;

    fn build_people(count: usize) -> Vec<Person> {
        (0..count)
            .map(|idx| {
                Person::new(
                    idx as u32,
                    (idx % 10) as u8,
                    (idx % 4) as u16,
                    idx as u32 + 10,
                )
            })
            .collect()
    }

    fn unemployed_person(id: u32, cash_balance: f64) -> Person {
        let mut person = Person::new_with_profile(
            id,
            EmploymentStatus::Unemployed,
            id + 100,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = cash_balance;
        person
    }

    fn employed_person(
        id: u32,
        job_type_id: u8,
        employer_company_id: u16,
        cash_balance: f64,
    ) -> Person {
        let mut person = Person::new_with_profile(
            id,
            EmploymentStatus::Employed {
                job_type_id,
                employer_company_id,
            },
            id + 100,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = cash_balance;
        person
    }

    fn run_world_in_pool(
        country_count: usize,
        people_per_country: usize,
        days: u64,
        threads: usize,
    ) -> World {
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("thread pool should build");

        pool.install(|| {
            let countries = (0..country_count)
                .map(|idx| {
                    Country::new(
                        (idx + 1) as u16,
                        format!("C{idx}"),
                        build_people(people_per_country),
                    )
                })
                .collect::<Vec<_>>();
            let mut world = World::new(countries);
            for _ in 0..days {
                world.tick_day();
            }
            world
        })
    }

    fn run_generated_world_in_pool(
        seed: u64,
        country_count: u16,
        population_per_country: u32,
        days: u64,
        threads: usize,
    ) -> World {
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("thread pool should build");

        pool.install(|| {
            let countries = generate_countries(&GenerationConfig::with_defaults(
                country_count,
                population_per_country,
                seed,
            ));
            let mut world = World::new_with_generation_seed(countries, seed);
            for _ in 0..days {
                world.tick_day();
            }
            world
        })
    }

    fn mix_hash(hash: &mut u64, value: u64) {
        *hash ^= value
            .wrapping_add(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(*hash << 6)
            .wrapping_add(*hash >> 2);
    }

    fn world_state_hash(world: &World) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;

        mix_hash(&mut hash, world.day);
        mix_hash(&mut hash, world.generation_seed);
        mix_hash(&mut hash, world.countries.len() as u64);

        for country in &world.countries {
            mix_hash(&mut hash, u64::from(country.id));
            mix_hash(&mut hash, country.day);
            mix_hash(&mut hash, country.month);
            mix_hash(&mut hash, country.people.len() as u64);
            mix_hash(&mut hash, country.companies.len() as u64);
            mix_hash(&mut hash, country.monthly_unmet_need_events);
            mix_hash(&mut hash, country.monthly_household_spending.to_bits());
            mix_hash(&mut hash, country.monthly_tariff_collected.to_bits());
            mix_hash(&mut hash, country.monthly_goods_import_spending.to_bits());
            mix_hash(&mut hash, country.monthly_goods_export_revenue.to_bits());
            mix_hash(&mut hash, country.monthly_essential_debt_spending.to_bits());
            mix_hash(&mut hash, country.total_money_supply.to_bits());
            mix_hash(&mut hash, country.bank.created_money_outstanding.to_bits());

            for tariff in country.export_tariffs_by_good {
                mix_hash(&mut hash, tariff.to_bits());
            }

            for company in &country.companies {
                mix_hash(&mut hash, u64::from(company.id));
                mix_hash(&mut hash, company.produced_good_id as u64);
                mix_hash(&mut hash, company.inventory_units as u64);
                mix_hash(&mut hash, company.unit_price.to_bits());
                mix_hash(&mut hash, company.cash_balance.to_bits());
                mix_hash(&mut hash, company.last_month_sales.to_bits());
                mix_hash(&mut hash, company.last_month_units_sold as u64);
                mix_hash(&mut hash, company.last_month_export_units_sold as u64);
                mix_hash(&mut hash, company.last_month_domestic_units_sold as u64);
            }

            for person in &country.people {
                mix_hash(&mut hash, u64::from(person.id));
                mix_hash(&mut hash, person.cash_balance.to_bits());
                mix_hash(&mut hash, u64::from(person.name_seed));
                mix_hash(&mut hash, u64::from(person.age_days));
                mix_hash(&mut hash, u64::from(person.last_month_goods_units_bought));
                mix_hash(&mut hash, u64::from(person.last_month_needs_met_mask));
                mix_hash(&mut hash, u64::from(person.last_month_unmet_needs_mask));
                for stock_units in person.superficial_stock_units {
                    mix_hash(&mut hash, u64::from(stock_units));
                }
                for carry in person.superficial_stock_consumption_carry {
                    mix_hash(&mut hash, u64::from(carry.to_bits()));
                }
            }
        }

        hash
    }

    fn parse_u64_decimal_or_hex(raw: &str) -> Option<u64> {
        let trimmed = raw.trim();
        if let Some(hex) = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
        {
            return u64::from_str_radix(hex, 16).ok();
        }
        trimmed.parse::<u64>().ok()
    }

    fn env_u64(name: &str, default: u64) -> u64 {
        env::var(name)
            .ok()
            .and_then(|raw| parse_u64_decimal_or_hex(&raw))
            .unwrap_or(default)
    }

    fn env_u16(name: &str, default: u16) -> u16 {
        env::var(name)
            .ok()
            .and_then(|raw| raw.trim().parse::<u16>().ok())
            .unwrap_or(default)
    }

    fn env_u32(name: &str, default: u32) -> u32 {
        env::var(name)
            .ok()
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .unwrap_or(default)
    }

    fn env_usize(name: &str, default: usize) -> usize {
        env::var(name)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .unwrap_or(default)
    }

    #[cfg(feature = "bench-trace")]
    fn env_bool(name: &str, default: bool) -> bool {
        match env::var(name) {
            Ok(raw) => matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => default,
        }
    }

    fn percentile_95_ms(samples_ms: &[f64]) -> f64 {
        if samples_ms.is_empty() {
            return 0.0;
        }

        let mut sorted = samples_ms.to_vec();
        sorted.sort_by(|left, right| left.total_cmp(right));
        let rank = ((sorted.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        sorted[rank]
    }

    fn ensure_parent_dir(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory should be creatable");
        }
    }

    #[derive(Debug, Serialize)]
    struct BenchmarkScenario {
        seed: String,
        countries: u16,
        population_per_country: u32,
        days: u64,
        sim_mode: String,
    }

    #[derive(Debug, Serialize)]
    struct BenchmarkMetrics {
        ticks_per_sec: f64,
        avg_tick_ms: f64,
        p95_tick_ms: f64,
        total_wall_clock_ms: f64,
        scenario: BenchmarkScenario,
        thread_count: usize,
        world_state_hash: String,
        timestamp: u64,
    }

    #[test]
    fn world_tick_increments_day_and_ticks_countries() {
        let country = Country::new(1, "Axion".to_string(), build_people(2));
        let mut world = World::new(vec![country]);

        world.tick_day();

        assert_eq!(world.day, 1);
        assert_eq!(world.countries[0].day, 1);
    }

    #[test]
    fn world_starts_with_initial_history() {
        let world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

        assert_eq!(world.world_history.len(), 1);
        assert_eq!(world.country_history.len(), 1);
        assert_eq!(world.country_history[0].len(), 1);
        assert_eq!(world.world_history[0].month, 0);
    }

    #[test]
    fn world_tick_advances_days_and_processes_monthly_boundaries() {
        let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

        for _ in 0..DAYS_PER_MONTH {
            world.tick_day();
        }
        assert_eq!(world.world_history.len(), 2);
        assert_eq!(world.countries[0].month, 1);
        assert_eq!(world.completed_month_boundaries(), 1);
    }

    #[test]
    fn world_history_retains_only_latest_limit() {
        let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(1))]);

        for _ in 0..(HISTORY_LIMIT as u64 + 10) {
            for _ in 0..DAYS_PER_MONTH {
                world.tick_day();
            }
        }

        assert_eq!(world.world_history.len(), HISTORY_LIMIT);
        assert_eq!(world.country_history[0].len(), HISTORY_LIMIT);
    }

    #[test]
    fn aggregate_history_matches_country_sums() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(2)),
            Country::new(2, "B".to_string(), build_people(3)),
        ];
        let world = World::new(countries);
        let aggregate = world.world_history[0];
        let country_sum = world.country_history.iter().map(|series| series[0]).fold(
            MetricPoint {
                month: 0,
                population: 0.0,
                total_money_supply: 0.0,
                created_money_outstanding: 0.0,
                bank_reserve: 0.0,
                treasury_balance: 0.0,
                budget_balance: 0.0,
            },
            |acc, point| MetricPoint {
                month: acc.month.max(point.month),
                population: acc.population + point.population,
                total_money_supply: acc.total_money_supply + point.total_money_supply,
                created_money_outstanding: acc.created_money_outstanding
                    + point.created_money_outstanding,
                bank_reserve: acc.bank_reserve + point.bank_reserve,
                treasury_balance: acc.treasury_balance + point.treasury_balance,
                budget_balance: acc.budget_balance + point.budget_balance,
            },
        );

        assert_eq!(aggregate.month, country_sum.month);
        assert!((aggregate.population - country_sum.population).abs() < f64::EPSILON);
        assert!(
            (aggregate.total_money_supply - country_sum.total_money_supply).abs() < f64::EPSILON
        );
        assert!(
            (aggregate.created_money_outstanding - country_sum.created_money_outstanding).abs()
                < f64::EPSILON
        );
        assert!((aggregate.bank_reserve - country_sum.bank_reserve).abs() < f64::EPSILON);
        assert!((aggregate.treasury_balance - country_sum.treasury_balance).abs() < f64::EPSILON);
        assert!((aggregate.budget_balance - country_sum.budget_balance).abs() < f64::EPSILON);
    }

    #[test]
    fn world_tick_parallel_paths_keep_invariants_small_world() {
        let single_thread = run_world_in_pool(2, 25, 120, 1);
        let multi_thread = run_world_in_pool(2, 25, 120, 4);

        assert_eq!(single_thread.day, multi_thread.day);
        assert_eq!(single_thread.countries.len(), multi_thread.countries.len());
        assert!(single_thread.validate_invariants().is_empty());
        assert!(multi_thread.validate_invariants().is_empty());
    }

    #[test]
    fn world_tick_parallel_paths_keep_invariants_large_world() {
        let single_thread = run_world_in_pool(8, 120, 90, 1);
        let multi_thread = run_world_in_pool(8, 120, 90, 8);

        assert_eq!(single_thread.day, multi_thread.day);
        assert_eq!(single_thread.countries.len(), multi_thread.countries.len());
        assert!(single_thread.validate_invariants().is_empty());
        assert!(multi_thread.validate_invariants().is_empty());
    }

    #[test]
    fn history_capture_counts_remain_consistent_at_month_boundaries_parallel_mode() {
        let world_one_thread = run_world_in_pool(3, 30, DAYS_PER_MONTH * 3, 1);
        let world_many_threads = run_world_in_pool(3, 30, DAYS_PER_MONTH * 3, 8);

        assert_eq!(
            world_one_thread.world_history.len(),
            world_many_threads.world_history.len()
        );
        assert_eq!(
            world_one_thread.country_history.len(),
            world_many_threads.country_history.len()
        );
    }

    #[test]
    fn generated_world_hash_is_deterministic_for_same_seed_at_day_zero() {
        let first = run_generated_world_in_pool(41, 4, 2_500, 0, 4);
        let second = run_generated_world_in_pool(41, 4, 2_500, 0, 4);

        assert_eq!(world_state_hash(&first), world_state_hash(&second));
    }

    #[test]
    fn generated_world_hash_differs_for_different_seeds_at_day_zero() {
        let first = run_generated_world_in_pool(77, 5, 2_000, 0, 4);
        let second = run_generated_world_in_pool(78, 5, 2_000, 0, 4);

        assert_ne!(world_state_hash(&first), world_state_hash(&second));
    }

    #[test]
    #[ignore = "release benchmark harness"]
    fn benchmark_release_protocol_emits_metrics() {
        let sim_mode = env::var("BENCH_SIM_MODE").unwrap_or_else(|_| "max-speed".to_string());
        let normalized_mode = sim_mode.trim().to_ascii_lowercase().replace('_', "-");
        assert_eq!(
            normalized_mode, "max-speed",
            "BENCH_SIM_MODE must be max-speed for protocol parity"
        );

        let seed = env_u64("BENCH_SEED", DEFAULT_BENCH_SEED);
        let country_count = env_u16("BENCH_COUNTRIES", DEFAULT_BENCH_COUNTRIES);
        let population_per_country =
            env_u32("BENCH_POP_PER_COUNTRY", DEFAULT_BENCH_POP_PER_COUNTRY);
        let days = env_u64("BENCH_DAYS", DEFAULT_BENCH_DAYS);
        let thread_count = env_usize("BENCH_THREADS", DEFAULT_BENCH_THREADS).max(1);

        let json_output = env::var("BENCH_OUTPUT_JSON")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target/bench/latest.json"));
        let csv_output = env::var("BENCH_OUTPUT_CSV")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target/bench/latest.csv"));

        #[cfg(feature = "bench-trace")]
        let trace_enabled = env_bool("BENCH_TRACE_ENABLED", false);
        #[cfg(feature = "bench-trace")]
        let trace_csv_output = env::var("BENCH_TRACE_OUTPUT_CSV")
            .ok()
            .map(PathBuf::from)
            .filter(|_| trace_enabled);
        #[cfg(feature = "bench-trace")]
        let trace_summary_output = env::var("BENCH_TRACE_SUMMARY_JSON")
            .ok()
            .map(PathBuf::from)
            .filter(|_| trace_enabled);

        let pool = ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .expect("thread pool should build");

        let mut tick_durations_ms = Vec::with_capacity(days as usize);
        #[cfg(feature = "bench-trace")]
        let mut day_traces = Vec::with_capacity(days as usize);
        let wall_started = Instant::now();
        let world = pool.install(|| {
            let countries = generate_countries(&GenerationConfig::with_defaults(
                country_count,
                population_per_country,
                seed,
            ));
            let mut world = World::new_with_generation_seed(countries, seed);
            for _ in 0..days {
                #[cfg(feature = "bench-trace")]
                if trace_enabled {
                    let trace = world.tick_day_with_trace();
                    tick_durations_ms.push(trace.tick_total_ms);
                    day_traces.push(trace);
                    continue;
                }

                let tick_started = Instant::now();
                world.tick_day();
                tick_durations_ms.push(tick_started.elapsed().as_secs_f64() * 1_000.0);
            }
            world
        });
        let total_wall_clock_ms = wall_started.elapsed().as_secs_f64() * 1_000.0;

        let avg_tick_ms = if tick_durations_ms.is_empty() {
            0.0
        } else {
            tick_durations_ms.iter().sum::<f64>() / tick_durations_ms.len() as f64
        };
        let ticks_per_sec = if total_wall_clock_ms <= 0.0 {
            0.0
        } else {
            days as f64 / (total_wall_clock_ms / 1_000.0)
        };
        let p95_tick_ms = percentile_95_ms(&tick_durations_ms);
        let world_hash = world_state_hash(&world);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_secs();

        let metrics = BenchmarkMetrics {
            ticks_per_sec,
            avg_tick_ms,
            p95_tick_ms,
            total_wall_clock_ms,
            scenario: BenchmarkScenario {
                seed: format!("0x{seed:016x}"),
                countries: country_count,
                population_per_country,
                days,
                sim_mode: "max-speed".to_string(),
            },
            thread_count,
            world_state_hash: format!("0x{world_hash:016x}"),
            timestamp,
        };

        ensure_parent_dir(&json_output);
        fs::write(
            &json_output,
            serde_json::to_string_pretty(&metrics).expect("benchmark json should serialize"),
        )
        .expect("benchmark json should be written");

        ensure_parent_dir(&csv_output);
        let csv = format!(
            "ticks_per_sec,avg_tick_ms,p95_tick_ms,total_wall_clock_ms,seed,countries,population_per_country,days,sim_mode,thread_count,world_state_hash,timestamp\n{:.6},{:.6},{:.6},{:.6},0x{:016x},{},{},{},max-speed,{},0x{:016x},{}\n",
            metrics.ticks_per_sec,
            metrics.avg_tick_ms,
            metrics.p95_tick_ms,
            metrics.total_wall_clock_ms,
            seed,
            country_count,
            population_per_country,
            days,
            thread_count,
            world_hash,
            timestamp
        );
        fs::write(&csv_output, csv).expect("benchmark csv should be written");

        #[cfg(feature = "bench-trace")]
        if trace_enabled {
            if let Some(path) = trace_csv_output.as_ref() {
                crate::sim::trace::write_day_traces_csv(path, &day_traces)
                    .expect("trace csv should be written");
            }
            if let Some(path) = trace_summary_output.as_ref() {
                let summary = crate::sim::trace::summarize_slowdown(&day_traces);
                crate::sim::trace::write_summary_json(path, &summary)
                    .expect("trace summary json should be written");
            }
        }

        assert!(metrics.total_wall_clock_ms > 0.0);
        assert!(metrics.ticks_per_sec > 0.0);
    }

    #[test]
    fn global_goods_market_chooses_cheapest_landed_supplier_and_collects_tariff() {
        let mut country_a_companies = vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)];
        country_a_companies[0].inventory_units = 20;
        country_a_companies[0].unit_price = 20.0;
        country_a_companies[0].cash_balance = 1_000.0;

        let mut country_b_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
        country_b_companies[0].inventory_units = 20;
        country_b_companies[0].unit_price = 10.0;
        country_b_companies[0].cash_balance = 1_000.0;

        let country_a = Country::with_economy(
            0,
            "A".to_string(),
            vec![unemployed_person(1, 200.0)],
            country_a_companies,
            1_200.0,
        );
        let country_b = Country::with_economy(
            1,
            "B".to_string(),
            vec![unemployed_person(2, 200.0)],
            country_b_companies,
            1_200.0,
        );

        let mut world = World::new(vec![country_a, country_b]);
        for _ in 0..20 {
            world.tick_day();
        }

        assert_eq!(world.countries[0].companies[0].last_month_units_sold, 0);
        assert!(world.countries[1].companies[0].last_month_export_units_sold > 0);
        assert!((world.countries[0].monthly_tariff_collected - 0.0).abs() < 0.01);
        assert!(world.countries[1].monthly_tariff_collected > 0.0);
        assert!(world.countries[0].monthly_goods_import_spending > 0.0);
        assert!(world.countries[1].monthly_goods_export_revenue > 0.0);
    }

    #[test]
    fn essentials_can_push_cash_negative_and_are_tracked_as_essential_debt_spending() {
        let mut companies = vec![Company::new(0, "Water".to_string(), vec![0, 5], 0)];
        companies[0].inventory_units = 20;
        companies[0].unit_price = 10.0;
        companies[0].cash_balance = 1_000.0;

        let country = Country::with_economy(
            0,
            "A".to_string(),
            vec![unemployed_person(1, 5.0)],
            companies,
            1_005.0,
        );

        let mut world = World::new(vec![country]);
        for _ in 0..15 {
            world.tick_day();
        }

        let person = &world.countries[0].people[0];
        assert!(world.countries[0].monthly_essential_debt_spending > 0.0);
        assert!(person.last_month_goods_spending > 0.0);
        assert!(person.cash_balance < 0.0);
        assert!((person.last_month_needs_met_mask & 1_u16) != 0);
    }

    #[test]
    fn non_essentials_respect_discretionary_budget_cap() {
        let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
        companies[0].inventory_units = 20;
        companies[0].unit_price = 50.0;
        companies[0].cash_balance = 1_000.0;

        let country = Country::with_economy(
            0,
            "A".to_string(),
            vec![unemployed_person(1, 100.0)],
            companies,
            1_100.0,
        );

        let mut world = World::new(vec![country]);
        for _ in 0..20 {
            world.tick_day();
        }

        assert_eq!(world.countries[0].companies[0].last_month_units_sold, 0);
        assert!((world.countries[0].people[0].cash_balance - 100.0).abs() < 0.01);
    }

    #[test]
    fn discretionary_superficial_purchase_requires_empty_stock() {
        let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
        companies[0].inventory_units = 500;
        companies[0].unit_price = 5.0;
        companies[0].cash_balance = 1_000.0;

        let mut person = Person::new_with_profile(
            1,
            EmploymentStatus::Unemployed,
            111,
            5 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        person.cash_balance = 10_000.0;
        person.superficial_stock_units[4] = 2;

        let country = Country::with_economy(0, "A".to_string(), vec![person], companies, 11_000.0);
        let mut world = World::new(vec![country]);
        for _ in 0..20 {
            world.tick_day();
        }

        assert_eq!(world.countries[0].people[0].superficial_stock_units[4], 2);
    }

    #[test]
    fn discretionary_superficial_purchase_obeys_affordability_gate_and_depletes_stock() {
        let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
        companies[0].inventory_units = 2_000;
        companies[0].unit_price = 5.0;
        companies[0].cash_balance = 1_000.0;

        let mut person = Person::new_with_profile(
            1,
            EmploymentStatus::Unemployed,
            222,
            5 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        person.cash_balance = 0.0;

        let country = Country::with_economy(0, "A".to_string(), vec![person], companies, 1_000.0);
        let mut world = World::new(vec![country]);
        for _ in 0..180 {
            world.tick_day();
        }
        assert_eq!(world.countries[0].people[0].superficial_stock_units[4], 0);

        world.countries[0].people[0].cash_balance = 100_000.0;
        let mut purchased_stock = 0_u16;
        for _ in 0..500 {
            world.tick_day();
            let stock = world.countries[0].people[0].superficial_stock_units[4];
            if stock > 0 {
                purchased_stock = stock;
                break;
            }
        }
        assert!(purchased_stock > 0);

        world.countries[0].people[0].cash_balance = 0.0;
        for _ in 0..40 {
            world.tick_day();
        }
        assert!(world.countries[0].people[0].superficial_stock_units[4] < purchased_stock);
    }

    #[test]
    fn production_uses_only_manufacturing_workers() {
        let mut companies = vec![Company::new(0, "Water".to_string(), vec![1, 5], 0)];
        companies[0].inventory_units = 0;
        companies[0].cash_balance = 1_000.0;

        let country = Country::with_economy(
            0,
            "A".to_string(),
            vec![
                employed_person(1, 5, 0, 100.0),
                employed_person(2, 1, 0, 100.0),
            ],
            companies,
            1_200.0,
        );

        let mut world = World::new(vec![country]);
        for _ in 0..29 {
            world.tick_day();
        }

        assert_eq!(
            world.countries[0].companies[0].manufacturing_employee_count,
            1
        );
        assert_eq!(world.countries[0].companies[0].last_month_units_produced, 9);
    }

    #[test]
    fn tariff_increases_for_exported_good_when_domestic_stock_is_depleted_and_need_unmet() {
        let mut country_a_companies = vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)];
        country_a_companies[0].inventory_units = 0;
        country_a_companies[0].unit_price = 50.0;
        country_a_companies[0].cash_balance = 1_000.0;

        let mut country_b_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
        country_b_companies[0].inventory_units = 4;
        country_b_companies[0].unit_price = 10.0;
        country_b_companies[0].cash_balance = 1_000.0;

        let country_a = Country::with_economy(
            0,
            "A".to_string(),
            vec![unemployed_person(1, 300.0)],
            country_a_companies,
            1_300.0,
        );
        let country_b = Country::with_economy(
            1,
            "B".to_string(),
            vec![unemployed_person(2, 300.0)],
            country_b_companies,
            1_300.0,
        );

        let mut world = World::new(vec![country_a, country_b]);
        let before = world.countries[1].export_tariffs_by_good[0];

        for _ in 0..20 {
            world.tick_day();
        }
        world.adjust_dynamic_country_tariffs();

        let after = world.countries[1].export_tariffs_by_good[0];

        assert!(after > before);
    }

    #[test]
    fn scarce_supply_is_shared_across_countries_with_round_robin_allocation() {
        let mut exporter_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
        exporter_companies[0].inventory_units = 2;
        exporter_companies[0].unit_price = 10.0;
        exporter_companies[0].cash_balance = 1_000.0;

        let country_a = Country::with_economy(
            0,
            "A".to_string(),
            vec![unemployed_person(1, 300.0)],
            vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)],
            1_300.0,
        );
        let country_b = Country::with_economy(
            1,
            "B".to_string(),
            vec![unemployed_person(2, 300.0)],
            exporter_companies,
            1_300.0,
        );

        let mut world = World::new(vec![country_a, country_b]);
        for _ in 0..20 {
            world.tick_day();
        }

        assert!(world.countries[0].monthly_household_spending > 0.0);
        assert!(world.countries[1].monthly_household_spending > 0.0);
    }
}
