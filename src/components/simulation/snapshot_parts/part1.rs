use crate::{
    components::simulation::ui_state::{
        ChartScope, DetailsView, EventsSort, RunningTab, SelectionFocus, UiState,
    },
    sim::{
        GameEngine, GamePhase,
        good::{common_goods, good_by_id},
        job::job_by_id,
        names::person_name_from_seed,
        runtime::SimulationMode,
        setup::SetupField,
        stock_market::StockRowSnapshot,
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
    pub avg_price: String,
    pub inventory: String,
    pub produced: String,
    pub sold: String,
    pub total_sold: String,
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
    pub company_money_history: Vec<f64>,
    pub country_goods_summary_lines: Vec<String>,
    pub chart_scope_label: String,
    pub chart_history: Vec<MetricPoint>,
    pub sim_mode: SimulationMode,
    pub ticks_per_second: f64,
    pub avg_tick_duration_ms: f64,
    pub queue_depth: usize,
    pub tick_queue_lag_ms: f64,
    pub snapshot_publish_count: u64,
    pub avg_snapshot_build_ms: f64,
    pub max_snapshot_age_ms: f64,
    pub invariant_warnings: Vec<String>,
    pub event_log_lines: Vec<String>,
    pub events_total_count: usize,
    pub events_scroll: usize,
    pub events_sort_label: &'static str,
    pub events_search_active: bool,
    pub events_search_query: String,
    // Stocks tab
    pub stock_rows: Vec<StockRowSnapshot>,
    pub stock_summary_lines: Vec<String>,
    pub stock_selected_index_price_history: Vec<f64>,
    pub stock_selected_index_volume_history: Vec<f64>,
    pub stock_country_index_history: Vec<f64>,
    pub stock_bank_exposure_history: Vec<f64>,
    pub stock_page: usize,
    pub stock_selected_row: usize,
    pub stock_person_details_lines: Vec<String>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct RuntimeStatsSnapshot {
    pub mode: SimulationMode,
    pub ticks_per_second: f64,
    pub avg_tick_duration_ms: f64,
    pub queue_depth: usize,
    pub tick_queue_lag_ms: f64,
    pub snapshot_publish_count: u64,
    pub avg_snapshot_build_ms: f64,
    pub max_snapshot_age_ms: f64,
}

impl RuntimeStatsSnapshot {
    pub(super) fn new(
        mode: SimulationMode,
        ticks_per_second: f64,
        avg_tick_duration_ms: f64,
        queue_depth: usize,
        tick_queue_lag_ms: f64,
        snapshot_publish_count: u64,
        avg_snapshot_build_ms: f64,
        max_snapshot_age_ms: f64,
    ) -> Self {
        Self {
            mode,
            ticks_per_second,
            avg_tick_duration_ms,
            queue_depth,
            tick_queue_lag_ms,
            snapshot_publish_count,
            avg_snapshot_build_ms,
            max_snapshot_age_ms,
        }
    }
}
