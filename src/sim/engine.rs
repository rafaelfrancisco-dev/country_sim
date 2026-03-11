use super::{
    disaster::{DisasterRequest, apply_disaster_requests},
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

/// Commands for the persistence subsystem (save/load).
#[derive(Debug, Clone, PartialEq)]
pub enum PersistenceCommand {
    /// Save the current running world.
    SaveCurrentRun,
    /// Setup mode: type a character into the save-name field.
    SaveNameEditChar(char),
    /// Setup mode: backspace in the save-name field.
    SaveNameBackspace,
    /// Setup mode: select previous save in the list.
    SaveListSelectPrev,
    /// Setup mode: select next save in the list.
    SaveListSelectNext,
    /// Setup mode: refresh the save list from disk.
    SaveListRefresh,
    /// Setup mode: load the currently selected save.
    LoadSelectedSave,
}

#[derive(Debug, Clone, PartialEq)]
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
    ApplyDisasters {
        country_idx: usize,
        requests: Vec<DisasterRequest>,
    },
    Tick,
    /// Persistence commands (save/load).
    Persist(PersistenceCommand),
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
    pub target_inflation_rate: f64,
    pub neutral_policy_rate: f64,
    pub inflation_coefficient: f64,
    pub unemployment_coefficient: f64,
    pub natural_unemployment_rate: f64,
    pub rate_floor: f64,
    pub rate_ceiling: f64,
    pub rate_change_event_threshold: f64,
    pub policy_rate_step: f64,
    pub mint_inflation_gap_weight: f64,
    pub mint_unemployment_gap_weight: f64,
    pub mint_max_fraction_expected_supply: f64,
}

impl CountryPolicyOverrides {
    pub fn from_country(country: &crate::sim::country::Country) -> Self {
        let tax = country.sim_params.tax;
        let fiscal = country.sim_params.fiscal;
        let monetary = country.sim_params.monetary;
        Self {
            bracket_1_max: tax.bracket_1_max,
            bracket_2_max: tax.bracket_2_max,
            rate_1: tax.rate_1,
            rate_2: tax.rate_2,
            rate_3: tax.rate_3,
            unemployment_benefit_rate: fiscal.unemployment_benefit_rate,
            median_income_reference: fiscal.median_income_reference,
            basic_needs_cash_threshold: fiscal.basic_needs_cash_threshold,
            target_inflation_rate: monetary.target_inflation_rate,
            neutral_policy_rate: monetary.neutral_policy_rate,
            inflation_coefficient: monetary.inflation_coefficient,
            unemployment_coefficient: monetary.unemployment_coefficient,
            natural_unemployment_rate: monetary.natural_unemployment_rate,
            rate_floor: monetary.rate_floor,
            rate_ceiling: monetary.rate_ceiling,
            rate_change_event_threshold: monetary.rate_change_event_threshold,
            policy_rate_step: monetary.policy_rate_step,
            mint_inflation_gap_weight: monetary.mint_inflation_gap_weight,
            mint_unemployment_gap_weight: monetary.mint_unemployment_gap_weight,
            mint_max_fraction_expected_supply: monetary.mint_max_fraction_expected_supply,
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
            target_inflation_rate: params.monetary.target_inflation_rate,
            neutral_policy_rate: params.monetary.neutral_policy_rate,
            inflation_coefficient: params.monetary.inflation_coefficient,
            unemployment_coefficient: params.monetary.unemployment_coefficient,
            natural_unemployment_rate: params.monetary.natural_unemployment_rate,
            rate_floor: params.monetary.rate_floor,
            rate_ceiling: params.monetary.rate_ceiling,
            rate_change_event_threshold: params.monetary.rate_change_event_threshold,
            policy_rate_step: params.monetary.policy_rate_step,
            mint_inflation_gap_weight: params.monetary.mint_inflation_gap_weight,
            mint_unemployment_gap_weight: params.monetary.mint_unemployment_gap_weight,
            mint_max_fraction_expected_supply: params.monetary.mint_max_fraction_expected_supply,
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

    /// Returns a reference to the running world if in Running phase.
    pub fn running_world(&self) -> Option<&World> {
        match &self.phase {
            GamePhase::Setup(_) => None,
            GamePhase::Running(world) => Some(world),
        }
    }

    /// Replaces the running world with a new one (used for load).
    pub fn replace_running_world(&mut self, world: World) {
        self.phase = GamePhase::Running(world);
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
            // Persistence commands and running-phase commands are no-ops during setup.
            GameCommand::TogglePause
            | GameCommand::StepOneDay
            | GameCommand::Tick
            | GameCommand::UpdateCountryPolicy { .. }
            | GameCommand::ApplyDisasters { .. }
            | GameCommand::Persist(_) => {}
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
                country.sim_params.monetary.target_inflation_rate =
                    policy.target_inflation_rate.max(0.0);
                country.sim_params.monetary.neutral_policy_rate =
                    policy.neutral_policy_rate.clamp(0.0, 1.0);
                country.sim_params.monetary.inflation_coefficient =
                    policy.inflation_coefficient.max(0.0);
                country.sim_params.monetary.unemployment_coefficient =
                    policy.unemployment_coefficient.max(0.0);
                country.sim_params.monetary.natural_unemployment_rate =
                    policy.natural_unemployment_rate.clamp(0.0, 1.0);
                country.sim_params.monetary.rate_floor = policy.rate_floor.clamp(0.0, 1.0);
                country.sim_params.monetary.rate_ceiling = policy
                    .rate_ceiling
                    .max(country.sim_params.monetary.rate_floor);
                country.sim_params.monetary.rate_change_event_threshold =
                    policy.rate_change_event_threshold.max(0.0);
                country.sim_params.monetary.policy_rate_step = policy.policy_rate_step.max(0.0);
                country.sim_params.monetary.mint_inflation_gap_weight =
                    policy.mint_inflation_gap_weight.max(0.0);
                country.sim_params.monetary.mint_unemployment_gap_weight =
                    policy.mint_unemployment_gap_weight.max(0.0);
                country
                    .sim_params
                    .monetary
                    .mint_max_fraction_expected_supply =
                    policy.mint_max_fraction_expected_supply.clamp(0.0, 1.0);
            }
            GameCommand::ApplyDisasters {
                country_idx,
                requests,
            } => {
                apply_disaster_requests(world, country_idx, &requests);
            }
            // Persistence commands are handled by the worker, not here.
            // Setup commands are no-ops in running phase.
            GameCommand::SetupEditChar(_)
            | GameCommand::SetupBackspace
            | GameCommand::SetupNextField
            | GameCommand::SetupPrevField
            | GameCommand::SetupSubmit
            | GameCommand::Persist(_) => {}
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
#[path = "engine_tests.rs"]
mod tests;
