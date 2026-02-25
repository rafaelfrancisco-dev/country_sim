use crate::{
    components::simulation::ui_state::{
        ChartScope, DetailsView, RunningTab, SelectionFocus, UiState,
    },
    sim::{
        GameEngine, GamePhase,
        good::{common_goods, good_by_id},
        job::job_by_id,
        names::person_name_from_seed,
        runtime::SimulationMode,
        setup::SetupField,
        world::{MetricPoint, World},
    },
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
    pub generation_seed_input: String,
    pub active_field: SetupField,
    pub validation_error: Option<String>,
    pub memory_warning: Option<String>,
    pub is_generating: bool,
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
pub(super) struct CompanyRowSnapshot {
    pub company_id: String,
    pub name: String,
    pub good: String,
    pub price: String,
    pub inventory: String,
    pub produced: String,
    pub sold: String,
    pub export_sold: String,
    pub domestic_export_split: String,
    pub sales: String,
    pub is_selected: bool,
    pub is_even: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RunningSnapshot {
    pub day: u64,
    pub generation_seed: u64,
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
    pub company_page: usize,
    pub total_company_pages: usize,
    pub country_rows: Vec<CountryRowSnapshot>,
    pub people_rows: Vec<PersonRowSnapshot>,
    pub company_rows: Vec<CompanyRowSnapshot>,
    pub has_selected_country: bool,
    pub person_details_lines: Vec<String>,
    pub country_details_lines: Vec<String>,
    pub company_details_lines: Vec<String>,
    pub company_sold_history: Vec<f64>,
    pub company_employee_history: Vec<f64>,
    pub company_price_history: Vec<f64>,
    pub company_loan_payment_history: Vec<f64>,
    pub country_goods_summary_lines: Vec<String>,
    pub chart_scope_label: String,
    pub chart_history: Vec<MetricPoint>,
    pub sim_mode: SimulationMode,
    pub ticks_per_second: f64,
    pub avg_tick_duration_ms: f64,
    pub queue_depth: usize,
    pub tick_queue_lag_ms: f64,
    pub invariant_warnings: Vec<String>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct RuntimeStatsSnapshot {
    pub mode: SimulationMode,
    pub ticks_per_second: f64,
    pub avg_tick_duration_ms: f64,
    pub queue_depth: usize,
    pub tick_queue_lag_ms: f64,
}

impl RuntimeStatsSnapshot {
    pub(super) fn new(
        mode: SimulationMode,
        ticks_per_second: f64,
        avg_tick_duration_ms: f64,
        queue_depth: usize,
        tick_queue_lag_ms: f64,
    ) -> Self {
        Self {
            mode,
            ticks_per_second,
            avg_tick_duration_ms,
            queue_depth,
            tick_queue_lag_ms,
        }
    }
}

impl UiSnapshot {
    pub(super) fn from_engine_and_ui(
        engine: &GameEngine,
        ui_state: &UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> Self {
        Self::from_phase_and_ui(engine.phase(), ui_state, runtime_stats)
    }

    pub(super) fn from_phase_and_ui(
        phase: &GamePhase,
        ui_state: &UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> Self {
        let phase = match phase {
            GamePhase::Setup(setup) => {
                SnapshotPhase::Setup(SetupSnapshot::from_setup_form(setup, false))
            }
            GamePhase::Running(world) => {
                SnapshotPhase::Running(Self::running_from_world(world, ui_state, runtime_stats))
            }
        };
        Self { phase }
    }

    fn running_from_world(
        world: &World,
        ui_state: &UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> RunningSnapshot {
        let mut ui = *ui_state;
        ui.ensure_valid(world);

        let selected_country = ui.selected_country(world);
        let selected_country_name = selected_country
            .map(|country| country.name.clone())
            .unwrap_or_else(|| "N/A".to_string());
        let selected_country_month = selected_country.map_or(0, |country| country.month);

        let country_page_start =
            ui.country_page() * crate::components::simulation::ui_state::COUNTRY_PAGE_SIZE;
        let country_rows = ui
            .selected_countries_page(world)
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
                    is_selected: idx == ui.selected_country,
                    is_even: idx.is_multiple_of(2),
                }
            })
            .collect::<Vec<_>>();

        let mut people_rows = Vec::new();
        if let Some(country) = selected_country {
            for (row_idx, person) in ui
                .selected_people_page(world)
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
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
                    is_selected: row_idx == ui.selected_person_row,
                    is_even: row_idx.is_multiple_of(2),
                });
            }
        }

        let company_page_start =
            ui.company_page * crate::components::simulation::ui_state::COMPANY_PAGE_SIZE;
        let mut company_rows = Vec::new();
        if let Some(_country) = selected_country {
            for (row_idx, company) in ui
                .selected_companies_page(world)
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
                let absolute_idx = company_page_start + row_idx;
                let good_name = good_by_id(company.produced_good_id)
                    .map_or("Unknown", |good| good.name)
                    .to_string();

                company_rows.push(CompanyRowSnapshot {
                    company_id: company.id.to_string(),
                    name: company.name.clone(),
                    good: good_name,
                    price: money(company.unit_price),
                    inventory: company.inventory_units.to_string(),
                    produced: company.last_month_units_produced.to_string(),
                    sold: company.last_month_units_sold.to_string(),
                    export_sold: company.last_month_export_units_sold.to_string(),
                    domestic_export_split: format!(
                        "{}/{}",
                        company.last_month_domestic_units_sold,
                        company.last_month_export_units_sold
                    ),
                    sales: money(company.last_month_sales),
                    is_selected: absolute_idx == ui.selected_company_absolute_index(),
                    is_even: row_idx.is_multiple_of(2),
                });
            }
        }

        let person_details_lines = person_details_lines(world, &ui);
        let country_details_lines = country_details_lines(world, &ui);
        let company_details_lines = company_details_lines(world, &ui);
        let (
            company_sold_history,
            company_employee_history,
            company_price_history,
            company_loan_payment_history,
        ) =
            selected_company_histories(world, &ui);
        let country_goods_summary_lines = country_goods_summary_lines(world, &ui);
        let chart_scope_label = chart_scope_label(world, &ui);
        let chart_history = match ui.chart_scope {
            ChartScope::SelectedCountry => world
                .country_history
                .get(ui.selected_country)
                .cloned()
                .unwrap_or_default(),
            ChartScope::WorldAggregate => world.world_history.clone(),
        };

        let invariant_warnings = world
            .validate_invariants()
            .into_iter()
            .map(|v| format!("{}: {}", v.code, v.message))
            .collect();

        RunningSnapshot {
            day: world.day,
            generation_seed: world.generation_seed,
            selected_country_month,
            paused: world.paused,
            active_tab: ui.active_tab,
            details_view: ui.details_view,
            selection_focus: ui.selection_focus,
            selected_country_name,
            total_population: world.total_population().to_string(),
            country_page: ui.country_page(),
            total_country_pages: ui.total_country_pages(world),
            people_page: ui.people_page,
            total_people_pages: ui.total_people_pages(world),
            company_page: ui.company_page,
            total_company_pages: ui.total_company_pages(world),
            country_rows,
            people_rows,
            company_rows,
            has_selected_country: selected_country.is_some(),
            person_details_lines,
            country_details_lines,
            company_details_lines,
            company_sold_history,
            company_employee_history,
            company_price_history,
            company_loan_payment_history,
            country_goods_summary_lines,
            chart_scope_label,
            chart_history,
            sim_mode: runtime_stats.mode,
            ticks_per_second: runtime_stats.ticks_per_second,
            avg_tick_duration_ms: runtime_stats.avg_tick_duration_ms,
            queue_depth: runtime_stats.queue_depth,
            tick_queue_lag_ms: runtime_stats.tick_queue_lag_ms,
            invariant_warnings,
        }
    }
}

impl SetupSnapshot {
    fn from_setup_form(setup: &crate::sim::setup::SetupForm, is_generating: bool) -> Self {
        Self {
            country_count_input: setup.country_count_input.clone(),
            millions_per_country_input: setup.millions_per_country_input.clone(),
            generation_seed_input: setup.generation_seed_input.clone(),
            active_field: setup.active_field,
            validation_error: setup.validation_error.clone(),
            memory_warning: setup.memory_warning.clone(),
            is_generating,
        }
    }
}

fn person_details_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let Some(person) = ui.selected_person_on_page(world) else {
        return vec![
            "No person available for the current selection. Press [Esc] to return.".to_string(),
        ];
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
        format!("MTD Gross Pay: {}", money(person.last_month_gross_pay)),
        format!("MTD Tax Paid: {}", money(person.last_month_tax_paid)),
        format!("MTD Net Pay: {}", money(person.last_month_net_pay)),
        format!("MTD Spending: {}", money(person.last_month_spending)),
        format!(
            "MTD Loan Payment: {}",
            money(person.last_month_loan_payment)
        ),
        format!("MTD New Loan: {}", money(person.last_month_new_loan)),
        String::new(),
        format!(
            "MTD Goods Spending: {}",
            money(person.last_month_goods_spending)
        ),
        format!(
            "MTD Imported Goods Spending: {}",
            money(person.last_month_imported_goods_spending)
        ),
        format!(
            "MTD Goods Units Bought: {}",
            person.last_month_goods_units_bought
        ),
        format!(
            "Needs Met: {}",
            goods_mask_names(person.last_month_needs_met_mask)
        ),
        format!(
            "Needs Unmet: {}",
            goods_mask_names(person.last_month_unmet_needs_mask)
        ),
        "Press [c] to open employer company details in Goods tab.".to_string(),
    ]
}

fn country_details_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
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
        format!(
            "MTD Tax Collected: {}",
            money(country.monthly_tax_collected)
        ),
        format!(
            "MTD Household Spending: {}",
            money(country.monthly_household_spending)
        ),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected)
        ),
        format!(
            "People Page: {}/{}",
            ui.people_page + 1,
            ui.total_people_pages(world)
        ),
    ]
}

fn company_details_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let Some(company) = ui.selected_company_on_page(world) else {
        return vec!["No company available for the current selection.".to_string()];
    };

    let good_label = good_by_id(company.produced_good_id)
        .map_or("Unknown", |good| good.name)
        .to_string();

    vec![
        format!("Country: {}", country.name),
        format!("Company: {}", company.name),
        format!("Company ID: {}", company.id),
        format!("Produced Good: {}", good_label),
        String::new(),
        format!("Employees: {}", company.employee_count),
        format!(
            "Manufacturing Employees: {}",
            company.manufacturing_employee_count
        ),
        format!("Open Positions: {}", company.open_positions.len()),
        format!("Unit Price: {}", money(company.unit_price)),
        format!("Inventory Units: {}", company.inventory_units),
        format!("MTD Produced Units: {}", company.last_month_units_produced),
        format!("MTD Sold Units: {}", company.last_month_units_sold),
        format!(
            "MTD Domestic/Export Units: {}/{}",
            company.last_month_domestic_units_sold, company.last_month_export_units_sold
        ),
        format!(
            "MTD Sell Through: {:.2}%",
            company.last_month_sell_through * 100.0
        ),
        String::new(),
        format!("MTD Sales: {}", money(company.last_month_sales)),
        format!(
            "MTD Payroll Paid: {}",
            money(company.last_month_payroll_paid)
        ),
        format!("MTD Profit: {}", money(company.last_month_profit)),
        format!(
            "MTD Loan Payment: {}",
            money(company.last_month_loan_payment)
        ),
        format!("MTD New Loan: {}", money(company.last_month_new_loan)),
        format!("Cash Balance: {}", money(company.cash_balance)),
        format!(
            "Loan Debt: {}",
            money(company.loan.as_ref().map_or(0.0, |loan| loan.debt_balance))
        ),
    ]
}

fn selected_company_histories(
    world: &World,
    ui: &UiState,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let Some(company) = ui.selected_company_on_page(world) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };

    (
        company
            .sold_units_history
            .iter()
            .map(|units| f64::from(*units))
            .collect(),
        company
            .employee_count_history
            .iter()
            .map(|count| f64::from(*count))
            .collect(),
        company.price_history.clone(),
        company.loan_payment_history.clone(),
    )
}

fn country_goods_summary_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    vec![
        format!("Country: {} (Month {})", country.name, country.month),
        format!(
            "MTD Household Goods Spending: {}",
            money(country.monthly_household_spending)
        ),
        format!(
            "MTD Imported Goods Spending: {}",
            money(country.monthly_goods_import_spending)
        ),
        format!(
            "MTD Goods Export Revenue: {}",
            money(country.monthly_goods_export_revenue)
        ),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected)
        ),
        format!(
            "MTD Essential Debt Spending: {}",
            money(country.monthly_essential_debt_spending)
        ),
        format!(
            "MTD Unmet Need Events: {}",
            country.monthly_unmet_need_events
        ),
    ]
}

fn chart_scope_label(world: &World, ui: &UiState) -> String {
    match ui.chart_scope {
        ChartScope::SelectedCountry => {
            let country_name = ui
                .selected_country(world)
                .map(|country| country.name.as_str())
                .unwrap_or("N/A");
            format!("Selected: {country_name}")
        }
        ChartScope::WorldAggregate => "World Aggregate".to_string(),
    }
}

fn goods_mask_names(mask: u16) -> String {
    let names = common_goods()
        .iter()
        .filter(|good| (mask & (1_u16 << good.id)) != 0)
        .map(|good| good.name)
        .collect::<Vec<_>>();

    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

fn money(amount: f64) -> String {
    format!("{amount:.2}")
}
