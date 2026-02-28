use super::{
    generator::{GenerationConfig, generate_countries},
    setup::{CITIZENS_PER_MILLION, SetupForm, SetupValues},
    tickable::Tickable,
    world::{
        MonthlyExportData, StockDailyCountryExportData, StockDailyEquityExportData, World,
    },
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
    ) -> (Vec<StockDailyCountryExportData>, Vec<StockDailyEquityExportData>) {
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
    use crate::csv_export::CsvExporter;
    use std::path::PathBuf;

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

    #[test]
    #[ignore = "manual harness to generate one-year CSV export data"]
    fn export_csv_for_one_year_headless() {
        let out_dir = std::env::var("EXPORT_CSV_TEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/country_sim_year1"));

        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir).expect("remove prior output dir");
        }
        std::fs::create_dir_all(&out_dir).expect("create output dir");

        let mut engine = GameEngine::new(DEFAULT_GENERATION_SEED);
        submit_setup(&mut engine, "1", "0.1");
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

    #[test]
    #[ignore = "manual harness: 3 countries × 100k × 365 days with stock CSV export"]
    fn export_stock_csv_for_one_year_headless() {
        fn env_u64(name: &str, default: u64) -> u64 {
            std::env::var(name)
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(default)
        }
        fn env_u16(name: &str, default: u16) -> u16 {
            std::env::var(name)
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(default)
        }
        fn env_f64(name: &str, default: f64) -> f64 {
            std::env::var(name)
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(default)
        }

        let days = env_u64("STOCK_EXPORT_DAYS", 365);
        let country_count = env_u16("STOCK_EXPORT_COUNTRIES", 3);
        let millions_per_country = env_f64("STOCK_EXPORT_MILLIONS", 0.1);

        let out_dir = std::env::var("EXPORT_CSV_TEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/country_sim_stock_year1"));

        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir).expect("remove prior output dir");
        }
        std::fs::create_dir_all(&out_dir).expect("create output dir");

        let mut engine = GameEngine::new(DEFAULT_GENERATION_SEED);
        submit_setup(
            &mut engine,
            &country_count.to_string(),
            &format!("{millions_per_country:.3}"),
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

            // Monthly export
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

            // Daily stock export
            let (country_rows, equity_rows) = engine.drain_stock_export_queues();
            if !country_rows.is_empty() {
                if exporter.is_none() {
                    // Shouldn't happen if monthly ran first, but handle gracefully
                    let seed = DEFAULT_GENERATION_SEED;
                    exporter = Some(CsvExporter::new(&out_dir, seed).expect("create exporter"));
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
        // Stock country rows: one row per country per day (only days with listings)
        assert!(
            stock_country_rows > 0,
            "expected at least some stock country rows"
        );
        // Stock equity rows: >= stock_country_rows (multiple equities per country)
        assert!(
            stock_equity_rows >= stock_country_rows,
            "expected at least as many equity rows as country rows"
        );
        assert_eq!(
            invariant_warnings, 0,
            "invariant violations detected: {invariant_warnings}"
        );

        // Verify output files exist
        let world = engine.world().expect("world should exist");
        let seed = world.generation_seed;
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
}
