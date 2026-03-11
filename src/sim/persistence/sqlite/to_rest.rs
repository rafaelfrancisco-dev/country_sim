use std::collections::HashMap;
use std::sync::Arc;

use crate::sim::{
    bank::Bank,
    essentials::Household,
    event::SimEvent,
    stock_market::{DailyIndexPoint, ListedEquity, Portfolio, StockMarket},
};

use super::super::dto::{
    BankDto, BankJobOpeningDto, BankStockPositionDto, DailyIndexPointDto, HouseholdDto,
    ListedEquityDto, PortfolioDto, PositionDto, SimEventDto, StockMarketDto,
};

pub(super) fn bank_to_dto(b: &Bank) -> BankDto {
    let kind = match b.kind {
        crate::sim::bank::BankKind::Public => 0,
        crate::sim::bank::BankKind::Private => 1,
    };

    let open_positions = b
        .open_positions
        .iter()
        .map(|op| BankJobOpeningDto {
            job_type_id: op.job_type_id,
            slots: op.slots,
            offered_monthly_salary: op.offered_monthly_salary,
        })
        .collect();

    let stock_positions = b
        .stock_positions
        .iter()
        .map(|sp| BankStockPositionDto {
            security_country_id: sp.security_id.country_id,
            security_company_id: sp.security_id.company_id,
            shares: sp.shares,
            avg_cost: sp.avg_cost,
        })
        .collect();

    BankDto {
        id: b.id,
        name: b.name.to_string(),
        kind,
        reserve_balance: b.reserve_balance,
        created_money_outstanding: b.created_money_outstanding,
        total_originated: b.total_originated,
        total_principal_repaid: b.total_principal_repaid,
        deposit_liabilities: b.deposit_liabilities,
        foreign_deposit_liabilities: b.foreign_deposit_liabilities,
        equity_capital: b.equity_capital,
        run_intensity: b.run_intensity,
        is_in_resolution: b.is_in_resolution,
        months_in_resolution: b.months_in_resolution,
        resolution_distress_streak: b.resolution_distress_streak,
        resolution_stable_streak: b.resolution_stable_streak,
        open_positions,
        employee_count: b.employee_count,
        monthly_payroll_budget: b.monthly_payroll_budget,
        last_month_payroll_paid: b.last_month_payroll_paid,
        wage_offer_multiplier: b.wage_offer_multiplier,
        last_month_interest_income: b.last_month_interest_income,
        last_month_deposit_interest_expense: b.last_month_deposit_interest_expense,
        last_month_credit_losses: b.last_month_credit_losses,
        last_month_profit: b.last_month_profit,
        stock_positions,
    }
}

pub(super) fn household_to_dto(h: &Household) -> HouseholdDto {
    HouseholdDto {
        id: h.id,
        member_count: h.member_count,
        sheltered: h.sheltered,
    }
}

pub(super) fn stock_market_to_dto(sm: &StockMarket) -> StockMarketDto {
    let person_portfolios: HashMap<String, PortfolioDto> = sm
        .person_portfolios
        .iter()
        .map(|(k, v)| (k.to_string(), portfolio_to_dto(v)))
        .collect();

    let listed: HashMap<String, ListedEquityDto> = sm
        .listed
        .iter()
        .map(|(k, v)| (k.to_string(), listed_equity_to_dto(v)))
        .collect();

    let daily_index_history: Vec<DailyIndexPointDto> = sm
        .daily_index_history
        .iter()
        .map(|p| DailyIndexPointDto {
            day: p.day,
            index_value: p.index_value,
            total_turnover: p.total_turnover,
            bank_equity_exposure: p.bank_equity_exposure,
        })
        .collect();

    StockMarketDto {
        person_portfolios,
        bank_portfolio: portfolio_to_dto(&sm.bank_portfolio),
        listed,
        daily_index_history,
        index_base: sm.index_base,
        ipo_raised_mtd: sm.ipo_raised_mtd,
        dividends_paid_mtd: sm.dividends_paid_mtd,
        last_completed_month_dividends_paid: sm.last_completed_month_dividends_paid,
        forced_liquidations_mtd: sm.forced_liquidations_mtd,
    }
}

pub(super) fn portfolio_to_dto(p: &Portfolio) -> PortfolioDto {
    PortfolioDto {
        positions: p
            .positions
            .iter()
            .map(|pos| PositionDto {
                security_country_id: pos.security_id.country_id,
                security_company_id: pos.security_id.company_id,
                shares: pos.shares,
                avg_cost: pos.avg_cost,
                pledged_shares: pos.pledged_shares,
            })
            .collect(),
        realized_pnl_mtd: p.realized_pnl_mtd,
        dividends_mtd: p.dividends_mtd,
        last_completed_month_dividends: p.last_completed_month_dividends,
    }
}

pub(super) fn listed_equity_to_dto(e: &ListedEquity) -> ListedEquityDto {
    ListedEquityDto {
        company_id: e.company_id,
        current_company_id: e.current_company_id,
        ticker: e.ticker.to_string(),
        shares_outstanding: e.shares_outstanding,
        free_float_shares: e.free_float_shares,
        last_price: e.last_price,
        prev_close: e.prev_close,
        last_closed_day_pct_change: e.last_closed_day_pct_change,
        day_volume: e.day_volume,
        day_turnover: e.day_turnover,
        day_trades_count: e.day_trades_count,
        market_cap: e.market_cap,
        price_history: e.price_history.iter().copied().collect(),
        volume_history: e.volume_history.iter().copied().collect(),
        market_cap_history: e.market_cap_history.iter().copied().collect(),
        monthly_dps: e.monthly_dps.to_vec(),
        dps_cursor: e.dps_cursor,
    }
}

pub(super) fn sim_event_to_dto(event: &SimEvent) -> SimEventDto {
    let json_val = serde_json::to_value(sim_event_fields(event)).unwrap_or(serde_json::Value::Null);
    SimEventDto {
        kind: sim_event_kind_str(event).to_string(),
        payload: json_val,
    }
}

fn sim_event_kind_str(event: &SimEvent) -> &'static str {
    match event {
        SimEvent::PopBirth { .. } => "PopBirth",
        SimEvent::PopDeath { .. } => "PopDeath",
        SimEvent::LoanEmitted { .. } => "LoanEmitted",
        SimEvent::LoanEnded { .. } => "LoanEnded",
        SimEvent::LoanWrittenOff { .. } => "LoanWrittenOff",
        SimEvent::DebtDistress { .. } => "DebtDistress",
        SimEvent::GovTransfer { .. } => "GovTransfer",
        SimEvent::GovCompanyFunded { .. } => "GovCompanyFunded",
        SimEvent::CompanyFounded { .. } => "CompanyFounded",
        SimEvent::BankFounded { .. } => "BankFounded",
        SimEvent::BankRunStarted { .. } => "BankRunStarted",
        SimEvent::BankRunWithdrawal { .. } => "BankRunWithdrawal",
        SimEvent::BankResolutionStarted { .. } => "BankResolutionStarted",
        SimEvent::BankResolved { .. } => "BankResolved",
        SimEvent::BankClosed { .. } => "BankClosed",
        SimEvent::BankDepositHaircut { .. } => "BankDepositHaircut",
        SimEvent::CompanyWageCut { .. } => "CompanyWageCut",
        SimEvent::CompanyLayoffs { .. } => "CompanyLayoffs",
        SimEvent::LaborChurnMonthly { .. } => "LaborChurnMonthly",
        SimEvent::JobSwitched { .. } => "JobSwitched",
        SimEvent::PersonMigratedForWork { .. } => "PersonMigratedForWork",
        SimEvent::PersonMigratedForDistress { .. } => "PersonMigratedForDistress",
        SimEvent::CompanyBankrupt { .. } => "CompanyBankrupt",
        SimEvent::StockIpo { .. } => "StockIpo",
        SimEvent::StockDividend { .. } => "StockDividend",
        SimEvent::StockMarginCall { .. } => "StockMarginCall",
        SimEvent::StockDelisted { .. } => "StockDelisted",
        SimEvent::StockMarketClose { .. } => "StockMarketClose",
        SimEvent::RatePolicyChange { .. } => "RatePolicyChange",
        SimEvent::BondIssued { .. } => "BondIssued",
        SimEvent::CouponPaid { .. } => "CouponPaid",
        SimEvent::BondMatured { .. } => "BondMatured",
        SimEvent::BondDefault { .. } => "BondDefault",
        SimEvent::MoneyMinted { .. } => "MoneyMinted",
        SimEvent::StockSecondaryOffering { .. } => "StockSecondaryOffering",
        SimEvent::ForeignDepositOpened { .. } => "ForeignDepositOpened",
        SimEvent::ForeignDepositClosed { .. } => "ForeignDepositClosed",
        SimEvent::FxRateChanged { .. } => "FxRateChanged",
        SimEvent::FxReservePressure { .. } => "FxReservePressure",
        SimEvent::DisasterApplied { .. } => "DisasterApplied",
        SimEvent::HomePurchased { .. } => "HomePurchased",
        SimEvent::Foreclosure { .. } => "Foreclosure",
        SimEvent::MortgagePaidOff { .. } => "MortgagePaidOff",
        SimEvent::HousingPriceChanged { .. } => "HousingPriceChanged",
        SimEvent::RentUnpaid { .. } => "RentUnpaid",
    }
}

fn sim_event_fields(event: &SimEvent) -> serde_json::Value {
    use serde_json::json;
    match event {
        SimEvent::PopBirth { country_name, count, day } => json!({
            "country_name": country_name.as_ref(), "count": count, "day": day
        }),
        SimEvent::PopDeath { country_name, count, day } => json!({
            "country_name": country_name.as_ref(), "count": count, "day": day
        }),
        SimEvent::LoanEmitted { country_name, entity_id, principal, day, .. } => json!({
            "country_name": country_name.as_ref(), "entity_id": entity_id, "principal": principal, "day": day
        }),
        SimEvent::LoanEnded { country_name, entity_id, day, .. } => json!({
            "country_name": country_name.as_ref(), "entity_id": entity_id, "day": day
        }),
        SimEvent::LoanWrittenOff { country_name, entity_id, principal_lost, day } => json!({
            "country_name": country_name.as_ref(), "entity_id": entity_id, "principal_lost": principal_lost, "day": day
        }),
        SimEvent::DebtDistress { country_name, entity_id, unpaid_amount, day, .. } => json!({
            "country_name": country_name.as_ref(), "entity_id": entity_id, "unpaid_amount": unpaid_amount, "day": day
        }),
        SimEvent::GovTransfer { country_name, total_recipients, total_amount, day } => json!({
            "country_name": country_name.as_ref(), "total_recipients": total_recipients, "total_amount": total_amount, "day": day
        }),
        SimEvent::GovCompanyFunded { country_name, company_id, total_funding, day, .. } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "total_funding": total_funding, "day": day
        }),
        SimEvent::CompanyFounded { country_name, company_id, company_name, founders, job_type_id, produced_good_id, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "company_name": company_name.as_ref(),
            "founders": founders, "job_type_id": job_type_id, "produced_good_id": produced_good_id, "day": day
        }),
        SimEvent::BankFounded { country_name, bank_id, bank_name, seed_capital, day, .. } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "bank_name": bank_name.as_ref(), "seed_capital": seed_capital, "day": day
        }),
        SimEvent::BankRunStarted { country_name, bank_id, bank_name, health, day } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "bank_name": bank_name.as_ref(), "health": health, "day": day
        }),
        SimEvent::BankRunWithdrawal { country_name, bank_id, entities_affected, requested, haircut_loss, day } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "entities_affected": entities_affected,
            "requested": requested, "haircut_loss": haircut_loss, "day": day
        }),
        SimEvent::BankResolutionStarted { country_name, bank_id, treasury_support, day } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "treasury_support": treasury_support, "day": day
        }),
        SimEvent::BankResolved { country_name, bank_id, bank_name, day } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "bank_name": bank_name.as_ref(), "day": day
        }),
        SimEvent::BankClosed { country_name, bank_id, bank_name, acquired_by_bank_id, migrated_depositors, day, .. } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "bank_name": bank_name.as_ref(),
            "acquired_by_bank_id": acquired_by_bank_id, "migrated_depositors": migrated_depositors, "day": day
        }),
        SimEvent::BankDepositHaircut { country_name, bank_id, entity_id, amount_lost, day, .. } => json!({
            "country_name": country_name.as_ref(), "bank_id": bank_id, "entity_id": entity_id, "amount_lost": amount_lost, "day": day
        }),
        SimEvent::CompanyWageCut { country_name, company_id, cut_rate, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "cut_rate": cut_rate, "day": day
        }),
        SimEvent::CompanyLayoffs { country_name, company_id, laid_off_count, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "laid_off_count": laid_off_count, "day": day
        }),
        SimEvent::LaborChurnMonthly { country_name, quits, retirements, layoffs_non_distress, replacement_openings, day } => json!({
            "country_name": country_name.as_ref(), "quits": quits, "retirements": retirements,
            "layoffs_non_distress": layoffs_non_distress, "replacement_openings": replacement_openings, "day": day
        }),
        SimEvent::JobSwitched { country_name, person_id, from_company_id, to_company_id, monthly_salary, day } => json!({
            "country_name": country_name.as_ref(), "person_id": person_id, "from_company_id": from_company_id,
            "to_company_id": to_company_id, "monthly_salary": monthly_salary, "day": day
        }),
        SimEvent::PersonMigratedForWork { from_country_name, to_country_name, person_id, job_type_id, monthly_salary, day } => json!({
            "from_country_name": from_country_name.as_ref(), "to_country_name": to_country_name.as_ref(),
            "person_id": person_id, "job_type_id": job_type_id, "monthly_salary": monthly_salary, "day": day
        }),
        SimEvent::PersonMigratedForDistress { from_country_name, to_country_name, person_id, months_unemployed, months_on_benefits, day } => json!({
            "from_country_name": from_country_name.as_ref(), "to_country_name": to_country_name.as_ref(),
            "person_id": person_id, "months_unemployed": months_unemployed, "months_on_benefits": months_on_benefits, "day": day
        }),
        SimEvent::CompanyBankrupt { country_name, company_id, company_name, laid_off_count, principal_written_off, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "company_name": company_name.as_ref(),
            "laid_off_count": laid_off_count, "principal_written_off": principal_written_off, "day": day
        }),
        SimEvent::StockIpo { country_name, company_id, ticker, shares_issued, price_per_share, total_raised, day, .. } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "ticker": ticker.as_ref(),
            "shares_issued": shares_issued, "price_per_share": price_per_share, "total_raised": total_raised, "day": day
        }),
        SimEvent::StockDividend { country_name, company_id, ticker, total_amount, per_share, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "ticker": ticker.as_ref(),
            "total_amount": total_amount, "per_share": per_share, "day": day
        }),
        SimEvent::StockMarginCall { country_name, person_id, shares_liquidated, proceeds, day } => json!({
            "country_name": country_name.as_ref(), "person_id": person_id, "shares_liquidated": shares_liquidated, "proceeds": proceeds, "day": day
        }),
        SimEvent::StockDelisted { country_name, company_id, company_name, shares_wiped, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "company_name": company_name.as_ref(), "shares_wiped": shares_wiped, "day": day
        }),
        SimEvent::StockMarketClose { country_name, day, total_market_cap, total_turnover, listed_count } => json!({
            "country_name": country_name.as_ref(), "day": day, "total_market_cap": total_market_cap,
            "total_turnover": total_turnover, "listed_count": listed_count
        }),
        SimEvent::RatePolicyChange { country_name, new_rate, old_rate, inflation_rate, unemployment_rate, day } => json!({
            "country_name": country_name.as_ref(), "new_rate": new_rate, "old_rate": old_rate,
            "inflation_rate": inflation_rate, "unemployment_rate": unemployment_rate, "day": day
        }),
        SimEvent::BondIssued { country_name, face_value, coupon_rate, day } => json!({
            "country_name": country_name.as_ref(), "face_value": face_value, "coupon_rate": coupon_rate, "day": day
        }),
        SimEvent::CouponPaid { country_name, total_coupon, day } => json!({
            "country_name": country_name.as_ref(), "total_coupon": total_coupon, "day": day
        }),
        SimEvent::BondMatured { country_name, face_value, day } => json!({
            "country_name": country_name.as_ref(), "face_value": face_value, "day": day
        }),
        SimEvent::BondDefault { country_name, shortfall, monetized, day } => json!({
            "country_name": country_name.as_ref(), "shortfall": shortfall, "monetized": monetized, "day": day
        }),
        SimEvent::MoneyMinted { country_name, amount, inflation_shortfall, unemployment_gap, day } => json!({
            "country_name": country_name.as_ref(), "amount": amount, "inflation_shortfall": inflation_shortfall,
            "unemployment_gap": unemployment_gap, "day": day
        }),
        SimEvent::StockSecondaryOffering { country_name, company_id, ticker, shares_issued, price, total_raised, day } => json!({
            "country_name": country_name.as_ref(), "company_id": company_id, "ticker": ticker.as_ref(),
            "shares_issued": shares_issued, "price": price, "total_raised": total_raised, "day": day
        }),
        SimEvent::ForeignDepositOpened { home_country_name, foreign_country_name, person_id, bank_id, home_amount, foreign_amount, fx_rate_home_per_foreign, annual_rate, day } => json!({
            "home_country_name": home_country_name.as_ref(), "foreign_country_name": foreign_country_name.as_ref(),
            "person_id": person_id, "bank_id": bank_id, "home_amount": home_amount, "foreign_amount": foreign_amount,
            "fx_rate_home_per_foreign": fx_rate_home_per_foreign, "annual_rate": annual_rate, "day": day
        }),
        SimEvent::ForeignDepositClosed { home_country_name, foreign_country_name, person_id, returned_home_amount, returned_foreign_amount, fx_rate_home_per_foreign, day } => json!({
            "home_country_name": home_country_name.as_ref(), "foreign_country_name": foreign_country_name.as_ref(),
            "person_id": person_id, "returned_home_amount": returned_home_amount, "returned_foreign_amount": returned_foreign_amount,
            "fx_rate_home_per_foreign": fx_rate_home_per_foreign, "day": day
        }),
        SimEvent::FxRateChanged { country_name, currency_code, old_rate, new_rate, pressure, reserves_base, day } => json!({
            "country_name": country_name.as_ref(), "currency_code": currency_code.as_ref(),
            "old_rate": old_rate, "new_rate": new_rate, "pressure": pressure, "reserves_base": reserves_base, "day": day
        }),
        SimEvent::FxReservePressure { country_name, currency_code, reserves_base, reserve_gap, pressure, day } => json!({
            "country_name": country_name.as_ref(), "currency_code": currency_code.as_ref(),
            "reserves_base": reserves_base, "reserve_gap": reserve_gap, "pressure": pressure, "day": day
        }),
        SimEvent::DisasterApplied { country_name, disaster_label, intensity_pct, summary, day } => json!({
            "country_name": country_name.as_ref(), "disaster_label": disaster_label.as_ref(),
            "intensity_pct": intensity_pct, "summary": summary.as_ref(), "day": day
        }),
        SimEvent::HomePurchased { country_name, person_id, price, day } => json!({
            "country_name": country_name.as_ref(), "person_id": person_id, "price": price, "day": day
        }),
        SimEvent::Foreclosure { country_name, person_id, principal_written_off, day } => json!({
            "country_name": country_name.as_ref(), "person_id": person_id, "principal_written_off": principal_written_off, "day": day
        }),
        SimEvent::MortgagePaidOff { country_name, person_id, day } => json!({
            "country_name": country_name.as_ref(), "person_id": person_id, "day": day
        }),
        SimEvent::HousingPriceChanged { country_name, new_price_index, change_pct, day } => json!({
            "country_name": country_name.as_ref(), "new_price_index": new_price_index, "change_pct": change_pct, "day": day
        }),
        SimEvent::RentUnpaid { country_name, person_id, amount_due, amount_paid, day } => json!({
            "country_name": country_name.as_ref(), "person_id": person_id, "amount_due": amount_due, "amount_paid": amount_paid, "day": day
        }),
    }
}
