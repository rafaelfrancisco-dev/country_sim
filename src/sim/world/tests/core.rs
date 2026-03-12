use super::*;
use crate::sim::{
    bank::{BankKind, LoanAccount},
    company::build_default_companies,
    event::{IpoReason, SimEvent},
    generator::{GenerationConfig, generate_countries},
    money::round_cents,
    params::SimulationParams,
    person::{ForeignDeposit, Person},
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

fn migration_params() -> SimulationParams {
    let mut params = SimulationParams::default();
    params.migration.enabled = true;
    params.migration.base_monthly_migration_rate = 1.0;
    params.migration.long_term_unemployment_months = 6;
    params.migration.benefit_exhaustion_bonus_months = 0;
    params.migration.destination_unemployment_guard = 0.25;
    params.migration.irresistible_offer_salary_premium = 0.20;
    params.migration.irresistible_offer_min_open_slots = 1;
    params.migration.migration_cooldown_months = 6;
    params.migration.cash_buffer_required = 0.0;
    params.fiscal.max_benefit_months = 3;
    params
}

fn configured_country(
    id: u16,
    name: &str,
    people: Vec<Person>,
    params: SimulationParams,
) -> Country {
    let mut companies = build_default_companies(name, 4);
    for company in &mut companies {
        company.open_positions.clear();
    }
    Country::with_economy_and_params(id, name.to_string(), people, companies, 0.0, params)
}

#[test]
fn distressed_worker_migrates_to_country_with_compatible_demand() {
    let params = migration_params();
    let mut distressed = unemployed_person(1, 50.0);
    distressed.months_unemployed = 8;
    distressed.months_on_benefits = 4;
    distressed.consecutive_severe_unmet_months = 5;

    let source = configured_country(1, "A", vec![distressed], params.clone());
    let mut destination = configured_country(
        2,
        "B",
        vec![
            employed_person(2, 0, 0, 500.0),
            employed_person(3, 0, 1, 500.0),
        ],
        params,
    );
    destination.companies[0].add_opening_with_salary(0, 1, 3_000.0);

    let mut world = World::new(vec![source, destination]);
    world.process_cross_country_migration();

    assert_eq!(world.countries[0].population(), 0);
    assert_eq!(world.countries[1].population(), 3);
    assert_eq!(world.countries[0].monthly_emigrants(), 1);
    assert_eq!(world.countries[1].monthly_immigrants(), 1);
    assert_eq!(world.countries[1].monthly_migrants_for_distress(), 1);
}

#[test]
fn monthly_tick_executes_cross_country_migration_on_day_30() {
    let params = migration_params();
    let mut distressed = unemployed_person(1, 50.0);
    distressed.months_unemployed = 8;
    distressed.months_on_benefits = 4;
    distressed.consecutive_severe_unmet_months = 5;

    let source = configured_country(1, "A", vec![distressed], params.clone());
    let mut destination = configured_country(
        2,
        "B",
        vec![
            employed_person(2, 0, 0, 500.0),
            employed_person(3, 0, 1, 500.0),
        ],
        params,
    );
    destination.companies[0].add_opening_with_salary(0, 1, 3_000.0);

    let mut world = World::new(vec![source, destination]);
    for _ in 0..crate::sim::country::DAYS_PER_MONTH {
        world.tick_day();
    }

    assert_eq!(world.day, crate::sim::country::DAYS_PER_MONTH);
    assert_eq!(world.countries[0].population(), 0);
    assert_eq!(world.countries[1].population(), 3);
    assert_eq!(world.completed_month_boundaries(), 1);
    assert_eq!(world.world_history.len(), 2);
}

#[test]
fn initial_fx_reserves_are_split_across_foreign_currencies_only() {
    let world = World::new(vec![
        Country::new(1, "A".to_string(), vec![Person::new(1, 30, 0, 11)]),
        Country::new(2, "B".to_string(), vec![Person::new(2, 30, 0, 12)]),
        Country::new(3, "C".to_string(), vec![Person::new(3, 30, 0, 13)]),
    ]);

    for country_idx in 0..world.country_count() {
        let own_currency_idx = world.country(country_idx).unwrap().currency_idx();
        let expected_foreign_count = (0..world.country_count())
            .filter(|&src| {
                src != country_idx
                    && world.country(src).unwrap().currency_idx() != own_currency_idx
            })
            .count();

        let country = world.country(country_idx).unwrap();
        let reserves_by_source = country.fx_reserves_by_source();
        let attributed_total = round_cents(reserves_by_source.values().copied().sum::<f64>());

        // Only truly-foreign-currency countries should appear as reserve sources.
        assert_eq!(reserves_by_source.len(), expected_foreign_count);
        assert!(!reserves_by_source.contains_key(&country_idx));
        assert_eq!(attributed_total, country.fx_reserves_base());

        // No same-currency partner should appear in the reserve breakdown.
        for &src in reserves_by_source.keys() {
            assert_ne!(
                world.country(src).unwrap().currency_idx(),
                own_currency_idx,
                "country {country_idx} holds reserves from same-currency country {src}"
            );
        }

        // When there are ≥2 foreign sources the slices should be balanced (within 1 cent).
        if expected_foreign_count >= 2 {
            let mut reserve_slices = reserves_by_source.values().copied().collect::<Vec<_>>();
            reserve_slices.sort_by(|left, right| left.total_cmp(right));
            assert!(
                (reserve_slices[0] - reserve_slices.last().unwrap()).abs() <= 0.01,
                "reserves not evenly split for country {country_idx}"
            );
        }
    }
}

#[test]
fn migration_respects_destination_unemployment_guard() {
    let params = migration_params();
    let mut distressed = unemployed_person(1, 0.0);
    distressed.months_unemployed = 8;
    distressed.months_on_benefits = 4;
    distressed.consecutive_severe_unmet_months = 5;

    let source = configured_country(1, "A", vec![distressed], params.clone());
    let mut destination = configured_country(
        2,
        "B",
        vec![unemployed_person(2, 0.0), unemployed_person(3, 0.0)],
        params,
    );
    destination.companies[0].add_opening_with_salary(0, 1, 3_000.0);

    let mut world = World::new(vec![source, destination]);
    world.process_cross_country_migration();

    assert_eq!(world.countries[0].population(), 1);
    assert_eq!(world.countries[1].population(), 2);
    assert_eq!(world.countries[0].monthly_emigrants(), 0);
}

#[test]
fn employed_worker_migrates_for_irresistible_foreign_offer() {
    let params = migration_params();
    let mut worker = employed_person(1, 0, 0, 400.0);
    worker.birth_country_id = 1;
    worker.current_country_id = 1;

    let source = configured_country(1, "A", vec![worker], params.clone());
    let mut destination = configured_country(
        2,
        "B",
        vec![
            employed_person(2, 0, 0, 100.0),
            employed_person(3, 0, 1, 100.0),
        ],
        params,
    );
    destination.companies[0].add_opening_with_salary(0, 1, 4_500.0);

    let mut world = World::new(vec![source, destination]);
    world.process_cross_country_migration();

    let moved = &world.countries[1].people[2];
    assert_eq!(moved.current_country_id, 2);
    assert_eq!(moved.employer_company_id(), Some(0));
    assert_eq!(moved.monthly_salary(), 4_500.0);
    assert_eq!(world.countries[1].monthly_migrants_for_work(), 1);
}

#[test]
fn migrating_with_loan_transfers_credit_and_preserves_invariants() {
    let params = migration_params();
    let mut distressed = unemployed_person(1, 0.0);
    distressed.months_unemployed = 8;
    distressed.months_on_benefits = 4;
    distressed.consecutive_severe_unmet_months = 5;
    distressed.loan = Some(LoanAccount::new_with_lender(1_200.0, 0.10, 12, 0));

    let mut source = configured_country(1, "A", vec![distressed], params.clone());
    let mut destination = configured_country(
        2,
        "B",
        vec![
            employed_person(2, 0, 0, 100.0),
            employed_person(3, 0, 1, 100.0),
        ],
        params,
    );
    source.banks[0].created_money_outstanding = 1_200.0;
    source.recompute_aggregates();
    destination.companies[0].add_opening_with_salary(0, 1, 3_000.0);

    let mut world = World::new(vec![source, destination]);
    world.process_cross_country_migration();

    assert_eq!(world.countries[0].banks[0].created_money_outstanding, 0.0);
    assert_eq!(
        world.countries[1].banks[0].created_money_outstanding,
        1_200.0
    );
    assert!(world.countries[0].validate_money_invariants().is_empty());
    assert!(world.countries[1].validate_money_invariants().is_empty());
}

#[test]
fn migration_cooldown_blocks_immediate_return_move() {
    let params = migration_params();
    let worker = employed_person(1, 0, 0, 200.0);
    let source = configured_country(1, "A", vec![worker], params.clone());
    let mut destination = configured_country(
        2,
        "B",
        vec![
            employed_person(2, 0, 0, 100.0),
            employed_person(3, 0, 1, 100.0),
        ],
        params,
    );
    destination.companies[0].add_opening_with_salary(0, 1, 4_500.0);

    let mut world = World::new(vec![source, destination]);
    world.process_cross_country_migration();
    world.countries[0].companies[1].add_opening_with_salary(0, 1, 6_000.0);
    world.process_cross_country_migration();

    let migrated = world.countries[1]
        .people
        .iter()
        .find(|person| person.id == 1)
        .expect("migrated worker should remain in destination country");
    assert_eq!(migrated.current_country_id, 2);
    assert_eq!(migrated.lifetime_migrations, 1);
}

#[test]
fn migration_preserves_world_invariants_when_destination_ids_collide() {
    let params = migration_params();
    let source = configured_country(
        1,
        "A",
        vec![employed_person(1, 0, 0, 200.0)],
        params.clone(),
    );
    let mut destination = configured_country(
        2,
        "B",
        vec![
            employed_person(1, 0, 0, 100.0),
            employed_person(2, 0, 1, 100.0),
        ],
        params,
    );
    destination.companies[0].add_opening_with_salary(0, 1, 4_500.0);

    let mut world = World::new(vec![source, destination]);
    world.process_cross_country_migration();

    assert!(
        world.validate_invariants().is_empty(),
        "unexpected invariant violations: {:?}",
        world.validate_invariants()
    );
}
