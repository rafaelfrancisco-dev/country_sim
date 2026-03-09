use super::*;
use crate::sim::{
    bank::BankKind,
    event::{IpoReason, SimEvent},
    generator::{GenerationConfig, generate_countries},
    money::round_cents,
    person::ForeignDeposit,
};
use rayon::ThreadPoolBuilder;
use std::collections::HashMap;

mod deposit_and_trace;
mod history_parallel;
mod long_run;

fn labor_force_unemployment(country: &Country) -> f64 {
    let mut labor_force = 0_usize;
    let mut unemployed = 0_usize;
    for person in &country.people {
        if !person.is_labor_force_participant() {
            continue;
        }
        labor_force += 1;
        if !person.is_employed() {
            unemployed += 1;
        }
    }
    if labor_force == 0 {
        0.0
    } else {
        unemployed as f64 / labor_force as f64
    }
}

fn run_generated_world_with_acceptance_metrics_in_pool(
    seed: u64,
    country_count: u16,
    population_per_country: u32,
    days: u64,
    threads: usize,
) -> (World, u64, HashMap<String, u64>) {
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
        let mut companies_founded_total = 0_u64;
        let mut growth_ipos_by_country: HashMap<String, u64> = HashMap::new();

        for _ in 0..days {
            world.tick_day();
            for event in world.event_log.drain(..) {
                match event {
                    SimEvent::CompanyFounded { .. } => {
                        companies_founded_total = companies_founded_total.saturating_add(1);
                    }
                    SimEvent::StockIpo {
                        country_name,
                        reason: IpoReason::Growth,
                        ..
                    } => {
                        let entry = growth_ipos_by_country
                            .entry(country_name.to_string())
                            .or_insert(0);
                        *entry = entry.saturating_add(1);
                    }
                    _ => {}
                }
            }
        }

        (world, companies_founded_total, growth_ipos_by_country)
    })
}
