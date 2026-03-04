pub const PEOPLE_PAGE_SIZE: usize = 50;
pub const COUNTRY_PAGE_SIZE: usize = 10;
pub const COMPANY_PAGE_SIZE: usize = 12;
pub const CHART_PAGE_COUNT: usize = 3;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CountryPolicyField {
    TaxBracket1Max,
    TaxBracket2Max,
    TaxRate1,
    TaxRate2,
    TaxRate3,
    UnemploymentBenefitRate,
    MedianIncomeReference,
    BasicNeedsCashThreshold,
}

impl CountryPolicyField {
    pub fn next(self) -> Self {
        match self {
            Self::TaxBracket1Max => Self::TaxBracket2Max,
            Self::TaxBracket2Max => Self::TaxRate1,
            Self::TaxRate1 => Self::TaxRate2,
            Self::TaxRate2 => Self::TaxRate3,
            Self::TaxRate3 => Self::UnemploymentBenefitRate,
            Self::UnemploymentBenefitRate => Self::MedianIncomeReference,
            Self::MedianIncomeReference => Self::BasicNeedsCashThreshold,
            Self::BasicNeedsCashThreshold => Self::TaxBracket1Max,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::TaxBracket1Max => Self::BasicNeedsCashThreshold,
            Self::TaxBracket2Max => Self::TaxBracket1Max,
            Self::TaxRate1 => Self::TaxBracket2Max,
            Self::TaxRate2 => Self::TaxRate1,
            Self::TaxRate3 => Self::TaxRate2,
            Self::UnemploymentBenefitRate => Self::TaxRate3,
            Self::MedianIncomeReference => Self::UnemploymentBenefitRate,
            Self::BasicNeedsCashThreshold => Self::MedianIncomeReference,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CountryPolicyFormState {
    pub active_field: CountryPolicyField,
    pub is_editing: bool,
    pub draft_input: String,
    pub validation_error: Option<String>,
}

impl Default for CountryPolicyFormState {
    fn default() -> Self {
        Self {
            active_field: CountryPolicyField::TaxBracket1Max,
            is_editing: false,
            draft_input: String::new(),
            validation_error: None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DetailsView {
    List,
    Person,
    Country,
    CountryPolicy,
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
pub enum EventsCategoryFilter {
    All,
    StockMarket,
    BirthsDeaths,
    BankruptciesDistress,
    LoansDebt,
    CompaniesLabor,
    GovernmentPolicy,
}

impl EventsCategoryFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::StockMarket => "Stock Market",
            Self::BirthsDeaths => "Births/Deaths",
            Self::BankruptciesDistress => "Bankruptcies & Distress",
            Self::LoansDebt => "Loans & Debt",
            Self::CompaniesLabor => "Companies & Labor",
            Self::GovernmentPolicy => "Government & Policy",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::StockMarket,
            Self::StockMarket => Self::BirthsDeaths,
            Self::BirthsDeaths => Self::BankruptciesDistress,
            Self::BankruptciesDistress => Self::LoansDebt,
            Self::LoansDebt => Self::CompaniesLabor,
            Self::CompaniesLabor => Self::GovernmentPolicy,
            Self::GovernmentPolicy => Self::All,
        }
    }
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
    pub events_category: EventsCategoryFilter,
    pub events_search_active: bool,
    pub events_search_query: String,
    pub stock_selected_row: usize,
    pub stock_page: usize,
    pub country_policy_form: CountryPolicyFormState,
    pub currency_symbol: String,
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
            events_category: EventsCategoryFilter::All,
            events_search_active: false,
            events_search_query: String::new(),
            stock_selected_row: 0,
            stock_page: 0,
            country_policy_form: CountryPolicyFormState::default(),
            currency_symbol: crate::sim::setup::DEFAULT_CURRENCY_SYMBOL.to_string(),
        }
    }
}
