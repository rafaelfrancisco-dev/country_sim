use super::*;
use rayon::ThreadPoolBuilder;
use serde::Serialize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::sim::tickable::Tickable;
use crate::sim::{generator::GenerationConfig, generator::generate_countries, world::World};

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
    }
    hash
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
    phase_report: Option<BenchmarkPhaseReport>,
    thread_sweep: Vec<ThreadSweepEntry>,
}

#[derive(Debug, Serialize)]
struct BenchmarkPhaseReport {
    country_daily_ms_avg: f64,
    goods_market_ms_avg: f64,
    stock_market_ms_avg: f64,
    month_boundary_ms_avg: f64,
    event_draining_ms_avg: f64,
}

#[derive(Debug, Serialize)]
struct ThreadSweepEntry {
    thread_count: usize,
    ticks_per_sec: f64,
    avg_tick_ms: f64,
    p95_tick_ms: f64,
    world_state_hash: String,
    matches_requested_hash: bool,
}

struct BenchmarkRunOutput {
    metrics: BenchmarkMetrics,
    #[cfg(feature = "bench-trace")]
    day_traces: Vec<crate::sim::trace::DayTraceRow>,
}

#[cfg(feature = "bench-trace")]
fn build_phase_report(
    day_traces: &[crate::sim::trace::DayTraceRow],
) -> Option<BenchmarkPhaseReport> {
    if day_traces.is_empty() {
        return None;
    }

    let avg = |f: fn(&crate::sim::trace::DayTraceRow) -> f64| {
        day_traces.iter().map(f).sum::<f64>() / day_traces.len() as f64
    };

    Some(BenchmarkPhaseReport {
        country_daily_ms_avg: avg(|row| row.world_country_tick_ms),
        goods_market_ms_avg: avg(|row| row.world_goods_ms),
        stock_market_ms_avg: avg(|row| row.world_stock_ms),
        month_boundary_ms_avg: avg(|row| row.world_monthly_total_ms),
        event_draining_ms_avg: avg(|row| row.world_drain_events_ms),
    })
}

fn thread_sweep_counts(requested_threads: usize) -> Vec<usize> {
    let max_threads = requested_threads.max(
        std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(requested_threads.max(1)),
    );
    let mut counts = vec![1, 2, 4, 8, max_threads];
    counts.sort_unstable();
    counts.dedup();
    counts
}

fn run_benchmark_scenario(
    seed: u64,
    country_count: u16,
    population_per_country: u32,
    days: u64,
    thread_count: usize,
    trace_enabled: bool,
) -> BenchmarkRunOutput {
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

    BenchmarkRunOutput {
        metrics: BenchmarkMetrics {
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
            phase_report: {
                #[cfg(feature = "bench-trace")]
                {
                    if trace_enabled {
                        build_phase_report(&day_traces)
                    } else {
                        None
                    }
                }
                #[cfg(not(feature = "bench-trace"))]
                {
                    let _ = trace_enabled;
                    None
                }
            },
            thread_sweep: Vec::new(),
        },
        #[cfg(feature = "bench-trace")]
        day_traces,
    }
}

pub fn run_benchmark_release_protocol_emits_metrics() {
    let sim_mode = env::var("BENCH_SIM_MODE").unwrap_or_else(|_| "max-speed".to_string());
    let normalized_mode = sim_mode.trim().to_ascii_lowercase().replace('_', "-");
    assert_eq!(
        normalized_mode, "max-speed",
        "BENCH_SIM_MODE must be max-speed for protocol parity"
    );

    let seed = env_u64("BENCH_SEED", DEFAULT_BENCH_SEED);
    let country_count = env_u16("BENCH_COUNTRIES", DEFAULT_BENCH_COUNTRIES);
    let population_per_country = env_u32("BENCH_POP_PER_COUNTRY", DEFAULT_BENCH_POP_PER_COUNTRY);
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
    #[cfg(not(feature = "bench-trace"))]
    let trace_enabled = false;
    let thread_sweep_enabled = env_bool("BENCH_THREAD_SWEEP", false);
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

    let mut run = run_benchmark_scenario(
        seed,
        country_count,
        population_per_country,
        days,
        thread_count,
        trace_enabled,
    );
    if thread_sweep_enabled {
        let requested_hash = run.metrics.world_state_hash.clone();
        run.metrics.thread_sweep = thread_sweep_counts(thread_count)
            .into_iter()
            .map(|threads| {
                let sweep = run_benchmark_scenario(
                    seed,
                    country_count,
                    population_per_country,
                    days,
                    threads,
                    false,
                );
                ThreadSweepEntry {
                    thread_count: threads,
                    ticks_per_sec: sweep.metrics.ticks_per_sec,
                    avg_tick_ms: sweep.metrics.avg_tick_ms,
                    p95_tick_ms: sweep.metrics.p95_tick_ms,
                    world_state_hash: sweep.metrics.world_state_hash.clone(),
                    matches_requested_hash: sweep.metrics.world_state_hash == requested_hash,
                }
            })
            .collect();
    }
    let metrics = &run.metrics;

    ensure_parent_dir(&json_output);
    fs::write(
        &json_output,
        serde_json::to_string_pretty(&metrics).expect("benchmark json should serialize"),
    )
    .expect("benchmark json should be written");

    ensure_parent_dir(&csv_output);
    let mut csv = format!(
        "run_kind,ticks_per_sec,avg_tick_ms,p95_tick_ms,total_wall_clock_ms,seed,countries,population_per_country,days,sim_mode,thread_count,world_state_hash,timestamp,hash_match\nrequested,{:.6},{:.6},{:.6},{:.6},0x{:016x},{},{},{},max-speed,{},{},{},true\n",
        metrics.ticks_per_sec,
        metrics.avg_tick_ms,
        metrics.p95_tick_ms,
        metrics.total_wall_clock_ms,
        seed,
        country_count,
        population_per_country,
        days,
        thread_count,
        metrics.world_state_hash,
        metrics.timestamp
    );
    for sweep in &metrics.thread_sweep {
        csv.push_str(&format!(
            "thread-sweep,{:.6},{:.6},{:.6},0.0,0x{:016x},{},{},{},max-speed,{},{},{},{}\n",
            sweep.ticks_per_sec,
            sweep.avg_tick_ms,
            sweep.p95_tick_ms,
            seed,
            country_count,
            population_per_country,
            days,
            sweep.thread_count,
            sweep.world_state_hash,
            metrics.timestamp,
            sweep.matches_requested_hash
        ));
    }
    fs::write(&csv_output, csv).expect("benchmark csv should be written");

    #[cfg(feature = "bench-trace")]
    if trace_enabled {
        if let Some(path) = trace_csv_output.as_ref() {
            crate::sim::trace::write_day_traces_csv(path, &run.day_traces)
                .expect("trace csv should be written");
        }
        if let Some(path) = trace_summary_output.as_ref() {
            let summary = crate::sim::trace::summarize_slowdown(&run.day_traces);
            crate::sim::trace::write_summary_json(path, &summary)
                .expect("trace summary json should be written");
        }
    }

    assert!(metrics.total_wall_clock_ms > 0.0);
    assert!(metrics.ticks_per_sec > 0.0);
}
