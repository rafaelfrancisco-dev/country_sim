use super::*;
use crate::sim::{country::Country, person::Person};

fn person(id: u32) -> Person {
    Person::new(id, (id % 10) as u8, (id % 4) as u16, id + 1)
}

fn country(id: u16, name: &str, people_count: usize) -> Country {
    let people = (0..people_count).map(|idx| person(idx as u32)).collect();
    Country::new(id, name.to_string(), people)
}

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

fn submit_setup_with_seed(engine: &mut GameEngine, countries: &str, thousands: &str, seed: &str) {
    for input in countries.chars() {
        engine.apply(GameCommand::SetupEditChar(input));
    }
    engine.apply(GameCommand::SetupNextField);
    for input in thousands.chars() {
        engine.apply(GameCommand::SetupEditChar(input));
    }
    engine.apply(GameCommand::SetupNextField);
    for _ in 0..32 {
        engine.apply(GameCommand::SetupBackspace);
    }
    for input in seed.chars() {
        engine.apply(GameCommand::SetupEditChar(input));
    }
    engine.apply(GameCommand::SetupSubmit);
}

#[test]
fn valid_setup_submit_transitions_to_running() {
    let mut engine = GameEngine::with_population_scale(7, 10);

    submit_setup(&mut engine, "2", "100");

    assert!(matches!(engine.phase(), GamePhase::Running(_)));
}

#[test]
fn invalid_setup_submit_keeps_setup_and_sets_error() {
    let mut engine = GameEngine::default();

    submit_setup(&mut engine, "0", "100");

    assert!(matches!(engine.phase(), GamePhase::Setup(_)));
    let setup = engine.setup().expect("setup should still be active");
    assert_eq!(
        setup.validation_error.as_deref(),
        Some("Country count must be between 1 and 50.")
    );
}

#[test]
fn tick_increments_day_only_when_running_and_unpaused() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 1)])),
        ..GameEngine::default()
    };

    engine.apply(GameCommand::Tick);
    assert_eq!(engine.world().expect("world exists").day, 1);

    engine.apply(GameCommand::TogglePause);
    engine.apply(GameCommand::Tick);
    assert_eq!(engine.world().expect("world exists").day, 1);
}

#[test]
fn step_one_day_works_only_when_paused() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 1)])),
        ..GameEngine::default()
    };

    engine.apply(GameCommand::StepOneDay);
    assert_eq!(engine.world().expect("world exists").day, 0);

    engine.apply(GameCommand::TogglePause);
    engine.apply(GameCommand::StepOneDay);
    assert_eq!(engine.world().expect("world exists").day, 1);
}

#[cfg(not(feature = "runtime-invariants"))]
#[test]
fn apply_returns_no_warnings_without_runtime_invariant_feature() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 1)])),
        ..GameEngine::default()
    };
    let GamePhase::Running(world) = &mut engine.phase else {
        panic!("world should exist");
    };
    world.country_history.clear();

    let outcome = engine.apply(GameCommand::TogglePause);
    assert!(outcome.warnings.is_empty());
}

#[cfg(feature = "runtime-invariants")]
#[test]
fn apply_emits_warnings_with_runtime_invariant_feature() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 1)])),
        ..GameEngine::default()
    };
    let GamePhase::Running(world) = &mut engine.phase else {
        panic!("world should exist");
    };
    world.country_history.clear();

    let outcome = engine.apply(GameCommand::TogglePause);
    assert!(!outcome.warnings.is_empty());
    assert!(outcome.warnings[0].contains("country_history_len"));
}

#[test]
fn setup_seed_field_overrides_engine_default_seed() {
    let mut engine = GameEngine::new(7);
    submit_setup_with_seed(&mut engine, "1", "100", "0x123");

    let world = engine.world().expect("world should exist");
    assert_eq!(world.generation_seed, 0x123);
}

#[test]
fn update_country_policy_changes_only_target_country() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 2), country(2, "B", 2)])),
        ..GameEngine::default()
    };
    let original_first = engine.world().expect("world exists").countries[0]
        .sim_params
        .clone();

    let policy = CountryPolicyOverrides {
        bracket_1_max: 2_500.0,
        bracket_2_max: 6_000.0,
        rate_1: 0.08,
        rate_2: 0.18,
        rate_3: 0.28,
        unemployment_benefit_rate: 0.42,
        median_income_reference: 2_400.0,
        basic_needs_cash_threshold: 650.0,
        mint_inflation_gap_weight: 0.33,
        mint_unemployment_gap_weight: 0.22,
        mint_max_fraction_expected_supply: 0.07,
    };
    engine.apply(GameCommand::UpdateCountryPolicy {
        country_idx: 1,
        policy,
    });

    let world = engine.world().expect("world exists");
    let first = &world.countries[0];
    let second = &world.countries[1];

    assert_eq!(first.sim_params, original_first);
    assert!((second.sim_params.tax.bracket_1_max - 2_500.0).abs() < f64::EPSILON);
    assert!((second.sim_params.tax.bracket_2_max - 6_000.0).abs() < f64::EPSILON);
    assert!((second.sim_params.tax.rate_1 - 0.08).abs() < f64::EPSILON);
    assert!((second.sim_params.tax.rate_2 - 0.18).abs() < f64::EPSILON);
    assert!((second.sim_params.tax.rate_3 - 0.28).abs() < f64::EPSILON);
    assert!((second.sim_params.fiscal.unemployment_benefit_rate - 0.42).abs() < f64::EPSILON);
    assert!((second.sim_params.fiscal.median_income_reference - 2_400.0).abs() < f64::EPSILON);
    assert!((second.sim_params.fiscal.basic_needs_cash_threshold - 650.0).abs() < f64::EPSILON);
    assert!((second.sim_params.monetary.mint_inflation_gap_weight - 0.33).abs() < f64::EPSILON);
    assert!((second.sim_params.monetary.mint_unemployment_gap_weight - 0.22).abs() < f64::EPSILON);
    assert!(
        (second.sim_params.monetary.mint_max_fraction_expected_supply - 0.07).abs() < f64::EPSILON
    );
}

#[test]
fn update_country_policy_ignores_out_of_bounds_country_index() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 1)])),
        ..GameEngine::default()
    };
    let before = engine.world().expect("world exists").countries[0]
        .sim_params
        .clone();

    engine.apply(GameCommand::UpdateCountryPolicy {
        country_idx: 99,
        policy: CountryPolicyOverrides {
            bracket_1_max: 1_000.0,
            bracket_2_max: 2_000.0,
            rate_1: 0.05,
            rate_2: 0.10,
            rate_3: 0.15,
            unemployment_benefit_rate: 0.10,
            median_income_reference: 1_000.0,
            basic_needs_cash_threshold: 100.0,
            mint_inflation_gap_weight: 0.1,
            mint_unemployment_gap_weight: 0.2,
            mint_max_fraction_expected_supply: 0.3,
        },
    });

    let after = &engine.world().expect("world exists").countries[0].sim_params;
    assert_eq!(*after, before);
}

#[test]
fn apply_disasters_bankruptcy_wave_removes_companies_and_logs_event() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 40)])),
        ..GameEngine::default()
    };
    let before_companies = engine.world().expect("world exists").countries[0]
        .companies
        .len();

    engine.apply(GameCommand::ApplyDisasters {
        country_idx: 0,
        requests: vec![crate::sim::DisasterRequest {
            kind: crate::sim::DisasterKind::CompanyBankruptcyWave,
            intensity_pct: 25,
        }],
    });

    let world = engine.world().expect("world exists");
    let after_companies = world.countries[0].companies.len();
    assert!(after_companies < before_companies);
    assert!(
        world
            .event_log
            .iter()
            .any(|event| matches!(event, crate::sim::event::SimEvent::DisasterApplied { .. }))
    );
}

#[test]
fn apply_disasters_ignores_out_of_bounds_country_index() {
    let mut engine = GameEngine {
        phase: GamePhase::Running(World::new(vec![country(1, "A", 10)])),
        ..GameEngine::default()
    };
    let before = engine.world().expect("world exists").countries[0]
        .companies
        .len();

    engine.apply(GameCommand::ApplyDisasters {
        country_idx: 99,
        requests: vec![crate::sim::DisasterRequest {
            kind: crate::sim::DisasterKind::CompanyBankruptcyWave,
            intensity_pct: 50,
        }],
    });

    let world = engine.world().expect("world exists");
    assert_eq!(world.countries[0].companies.len(), before);
    assert!(world.event_log.is_empty());
}
