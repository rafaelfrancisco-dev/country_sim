use super::{
    generator::{GenerationConfig, generate_countries},
    setup::{CITIZENS_PER_MILLION, SetupForm, SetupValues},
    tickable::Tickable,
    world::World,
};

pub const DEFAULT_GENERATION_SEED: u64 = 0x5eed_c0de_u64;

#[derive(Debug, Clone, PartialEq)]
pub enum GamePhase {
    Setup(SetupForm),
    Running(World),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GameCommand {
    SetupEditChar(char),
    SetupBackspace,
    SetupNextField,
    SetupPrevField,
    SetupSubmit,
    TogglePause,
    StepOneDay,
    Tick,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyOutcome {
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEngine {
    phase: GamePhase,
    generation_seed: u64,
    citizens_per_million: u32,
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new(DEFAULT_GENERATION_SEED)
    }
}

impl GameEngine {
    pub fn new(generation_seed: u64) -> Self {
        Self::with_population_scale(generation_seed, CITIZENS_PER_MILLION)
    }

    pub fn with_population_scale(generation_seed: u64, citizens_per_million: u32) -> Self {
        Self {
            phase: GamePhase::Setup(SetupForm::with_generation_seed(generation_seed)),
            generation_seed,
            citizens_per_million: citizens_per_million.max(1),
        }
    }

    pub fn phase(&self) -> &GamePhase {
        &self.phase
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

        let mut outcome = ApplyOutcome::default();
        if let GamePhase::Running(world) = &self.phase {
            for violation in world.validate_invariants() {
                outcome
                    .warnings
                    .push(format!("{}: {}", violation.code, violation.message));
            }
        }
        outcome
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
            _ => {}
        }
    }

    fn build_world(&self, values: SetupValues) -> World {
        let population_per_country = (values.millions_per_country
            * f64::from(self.citizens_per_million))
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32;

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

    fn submit_setup(engine: &mut GameEngine, countries: &str, millions: &str) {
        for input in countries.chars() {
            engine.apply(GameCommand::SetupEditChar(input));
        }
        engine.apply(GameCommand::SetupNextField);
        for input in millions.chars() {
            engine.apply(GameCommand::SetupEditChar(input));
        }
        engine.apply(GameCommand::SetupSubmit);
    }

    fn submit_setup_with_seed(
        engine: &mut GameEngine,
        countries: &str,
        millions: &str,
        seed: &str,
    ) {
        for input in countries.chars() {
            engine.apply(GameCommand::SetupEditChar(input));
        }
        engine.apply(GameCommand::SetupNextField);
        for input in millions.chars() {
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

        submit_setup(&mut engine, "2", "1");

        assert!(matches!(engine.phase(), GamePhase::Running(_)));
    }

    #[test]
    fn invalid_setup_submit_keeps_setup_and_sets_error() {
        let mut engine = GameEngine::default();

        submit_setup(&mut engine, "0", "1");

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

    #[test]
    fn setup_seed_field_overrides_engine_default_seed() {
        let mut engine = GameEngine::new(7);
        submit_setup_with_seed(&mut engine, "1", "1", "0x123");

        let world = engine.world().expect("world should exist");
        assert_eq!(world.generation_seed, 0x123);
    }
}
