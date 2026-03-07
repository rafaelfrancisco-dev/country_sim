use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    csv_export::CsvExporter,
    sim::{GameCommand, GameEngine},
};

const DEFAULT_BENCH_SEED: u64 = 0x5eed_c0de;
const DEFAULT_BENCH_COUNTRIES: u16 = 5;
const DEFAULT_BENCH_POP_PER_COUNTRY: u32 = 1_000_000;
const DEFAULT_BENCH_DAYS: u64 = 180;
const DEFAULT_BENCH_THREADS: usize = 8;
const DEFAULT_GENERATION_SEED: u64 = 0x5eed_c0de_u64;

mod benchmark;
pub use benchmark::run_benchmark_release_protocol_emits_metrics;

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

fn env_bool(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(raw) => matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
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

    for _ in 0..days {
        engine.apply(GameCommand::Tick);

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
