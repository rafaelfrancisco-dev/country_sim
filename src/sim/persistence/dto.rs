use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::sim::params::{
    AnnualWageRaiseParams, BankingSystemParams, CompanyDistressParams, CompanyFormationParams,
    CompanyGrowthParams, DemographicParams, EducationParams, EssentialEconomyParams,
    FiscalParams, GoodsMarketParams, GovernmentBackfillParams, HousingMarketParams, LaborChurnParams,
    LaborDemandParams, LaborMobilityParams, LaborParams, MigrationParams, MonetaryPolicyParams,
    SimulationParams, StockMarketParams, TaxPolicyParams,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoanAccountDto {
    pub debt_balance: f64,
    pub created_principal_outstanding: f64,
    pub monthly_payment: f64,
    pub annual_rate: f64,
    pub term_months: u16,
    pub months_remaining: u16,
    pub lender_bank_id: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MortgageAccountDto {
    pub principal_balance: f64,
    pub original_principal: f64,
    pub monthly_payment: f64,
    pub annual_rate: f64,
    pub term_months: u16,
    pub months_remaining: u16,
    pub months_missed: u8,
    pub lender_bank_id: u16,
    pub collateral_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HousingStatusDto {
    Unhoused,
    Renter {
        landlord_company_id: u16,
        landlord_country_id: u16,
    },
    Owner,
    Mortgaged {
        mortgage: MortgageAccountDto,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EmploymentStatusDto {
    Unemployed,
    Employed {
        job_type_id: u8,
        employer_company_id: u16,
    },
    EmployedAtBank {
        job_type_id: u8,
        bank_id: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignDepositDto {
    pub country_id: u16,
    pub bank_id: u16,
    pub balance: f64,
    pub home_amount_at_open: f64,
    pub opening_fx_rate_home_per_foreign: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonDto {
    pub id: u32,
    pub birth_country_id: u16,
    pub current_country_id: u16,
    pub employment: EmploymentStatusDto,
    pub employment_start_day: Option<u64>,
    pub agreed_monthly_salary: f64,
    pub cash_balance: f64,
    pub deposit_bank_id: Option<u16>,
    pub household_id: u32,
    pub name_seed: u32,
    pub age_days: u16,
    pub education_track: u8, // 0=BasicOnly,1=HighSchool,2=Bachelor,3=Doctor
    pub habilitation: u8,    // 0=NoSchoolYet,1=BasicSchool,2=HighSchool,3=HigherLearning
    pub loan: Option<LoanAccountDto>,
    pub last_month_gross_pay: f64,
    pub last_month_tax_paid: f64,
    pub last_month_net_pay: f64,
    pub last_completed_month_gross_pay: f64,
    pub last_completed_month_tax_paid: f64,
    pub last_completed_month_net_pay: f64,
    pub last_month_spending: f64,
    pub last_month_goods_spending: f64,
    pub last_month_imported_goods_spending: f64,
    pub last_month_goods_units_bought: u32,
    pub last_month_needs_met_mask: u16,
    pub last_month_unmet_needs_mask: u16,
    pub last_month_severe_unmet_needs_mask: u16,
    pub consecutive_severe_unmet_months: u8,
    pub last_month_chronic_unmet: bool,
    pub last_month_loan_payment: f64,
    pub last_month_new_loan: f64,
    pub last_month_savings_interest: f64,
    pub last_month_rent_paid: f64,
    pub last_month_mortgage_payment: f64,
    pub housing: HousingStatusDto,
    pub months_unemployed: u32,
    pub months_on_benefits: u32,
    pub lifetime_migrations: u32,
    pub last_migration_day: Option<u64>,
    pub essential_stock_units: Vec<f32>,
    pub superficial_stock_units: Vec<u16>,
    pub superficial_stock_consumption_carry: Vec<f32>,
    pub foreign_deposit: Option<ForeignDepositDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobOpeningDto {
    pub job_type_id: u8,
    pub slots: u16,
    pub offered_monthly_salary: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FounderStakeDto {
    pub person_id: u32,
    pub shares_owned: u64,
    pub cash_contributed: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanyDto {
    pub id: u16,
    pub historical_ids: Vec<u16>,
    pub name: String,
    pub is_public_utility: bool,
    pub is_essential_backfill: bool,
    pub offered_job_types: Vec<u8>,
    pub produced_good_id: u8,
    pub open_positions: Vec<JobOpeningDto>,
    pub founder_stakes: Vec<FounderStakeDto>,
    pub employee_count: u32,
    pub manufacturing_employee_count: u32,
    pub inventory_units: u32,
    pub unit_price: f64,
    pub wage_offer_multiplier: f64,
    pub consecutive_missed_loan_payments: u8,
    pub missed_loan_payment_this_month: bool,
    pub production_effort_carry: f64,
    pub cash_balance: f64,
    pub deposit_bank_id: Option<u16>,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub last_completed_month_payroll_paid: f64,
    pub last_month_sales: f64,
    pub last_month_profit: f64,
    pub last_completed_month_profit: f64,
    pub consecutive_profitable_months: u8,
    pub consecutive_loss_months: u8,
    pub last_month_loan_payment: f64,
    pub last_completed_month_loan_payment: f64,
    pub last_month_new_loan: f64,
    pub last_completed_month_new_loan: f64,
    pub last_month_savings_interest: f64,
    pub last_completed_month_savings_interest: f64,
    pub last_month_units_produced: u32,
    pub last_month_units_sold: u32,
    pub last_month_export_units_sold: u32,
    pub last_month_domestic_units_sold: u32,
    pub last_month_sell_through: f64,
    pub sales_history: Vec<f64>,
    pub profit_history: Vec<f64>,
    pub sold_units_history: Vec<u32>,
    pub employee_count_history: Vec<u32>,
    pub price_history: Vec<f64>,
    pub loan_payment_history: Vec<f64>,
    pub cash_balance_history: Vec<f64>,
    pub age_months: u16,
    pub ipo_cooldown_months: u8,
    pub failed_ipo_attempts: u8,
    pub loan: Option<LoanAccountDto>,
    pub rental_units_owned: u32,
    pub last_month_rent_revenue: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankJobOpeningDto {
    pub job_type_id: u8,
    pub slots: u16,
    pub offered_monthly_salary: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankStockPositionDto {
    pub security_country_id: u16,
    pub security_company_id: u16,
    pub shares: u64,
    pub avg_cost: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankDto {
    pub id: u16,
    pub name: String,
    pub kind: u8, // 0=Public, 1=Private
    pub reserve_balance: f64,
    pub created_money_outstanding: f64,
    pub total_originated: f64,
    pub total_principal_repaid: f64,
    pub deposit_liabilities: f64,
    pub foreign_deposit_liabilities: f64,
    pub equity_capital: f64,
    pub run_intensity: f64,
    pub is_in_resolution: bool,
    pub months_in_resolution: u16,
    pub resolution_distress_streak: u16,
    pub resolution_stable_streak: u16,
    pub open_positions: Vec<BankJobOpeningDto>,
    pub employee_count: u32,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub wage_offer_multiplier: f64,
    pub last_month_interest_income: f64,
    pub last_month_deposit_interest_expense: f64,
    pub last_month_credit_losses: f64,
    pub last_month_profit: f64,
    pub stock_positions: Vec<BankStockPositionDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseholdDto {
    pub id: u32,
    pub member_count: u16,
    pub sheltered: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricPointDto {
    pub month: u64,
    pub fx_rate_local_per_base: f64,
    pub fx_reserves_base: f64,
    pub fx_net_flow_base: f64,
    pub fx_pressure: f64,
    pub population: f64,
    pub total_money_supply: f64,
    pub household_cash_total: f64,
    pub company_cash_total: f64,
    pub expected_money_supply: f64,
    pub money_supply_gap: f64,
    pub created_money_outstanding: f64,
    pub bank_reserve: f64,
    pub treasury_balance: f64,
    pub budget_balance: f64,
    pub monthly_tax_collected: f64,
    pub monthly_gov_spending: f64,
    pub gov_debt: f64,
    pub monthly_money_minted: f64,
    pub cumulative_money_minted: f64,
    pub monthly_mint_fraction: f64,
    pub monthly_goods_import_spending: f64,
    pub monthly_goods_export_revenue: f64,
    pub monthly_unmet_need_events: f64,
    pub monthly_severe_unmet_need_events: f64,
    pub monthly_chronic_unmet_need_count: f64,
    pub monthly_essential_debt_spending: f64,
    pub cpi_index: f64,
    pub inflation_rate: f64,
    pub policy_rate: f64,
    pub deposit_rate: f64,
    pub loan_rate: f64,
    pub monthly_savings_interest_paid: f64,
    pub bonds_outstanding: f64,
    pub stock_market_cap: f64,
    pub stock_bank_equity_exposure: f64,
    pub housing_price_index: f64,
    pub monthly_rent: f64,
    pub homeownership_rate: f64,
    pub housing_vacancy_rate: f64,
    pub monthly_foreclosures: f64,
    pub monthly_home_purchases: f64,
    pub monthly_immigrants: f64,
    pub monthly_emigrants: f64,
    pub monthly_migrants_for_work: f64,
    pub monthly_migrants_for_distress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankMetricPointDto {
    pub month: u64,
    pub reserve: f64,
    pub deposits: f64,
    pub created_money: f64,
    pub equity: f64,
    pub run_intensity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionDto {
    pub security_country_id: u16,
    pub security_company_id: u16,
    pub shares: u64,
    pub avg_cost: f64,
    pub pledged_shares: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioDto {
    pub positions: Vec<PositionDto>,
    pub realized_pnl_mtd: f64,
    pub dividends_mtd: f64,
    pub last_completed_month_dividends: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListedEquityDto {
    pub company_id: u16,
    pub current_company_id: u16,
    pub ticker: String,
    pub shares_outstanding: u64,
    pub free_float_shares: u64,
    pub last_price: f64,
    pub prev_close: f64,
    pub last_closed_day_pct_change: f64,
    pub day_volume: u64,
    pub day_turnover: f64,
    pub day_trades_count: u64,
    pub market_cap: f64,
    pub price_history: Vec<f64>,
    pub volume_history: Vec<u64>,
    pub market_cap_history: Vec<f64>,
    pub monthly_dps: Vec<f64>,
    pub dps_cursor: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyIndexPointDto {
    pub day: u64,
    pub index_value: f64,
    pub total_turnover: f64,
    pub bank_equity_exposure: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockMarketDto {
    /// key = person_id as string
    pub person_portfolios: HashMap<String, PortfolioDto>,
    pub bank_portfolio: PortfolioDto,
    /// key = company_id as string
    pub listed: HashMap<String, ListedEquityDto>,
    pub daily_index_history: Vec<DailyIndexPointDto>,
    pub index_base: f64,
    pub ipo_raised_mtd: f64,
    pub dividends_paid_mtd: f64,
    pub last_completed_month_dividends_paid: f64,
    pub forced_liquidations_mtd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimEventDto {
    pub kind: String,
    pub day: u64,
    pub entity_kind: Option<u8>,
    pub bank_kind: Option<u8>,
    pub ipo_reason: Option<u8>,
    pub country_name: Option<String>,
    pub from_country_name: Option<String>,
    pub to_country_name: Option<String>,
    pub home_country_name: Option<String>,
    pub foreign_country_name: Option<String>,
    pub currency_code: Option<String>,
    pub company_name: Option<String>,
    pub bank_name: Option<String>,
    pub acquired_by_bank_name: Option<String>,
    pub ticker: Option<String>,
    pub disaster_label: Option<String>,
    pub summary: Option<String>,
    pub count: Option<u32>,
    pub entity_id: Option<u32>,
    pub person_id: Option<u32>,
    pub company_id: Option<u16>,
    pub bank_id: Option<u16>,
    pub acquired_by_bank_id: Option<u16>,
    pub from_company_id: Option<u16>,
    pub to_company_id: Option<u16>,
    pub produced_good_id: Option<u8>,
    pub founders: Option<u16>,
    pub job_type_id: Option<u8>,
    pub intensity_pct: Option<u8>,
    pub months_unemployed: Option<u32>,
    pub months_on_benefits: Option<u32>,
    pub total_recipients: Option<u32>,
    pub entities_affected: Option<u32>,
    pub migrated_depositors: Option<u32>,
    pub migrated_companies: Option<u32>,
    pub migrated_loans: Option<u32>,
    pub quits: Option<u32>,
    pub retirements: Option<u32>,
    pub layoffs_non_distress: Option<u32>,
    pub replacement_openings: Option<u32>,
    pub laid_off_count: Option<u16>,
    pub listed_count: Option<u64>,
    pub shares_issued: Option<u64>,
    pub primary_shares_issued: Option<u64>,
    pub secondary_shares_sold: Option<u64>,
    pub shares_liquidated: Option<u64>,
    pub shares_wiped: Option<u64>,
    pub principal: Option<f64>,
    pub principal_lost: Option<f64>,
    pub unpaid_amount: Option<f64>,
    pub total_amount: Option<f64>,
    pub total_funding: Option<f64>,
    pub treasury_funding: Option<f64>,
    pub deficit_funding: Option<f64>,
    pub seed_capital: Option<f64>,
    pub health: Option<f64>,
    pub requested: Option<f64>,
    pub haircut_loss: Option<f64>,
    pub treasury_support: Option<f64>,
    pub amount_lost: Option<f64>,
    pub cut_rate: Option<f64>,
    pub monthly_salary: Option<f64>,
    pub principal_written_off: Option<f64>,
    pub price_per_share: Option<f64>,
    pub total_raised: Option<f64>,
    pub primary_raised: Option<f64>,
    pub secondary_raised: Option<f64>,
    pub founder_payout_total: Option<f64>,
    pub per_share: Option<f64>,
    pub proceeds: Option<f64>,
    pub total_market_cap: Option<f64>,
    pub total_turnover: Option<f64>,
    pub new_rate: Option<f64>,
    pub old_rate: Option<f64>,
    pub inflation_rate: Option<f64>,
    pub unemployment_rate: Option<f64>,
    pub face_value: Option<f64>,
    pub coupon_rate: Option<f64>,
    pub total_coupon: Option<f64>,
    pub shortfall: Option<f64>,
    pub monetized: Option<f64>,
    pub amount: Option<f64>,
    pub inflation_shortfall: Option<f64>,
    pub unemployment_gap: Option<f64>,
    pub price: Option<f64>,
    pub home_amount: Option<f64>,
    pub foreign_amount: Option<f64>,
    pub fx_rate_home_per_foreign: Option<f64>,
    pub annual_rate: Option<f64>,
    pub returned_home_amount: Option<f64>,
    pub returned_foreign_amount: Option<f64>,
    pub reserves_base: Option<f64>,
    pub reserve_gap: Option<f64>,
    pub pressure: Option<f64>,
    pub new_price_index: Option<f64>,
    pub change_pct: Option<f64>,
    pub amount_due: Option<f64>,
    pub amount_paid: Option<f64>,
    pub transferred_reserves: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencySpecDto {
    pub code: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationParamsDto {
    pub demographic: DemographicParams,
    pub education: EducationParams,
    pub labor: LaborParams,
    pub migration: MigrationParams,
    pub essentials: EssentialEconomyParams,
    pub annual_wage_raise: AnnualWageRaiseParams,
    pub labor_churn: LaborChurnParams,
    pub labor_demand: LaborDemandParams,
    pub company_growth: CompanyGrowthParams,
    pub company_formation: CompanyFormationParams,
    pub company_distress: CompanyDistressParams,
    pub labor_mobility: LaborMobilityParams,
    pub goods_market: GoodsMarketParams,
    pub tax: TaxPolicyParams,
    pub fiscal: FiscalParams,
    pub government_backfill: GovernmentBackfillParams,
    pub stock_market: StockMarketParams,
    pub monetary: MonetaryPolicyParams,
    pub banking: BankingSystemParams,
    pub housing: HousingMarketParams,
}

impl From<SimulationParams> for SimulationParamsDto {
    fn from(value: SimulationParams) -> Self {
        Self {
            demographic: value.demographic,
            education: value.education,
            labor: value.labor,
            migration: value.migration,
            essentials: value.essentials,
            annual_wage_raise: value.annual_wage_raise,
            labor_churn: value.labor_churn,
            labor_demand: value.labor_demand,
            company_growth: value.company_growth,
            company_formation: value.company_formation,
            company_distress: value.company_distress,
            labor_mobility: value.labor_mobility,
            goods_market: value.goods_market,
            tax: value.tax,
            fiscal: value.fiscal,
            government_backfill: value.government_backfill,
            stock_market: value.stock_market,
            monetary: value.monetary,
            banking: value.banking,
            housing: value.housing,
        }
    }
}

impl From<SimulationParamsDto> for SimulationParams {
    fn from(value: SimulationParamsDto) -> Self {
        Self {
            demographic: value.demographic,
            education: value.education,
            labor: value.labor,
            migration: value.migration,
            essentials: value.essentials,
            annual_wage_raise: value.annual_wage_raise,
            labor_churn: value.labor_churn,
            labor_demand: value.labor_demand,
            company_growth: value.company_growth,
            company_formation: value.company_formation,
            company_distress: value.company_distress,
            labor_mobility: value.labor_mobility,
            goods_market: value.goods_market,
            tax: value.tax,
            fiscal: value.fiscal,
            government_backfill: value.government_backfill,
            stock_market: value.stock_market,
            monetary: value.monetary,
            banking: value.banking,
            housing: value.housing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountryDto {
    pub id: u16,
    pub name: String,
    pub currency: CurrencySpecDto,
    pub day: u64,
    pub month: u64,
    pub next_startup_serial: u32,
    pub next_person_id: u32,
    pub birth_carry: f64,
    pub last_day_births: u32,
    pub last_day_deaths: u32,
    pub sim_params: SimulationParamsDto,
    pub treasury_balance: f64,
    pub treasury_bank_id: Option<u16>,
    pub monthly_tax_collected: f64,
    pub monthly_corporate_tax_collected: f64,
    pub monthly_household_spending: f64,
    pub monthly_tariff_collected: f64,
    pub monthly_goods_import_spending: f64,
    pub monthly_goods_import_spending_base: f64,
    pub monthly_goods_export_revenue: f64,
    pub monthly_goods_export_revenue_base: f64,
    pub monthly_unmet_need_events: u64,
    pub monthly_severe_unmet_need_events: u64,
    pub monthly_chronic_unmet_need_count: u64,
    pub monthly_essential_debt_spending: f64,
    pub monthly_hires_total: u64,
    pub monthly_immigrants: u64,
    pub monthly_emigrants: u64,
    pub monthly_migrants_for_work: u64,
    pub monthly_migrants_for_distress: u64,
    pub monthly_private_banks_founded: u64,
    pub cumulative_private_banks_founded: u64,
    pub monthly_private_banks_closed: u64,
    pub cumulative_private_banks_closed: u64,
    pub monthly_gov_spending: f64,
    pub monthly_budget_balance: f64,
    pub monthly_gdp: f64,
    pub austerity_active: bool,
    pub gov_debt: f64,
    pub bonds_outstanding: f64,
    pub bond_coupon_rate: f64,
    pub bond_months_to_maturity: u8,
    pub monthly_coupon_paid: f64,
    pub monthly_bonds_issued: f64,
    pub monthly_bonds_matured: f64,
    pub monthly_savings_interest_paid: f64,
    pub monthly_fx_financial_inflows_base: f64,
    pub monthly_fx_financial_outflows_base: f64,
    pub monthly_fx_net_flow_base: f64,
    pub monthly_money_minted: f64,
    pub cumulative_money_minted: f64,
    pub last_mint_inflation_shortfall: f64,
    pub last_mint_unemployment_gap: f64,
    pub last_mint_fraction: f64,
    pub policy_rate: f64,
    pub fx_rate_local_per_base: f64,
    pub fx_reserves_base: f64,
    /// key = source_country_idx as string
    pub fx_reserves_by_source: HashMap<String, f64>,
    pub fx_pressure: f64,
    pub cpi_index: f64,
    pub last_month_cpi_index: f64,
    pub monthly_inflation_rate: f64,
    pub stock_market: StockMarketDto,
    pub export_tariffs_by_good: Vec<f64>,
    pub pending_events: Vec<SimEventDto>,
    pub initial_money_supply: f64,
    pub household_cash_total: f64,
    pub company_cash_total: f64,
    pub total_money_supply: f64,
    pub expected_money_supply: f64,
    pub money_supply_gap: f64,
    pub next_bank_id: u16,
    pub next_bank_serial: u32,
    pub public_bank_id: Option<u16>,
    pub housing_stock_total: u32,
    pub housing_stock_occupied_owned: u32,
    pub housing_stock_occupied_rented: u32,
    pub housing_stock_vacant: u32,
    pub housing_price_index: f64,
    pub monthly_rent: f64,
    pub monthly_rent_collected: f64,
    pub monthly_mortgage_payments: f64,
    pub monthly_home_purchases: u32,
    pub monthly_foreclosures: u32,
    pub monthly_housing_units_built: u32,
    pub people: Vec<PersonDto>,
    pub companies: Vec<CompanyDto>,
    pub banks: Vec<BankDto>,
    pub households: Vec<HouseholdDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldDto {
    pub day: u64,
    pub generation_seed: u64,
    pub paused: bool,
    pub policy_rate: f64,
    pub tariff_rates_by_good: Vec<Vec<f64>>,
    pub countries: Vec<CountryDto>,
    pub world_history: Vec<MetricPointDto>,
    pub country_history: Vec<Vec<MetricPointDto>>,
    /// key = source_country_idx as string
    pub country_fx_source_history: Vec<HashMap<String, Vec<f64>>>,
    pub bank_history: Vec<BankHistoryEntryDto>,
    pub event_log: Vec<SimEventDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankHistoryEntryDto {
    pub country_idx: usize,
    pub bank_id: u16,
    pub points: Vec<BankMetricPointDto>,
}
