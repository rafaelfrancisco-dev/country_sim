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
            | Self::CompanyWageCut { .. }
            | Self::CompanyLayoffs { .. }
            | Self::LaborChurnMonthly { .. }
            | Self::JobSwitched { .. } => EventCategory::CompaniesLabor,
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

    pub fn format(&self) -> String {
        match self {
            Self::PopBirth {
                country_name,
                count,
                day,
            } => {
                format!("[BIRTH] Day {day:>6} | {country_name}: +{count} birth(s)")
            }
            Self::PopDeath {
                country_name,
                count,
                day,
            } => {
                format!("[DEATH] Day {day:>6} | {country_name}: -{count} death(s)")
            }
            Self::LoanEmitted {
                country_name,
                entity_kind,
                entity_id,
                principal,
                day,
            } => {
                let kind = entity_kind_label(entity_kind);
                format!(
                    "[LOAN+] Day {day:>6} | {country_name}: {kind} #{entity_id} +${principal:.2} loan"
                )
            }
            Self::LoanEnded {
                country_name,
                entity_kind,
                entity_id,
                day,
            } => {
                let kind = entity_kind_label(entity_kind);
                format!("[LOAN=] Day {day:>6} | {country_name}: {kind} #{entity_id} loan paid off")
            }
            Self::LoanWrittenOff {
                country_name,
                entity_id,
                principal_lost,
                day,
            } => {
                format!(
                    "[LOSS]  Day {day:>6} | {country_name}: Person #{entity_id} loan written off ${principal_lost:.2}"
                )
            }
            Self::DebtDistress {
                country_name,
                entity_kind,
                entity_id,
                unpaid_amount,
                day,
            } => {
                let kind = entity_kind_label(entity_kind);
                format!(
                    "[DEBT!] Day {day:>6} | {country_name}: {kind} #{entity_id} missed payment ${unpaid_amount:.2}"
                )
            }
            Self::GovTransfer {
                country_name,
                total_recipients,
                total_amount,
                day,
            } => {
                format!(
                    "[GOV]   Day {day:>6} | {country_name}: {total_recipients} recipients +${total_amount:.2} benefits"
                )
            }
            Self::GovCompanyFunded {
                country_name,
                company_id,
                produced_good_id,
                total_funding,
                treasury_funding,
                deficit_funding,
                day,
            } => {
                format!(
                    "[GOVCO] Day {day:>6} | {country_name}: Company #{company_id} for essential good #{produced_good_id} funded ${total_funding:.2} (treasury ${treasury_funding:.2}, deficit ${deficit_funding:.2})"
                )
            }
            Self::CompanyFounded {
                country_name,
                company_id,
                company_name,
                founders,
                job_type_id,
                produced_good_id,
                day,
            } => {
                format!(
                    "[FOUND] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) founded with {founders} founder(s), core job #{job_type_id}, good #{produced_good_id}"
                )
            }
            Self::CompanyWageCut {
                country_name,
                company_id,
                cut_rate,
                day,
            } => {
                format!(
                    "[WAGE-] Day {day:>6} | {country_name}: Company #{company_id} cut wages by {:.1}%",
                    cut_rate * 100.0
                )
            }
            Self::CompanyLayoffs {
                country_name,
                company_id,
                laid_off_count,
                day,
            } => {
                format!(
                    "[LAYOF] Day {day:>6} | {country_name}: Company #{company_id} laid off {laid_off_count} worker(s)"
                )
            }
            Self::LaborChurnMonthly {
                country_name,
                quits,
                retirements,
                layoffs_non_distress,
                replacement_openings,
                day,
            } => {
                format!(
                    "[CHURN] Day {day:>6} | {country_name}: quits {quits}, retirements {retirements}, non-distress layoffs {layoffs_non_distress}, replacement openings {replacement_openings}"
                )
            }
            Self::JobSwitched {
                country_name,
                person_id,
                from_company_id,
                to_company_id,
                monthly_salary,
                day,
            } => {
                format!(
                    "[SWITCH] Day {day:>6} | {country_name}: Person #{person_id} switched Company #{from_company_id} -> #{to_company_id} at ${monthly_salary:.2}/mo"
                )
            }
            Self::CompanyBankrupt {
                country_name,
                company_id,
                company_name,
                laid_off_count,
                principal_written_off,
                day,
            } => {
                format!(
                    "[BANKR] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) bankrupt, {laid_off_count} laid off, ${principal_written_off:.2} written off"
                )
            }
            Self::StockIpo {
                country_name,
                company_id,
                company_name,
                ticker,
                reason,
                shares_issued,
                primary_shares_issued,
                secondary_shares_sold,
                price_per_share,
                total_raised,
                primary_raised,
                secondary_raised,
                founder_payout_total,
                day,
            } => {
                let reason_label = match reason {
                    IpoReason::Seed => "seed",
                    IpoReason::Growth => "growth",
                    IpoReason::Distress => "distress",
                };
                format!(
                    "[IPO]   Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) listed as {ticker} [{reason_label}], {shares_issued} shares @ ${price_per_share:.2}, raised ${total_raised:.2} (primary ${primary_raised:.2}/{primary_shares_issued} sh, secondary ${secondary_raised:.2}/{secondary_shares_sold} sh, founder payout ${founder_payout_total:.2})"
                )
            }
            Self::StockDividend {
                country_name,
                company_id,
                ticker,
                total_amount,
                per_share,
                day,
            } => {
                format!(
                    "[DIV]   Day {day:>6} | {country_name}: {ticker} (#{company_id}) paid ${total_amount:.2} dividends (${per_share:.4}/share)"
                )
            }
            Self::StockMarginCall {
                country_name,
                person_id,
                shares_liquidated,
                proceeds,
                day,
            } => {
                format!(
                    "[MARGN] Day {day:>6} | {country_name}: Person #{person_id} margin call, {shares_liquidated} shares liquidated for ${proceeds:.2}"
                )
            }
            Self::StockDelisted {
                country_name,
                company_id,
                company_name,
                shares_wiped,
                day,
            } => {
                format!(
                    "[DELIST] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) delisted, {shares_wiped} shares wiped"
                )
            }
            Self::StockMarketClose {
                country_name,
                day,
                total_market_cap,
                total_turnover,
                listed_count,
            } => {
                format!(
                    "[STK]   Day {day:>6} | {country_name}: Market close — cap ${total_market_cap:.2}, turnover ${total_turnover:.2}, {listed_count} listed"
                )
            }
            Self::RatePolicyChange {
                country_name,
                new_rate,
                old_rate,
                inflation_rate,
                unemployment_rate,
                day,
            } => {
                format!(
                    "[RATE]  Day {day:>6} | {country_name}: Policy rate {:.2}% → {:.2}% (inflation {:.2}%, unemp {:.2}%)",
                    old_rate * 100.0,
                    new_rate * 100.0,
                    inflation_rate * 100.0,
                    unemployment_rate * 100.0,
                )
            }
            Self::BondIssued {
                country_name,
                face_value,
                coupon_rate,
                day,
            } => {
                format!(
                    "[BOND+] Day {day:>6} | {country_name}: Issued ${face_value:.2} bonds at {:.2}% coupon",
                    coupon_rate * 100.0
                )
            }
            Self::CouponPaid {
                country_name,
                total_coupon,
                day,
            } => {
                format!("[COUP]  Day {day:>6} | {country_name}: Coupon paid ${total_coupon:.2}")
            }
            Self::BondMatured {
                country_name,
                face_value,
                day,
            } => {
                format!(
                    "[BOND=] Day {day:>6} | {country_name}: Bond matured, repaid ${face_value:.2}"
                )
            }
            Self::BondDefault {
                country_name,
                shortfall,
                monetized,
                day,
            } => {
                format!(
                    "[BDFLT] Day {day:>6} | {country_name}: Bond default — shortfall ${shortfall:.2}, monetized ${monetized:.2}"
                )
            }
            Self::StockSecondaryOffering {
                country_name,
                company_id: _,
                ticker,
                shares_issued,
                price,
                total_raised,
                day,
            } => {
                format!(
                    "[SEC]   Day {day:>6} | {country_name}: {ticker} secondary offering {shares_issued} shares @ ${price:.2}, raised ${total_raised:.2}"
                )
            }
        }
    }
}

fn entity_kind_label(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Person => "Person",
        EntityKind::Company => "Company",
    }
}

#[cfg(test)]
mod tests;
