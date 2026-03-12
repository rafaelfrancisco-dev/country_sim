use std::{ops::Deref, sync::Arc};

use crate::{
    components::simulation::ui_state::{DetailsView, GoodsMainPage, RunningTab, SelectionFocus},
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
    pub save_name_input: String,
    pub save_list: Vec<crate::sim::persistence::SaveDescriptor>,
    pub selected_save_row: usize,
    pub persistence_status: Option<String>,
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
    pub profit: String,
    pub is_profitable: bool,
    pub is_selected: bool,
    pub is_even: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct CompanyWorkerRowSnapshot {
    pub name: String,
    pub job: String,
    pub tenure_months: String,
    pub salary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct GoodsProductionDemandRowSnapshot {
    pub good: String,
    pub producer_count: String,
    pub avg_price: String,
    pub inventory_units: String,
    pub produced_units: String,
    pub sold_units: String,
    pub export_units: String,
    pub est_daily_demand_units: String,
    pub est_last_month_demand_units: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct BankRowSnapshot {
    pub bank_id: String,
    pub name: String,
    pub kind: String,
    pub reserve: String,
    pub deposits: String,
    pub created_money: String,
    pub health: String,
    pub run_intensity: String,
    pub resolution: String,
    pub is_selected: bool,
    pub is_even: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct BankDepositorRowSnapshot {
    pub entity_kind: String,
    pub entity: String,
    pub balance: String,
    pub share: String,
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
    pub currency_symbol: String,
    pub selected_country_month: u64,
    pub paused: bool,
    pub active_tab: RunningTab,
    pub goods_main_page: GoodsMainPage,
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
    pub bank_page: usize,
    pub total_bank_pages: usize,
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
    pub company_workers_scroll: usize,
    pub bank_depositors_scroll: usize,
    pub persistence_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) enum RunningTabSnapshot {
    Main(Box<MainTabSnapshot>),
    Goods(Box<GoodsTabSnapshot>),
    Banks(Box<BanksTabSnapshot>),
    Stocks(Box<StocksTabSnapshot>),
    Fx(Box<FxTabSnapshot>),
    Disasters(Box<DisastersTabSnapshot>),
    Charts(Box<ChartsTabSnapshot>),
    Events(Box<EventsTabSnapshot>),
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct MainTabSnapshot {
    pub country_rows: Vec<CountryRowSnapshot>,
    pub people_rows: Vec<PersonRowSnapshot>,
    pub has_selected_country: bool,
    pub person_details_lines: Vec<String>,
    pub person_related_events: EventsTabSnapshot,
    pub country_details_lines: Vec<String>,
    pub country_policy_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct GoodsTabSnapshot {
    pub company_rows: Vec<CompanyRowSnapshot>,
    pub company_details_lines: Vec<String>,
    pub company_worker_rows: Vec<CompanyWorkerRowSnapshot>,
    pub company_related_events: EventsTabSnapshot,
    pub company_sold_history: Arc<[f64]>,
    pub company_employee_history: Arc<[f64]>,
    pub company_price_history: Arc<[f64]>,
    pub company_loan_payment_history: Arc<[f64]>,
    pub company_money_history: Arc<[f64]>,
    pub disaster_days: Arc<[u64]>,
    pub country_goods_summary_lines: Vec<String>,
    pub production_demand_summary_lines: Vec<String>,
    pub production_demand_rows: Vec<GoodsProductionDemandRowSnapshot>,
    pub goods_production_page: usize,
    pub total_goods_production_pages: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct BanksTabSnapshot {
    pub bank_rows: Vec<BankRowSnapshot>,
    pub bank_details_lines: Vec<String>,
    pub bank_related_events: EventsTabSnapshot,
    pub bank_depositor_rows: Vec<BankDepositorRowSnapshot>,
    pub bank_reserve_history: Arc<[f64]>,
    pub bank_deposit_history: Arc<[f64]>,
    pub bank_created_money_history: Arc<[f64]>,
    pub bank_equity_history: Arc<[f64]>,
    pub bank_run_intensity_history: Arc<[f64]>,
    pub disaster_days: Arc<[u64]>,
    pub country_banks_summary_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct StocksTabSnapshot {
    pub stock_rows: Vec<StockRowSnapshot>,
    pub stock_summary_lines: Vec<String>,
    pub stock_selected_index_price_history: Arc<[f64]>,
    pub stock_selected_index_volume_history: Arc<[f64]>,
    pub stock_country_index_history: Arc<[f64]>,
    pub stock_bank_exposure_history: Arc<[f64]>,
    pub disaster_days: Arc<[u64]>,
    pub stock_page: usize,
    pub stock_selected_row: usize,
    pub stock_person_details_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct FxRowSnapshot {
    pub country: String,
    pub currency_code: String,
    /// Display string for the cross-rate vs the selected country ("—" for the selected row).
    pub current_fx_rate: String,
    /// Raw fx_rate_local_per_base used for sorting.
    pub fx_rate_raw: f64,
    pub month_move_pct: String,
    pub reserves: String,
    pub net_flow: String,
    pub pressure: String,
    pub export_import_ratio: String,
    pub is_selected: bool,
    pub is_issuer: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct FxReserveEntry {
    pub currency_symbol: String,
    pub currency_code: String,
    pub amount_local: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct FxTabSnapshot {
    pub selected_country_name: String,
    pub selected_currency_code: String,
    pub selected_currency_name: String,
    pub selected_currency_symbol: String,
    pub selected_fx_summary_lines: Vec<String>,
    pub pair_board_lines: Vec<String>,
    pub fx_rows: Vec<FxRowSnapshot>,
    /// Current reserve breakdown aggregated by currency for UI display.
    pub selected_reserves_breakdown: Vec<FxReserveEntry>,
    pub selected_fx_rate_history: Arc<[f64]>,
    /// Reserve history aggregated by currency: (currency_code, local-currency time series).
    /// Sorted by currency index so colors are stable across renders.
    pub selected_fx_reserves_by_currency: Vec<(String, Arc<[f64]>)>,
    pub selected_fx_net_flow_history: Arc<[f64]>,
    pub selected_fx_pressure_history: Arc<[f64]>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct DisasterRowSnapshot {
    pub label: String,
    pub enabled: bool,
    pub selected_intensity_idx: usize,
    pub intensity_options: [u8; crate::components::simulation::ui_state::DISASTER_INTENSITY_COUNT],
    pub is_selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct DisastersTabSnapshot {
    pub selected_country_name: String,
    pub population: String,
    pub companies: String,
    pub rows: Vec<DisasterRowSnapshot>,
    pub last_apply_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct ChartsTabSnapshot {
    pub chart_scope_label: String,
    /// Currency symbol for monetary chart axes. "~" for world-aggregate scope.
    pub chart_currency_symbol: Arc<str>,
    pub chart_page: usize,
    pub total_chart_pages: usize,
    pub chart_history: Arc<[MetricPoint]>,
    pub disaster_days: Arc<[u64]>,
    pub pie_page: PieChartsPageSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct PieSliceData {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct PieChartData {
    pub title: String,
    pub slices: Vec<PieSliceData>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::components::simulation) struct PieChartsPageSnapshot {
    pub population_by_country: PieChartData,
    pub money_supply_by_country: PieChartData,
    pub stock_cap_by_country: PieChartData,
    pub treasury_by_country: PieChartData,
    pub education_tracks: PieChartData,
    pub employment_status: PieChartData,
    pub age_brackets: PieChartData,
    pub wealth_breakdown: PieChartData,
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
