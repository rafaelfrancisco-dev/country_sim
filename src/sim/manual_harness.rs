use rayon::ThreadPoolBuilder;
use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    csv_export::CsvExporter,
    sim::{
        GameCommand, GameEngine, generator::GenerationConfig, generator::generate_countries,
        tickable::Tickable, world::World,
    },
};

const DEFAULT_BENCH_SEED: u64 = 0x5eed_c0de;
const DEFAULT_BENCH_COUNTRIES: u16 = 5;
const DEFAULT_BENCH_POP_PER_COUNTRY: u32 = 1_000_000;
const DEFAULT_BENCH_DAYS: u64 = 180;
const DEFAULT_BENCH_THREADS: usize = 8;
const DEFAULT_GENERATION_SEED: u64 = 0x5eed_c0de_u64;

fn submit_setup(engine: &mut GameEngine, countries: &str, thousands: &str) {
    for input in countries.chars() {
        engine.apply(GameCommand::SetupEditChar(input));
    }
    engine.apply(GameCommand::SetupNextField);
    for input in thousands.chars() {
        engine.apply(GameCommand::SetupEditChar(input));
    }
    engine.apply(GameCommand::SetupSubmit);
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
}

pub fn run_export_csv_for_one_year_headless() {
    let out_dir = env::var("EXPORT_CSV_TEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/country_sim_year1"));

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).expect("remove prior output dir");
    }
    fs::create_dir_all(&out_dir).expect("create output dir");

    let mut engine = GameEngine::new(DEFAULT_GENERATION_SEED);
    submit_setup(&mut engine, "1", "100");
    engine.enable_export_queue();

    let mut exporter: Option<CsvExporter> = None;
    let mut written_rows = 0usize;
    for _ in 0..365 {
        engine.apply(GameCommand::Tick);
        let export_rows = engine.drain_export_queue();
        if export_rows.is_empty() {
            continue;
        }

        if exporter.is_none() {
            let seed = export_rows[0].generation_seed;
            exporter = Some(CsvExporter::new(&out_dir, seed).expect("create exporter"));
        }

        if let Some(exporter) = &mut exporter {
            for month_data in export_rows {
                for country_data in month_data.countries {
                    exporter.write_row(&country_data).expect("write csv row");
                    written_rows += 1;
                }
            }
        }
    }

    assert_eq!(written_rows, 12, "365 days should produce 12 monthly rows");
}

pub fn run_export_stock_csv_for_one_year_headless() {
    let days = env_u64("STOCK_EXPORT_DAYS", 365);
    let country_count = env_u16("STOCK_EXPORT_COUNTRIES", 3);
    let thousands_per_country = env_u32("STOCK_EXPORT_THOUSANDS", 100);

    let out_dir = env::var("EXPORT_CSV_TEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/country_sim_stock_year1"));

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).expect("remove prior output dir");
    }
    fs::create_dir_all(&out_dir).expect("create output dir");

    let mut engine = GameEngine::new(DEFAULT_GENERATION_SEED);
    submit_setup(
        &mut engine,
        &country_count.to_string(),
        &thousands_per_country.to_string(),
    );
    engine.enable_export_queue();

    let mut exporter: Option<CsvExporter> = None;
    let mut monthly_rows = 0usize;
    let mut stock_country_rows = 0usize;
    let mut stock_equity_rows = 0usize;
    let mut invariant_warnings = 0usize;

    for _ in 0..days {
        let outcome = engine.apply(GameCommand::Tick);
        invariant_warnings += outcome.warnings.len();

        let export_rows = engine.drain_export_queue();
        if !export_rows.is_empty() {
            if exporter.is_none() {
                let seed = export_rows[0].generation_seed;
                exporter = Some(CsvExporter::new(&out_dir, seed).expect("create exporter"));
            }
            if let Some(exp) = &mut exporter {
                for month_data in export_rows {
                    for country_data in month_data.countries {
                        exp.write_row(&country_data).expect("write monthly csv row");
                        monthly_rows += 1;
                    }
                }
            }
        }

        let (country_rows, equity_rows) = engine.drain_stock_export_queues();
        if !country_rows.is_empty() {
            if exporter.is_none() {
                exporter = Some(
                    CsvExporter::new(&out_dir, DEFAULT_GENERATION_SEED).expect("create exporter"),
                );
            }
            if let Some(exp) = &mut exporter {
                for row in &country_rows {
                    exp.write_stock_country_row(row)
                        .expect("write stock country row");
                    stock_country_rows += 1;
                }
                for row in &equity_rows {
                    exp.write_stock_equity_row(row)
                        .expect("write stock equity row");
                    stock_equity_rows += 1;
                }
            }
        }
    }

    let expected_monthly_rows = ((days / 30) as usize) * country_count as usize;
    assert_eq!(
        monthly_rows, expected_monthly_rows,
        "expected {expected_monthly_rows} monthly rows (countries × completed months)"
    );
    assert!(
        stock_country_rows > 0,
        "expected at least some stock country rows"
    );
    assert!(
        stock_equity_rows >= stock_country_rows,
        "expected at least as many equity rows as country rows"
    );
    assert_eq!(
        invariant_warnings, 0,
        "invariant violations detected: {invariant_warnings}"
    );

    let seed = engine
        .world_generation_seed()
        .expect("world should exist and expose generation seed");
    assert!(
        out_dir
            .join(format!("stock_daily_country_{seed:016x}.csv"))
            .exists(),
        "stock_daily_country CSV missing"
    );
    assert!(
        out_dir
            .join(format!("stock_daily_equity_{seed:016x}.csv"))
            .exists(),
        "stock_daily_equity CSV missing"
    );
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
