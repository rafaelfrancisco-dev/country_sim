use super::{
    generator::{GenerationConfig, generate_countries},
    setup::{CITIZENS_PER_THOUSAND, SetupForm, SetupValues},
    tickable::Tickable,
    world::{MonthlyExportData, StockDailyCountryExportData, StockDailyEquityExportData, World},
};

pub const DEFAULT_GENERATION_SEED: u64 = 0x5eed_c0de_u64;

#[derive(Debug, Clone, PartialEq)]
pub enum GamePhase {
    Setup(SetupForm),
    Running(World),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GameCommand {
    SetupEditChar(char),
    SetupBackspace,
    SetupNextField,
    SetupPrevField,
    SetupSubmit,
    TogglePause,
    StepOneDay,
    UpdateCountryPolicy {
        country_idx: usize,
        policy: CountryPolicyOverrides,
    },
    Tick,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CountryPolicyOverrides {
    pub bracket_1_max: f64,
    pub bracket_2_max: f64,
    pub rate_1: f64,
    pub rate_2: f64,
    pub rate_3: f64,
    pub unemployment_benefit_rate: f64,
    pub median_income_reference: f64,
    pub basic_needs_cash_threshold: f64,
}

impl CountryPolicyOverrides {
    pub fn from_country(country: &crate::sim::country::Country) -> Self {
        let tax = country.sim_params.tax;
        let fiscal = country.sim_params.fiscal;
        Self {
            bracket_1_max: tax.bracket_1_max,
            bracket_2_max: tax.bracket_2_max,
            rate_1: tax.rate_1,
            rate_2: tax.rate_2,
            rate_3: tax.rate_3,
            unemployment_benefit_rate: fiscal.unemployment_benefit_rate,
            median_income_reference: fiscal.median_income_reference,
            basic_needs_cash_threshold: fiscal.basic_needs_cash_threshold,
        }
    }

    pub fn managed_defaults() -> Self {
        let params = crate::sim::params::SimulationParams::default();
        Self {
            bracket_1_max: params.tax.bracket_1_max,
            bracket_2_max: params.tax.bracket_2_max,
            rate_1: params.tax.rate_1,
            rate_2: params.tax.rate_2,
            rate_3: params.tax.rate_3,
            unemployment_benefit_rate: params.fiscal.unemployment_benefit_rate,
            median_income_reference: params.fiscal.median_income_reference,
            basic_needs_cash_threshold: params.fiscal.basic_needs_cash_threshold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyOutcome {
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEngine {
    phase: GamePhase,
    generation_seed: u64,
    citizens_per_thousand: u32,
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new(DEFAULT_GENERATION_SEED)
    }
}

impl GameEngine {
    pub fn new(generation_seed: u64) -> Self {
        Self::with_population_scale(generation_seed, CITIZENS_PER_THOUSAND)
    }

    pub fn with_population_scale(generation_seed: u64, citizens_per_thousand: u32) -> Self {
        Self {
            phase: GamePhase::Setup(SetupForm::with_generation_seed(generation_seed)),
            generation_seed,
            citizens_per_thousand: citizens_per_thousand.max(1),
        }
    }

    pub fn phase(&self) -> &GamePhase {
        &self.phase
    }

    pub fn drain_export_queue(&mut self) -> Vec<MonthlyExportData> {
        if let GamePhase::Running(world) = &mut self.phase {
            world.export_queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    /// Drain the daily stock export queues.
    /// Returns `(country_rows, equity_rows)`.
    pub fn drain_stock_export_queues(
        &mut self,
    ) -> (
        Vec<StockDailyCountryExportData>,
        Vec<StockDailyEquityExportData>,
    ) {
        if let GamePhase::Running(world) = &mut self.phase {
            (
                world.stock_country_export_queue.drain(..).collect(),
                world.stock_equity_export_queue.drain(..).collect(),
            )
        } else {
            (Vec::new(), Vec::new())
        }
    }

    pub fn enable_export_queue(&mut self) {
        if let GamePhase::Running(world) = &mut self.phase {
            world.export_queue_active = true;
        }
    }

    /// Returns the world's generation seed once the engine is in the Running phase.
    pub fn world_generation_seed(&self) -> Option<u64> {
        match &self.phase {
            GamePhase::Running(world) => Some(world.generation_seed),
            GamePhase::Setup(_) => None,
        }
    }

    #[cfg(test)]
    pub fn setup(&self) -> Option<&SetupForm> {
        match &self.phase {
            GamePhase::Setup(setup) => Some(setup),
            GamePhase::Running(_) => None,
        }
    }

    #[cfg(test)]
    pub fn world(&self) -> Option<&World> {
        match &self.phase {
            GamePhase::Setup(_) => None,
            GamePhase::Running(world) => Some(world),
        }
    }

    pub fn apply(&mut self, command: GameCommand) -> ApplyOutcome {
        match &mut self.phase {
            GamePhase::Setup(setup_form) => {
                if let Some(values) = Self::apply_setup_command(setup_form, command) {
                    let world = self.build_world(values);
                    self.phase = GamePhase::Running(world);
                }
            }
            GamePhase::Running(world) => Self::apply_running_command(world, command),
        }

        ApplyOutcome {
            warnings: self.collect_invariant_warnings(),
        }
    }

    #[cfg(feature = "runtime-invariants")]
    fn collect_invariant_warnings(&self) -> Vec<String> {
        let GamePhase::Running(world) = &self.phase else {
            return Vec::new();
        };
        world
            .validate_invariants()
            .into_iter()
            .map(|violation| format!("{}: {}", violation.code, violation.message))
            .collect()
    }

    #[cfg(not(feature = "runtime-invariants"))]
    fn collect_invariant_warnings(&self) -> Vec<String> {
        Vec::new()
    }

    fn apply_setup_command(
        setup_form: &mut SetupForm,
        command: GameCommand,
    ) -> Option<SetupValues> {
        match command {
            GameCommand::SetupEditChar(input) => setup_form.edit_char(input),
            GameCommand::SetupBackspace => setup_form.backspace(),
            GameCommand::SetupNextField => setup_form.next_field(),
            GameCommand::SetupPrevField => setup_form.prev_field(),
            GameCommand::SetupSubmit => {
                let result = setup_form.validate();
                return setup_form.apply_validation_result(result);
            }
            _ => {}
        }

        None
    }

    fn apply_running_command(world: &mut World, command: GameCommand) {
        match command {
            GameCommand::Tick => {
                if !world.paused {
                    world.tick_day();
                }
            }
            GameCommand::TogglePause => {
                world.paused = !world.paused;
            }
            GameCommand::StepOneDay => {
                if world.paused {
                    world.tick_day();
                }
            }
            GameCommand::UpdateCountryPolicy {
                country_idx,
                policy,
            } => {
                let Some(country) = world.countries.get_mut(country_idx) else {
                    return;
                };

                let bracket_1_max = policy.bracket_1_max.max(0.0);
                let bracket_2_max = policy.bracket_2_max.max(bracket_1_max);
                country.sim_params.tax.bracket_1_max = bracket_1_max;
                country.sim_params.tax.bracket_2_max = bracket_2_max;
                country.sim_params.tax.rate_1 = policy.rate_1.clamp(0.0, 1.0);
                country.sim_params.tax.rate_2 = policy.rate_2.clamp(0.0, 1.0);
                country.sim_params.tax.rate_3 = policy.rate_3.clamp(0.0, 1.0);

                country.sim_params.fiscal.unemployment_benefit_rate =
                    policy.unemployment_benefit_rate.clamp(0.0, 1.0);
                country.sim_params.fiscal.median_income_reference =
                    policy.median_income_reference.max(0.0);
                country.sim_params.fiscal.basic_needs_cash_threshold =
                    policy.basic_needs_cash_threshold.max(0.0);
            }
            _ => {}
        }
    }

    fn build_world(&self, values: SetupValues) -> World {
        let population_per_country = values
            .thousands_per_country
            .saturating_mul(self.citizens_per_thousand);

        let config = GenerationConfig::with_defaults(
            values.country_count,
            population_per_country,
            values.generation_seed,
        );

        let countries = generate_countries(&config);
        World::new_with_generation_seed(countries, values.generation_seed)
    }
}

#[cfg(test)]
mod tests {
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

    fn submit_setup_with_seed(
        engine: &mut GameEngine,
        countries: &str,
        thousands: &str,
        seed: &str,
    ) {
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
            },
        });

        let after = &engine.world().expect("world exists").countries[0].sim_params;
        assert_eq!(*after, before);
    }
}
