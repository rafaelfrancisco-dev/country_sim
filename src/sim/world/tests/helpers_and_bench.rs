    use super::*;
    use crate::sim::{
        country::Country,
        generator::{GenerationConfig, generate_countries},
        person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
    };
    use rayon::ThreadPoolBuilder;
    use serde::Serialize;
    use std::{
        env, fs,
        path::Path,
    };

    pub(super) const DEFAULT_BENCH_SEED: u64 = 0x5eed_c0de;
    pub(super) const DEFAULT_BENCH_COUNTRIES: u16 = 5;
    pub(super) const DEFAULT_BENCH_POP_PER_COUNTRY: u32 = 1_000_000;
    pub(super) const DEFAULT_BENCH_DAYS: u64 = 180;
    pub(super) const DEFAULT_BENCH_THREADS: usize = 8;

    pub(super) fn build_people(count: usize) -> Vec<Person> {
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

    pub(super) fn unemployed_person(id: u32, cash_balance: f64) -> Person {
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

    pub(super) fn employed_person(
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

    pub(super) fn run_world_in_pool(
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

    pub(super) fn run_generated_world_in_pool(
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

    pub(super) fn mix_hash(hash: &mut u64, value: u64) {
        *hash ^= value
            .wrapping_add(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(*hash << 6)
            .wrapping_add(*hash >> 2);
    }

    pub(super) fn world_state_hash(world: &World) -> u64 {
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

    pub(super) fn parse_u64_decimal_or_hex(raw: &str) -> Option<u64> {
        let trimmed = raw.trim();
        if let Some(hex) = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
        {
            return u64::from_str_radix(hex, 16).ok();
        }
        trimmed.parse::<u64>().ok()
    }

    pub(super) fn env_u64(name: &str, default: u64) -> u64 {
        env::var(name)
            .ok()
            .and_then(|raw| parse_u64_decimal_or_hex(&raw))
            .unwrap_or(default)
    }

    pub(super) fn env_u16(name: &str, default: u16) -> u16 {
        env::var(name)
            .ok()
            .and_then(|raw| raw.trim().parse::<u16>().ok())
            .unwrap_or(default)
    }

    pub(super) fn env_u32(name: &str, default: u32) -> u32 {
        env::var(name)
            .ok()
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .unwrap_or(default)
    }

    pub(super) fn env_usize(name: &str, default: usize) -> usize {
        env::var(name)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .unwrap_or(default)
    }

    #[cfg(feature = "bench-trace")]
    pub(super) fn env_bool(name: &str, default: bool) -> bool {
        match env::var(name) {
            Ok(raw) => matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => default,
        }
    }

    pub(super) fn percentile_95_ms(samples_ms: &[f64]) -> f64 {
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

    pub(super) fn ensure_parent_dir(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory should be creatable");
        }
    }

    #[derive(Debug, Serialize)]
    pub(super) struct BenchmarkScenario {
        pub(super) seed: String,
        pub(super) countries: u16,
        pub(super) population_per_country: u32,
        pub(super) days: u64,
        pub(super) sim_mode: String,
    }

    #[derive(Debug, Serialize)]
    pub(super) struct BenchmarkMetrics {
        pub(super) ticks_per_sec: f64,
        pub(super) avg_tick_ms: f64,
        pub(super) p95_tick_ms: f64,
        pub(super) total_wall_clock_ms: f64,
        pub(super) scenario: BenchmarkScenario,
        pub(super) thread_count: usize,
        pub(super) world_state_hash: String,
        pub(super) timestamp: u64,
    }


