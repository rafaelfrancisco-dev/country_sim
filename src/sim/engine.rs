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
    FocusCountrySelection,
    FocusPeopleSelection,
    SelectPrevCountryRow,
    SelectNextCountryRow,
    SelectPrevPersonRow,
    SelectNextPersonRow,
    CountryPageNext,
    CountryPagePrev,
    PeoplePageNext,
    PeoplePagePrev,
    OpenSelectedPersonDetails,
    OpenSelectedCountryDetails,
    CloseDetails,
    NextTab,
    PrevTab,
    ToggleChartScope,
    ChartPrevCountry,
    ChartNextCountry,
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

    pub fn world_mut(&mut self) -> Option<&mut World> {
        match &mut self.phase {
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
            GameCommand::FocusCountrySelection => world.focus_country_selection(),
            GameCommand::FocusPeopleSelection => world.focus_people_selection(),
            GameCommand::SelectPrevCountryRow => world.select_prev_country_row(),
            GameCommand::SelectNextCountryRow => world.select_next_country_row(),
            GameCommand::SelectPrevPersonRow => world.select_prev_person_row(),
            GameCommand::SelectNextPersonRow => world.select_next_person_row(),
            GameCommand::CountryPageNext => world.next_country_page(),
            GameCommand::CountryPagePrev => world.prev_country_page(),
            GameCommand::PeoplePageNext => world.next_people_page(),
            GameCommand::PeoplePagePrev => world.prev_people_page(),
            GameCommand::OpenSelectedPersonDetails => world.open_selected_person_details(),
            GameCommand::OpenSelectedCountryDetails => world.open_selected_country_details(),
            GameCommand::CloseDetails => world.close_details(),
            GameCommand::NextTab => world.next_tab(),
            GameCommand::PrevTab => world.prev_tab(),
            GameCommand::ToggleChartScope => world.toggle_chart_scope(),
            GameCommand::ChartPrevCountry => world.chart_prev_country(),
            GameCommand::ChartNextCountry => world.chart_next_country(),
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
    use crate::sim::{
        country::Country,
        person::Person,
        world::{COUNTRY_PAGE_SIZE, DetailsView, PEOPLE_PAGE_SIZE, SelectionFocus},
    };

    fn person(id: u32) -> Person {
        Person::new(id, (id % 10) as u8, (id % 4) as u16, id + 1)
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

        engine.apply(GameCommand::SelectPrevCountryRow);
        assert_eq!(engine.world().expect("world exists").selected_country, 0);

        engine.apply(GameCommand::SelectNextCountryRow);
        assert_eq!(engine.world().expect("world exists").selected_country, 1);
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
    fn person_row_selection_and_details_commands_mutate_running_world() {
        let mut engine = GameEngine::default();
        engine.phase = GamePhase::Running(World::new(vec![country(1, "A", 2)]));

        engine.apply(GameCommand::SelectNextPersonRow);
        assert_eq!(engine.world().expect("world exists").selected_person_row, 1);

        engine.apply(GameCommand::OpenSelectedPersonDetails);
        assert_eq!(
            engine.world().expect("world exists").details_view,
            DetailsView::Person
        );

        engine.apply(GameCommand::CloseDetails);
        assert_eq!(
            engine.world().expect("world exists").details_view,
            DetailsView::List
        );
    }

    #[test]
    fn new_running_commands_are_noop_in_setup_phase() {
        let mut engine = GameEngine::default();
        let setup_before = engine.setup().expect("setup should exist").clone();

        engine.apply(GameCommand::FocusCountrySelection);
        engine.apply(GameCommand::FocusPeopleSelection);
        engine.apply(GameCommand::SelectNextCountryRow);
        engine.apply(GameCommand::SelectPrevCountryRow);
        engine.apply(GameCommand::SelectNextPersonRow);
        engine.apply(GameCommand::SelectPrevPersonRow);
        engine.apply(GameCommand::CountryPageNext);
        engine.apply(GameCommand::CountryPagePrev);
        engine.apply(GameCommand::OpenSelectedPersonDetails);
        engine.apply(GameCommand::OpenSelectedCountryDetails);
        engine.apply(GameCommand::CloseDetails);
        engine.apply(GameCommand::NextTab);
        engine.apply(GameCommand::PrevTab);
        engine.apply(GameCommand::ToggleChartScope);
        engine.apply(GameCommand::ChartPrevCountry);
        engine.apply(GameCommand::ChartNextCountry);

        assert_eq!(
            engine.setup().expect("setup should still exist"),
            &setup_before
        );
    }

    #[test]
    fn focus_switch_and_country_page_commands_work_in_running_phase() {
        let mut engine = GameEngine::default();
        let countries = (0..(COUNTRY_PAGE_SIZE + 2))
            .map(|idx| country(idx as u16, "A", 1))
            .collect();
        engine.phase = GamePhase::Running(World::new(countries));

        engine.apply(GameCommand::FocusPeopleSelection);
        assert_eq!(
            engine.world().expect("world exists").selection_focus,
            SelectionFocus::People
        );

        engine.apply(GameCommand::FocusCountrySelection);
        assert_eq!(
            engine.world().expect("world exists").selection_focus,
            SelectionFocus::Country
        );

        engine.apply(GameCommand::SelectNextCountryRow);
        let selected_before_page = engine.world().expect("world exists").selected_country;
        engine.apply(GameCommand::CountryPageNext);
        assert!(engine.world().expect("world exists").selected_country > selected_before_page);
    }

    #[test]
    fn country_details_commands_mutate_running_world() {
        let mut engine = GameEngine::default();
        engine.phase = GamePhase::Running(World::new(vec![country(1, "A", 2)]));

        engine.apply(GameCommand::OpenSelectedCountryDetails);
        assert_eq!(
            engine.world().expect("world exists").details_view,
            DetailsView::Country
        );

        engine.apply(GameCommand::CloseDetails);
        assert_eq!(
            engine.world().expect("world exists").details_view,
            DetailsView::List
        );
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

    #[test]
    fn tab_scope_and_chart_commands_mutate_running_world() {
        let mut engine = GameEngine::default();
        engine.phase = GamePhase::Running(World::new(vec![country(1, "A", 1), country(2, "B", 1)]));

        engine.apply(GameCommand::NextTab);
        assert_eq!(
            engine.world().expect("world exists").active_tab,
            crate::sim::world::RunningTab::Charts
        );

        engine.apply(GameCommand::ToggleChartScope);
        assert_eq!(
            engine.world().expect("world exists").chart_scope,
            crate::sim::world::ChartScope::WorldAggregate
        );

        engine.apply(GameCommand::ChartNextCountry);
        assert_eq!(engine.world().expect("world exists").selected_country, 1);

        engine.apply(GameCommand::ChartPrevCountry);
        assert_eq!(engine.world().expect("world exists").selected_country, 0);
    }
}
