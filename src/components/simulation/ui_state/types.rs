pub const PEOPLE_PAGE_SIZE: usize = 50;
pub const COUNTRY_PAGE_SIZE: usize = 10;
pub const COMPANY_PAGE_SIZE: usize = 12;
pub const CHART_PAGE_COUNT: usize = 3;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DetailsView {
    List,
    Person,
    Country,
    Company,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectionFocus {
    Country,
    People,
    Company,
    Stock,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RunningTab {
    Main,
    Goods,
    Stocks,
    Charts,
    Events,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventsSort {
    NewestFirst,
    OldestFirst,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChartScope {
    SelectedCountry,
    WorldAggregate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub details_view: DetailsView,
    pub selection_focus: SelectionFocus,
    pub active_tab: RunningTab,
    pub chart_scope: ChartScope,
    pub chart_page: usize,
    pub details_page: usize,
    pub selected_country: usize,
    pub people_page: usize,
    pub selected_person_row: usize,
    pub company_page: usize,
    pub selected_company_row: usize,
    pub events_scroll: usize,
    pub events_sort: EventsSort,
    pub events_search_active: bool,
    pub events_search_query: String,
    pub stock_selected_row: usize,
    pub stock_page: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            details_view: DetailsView::List,
            selection_focus: SelectionFocus::Country,
            active_tab: RunningTab::Main,
            chart_scope: ChartScope::SelectedCountry,
            chart_page: 0,
            details_page: 0,
            selected_country: 0,
            people_page: 0,
            selected_person_row: 0,
            company_page: 0,
            selected_company_row: 0,
            events_scroll: 0,
            events_sort: EventsSort::NewestFirst,
            events_search_active: false,
            events_search_query: String::new(),
            stock_selected_row: 0,
            stock_page: 0,
        }
    }
}
