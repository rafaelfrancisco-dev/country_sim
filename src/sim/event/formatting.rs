use super::*;

impl SimEvent {
    pub fn format(&self) -> String {
        self.format_with_currency(crate::sim::setup::DEFAULT_CURRENCY_SYMBOL)
    }

    pub fn format_with_currency(&self, currency_symbol: &str) -> String {
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
                    "[LOAN+] Day {day:>6} | {country_name}: {kind} #{entity_id} +{} loan",
                    format_money(*principal, currency_symbol)
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
                    "[LOSS]  Day {day:>6} | {country_name}: Person #{entity_id} loan written off {}",
                    format_money(*principal_lost, currency_symbol)
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
                    "[DEBT!] Day {day:>6} | {country_name}: {kind} #{entity_id} missed payment {}",
                    format_money(*unpaid_amount, currency_symbol)
                )
            }
            Self::GovTransfer {
                country_name,
                total_recipients,
                total_amount,
                day,
            } => {
                format!(
                    "[GOV]   Day {day:>6} | {country_name}: {total_recipients} recipients +{} benefits",
                    format_money(*total_amount, currency_symbol)
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
                    "[GOVCO] Day {day:>6} | {country_name}: Company #{company_id} for essential good #{produced_good_id} funded {} (treasury {}, deficit {})",
                    format_money(*total_funding, currency_symbol),
                    format_money(*treasury_funding, currency_symbol),
                    format_money(*deficit_funding, currency_symbol),
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
            Self::BankFounded {
                country_name,
                bank_id,
                bank_name,
                kind,
                seed_capital,
                day,
            } => {
                let kind_label = match kind {
                    crate::sim::bank::BankKind::Public => "Public",
                    crate::sim::bank::BankKind::Private => "Private",
                };
                format!(
                    "[BNK+]  Day {day:>6} | {country_name}: {kind_label} bank #{bank_id} ({bank_name}) founded with seed {}",
                    format_money(*seed_capital, currency_symbol)
                )
            }
            Self::BankRunStarted {
                country_name,
                bank_id,
                bank_name,
                health,
                day,
            } => {
                format!(
                    "[RUN!]  Day {day:>6} | {country_name}: Bank #{bank_id} ({bank_name}) run started (health {:.1}%)",
                    health * 100.0
                )
            }
            Self::BankRunWithdrawal {
                country_name,
                bank_id,
                entities_affected,
                requested,
                haircut_loss,
                day,
            } => {
                format!(
                    "[RUN$]  Day {day:>6} | {country_name}: Bank #{bank_id} withdrawals {} by {entities_affected} entities, losses {}",
                    format_money(*requested, currency_symbol),
                    format_money(*haircut_loss, currency_symbol),
                )
            }
            Self::BankResolutionStarted {
                country_name,
                bank_id,
                treasury_support,
                day,
            } => {
                format!(
                    "[RESOL] Day {day:>6} | {country_name}: Bank #{bank_id} resolution started, treasury support {}",
                    format_money(*treasury_support, currency_symbol)
                )
            }
            Self::BankResolved {
                country_name,
                bank_id,
                bank_name,
                day,
            } => {
                format!(
                    "[RESOK] Day {day:>6} | {country_name}: Bank #{bank_id} ({bank_name}) resolved"
                )
            }
            Self::BankClosed {
                country_name,
                bank_id,
                bank_name,
                acquired_by_bank_id,
                acquired_by_bank_name,
                migrated_depositors,
                migrated_companies,
                migrated_loans,
                transferred_reserves,
                day,
            } => {
                format!(
                    "[BCLSD] Day {day:>6} | {country_name}: Bank #{bank_id} ({bank_name}) closed -> Bank #{acquired_by_bank_id} ({acquired_by_bank_name}), migrated {migrated_depositors} people, {migrated_companies} companies, {migrated_loans} loans, reserves {}",
                    format_money(*transferred_reserves, currency_symbol)
                )
            }
            Self::BankDepositHaircut {
                country_name,
                bank_id,
                entity_kind,
                entity_id,
                amount_lost,
                day,
            } => {
                let kind = entity_kind_label(entity_kind);
                format!(
                    "[HAIRC] Day {day:>6} | {country_name}: {kind} #{entity_id} lost {} in Bank #{bank_id} resolution",
                    format_money(*amount_lost, currency_symbol),
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
                    "[SWITCH] Day {day:>6} | {country_name}: Person #{person_id} switched Company #{from_company_id} -> #{to_company_id} at {}/mo",
                    format_money(*monthly_salary, currency_symbol)
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
                    "[BANKR] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) bankrupt, {laid_off_count} laid off, {} written off",
                    format_money(*principal_written_off, currency_symbol)
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
                    "[IPO]   Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) listed as {ticker} [{reason_label}], {shares_issued} shares @ {}, raised {} (primary {}/{primary_shares_issued} sh, secondary {}/{secondary_shares_sold} sh, founder payout {})",
                    format_money(*price_per_share, currency_symbol),
                    format_money(*total_raised, currency_symbol),
                    format_money(*primary_raised, currency_symbol),
                    format_money(*secondary_raised, currency_symbol),
                    format_money(*founder_payout_total, currency_symbol),
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
                    "[DIV]   Day {day:>6} | {country_name}: {ticker} (#{company_id}) paid {} dividends ({}/share)",
                    format_money(*total_amount, currency_symbol),
                    format_money_fixed(*per_share, currency_symbol, 4),
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
                    "[MARGN] Day {day:>6} | {country_name}: Person #{person_id} margin call, {shares_liquidated} shares liquidated for {}",
                    format_money(*proceeds, currency_symbol)
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
                    "[STK]   Day {day:>6} | {country_name}: Market close — cap {}, turnover {}, {listed_count} listed",
                    format_money(*total_market_cap, currency_symbol),
                    format_money(*total_turnover, currency_symbol),
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
                    "[BOND+] Day {day:>6} | {country_name}: Issued {} bonds at {:.2}% coupon",
                    format_money(*face_value, currency_symbol),
                    coupon_rate * 100.0
                )
            }
            Self::CouponPaid {
                country_name,
                total_coupon,
                day,
            } => {
                format!(
                    "[COUP]  Day {day:>6} | {country_name}: Coupon paid {}",
                    format_money(*total_coupon, currency_symbol)
                )
            }
            Self::BondMatured {
                country_name,
                face_value,
                day,
            } => {
                format!(
                    "[BOND=] Day {day:>6} | {country_name}: Bond matured, repaid {}",
                    format_money(*face_value, currency_symbol)
                )
            }
            Self::BondDefault {
                country_name,
                shortfall,
                monetized,
                day,
            } => {
                format!(
                    "[BDFLT] Day {day:>6} | {country_name}: Bond default — shortfall {}, monetized {}",
                    format_money(*shortfall, currency_symbol),
                    format_money(*monetized, currency_symbol),
                )
            }
            Self::MoneyMinted {
                country_name,
                amount,
                inflation_shortfall,
                unemployment_gap,
                day,
            } => {
                format!(
                    "[MINT]  Day {day:>6} | {country_name}: Minted {} (inflation gap {:.2}pp, unemp gap {:.2}pp)",
                    format_money(*amount, currency_symbol),
                    inflation_shortfall * 100.0,
                    unemployment_gap * 100.0,
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
                    "[SEC]   Day {day:>6} | {country_name}: {ticker} secondary offering {shares_issued} shares @ {}, raised {}",
                    format_money(*price, currency_symbol),
                    format_money(*total_raised, currency_symbol),
                )
            }
            Self::ForeignDepositOpened {
                home_country_name,
                foreign_country_name,
                person_id,
                bank_id: _,
                amount,
                annual_rate,
                day,
            } => {
                format!(
                    "[FXDEP] Day {day:>6} | Person {person_id} ({home_country_name}) opened foreign deposit in {foreign_country_name}: {} @ {:.2}% p.a.",
                    format_money(*amount, currency_symbol),
                    annual_rate * 100.0,
                )
            }
            Self::ForeignDepositClosed {
                home_country_name,
                foreign_country_name,
                person_id,
                returned_amount,
                day,
            } => {
                format!(
                    "[FXDEP] Day {day:>6} | Person {person_id} ({home_country_name}) withdrew {} from {foreign_country_name}",
                    format_money(*returned_amount, currency_symbol),
                )
            }
            Self::DisasterApplied {
                country_name,
                disaster_label,
                intensity_pct,
                summary,
                day,
            } => {
                format!(
                    "[DISAS] Day {day:>6} | {country_name}: {disaster_label} ({intensity_pct}%) - {summary}"
                )
            }
            Self::HomePurchased {
                country_name,
                person_id,
                price,
                day,
            } => {
                format!(
                    "[HOME+] Day {day:>6} | {country_name}: Person #{person_id} purchased home for {}",
                    format_money(*price, currency_symbol)
                )
            }
            Self::Foreclosure {
                country_name,
                person_id,
                principal_written_off,
                day,
            } => {
                format!(
                    "[FRCLS] Day {day:>6} | {country_name}: Person #{person_id} foreclosed, {} written off",
                    format_money(*principal_written_off, currency_symbol)
                )
            }
            Self::MortgagePaidOff {
                country_name,
                person_id,
                day,
            } => {
                format!(
                    "[MORT=] Day {day:>6} | {country_name}: Person #{person_id} paid off mortgage"
                )
            }
            Self::HousingPriceChanged {
                country_name,
                new_price_index,
                change_pct,
                day,
            } => {
                let dir = if *change_pct >= 0.0 { "▲" } else { "▼" };
                format!(
                    "[HPRIC] Day {day:>6} | {country_name}: Housing price index {} ({dir}{:.2}%)",
                    format_money(*new_price_index, currency_symbol),
                    change_pct.abs() * 100.0
                )
            }
            Self::RentUnpaid {
                country_name,
                person_id,
                amount_due,
                amount_paid,
                day,
            } => {
                format!(
                    "[RENT!] Day {day:>6} | {country_name}: Person #{person_id} underpaid rent — due {}, paid {}",
                    format_money(*amount_due, currency_symbol),
                    format_money(*amount_paid, currency_symbol),
                )
            }
        }
    }
}

fn entity_kind_label(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Person => "Person",
        EntityKind::Company => "Company",
        EntityKind::Country => "Country",
    }
}

fn format_money(amount: f64, currency_symbol: &str) -> String {
    let symbol = normalized_currency_symbol(currency_symbol);
    let sign = if amount < 0.0 { "-" } else { "" };
    format!("{sign}{symbol}{}", compact_money_amount(amount.abs()))
}

fn format_money_fixed(amount: f64, currency_symbol: &str, precision: usize) -> String {
    let symbol = normalized_currency_symbol(currency_symbol);
    let sign = if amount < 0.0 { "-" } else { "" };
    format!(
        "{sign}{symbol}{value:.precision$}",
        value = amount.abs(),
        precision = precision
    )
}

fn normalized_currency_symbol(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        crate::sim::setup::DEFAULT_CURRENCY_SYMBOL
    } else {
        trimmed
    }
}

fn compact_money_amount(amount: f64) -> String {
    let abs = amount.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{amount:.2}");
    };
    let scaled = amount / divisor;
    let rounded = (scaled * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}{suffix}")
    } else {
        format!("{rounded:.1}{suffix}")
    }
}
