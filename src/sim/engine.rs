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
    SetupEditDigit(u8),
    SetupBackspace,
    SetupNextField,
    SetupPrevField,
    SetupSubmit,
    TogglePause,
    StepOneDay,
    SelectPrevCountry,
    SelectNextCountry,
    PeoplePageNext,
    PeoplePagePrev,
    Tick,
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
            phase: GamePhase::Setup(SetupForm::default()),
            generation_seed,
            citizens_per_million: citizens_per_million.max(1),
        }
    }

    pub fn phase(&self) -> &GamePhase {
        &self.phase
    }

    pub fn setup(&self) -> Option<&SetupForm> {
        match &self.phase {
            GamePhase::Setup(setup) => Some(setup),
            GamePhase::Running(_) => None,
        }
    }

    pub fn world(&self) -> Option<&World> {
        match &self.phase {
            GamePhase::Setup(_) => None,
            GamePhase::Running(world) => Some(world),
        }
    }

    pub fn apply(&mut self, command: GameCommand) {
        match &mut self.phase {
            GamePhase::Setup(setup_form) => {
                if let Some(values) = Self::apply_setup_command(setup_form, command) {
                    let world = self.build_world(values);
                    self.phase = GamePhase::Running(world);
                }
            }
            GamePhase::Running(world) => Self::apply_running_command(world, command),
        }
    }

    fn apply_setup_command(
        setup_form: &mut SetupForm,
        command: GameCommand,
    ) -> Option<SetupValues> {
        match command {
            GameCommand::SetupEditDigit(digit) => setup_form.edit_digit(digit),
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
            GameCommand::SelectPrevCountry => world.select_prev_country(),
            GameCommand::SelectNextCountry => world.select_next_country(),
            GameCommand::PeoplePageNext => world.next_people_page(),
            GameCommand::PeoplePagePrev => world.prev_people_page(),
            _ => {}
        }
    }

    fn build_world(&self, values: SetupValues) -> World {
        let population_per_country = values
            .millions_per_country
            .checked_mul(self.citizens_per_million)
            .expect("population_per_country overflowed while generating world");

        let config = GenerationConfig::with_defaults(
            values.country_count,
            population_per_country,
            self.generation_seed,
        );

        let countries = generate_countries(&config);
        World::new(countries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{country::Country, person::Person, world::PEOPLE_PAGE_SIZE};

    fn person(id: u32) -> Person {
        Person::new(id, 100, 60, id + 1)
    }

    fn country(id: u16, name: &str, people_count: usize) -> Country {
        let people = (0..people_count).map(|idx| person(idx as u32)).collect();
        Country::new(id, name.to_string(), people)
    }

    fn submit_setup(engine: &mut GameEngine, countries: &str, millions: &str) {
        for digit in countries.bytes() {
            engine.apply(GameCommand::SetupEditDigit(digit - b'0'));
        }
        engine.apply(GameCommand::SetupNextField);
        for digit in millions.bytes() {
            engine.apply(GameCommand::SetupEditDigit(digit - b'0'));
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
        let mut engine = GameEngine::default();
        engine.phase = GamePhase::Running(World::new(vec![country(1, "A", 1)]));

        engine.apply(GameCommand::Tick);
        assert_eq!(engine.world().expect("world exists").day, 1);

        engine.apply(GameCommand::TogglePause);
        engine.apply(GameCommand::Tick);
        assert_eq!(engine.world().expect("world exists").day, 1);
    }

    #[test]
    fn step_one_day_works_only_when_paused() {
        let mut engine = GameEngine::default();
        engine.phase = GamePhase::Running(World::new(vec![country(1, "A", 1)]));

        engine.apply(GameCommand::StepOneDay);
        assert_eq!(engine.world().expect("world exists").day, 0);

        engine.apply(GameCommand::TogglePause);
        engine.apply(GameCommand::StepOneDay);
        assert_eq!(engine.world().expect("world exists").day, 1);
    }

    #[test]
    fn country_selection_wraps() {
        let mut engine = GameEngine::default();
        engine.phase = GamePhase::Running(World::new(vec![country(1, "A", 1), country(2, "B", 1)]));

        engine.apply(GameCommand::SelectPrevCountry);
        assert_eq!(engine.world().expect("world exists").selected_country, 1);

        engine.apply(GameCommand::SelectNextCountry);
        assert_eq!(engine.world().expect("world exists").selected_country, 0);
    }

    #[test]
    fn people_pagination_clamps() {
        let mut engine = GameEngine::default();
        engine.phase =
            GamePhase::Running(World::new(vec![country(1, "Paged", PEOPLE_PAGE_SIZE + 3)]));

        engine.apply(GameCommand::PeoplePageNext);
        assert_eq!(engine.world().expect("world exists").people_page, 1);

        engine.apply(GameCommand::PeoplePageNext);
        assert_eq!(engine.world().expect("world exists").people_page, 1);

        engine.apply(GameCommand::PeoplePagePrev);
        engine.apply(GameCommand::PeoplePagePrev);
        assert_eq!(engine.world().expect("world exists").people_page, 0);
    }

    #[test]
    fn integration_sequence_from_setup_to_ticking_is_deterministic() {
        let mut engine = GameEngine::with_population_scale(99, 5);

        submit_setup(&mut engine, "1", "1");
        engine.apply(GameCommand::Tick);
        engine.apply(GameCommand::Tick);
        engine.apply(GameCommand::TogglePause);
        engine.apply(GameCommand::StepOneDay);

        let world = engine.world().expect("world should exist after setup");
        assert_eq!(world.day, 3);
        assert_eq!(world.countries.len(), 1);
        assert_eq!(world.countries[0].people.len(), 5);
    }
}
