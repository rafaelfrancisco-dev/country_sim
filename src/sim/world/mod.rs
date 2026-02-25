mod goods_market;
mod history;

use rayon::prelude::*;

use super::{country::Country, country::DAYS_PER_MONTH, tickable::Tickable};

pub const HISTORY_LIMIT: usize = 120;
const COUNTRY_CHUNK_SIZE: usize = 8;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MetricPoint {
    pub month: u64,
    pub population: f64,
    pub total_money_supply: f64,
    pub created_money_outstanding: f64,
    pub bank_reserve: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub day: u64,
    pub countries: Vec<Country>,
    pub paused: bool,
    pub world_history: Vec<MetricPoint>,
    pub country_history: Vec<Vec<MetricPoint>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    pub code: &'static str,
    pub message: String,
}

impl World {
    pub fn new(countries: Vec<Country>) -> Self {
        let country_count = countries.len();
        let mut world = Self {
            day: 0,
            countries,
            paused: false,
            world_history: Vec::with_capacity(HISTORY_LIMIT),
            country_history: vec![Vec::with_capacity(HISTORY_LIMIT); country_count],
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

        self.countries
            .par_chunks_mut(COUNTRY_CHUNK_SIZE)
            .for_each(|chunk| {
                for country in chunk {
                    country.tick_day();
                }
            });

        self.run_daily_goods_economy();

        if self.day.is_multiple_of(DAYS_PER_MONTH) {
            self.countries
                .par_chunks_mut(COUNTRY_CHUNK_SIZE)
                .for_each(|chunk| {
                    for country in chunk {
                        country.process_month_end();
                    }
                });
            self.adjust_dynamic_country_tariffs();
            self.capture_snapshot();
            self.countries
                .par_chunks_mut(COUNTRY_CHUNK_SIZE)
                .for_each(|chunk| {
                    for country in chunk {
                        country.reset_month_trackers();
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{
        company::Company,
        country::Country,
        person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
    };
    use rayon::ThreadPoolBuilder;

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
            },
            |acc, point| MetricPoint {
                month: acc.month.max(point.month),
                population: acc.population + point.population,
                total_money_supply: acc.total_money_supply + point.total_money_supply,
                created_money_outstanding: acc.created_money_outstanding
                    + point.created_money_outstanding,
                bank_reserve: acc.bank_reserve + point.bank_reserve,
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
    }

    #[test]
    fn world_tick_parallel_matches_single_thread_reference_small_world() {
        let single_thread = run_world_in_pool(2, 25, 120, 1);
        let multi_thread = run_world_in_pool(2, 25, 120, 4);
        assert_eq!(single_thread, multi_thread);
    }

    #[test]
    fn world_tick_parallel_matches_single_thread_reference_large_world() {
        let single_thread = run_world_in_pool(8, 120, 90, 1);
        let multi_thread = run_world_in_pool(8, 120, 90, 8);
        assert_eq!(single_thread, multi_thread);
    }

    #[test]
    fn history_capture_unchanged_at_month_boundaries_parallel_mode() {
        let world_one_thread = run_world_in_pool(3, 30, DAYS_PER_MONTH * 3, 1);
        let world_many_threads = run_world_in_pool(3, 30, DAYS_PER_MONTH * 3, 8);
        assert_eq!(
            world_one_thread.world_history,
            world_many_threads.world_history
        );
        assert_eq!(
            world_one_thread.country_history,
            world_many_threads.country_history
        );
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
