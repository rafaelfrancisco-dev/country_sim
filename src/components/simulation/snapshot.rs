use crate::sim::{
    GameEngine, GamePhase,
    job::job_by_id,
    names::person_name_from_seed,
    setup::SetupField,
    world::{ChartScope, DetailsView, MetricPoint, RunningTab, SelectionFocus, World},
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct UiSnapshot {
    pub phase: SnapshotPhase,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum SnapshotPhase {
    Setup(SetupSnapshot),
    Running(RunningSnapshot),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SetupSnapshot {
    pub country_count_input: String,
    pub millions_per_country_input: String,
    pub active_field: SetupField,
    pub validation_error: Option<String>,
    pub memory_warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CountryRowSnapshot {
    pub name: String,
    pub population: String,
    pub household_cash: String,
    pub company_cash: String,
    pub treasury: String,
    pub loans: String,
    pub is_selected: bool,
    pub is_even: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PersonRowSnapshot {
    pub citizen_id: String,
    pub name: String,
    pub age: String,
    pub education: String,
    pub employment: String,
    pub employer: String,
    pub salary: String,
    pub cash: String,
    pub loan: String,
    pub is_selected: bool,
    pub is_even: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RunningSnapshot {
    pub day: u64,
    pub selected_country_month: u64,
    pub paused: bool,
    pub active_tab: RunningTab,
    pub details_view: DetailsView,
    pub selection_focus: SelectionFocus,
    pub selected_country_name: String,
    pub total_population: String,
    pub country_page: usize,
    pub total_country_pages: usize,
    pub people_page: usize,
    pub total_people_pages: usize,
    pub country_rows: Vec<CountryRowSnapshot>,
    pub people_rows: Vec<PersonRowSnapshot>,
    pub has_selected_country: bool,
    pub person_details_lines: Vec<String>,
    pub country_details_lines: Vec<String>,
    pub chart_scope_label: String,
    pub chart_history: Vec<MetricPoint>,
}

impl UiSnapshot {
    pub(super) fn from_engine(engine: &GameEngine) -> Self {
        let phase = match engine.phase() {
            GamePhase::Setup(setup) => SnapshotPhase::Setup(SetupSnapshot {
                country_count_input: setup.country_count_input.clone(),
                millions_per_country_input: setup.millions_per_country_input.clone(),
                active_field: setup.active_field,
                validation_error: setup.validation_error.clone(),
                memory_warning: setup.memory_warning.clone(),
            }),
            GamePhase::Running(world) => SnapshotPhase::Running(Self::running_from_world(world)),
        };
        Self { phase }
    }

    fn running_from_world(world: &World) -> RunningSnapshot {
        let selected_country = world.selected_country();
        let selected_country_name = selected_country
            .map(|country| country.name.clone())
            .unwrap_or_else(|| "N/A".to_string());
        let selected_country_month = selected_country.map_or(0, |country| country.month);

        let country_page_start = world.country_page() * crate::sim::world::COUNTRY_PAGE_SIZE;
        let country_rows = world
            .selected_countries_page()
            .iter()
            .enumerate()
            .map(|(page_idx, country)| {
                let idx = country_page_start + page_idx;
                CountryRowSnapshot {
                    name: country.name.clone(),
                    population: country.population().to_string(),
                    household_cash: money(country.household_cash_total),
                    company_cash: money(country.company_cash_total),
                    treasury: money(country.treasury_balance),
                    loans: money(country.bank.created_money_outstanding),
                    is_selected: idx == world.selected_country,
                    is_even: idx % 2 == 0,
                }
            })
            .collect::<Vec<_>>();

        let mut people_rows = Vec::new();
        if let Some(country) = selected_country {
            for (row_idx, person) in world.selected_people_page().unwrap_or(&[]).iter().enumerate() {
                let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
                let name = person_name_from_seed(name_seed);
                let job_name = person
                    .job_type_id()
                    .and_then(job_by_id)
                    .map_or("N/A".to_string(), |job| job.name.to_string());
                let employment = if person.is_employed() {
                    format!("Employed ({job_name})")
                } else {
                    "Unemployed".to_string()
                };
                let employer = person
                    .employer_company_id()
                    .and_then(|company_id| country.company_name(company_id))
                    .unwrap_or("N/A")
                    .to_string();

                people_rows.push(PersonRowSnapshot {
                    citizen_id: person.id.to_string(),
                    name,
                    age: person.age_years().to_string(),
                    education: person.habilitation.label().to_string(),
                    employment,
                    employer,
                    salary: money(person.monthly_salary()),
                    cash: money(person.cash_balance),
                    loan: money(person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance)),
                    is_selected: row_idx == world.selected_person_row,
                    is_even: row_idx % 2 == 0,
                });
            }
        }

        let person_details_lines = person_details_lines(world);
        let country_details_lines = country_details_lines(world);
        let chart_scope_label = chart_scope_label(world);
        let chart_history = match world.chart_scope {
            ChartScope::SelectedCountry => world
                .country_history
                .get(world.selected_country)
                .cloned()
                .unwrap_or_default(),
            ChartScope::WorldAggregate => world.world_history.clone(),
        };

        RunningSnapshot {
            day: world.day,
            selected_country_month,
            paused: world.paused,
            active_tab: world.active_tab,
            details_view: world.details_view,
            selection_focus: world.selection_focus,
            selected_country_name,
            total_population: world.total_population().to_string(),
            country_page: world.country_page(),
            total_country_pages: world.total_country_pages(),
            people_page: world.people_page,
            total_people_pages: world.total_people_pages(),
            country_rows,
            people_rows,
            has_selected_country: selected_country.is_some(),
            person_details_lines,
            country_details_lines,
            chart_scope_label,
            chart_history,
        }
    }
}

fn person_details_lines(world: &World) -> Vec<String> {
    let Some(country) = world.selected_country() else {
        return vec!["No country is currently selected.".to_string()];
    };

    let Some(person) = world.selected_person_on_page() else {
        return vec!["No person available for the current selection. Press [Esc] to return.".to_string()];
    };

    let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
    let name = person_name_from_seed(name_seed);
    let job_label = person
        .job_type_id()
        .and_then(job_by_id)
        .map_or("Unemployed".to_string(), |job| job.name.to_string());
    let employer_label = person
        .employer_company_id()
        .and_then(|company_id| country.company_name(company_id))
        .unwrap_or("N/A")
        .to_string();
    let employment_label = if person.is_employed() {
        "Employed"
    } else {
        "Unemployed"
    };

    vec![
        format!("Name: {name}"),
        format!("Citizen ID: {}", person.id),
        format!("Country: {}", country.name),
        format!("Day: {}", world.day),
        format!("Age: {}", person.age_years()),
        format!("Education Track: {}", person.education_track.label()),
        format!("Habilitation: {}", person.habilitation.label()),
        String::new(),
        format!("Employment: {employment_label}"),
        format!("Job: {job_label}"),
        format!("Employer: {employer_label}"),
        format!("Monthly Salary: {}", money(person.monthly_salary())),
        format!("Cash Balance: {}", money(person.cash_balance)),
        format!(
            "Loan Debt: {}",
            money(person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance))
        ),
        String::new(),
        format!("Last Gross Pay: {}", money(person.last_month_gross_pay)),
        format!("Last Tax Paid: {}", money(person.last_month_tax_paid)),
        format!("Last Net Pay: {}", money(person.last_month_net_pay)),
        format!("Last Spending: {}", money(person.last_month_spending)),
        format!("Last Loan Payment: {}", money(person.last_month_loan_payment)),
        format!("Last New Loan: {}", money(person.last_month_new_loan)),
    ]
}

fn country_details_lines(world: &World) -> Vec<String> {
    let Some(country) = world.selected_country() else {
        return vec!["No country is currently selected.".to_string()];
    };

    vec![
        format!("Name: {}", country.name),
        format!("Country ID: {}", country.id),
        format!("Day: {}", world.day),
        format!("Month: {}", country.month),
        String::new(),
        format!("Population: {}", country.population()),
        format!("Household Cash: {}", money(country.household_cash_total)),
        format!("Company Cash: {}", money(country.company_cash_total)),
        format!("Treasury: {}", money(country.treasury_balance)),
        format!("Bank Reserve: {}", money(country.bank.reserve_balance)),
        format!(
            "Loans Outstanding: {}",
            money(country.bank.created_money_outstanding)
        ),
        format!("Money Supply: {}", money(country.total_money_supply)),
        format!("Expected Supply: {}", money(country.expected_money_supply)),
        format!("Supply Gap: {}", money(country.money_supply_gap)),
        format!("Last Monthly Tax: {}", money(country.monthly_tax_collected)),
        format!(
            "Last Monthly Spending: {}",
            money(country.monthly_household_spending)
        ),
        format!("People Page: {}/{}", world.people_page + 1, world.total_people_pages()),
    ]
}

fn chart_scope_label(world: &World) -> String {
    match world.chart_scope {
        ChartScope::SelectedCountry => {
            let country_name = world
                .selected_country()
                .map(|country| country.name.as_str())
                .unwrap_or("N/A");
            format!("Selected: {country_name}")
        }
        ChartScope::WorldAggregate => "World Aggregate".to_string(),
    }
}

fn money(amount: f64) -> String {
    format!("{amount:.2}")
}
