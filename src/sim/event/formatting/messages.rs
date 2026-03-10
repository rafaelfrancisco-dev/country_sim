use super::utils::{entity_kind_label, format_money, format_money_fixed};
use crate::sim::event::{IpoReason, SimEvent};

pub(super) fn format_event(event: &SimEvent, currency_symbol: &str) -> String {
    match event {
        SimEvent::PopBirth {
            country_name,
            count,
            day,
        } => format!("[BIRTH] Day {day:>6} | {country_name}: +{count} birth(s)"),
        SimEvent::PopDeath {
            country_name,
            count,
            day,
        } => format!("[DEATH] Day {day:>6} | {country_name}: -{count} death(s)"),
        SimEvent::LoanEmitted {
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
        SimEvent::LoanEnded {
            country_name,
            entity_kind,
            entity_id,
            day,
        } => {
            let kind = entity_kind_label(entity_kind);
            format!("[LOAN=] Day {day:>6} | {country_name}: {kind} #{entity_id} loan paid off")
        }
        SimEvent::LoanWrittenOff {
            country_name,
            entity_id,
            principal_lost,
            day,
        } => format!(
            "[LOSS]  Day {day:>6} | {country_name}: Person #{entity_id} loan written off {}",
            format_money(*principal_lost, currency_symbol)
        ),
        SimEvent::DebtDistress {
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
        SimEvent::GovTransfer {
            country_name,
            total_recipients,
            total_amount,
            day,
        } => format!(
            "[GOV]   Day {day:>6} | {country_name}: {total_recipients} recipients +{} benefits",
            format_money(*total_amount, currency_symbol)
        ),
        SimEvent::GovCompanyFunded {
            country_name,
            company_id,
            produced_good_id,
            total_funding,
            treasury_funding,
            deficit_funding,
            day,
        } => format!(
            "[GOVCO] Day {day:>6} | {country_name}: Company #{company_id} for essential good #{produced_good_id} funded {} (treasury {}, deficit {})",
            format_money(*total_funding, currency_symbol),
            format_money(*treasury_funding, currency_symbol),
            format_money(*deficit_funding, currency_symbol),
        ),
        SimEvent::CompanyFounded {
            country_name,
            company_id,
            company_name,
            founders,
            job_type_id,
            produced_good_id,
            day,
        } => format!(
            "[FOUND] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) founded with {founders} founder(s), core job #{job_type_id}, good #{produced_good_id}"
        ),
        SimEvent::BankFounded {
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
        SimEvent::BankRunStarted {
            country_name,
            bank_id,
            bank_name,
            health,
            day,
        } => format!(
            "[RUN!]  Day {day:>6} | {country_name}: Bank #{bank_id} ({bank_name}) run started (health {:.1}%)",
            health * 100.0
        ),
        SimEvent::BankRunWithdrawal {
            country_name,
            bank_id,
            entities_affected,
            requested,
            haircut_loss,
            day,
        } => format!(
            "[RUN$]  Day {day:>6} | {country_name}: Bank #{bank_id} withdrawals {} by {entities_affected} entities, losses {}",
            format_money(*requested, currency_symbol),
            format_money(*haircut_loss, currency_symbol),
        ),
        SimEvent::BankResolutionStarted {
            country_name,
            bank_id,
            treasury_support,
            day,
        } => format!(
            "[RESOL] Day {day:>6} | {country_name}: Bank #{bank_id} resolution started, treasury support {}",
            format_money(*treasury_support, currency_symbol)
        ),
        SimEvent::BankResolved {
            country_name,
            bank_id,
            bank_name,
            day,
        } => {
            format!("[RESOK] Day {day:>6} | {country_name}: Bank #{bank_id} ({bank_name}) resolved")
        }
        SimEvent::BankClosed {
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
        } => format!(
            "[BCLSD] Day {day:>6} | {country_name}: Bank #{bank_id} ({bank_name}) closed -> Bank #{acquired_by_bank_id} ({acquired_by_bank_name}), migrated {migrated_depositors} people, {migrated_companies} companies, {migrated_loans} loans, reserves {}",
            format_money(*transferred_reserves, currency_symbol)
        ),
        SimEvent::BankDepositHaircut {
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
        SimEvent::CompanyWageCut {
            country_name,
            company_id,
            cut_rate,
            day,
        } => format!(
            "[WAGE-] Day {day:>6} | {country_name}: Company #{company_id} cut wages by {:.1}%",
            cut_rate * 100.0
        ),
        SimEvent::CompanyLayoffs {
            country_name,
            company_id,
            laid_off_count,
            day,
        } => format!(
            "[LAYOF] Day {day:>6} | {country_name}: Company #{company_id} laid off {laid_off_count} worker(s)"
        ),
        SimEvent::LaborChurnMonthly {
            country_name,
            quits,
            retirements,
            layoffs_non_distress,
            replacement_openings,
            day,
        } => format!(
            "[CHURN] Day {day:>6} | {country_name}: quits {quits}, retirements {retirements}, non-distress layoffs {layoffs_non_distress}, replacement openings {replacement_openings}"
        ),
        SimEvent::JobSwitched {
            country_name,
            person_id,
            from_company_id,
            to_company_id,
            monthly_salary,
            day,
        } => format!(
            "[SWITCH] Day {day:>6} | {country_name}: Person #{person_id} switched Company #{from_company_id} -> #{to_company_id} at {}/mo",
            format_money(*monthly_salary, currency_symbol)
        ),
        SimEvent::PersonMigratedForWork {
            from_country_name,
            to_country_name,
            person_id,
            job_type_id,
            monthly_salary,
            day,
        } => format!(
            "[MOVE+] Day {day:>6} | {from_country_name} -> {to_country_name}: Person #{person_id} moved for job #{job_type_id} at {}/mo",
            format_money(*monthly_salary, currency_symbol)
        ),
        SimEvent::PersonMigratedForDistress {
            from_country_name,
            to_country_name,
            person_id,
            months_unemployed,
            months_on_benefits,
            day,
        } => format!(
            "[MOVE!] Day {day:>6} | {from_country_name} -> {to_country_name}: Person #{person_id} relocated after {months_unemployed} unemployed month(s) and {months_on_benefits} benefit month(s)"
        ),
        SimEvent::CompanyBankrupt {
            country_name,
            company_id,
            company_name,
            laid_off_count,
            principal_written_off,
            day,
        } => format!(
            "[BANKR] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) bankrupt, {laid_off_count} laid off, {} written off",
            format_money(*principal_written_off, currency_symbol)
        ),
        SimEvent::StockIpo {
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
        SimEvent::StockDividend {
            country_name,
            company_id,
            ticker,
            total_amount,
            per_share,
            day,
        } => format!(
            "[DIV]   Day {day:>6} | {country_name}: {ticker} (#{company_id}) paid {} dividends ({}/share)",
            format_money(*total_amount, currency_symbol),
            format_money_fixed(*per_share, currency_symbol, 4),
        ),
        SimEvent::StockMarginCall {
            country_name,
            person_id,
            shares_liquidated,
            proceeds,
            day,
        } => format!(
            "[MARGN] Day {day:>6} | {country_name}: Person #{person_id} margin call, {shares_liquidated} shares liquidated for {}",
            format_money(*proceeds, currency_symbol)
        ),
        SimEvent::StockDelisted {
            country_name,
            company_id,
            company_name,
            shares_wiped,
            day,
        } => format!(
            "[DELIST] Day {day:>6} | {country_name}: Company #{company_id} ({company_name}) delisted, {shares_wiped} shares wiped"
        ),
        SimEvent::StockMarketClose {
            country_name,
            day,
            total_market_cap,
            total_turnover,
            listed_count,
        } => format!(
            "[STK]   Day {day:>6} | {country_name}: Market close - cap {}, turnover {}, {listed_count} listed",
            format_money(*total_market_cap, currency_symbol),
            format_money(*total_turnover, currency_symbol),
        ),
        SimEvent::RatePolicyChange {
            country_name,
            new_rate,
            old_rate,
            inflation_rate,
            unemployment_rate,
            day,
        } => format!(
            "[RATE]  Day {day:>6} | {country_name}: Policy rate {:.2}% -> {:.2}% (inflation {:.2}%, unemp {:.2}%)",
            old_rate * 100.0,
            new_rate * 100.0,
            inflation_rate * 100.0,
            unemployment_rate * 100.0,
        ),
        SimEvent::BondIssued {
            country_name,
            face_value,
            coupon_rate,
            day,
        } => format!(
            "[BOND+] Day {day:>6} | {country_name}: Issued {} bonds at {:.2}% coupon",
            format_money(*face_value, currency_symbol),
            coupon_rate * 100.0
        ),
        SimEvent::CouponPaid {
            country_name,
            total_coupon,
            day,
        } => format!(
            "[COUP]  Day {day:>6} | {country_name}: Coupon paid {}",
            format_money(*total_coupon, currency_symbol)
        ),
        SimEvent::BondMatured {
            country_name,
            face_value,
            day,
        } => format!(
            "[BOND=] Day {day:>6} | {country_name}: Bond matured, repaid {}",
            format_money(*face_value, currency_symbol)
        ),
        SimEvent::BondDefault {
            country_name,
            shortfall,
            monetized,
            day,
        } => format!(
            "[BDFLT] Day {day:>6} | {country_name}: Bond default - shortfall {}, monetized {}",
            format_money(*shortfall, currency_symbol),
            format_money(*monetized, currency_symbol),
        ),
        SimEvent::MoneyMinted {
            country_name,
            amount,
            inflation_shortfall,
            unemployment_gap,
            day,
        } => format!(
            "[MINT]  Day {day:>6} | {country_name}: Minted {} (inflation gap {:.2}pp, unemp gap {:.2}pp)",
            format_money(*amount, currency_symbol),
            inflation_shortfall * 100.0,
            unemployment_gap * 100.0,
        ),
        SimEvent::StockSecondaryOffering {
            country_name,
            company_id: _,
            ticker,
            shares_issued,
            price,
            total_raised,
            day,
        } => format!(
            "[SEC]   Day {day:>6} | {country_name}: {ticker} secondary offering {shares_issued} shares @ {}, raised {}",
            format_money(*price, currency_symbol),
            format_money(*total_raised, currency_symbol),
        ),
        SimEvent::ForeignDepositOpened {
            home_country_name,
            foreign_country_name,
            person_id,
            bank_id: _,
            home_amount,
            foreign_amount,
            fx_rate_home_per_foreign,
            annual_rate,
            day,
        } => format!(
            "[FXDEP] Day {day:>6} | Person {person_id} ({home_country_name}) opened foreign deposit in {foreign_country_name}: {} -> {} @ FX {:.4}, {:.2}% p.a.",
            format_money(*home_amount, currency_symbol),
            format_money(*foreign_amount, currency_symbol),
            fx_rate_home_per_foreign,
            annual_rate * 100.0,
        ),
        SimEvent::ForeignDepositClosed {
            home_country_name,
            foreign_country_name,
            person_id,
            returned_home_amount,
            returned_foreign_amount,
            fx_rate_home_per_foreign,
            day,
        } => format!(
            "[FXDEP] Day {day:>6} | Person {person_id} ({home_country_name}) withdrew {} from {foreign_country_name} (foreign {}, FX {:.4})",
            format_money(*returned_home_amount, currency_symbol),
            format_money(*returned_foreign_amount, currency_symbol),
            fx_rate_home_per_foreign,
        ),
        SimEvent::FxRateChanged {
            country_name,
            currency_code,
            old_rate,
            new_rate,
            pressure,
            reserves_base,
            day,
        } => format!(
            "[FX]    Day {day:>6} | {country_name} ({currency_code}) rate {:.4} -> {:.4}, pressure {:.2}, reserves {} base",
            old_rate,
            new_rate,
            pressure,
            format_money(*reserves_base, ""),
        ),
        SimEvent::FxReservePressure {
            country_name,
            currency_code,
            reserves_base,
            reserve_gap,
            pressure,
            day,
        } => format!(
            "[FX!]   Day {day:>6} | {country_name} ({currency_code}) reserve pressure: gap {:.1}%, pressure {:.2}, reserves {} base",
            reserve_gap * 100.0,
            pressure,
            format_money(*reserves_base, ""),
        ),
        SimEvent::DisasterApplied {
            country_name,
            disaster_label,
            intensity_pct,
            summary,
            day,
        } => format!(
            "[DISAS] Day {day:>6} | {country_name}: {disaster_label} ({intensity_pct}%) - {summary}"
        ),
        SimEvent::HomePurchased { .. }
        | SimEvent::Foreclosure { .. }
        | SimEvent::MortgagePaidOff { .. }
        | SimEvent::HousingPriceChanged { .. }
        | SimEvent::RentUnpaid { .. } => super::housing::format_event(event, currency_symbol),
    }
}
