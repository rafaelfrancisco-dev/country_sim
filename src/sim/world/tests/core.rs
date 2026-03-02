use super::*;

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


