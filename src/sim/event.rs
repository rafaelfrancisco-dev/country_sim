use crate::sim::bank::BankKind;

pub const EVENT_LOG_LIMIT: usize = 10_000;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventCategory {
    StockMarket,
    BirthsDeaths,
    BankruptciesDistress,
    LoansDebt,
    CompaniesLabor,
    GovernmentPolicy,
}

impl EventCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::StockMarket => "Stock Market",
            Self::BirthsDeaths => "Births/Deaths",
            Self::BankruptciesDistress => "Bankruptcies & Distress",
            Self::LoansDebt => "Loans & Debt",
            Self::CompaniesLabor => "Companies & Labor",
            Self::GovernmentPolicy => "Government & Policy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    Person,
    Company,
    Country,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IpoReason {
    Seed,
    Growth,
    Distress,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimEvent {
    PopBirth {
        country_name: String,
        count: u32,
        day: u64,
    },
    PopDeath {
        country_name: String,
        count: u32,
        day: u64,
    },
    LoanEmitted {
        country_name: String,
        entity_kind: EntityKind,
        entity_id: u32,
        principal: f64,
        day: u64,
    },
    LoanEnded {
        country_name: String,
        entity_kind: EntityKind,
        entity_id: u32,
        day: u64,
    },
    LoanWrittenOff {
        country_name: String,
        entity_id: u32,
        principal_lost: f64,
        day: u64,
    },
    DebtDistress {
        country_name: String,
        entity_kind: EntityKind,
        entity_id: u32,
        unpaid_amount: f64,
        day: u64,
    },
    GovTransfer {
        country_name: String,
        total_recipients: u32,
        total_amount: f64,
        day: u64,
    },
    GovCompanyFunded {
        country_name: String,
        company_id: u16,
        produced_good_id: u8,
        total_funding: f64,
        treasury_funding: f64,
        deficit_funding: f64,
        day: u64,
    },
    CompanyFounded {
        country_name: String,
        company_id: u16,
        company_name: String,
        founders: u16,
        job_type_id: u8,
        produced_good_id: u8,
        day: u64,
    },
    BankFounded {
        country_name: String,
        bank_id: u16,
        bank_name: String,
        kind: BankKind,
        seed_capital: f64,
        day: u64,
    },
    BankRunStarted {
        country_name: String,
        bank_id: u16,
        bank_name: String,
        health: f64,
        day: u64,
    },
    BankRunWithdrawal {
        country_name: String,
        bank_id: u16,
        entities_affected: u32,
        requested: f64,
        haircut_loss: f64,
        day: u64,
    },
    BankResolutionStarted {
        country_name: String,
        bank_id: u16,
        treasury_support: f64,
        day: u64,
    },
    BankResolved {
        country_name: String,
        bank_id: u16,
        bank_name: String,
        day: u64,
    },
    BankDepositHaircut {
        country_name: String,
        bank_id: u16,
        entity_kind: EntityKind,
        entity_id: u32,
        amount_lost: f64,
        day: u64,
    },
    CompanyWageCut {
        country_name: String,
        company_id: u16,
        cut_rate: f64,
        day: u64,
    },
    CompanyLayoffs {
        country_name: String,
        company_id: u16,
        laid_off_count: u16,
        day: u64,
    },
    LaborChurnMonthly {
        country_name: String,
        quits: u32,
        retirements: u32,
        layoffs_non_distress: u32,
        replacement_openings: u32,
        day: u64,
    },
    JobSwitched {
        country_name: String,
        person_id: u32,
        from_company_id: u16,
        to_company_id: u16,
        monthly_salary: f64,
        day: u64,
    },
    CompanyBankrupt {
        country_name: String,
        company_id: u16,
        company_name: String,
        laid_off_count: u16,
        principal_written_off: f64,
        day: u64,
    },
    StockIpo {
        country_name: String,
        company_id: u16,
        company_name: String,
        ticker: String,
        reason: IpoReason,
        shares_issued: u64,
        primary_shares_issued: u64,
        secondary_shares_sold: u64,
        price_per_share: f64,
        total_raised: f64,
        primary_raised: f64,
        secondary_raised: f64,
        founder_payout_total: f64,
        day: u64,
    },
    StockDividend {
        country_name: String,
        company_id: u16,
        ticker: String,
        total_amount: f64,
        per_share: f64,
        day: u64,
    },
    StockMarginCall {
        country_name: String,
        person_id: u32,
        shares_liquidated: u64,
        proceeds: f64,
        day: u64,
    },
    StockDelisted {
        country_name: String,
        company_id: u16,
        company_name: String,
        shares_wiped: u64,
        day: u64,
    },
    StockMarketClose {
        country_name: String,
        day: u64,
        total_market_cap: f64,
        total_turnover: f64,
        listed_count: usize,
    },
    RatePolicyChange {
        country_name: String,
        new_rate: f64,
        old_rate: f64,
        inflation_rate: f64,
        unemployment_rate: f64,
        day: u64,
    },
    BondIssued {
        country_name: String,
        face_value: f64,
        coupon_rate: f64,
        day: u64,
    },
    CouponPaid {
        country_name: String,
        total_coupon: f64,
        day: u64,
    },
    BondMatured {
        country_name: String,
        face_value: f64,
        day: u64,
    },
    BondDefault {
        country_name: String,
        shortfall: f64,
        monetized: f64,
        day: u64,
    },
    StockSecondaryOffering {
        country_name: String,
        company_id: u16,
        ticker: String,
        shares_issued: u64,
        price: f64,
        total_raised: f64,
        day: u64,
    },
}

impl SimEvent {
    pub fn category(&self) -> EventCategory {
        match self {
            Self::PopBirth { .. } | Self::PopDeath { .. } => EventCategory::BirthsDeaths,
            Self::CompanyBankrupt { .. }
            | Self::LoanWrittenOff { .. }
            | Self::DebtDistress { .. } => EventCategory::BankruptciesDistress,
            Self::LoanEmitted { .. } | Self::LoanEnded { .. } => EventCategory::LoansDebt,
            Self::CompanyFounded { .. }
            | Self::BankFounded { .. }
            | Self::CompanyWageCut { .. }
            | Self::CompanyLayoffs { .. }
            | Self::LaborChurnMonthly { .. }
            | Self::JobSwitched { .. } => EventCategory::CompaniesLabor,
            Self::BankRunStarted { .. }
            | Self::BankRunWithdrawal { .. }
            | Self::BankResolutionStarted { .. }
            | Self::BankResolved { .. }
            | Self::BankDepositHaircut { .. } => EventCategory::BankruptciesDistress,
            Self::StockIpo { .. }
            | Self::StockDividend { .. }
            | Self::StockMarginCall { .. }
            | Self::StockDelisted { .. }
            | Self::StockMarketClose { .. }
            | Self::StockSecondaryOffering { .. } => EventCategory::StockMarket,
            Self::GovTransfer { .. }
            | Self::GovCompanyFunded { .. }
            | Self::RatePolicyChange { .. }
            | Self::BondIssued { .. }
            | Self::CouponPaid { .. }
            | Self::BondMatured { .. }
            | Self::BondDefault { .. } => EventCategory::GovernmentPolicy,
        }
    }
}

mod formatting;

#[cfg(test)]
mod tests;
