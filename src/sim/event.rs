pub const EVENT_LOG_LIMIT: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    Person,
    Company,
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
        shares_issued: u64,
        price_per_share: f64,
        total_raised: f64,
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
}

impl SimEvent {
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
                shares_issued,
                price_per_share,
                total_raised,
                day,
            } => {
                format!(
                    "[IPO]   Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) listed as {ticker}, {shares_issued} shares @ ${price_per_share:.2}, raised ${total_raised:.2}"
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
        }
    }
}

fn entity_kind_label(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Person => "Person",
        EntityKind::Company => "Company",
    }
}
