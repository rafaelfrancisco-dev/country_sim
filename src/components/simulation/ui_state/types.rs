pub const PEOPLE_PAGE_SIZE: usize = 50;
pub const COUNTRY_PAGE_SIZE: usize = 10;
pub const COMPANY_PAGE_SIZE: usize = 12;
pub const BANK_PAGE_SIZE: usize = 12;
pub const GOODS_PRODUCTION_PAGE_SIZE: usize = 8;
pub const CHART_PAGE_COUNT: usize = 8;
pub const DISASTER_INTENSITY_COUNT: usize = crate::sim::DISASTER_INTENSITY_LEVELS.len();
pub const DISASTER_KIND_COUNT: usize = crate::sim::DISASTER_KIND_COUNT;

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
    TargetInflationRate,
    NeutralPolicyRate,
    InflationCoefficient,
    UnemploymentCoefficient,
    NaturalUnemploymentRate,
    RateFloor,
    RateCeiling,
    RateChangeEventThreshold,
    PolicyRateStep,
    MintInflationGapWeight,
    MintUnemploymentGapWeight,
    MintMaxFractionExpectedSupply,
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
            Self::BasicNeedsCashThreshold => Self::TargetInflationRate,
            Self::TargetInflationRate => Self::NeutralPolicyRate,
            Self::NeutralPolicyRate => Self::InflationCoefficient,
            Self::InflationCoefficient => Self::UnemploymentCoefficient,
            Self::UnemploymentCoefficient => Self::NaturalUnemploymentRate,
            Self::NaturalUnemploymentRate => Self::RateFloor,
            Self::RateFloor => Self::RateCeiling,
            Self::RateCeiling => Self::RateChangeEventThreshold,
            Self::RateChangeEventThreshold => Self::PolicyRateStep,
            Self::PolicyRateStep => Self::MintInflationGapWeight,
            Self::MintInflationGapWeight => Self::MintUnemploymentGapWeight,
            Self::MintUnemploymentGapWeight => Self::MintMaxFractionExpectedSupply,
            Self::MintMaxFractionExpectedSupply => Self::TaxBracket1Max,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::TaxBracket1Max => Self::MintMaxFractionExpectedSupply,
            Self::TaxBracket2Max => Self::TaxBracket1Max,
            Self::TaxRate1 => Self::TaxBracket2Max,
            Self::TaxRate2 => Self::TaxRate1,
            Self::TaxRate3 => Self::TaxRate2,
            Self::UnemploymentBenefitRate => Self::TaxRate3,
            Self::MedianIncomeReference => Self::UnemploymentBenefitRate,
            Self::BasicNeedsCashThreshold => Self::MedianIncomeReference,
            Self::TargetInflationRate => Self::BasicNeedsCashThreshold,
            Self::NeutralPolicyRate => Self::TargetInflationRate,
            Self::InflationCoefficient => Self::NeutralPolicyRate,
            Self::UnemploymentCoefficient => Self::InflationCoefficient,
            Self::NaturalUnemploymentRate => Self::UnemploymentCoefficient,
            Self::RateFloor => Self::NaturalUnemploymentRate,
            Self::RateCeiling => Self::RateFloor,
            Self::RateChangeEventThreshold => Self::RateCeiling,
            Self::PolicyRateStep => Self::RateChangeEventThreshold,
            Self::MintInflationGapWeight => Self::PolicyRateStep,
            Self::MintUnemploymentGapWeight => Self::MintInflationGapWeight,
            Self::MintMaxFractionExpectedSupply => Self::MintUnemploymentGapWeight,
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
    Bank,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectionFocus {
    Country,
    People,
    Company,
    Bank,
    Stock,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RunningTab {
    Main,
    Goods,
    Banks,
    Stocks,
    Disasters,
    Charts,
    Events,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GoodsMainPage {
    Companies,
    ProductionDemand,
}

impl GoodsMainPage {
    pub fn toggle(self) -> Self {
        match self {
            Self::Companies => Self::ProductionDemand,
            Self::ProductionDemand => Self::Companies,
        }
    }
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
    pub goods_main_page: GoodsMainPage,
    pub chart_scope: ChartScope,
    pub chart_page: usize,
    pub details_page: usize,
    pub selected_country: usize,
    pub people_page: usize,
    pub selected_person_row: usize,
    pub company_page: usize,
    pub selected_company_row: usize,
    pub goods_production_page: usize,
    pub bank_page: usize,
    pub selected_bank_row: usize,
    pub events_scroll: usize,
    pub company_workers_scroll: usize,
    pub bank_depositors_scroll: usize,
    pub events_sort: EventsSort,
    pub events_category: EventsCategoryFilter,
    pub events_search_active: bool,
    pub events_search_query: String,
    pub stock_selected_row: usize,
    pub stock_page: usize,
    pub disaster_selected_row: usize,
    pub disaster_enabled: [bool; DISASTER_KIND_COUNT],
    pub disaster_intensity_index: [usize; DISASTER_KIND_COUNT],
    pub disaster_last_apply_status: Option<String>,
    pub country_policy_form: CountryPolicyFormState,
    pub currency_symbol: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            details_view: DetailsView::List,
            selection_focus: SelectionFocus::Country,
            active_tab: RunningTab::Main,
            goods_main_page: GoodsMainPage::Companies,
            chart_scope: ChartScope::SelectedCountry,
            chart_page: 0,
            details_page: 0,
            selected_country: 0,
            people_page: 0,
            selected_person_row: 0,
            company_page: 0,
            selected_company_row: 0,
            goods_production_page: 0,
            bank_page: 0,
            selected_bank_row: 0,
            events_scroll: 0,
            company_workers_scroll: 0,
            bank_depositors_scroll: 0,
            events_sort: EventsSort::NewestFirst,
            events_category: EventsCategoryFilter::All,
            events_search_active: false,
            events_search_query: String::new(),
            stock_selected_row: 0,
            stock_page: 0,
            disaster_selected_row: 0,
            disaster_enabled: [false; DISASTER_KIND_COUNT],
            disaster_intensity_index: [0; DISASTER_KIND_COUNT],
            disaster_last_apply_status: None,
            country_policy_form: CountryPolicyFormState::default(),
            currency_symbol: crate::sim::setup::DEFAULT_CURRENCY_SYMBOL.to_string(),
        }
    }
}
