use std::{ops::Deref, sync::Arc};

use crate::{
    components::simulation::ui_state::{DetailsView, RunningTab, SelectionFocus},
    sim::{
        runtime::SimulationMode, setup::SetupField, stock_market::StockRowSnapshot,
        world::MetricPoint,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct UiSnapshot {
    pub phase: SnapshotPhase,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) enum SnapshotPhase {
    Setup(Box<SetupSnapshot>),
    Running(Box<RunningSnapshot>),
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct SetupSnapshot {
    pub country_count_input: String,
    pub thousands_per_country_input: String,
    pub generation_seed_input: String,
    pub active_field: SetupField,
    pub validation_error: Option<String>,
    pub memory_warning: Option<String>,
    pub is_generating: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct CountryRowSnapshot {
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
pub(in crate::components::simulation) struct PersonRowSnapshot {
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
pub(in crate::components::simulation) struct CompanyRowSnapshot {
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
pub(in crate::components::simulation) struct RunningSnapshot {
    pub core: RunningCoreSnapshot,
    pub tab: RunningTabSnapshot,
}

impl Deref for RunningSnapshot {
    type Target = RunningCoreSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct RunningCoreSnapshot {
    pub day: u64,
    pub generation_seed: u64,
    pub selected_country_month: u64,
    pub paused: bool,
    pub active_tab: RunningTab,
    pub details_view: DetailsView,
    pub details_page: usize,
    pub selection_focus: SelectionFocus,
    pub selected_country_name: String,
    pub total_population: String,
    pub country_page: usize,
    pub total_country_pages: usize,
    pub people_page: usize,
    pub total_people_pages: usize,
    pub company_page: usize,
    pub total_company_pages: usize,
    pub country_policy_is_editing: bool,
    pub sim_mode: SimulationMode,
    pub ticks_per_second: f64,
    pub avg_tick_duration_ms: f64,
    pub queue_depth: usize,
    pub tick_queue_lag_ms: f64,
    pub snapshot_publish_count: u64,
    pub avg_snapshot_build_ms: f64,
    pub max_snapshot_age_ms: f64,
    pub invariant_warnings: Vec<String>,
    pub events_search_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) enum RunningTabSnapshot {
    Main(Box<MainTabSnapshot>),
    Goods(Box<GoodsTabSnapshot>),
    Stocks(Box<StocksTabSnapshot>),
    Charts(Box<ChartsTabSnapshot>),
    Events(Box<EventsTabSnapshot>),
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct MainTabSnapshot {
    pub country_rows: Vec<CountryRowSnapshot>,
    pub people_rows: Vec<PersonRowSnapshot>,
    pub has_selected_country: bool,
    pub person_details_lines: Vec<String>,
    pub country_details_lines: Vec<String>,
    pub country_policy_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct GoodsTabSnapshot {
    pub company_rows: Vec<CompanyRowSnapshot>,
    pub company_details_lines: Vec<String>,
    pub company_sold_history: Arc<[f64]>,
    pub company_employee_history: Arc<[f64]>,
    pub company_price_history: Arc<[f64]>,
    pub company_loan_payment_history: Arc<[f64]>,
    pub company_money_history: Arc<[f64]>,
    pub country_goods_summary_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct StocksTabSnapshot {
    pub stock_rows: Vec<StockRowSnapshot>,
    pub stock_summary_lines: Vec<String>,
    pub stock_selected_index_price_history: Arc<[f64]>,
    pub stock_selected_index_volume_history: Arc<[f64]>,
    pub stock_country_index_history: Arc<[f64]>,
    pub stock_bank_exposure_history: Arc<[f64]>,
    pub stock_page: usize,
    pub stock_selected_row: usize,
    pub stock_person_details_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct ChartsTabSnapshot {
    pub chart_scope_label: String,
    pub chart_page: usize,
    pub total_chart_pages: usize,
    pub chart_history: Arc<[MetricPoint]>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct EventsTabSnapshot {
    pub event_log_lines: Vec<String>,
    pub events_total_count: usize,
    pub events_scroll: usize,
    pub events_sort_label: &'static str,
    pub events_category_label: &'static str,
    pub events_search_query: String,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(in crate::components::simulation) struct RuntimeStatsSnapshot {
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
    #[expect(
        clippy::too_many_arguments,
        reason = "constructor mirrors runtime stats payload fields"
    )]
    pub(in crate::components::simulation) fn new(
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
