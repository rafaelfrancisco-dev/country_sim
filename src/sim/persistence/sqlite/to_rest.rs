use std::collections::HashMap;

use crate::sim::{
    bank::Bank,
    essentials::Household,
    event::{EntityKind, IpoReason, SimEvent},
    stock_market::{ListedEquity, Portfolio, StockMarket},
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
    let kind = sim_event_kind_str(event).to_string();
    let day = sim_event_day(event);
    let mut dto = blank_event_dto(kind, day);

    match event {
        SimEvent::PopBirth { country_name, count, .. }
        | SimEvent::PopDeath { country_name, count, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.count = Some(*count);
        }
        SimEvent::LoanEmitted { country_name, entity_kind, entity_id, principal, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.entity_kind = Some(entity_kind_to_code(entity_kind));
            dto.entity_id = Some(*entity_id);
            dto.principal = Some(*principal);
        }
        SimEvent::LoanEnded { country_name, entity_kind, entity_id, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.entity_kind = Some(entity_kind_to_code(entity_kind));
            dto.entity_id = Some(*entity_id);
        }
        SimEvent::LoanWrittenOff { country_name, entity_id, principal_lost, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.entity_id = Some(*entity_id);
            dto.principal_lost = Some(*principal_lost);
        }
        SimEvent::DebtDistress { country_name, entity_kind, entity_id, unpaid_amount, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.entity_kind = Some(entity_kind_to_code(entity_kind));
            dto.entity_id = Some(*entity_id);
            dto.unpaid_amount = Some(*unpaid_amount);
        }
        SimEvent::GovTransfer { country_name, total_recipients, total_amount, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.total_recipients = Some(*total_recipients);
            dto.total_amount = Some(*total_amount);
        }
        SimEvent::GovCompanyFunded {
            country_name,
            company_id,
            produced_good_id,
            total_funding,
            treasury_funding,
            deficit_funding,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.produced_good_id = Some(*produced_good_id);
            dto.total_funding = Some(*total_funding);
            dto.treasury_funding = Some(*treasury_funding);
            dto.deficit_funding = Some(*deficit_funding);
        }
        SimEvent::CompanyFounded {
            country_name,
            company_id,
            company_name,
            founders,
            job_type_id,
            produced_good_id,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.company_name = Some(company_name.to_string());
            dto.founders = Some(*founders);
            dto.job_type_id = Some(*job_type_id);
            dto.produced_good_id = Some(*produced_good_id);
        }
        SimEvent::BankFounded {
            country_name,
            bank_id,
            bank_name,
            kind,
            seed_capital,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.bank_name = Some(bank_name.to_string());
            dto.bank_kind = Some(bank_kind_to_code(*kind));
            dto.seed_capital = Some(*seed_capital);
        }
        SimEvent::BankRunStarted { country_name, bank_id, bank_name, health, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.bank_name = Some(bank_name.to_string());
            dto.health = Some(*health);
        }
        SimEvent::BankRunWithdrawal {
            country_name,
            bank_id,
            entities_affected,
            requested,
            haircut_loss,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.entities_affected = Some(*entities_affected);
            dto.requested = Some(*requested);
            dto.haircut_loss = Some(*haircut_loss);
        }
        SimEvent::BankResolutionStarted { country_name, bank_id, treasury_support, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.treasury_support = Some(*treasury_support);
        }
        SimEvent::BankResolved { country_name, bank_id, bank_name, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.bank_name = Some(bank_name.to_string());
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
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.bank_name = Some(bank_name.to_string());
            dto.acquired_by_bank_id = Some(*acquired_by_bank_id);
            dto.acquired_by_bank_name = Some(acquired_by_bank_name.to_string());
            dto.migrated_depositors = Some(*migrated_depositors);
            dto.migrated_companies = Some(*migrated_companies);
            dto.migrated_loans = Some(*migrated_loans);
            dto.transferred_reserves = Some(*transferred_reserves);
        }
        SimEvent::BankDepositHaircut { country_name, bank_id, entity_kind, entity_id, amount_lost, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.bank_id = Some(*bank_id);
            dto.entity_kind = Some(entity_kind_to_code(entity_kind));
            dto.entity_id = Some(*entity_id);
            dto.amount_lost = Some(*amount_lost);
        }
        SimEvent::CompanyWageCut { country_name, company_id, cut_rate, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.cut_rate = Some(*cut_rate);
        }
        SimEvent::CompanyLayoffs { country_name, company_id, laid_off_count, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.laid_off_count = Some(*laid_off_count);
        }
        SimEvent::LaborChurnMonthly {
            country_name,
            quits,
            retirements,
            layoffs_non_distress,
            replacement_openings,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.quits = Some(*quits);
            dto.retirements = Some(*retirements);
            dto.layoffs_non_distress = Some(*layoffs_non_distress);
            dto.replacement_openings = Some(*replacement_openings);
        }
        SimEvent::JobSwitched {
            country_name,
            person_id,
            from_company_id,
            to_company_id,
            monthly_salary,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.from_company_id = Some(*from_company_id);
            dto.to_company_id = Some(*to_company_id);
            dto.monthly_salary = Some(*monthly_salary);
        }
        SimEvent::PersonMigratedForWork {
            from_country_name,
            to_country_name,
            person_id,
            job_type_id,
            monthly_salary,
            ..
        } => {
            dto.from_country_name = Some(from_country_name.to_string());
            dto.to_country_name = Some(to_country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.job_type_id = Some(*job_type_id);
            dto.monthly_salary = Some(*monthly_salary);
        }
        SimEvent::PersonMigratedForDistress {
            from_country_name,
            to_country_name,
            person_id,
            months_unemployed,
            months_on_benefits,
            ..
        } => {
            dto.from_country_name = Some(from_country_name.to_string());
            dto.to_country_name = Some(to_country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.months_unemployed = Some(*months_unemployed);
            dto.months_on_benefits = Some(*months_on_benefits);
        }
        SimEvent::CompanyBankrupt {
            country_name,
            company_id,
            company_name,
            laid_off_count,
            principal_written_off,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.company_name = Some(company_name.to_string());
            dto.laid_off_count = Some(*laid_off_count);
            dto.principal_written_off = Some(*principal_written_off);
        }
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
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.company_name = Some(company_name.to_string());
            dto.ticker = Some(ticker.to_string());
            dto.ipo_reason = Some(ipo_reason_to_code(reason));
            dto.shares_issued = Some(*shares_issued);
            dto.primary_shares_issued = Some(*primary_shares_issued);
            dto.secondary_shares_sold = Some(*secondary_shares_sold);
            dto.price_per_share = Some(*price_per_share);
            dto.total_raised = Some(*total_raised);
            dto.primary_raised = Some(*primary_raised);
            dto.secondary_raised = Some(*secondary_raised);
            dto.founder_payout_total = Some(*founder_payout_total);
        }
        SimEvent::StockDividend { country_name, company_id, ticker, total_amount, per_share, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.ticker = Some(ticker.to_string());
            dto.total_amount = Some(*total_amount);
            dto.per_share = Some(*per_share);
        }
        SimEvent::StockMarginCall { country_name, person_id, shares_liquidated, proceeds, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.shares_liquidated = Some(*shares_liquidated);
            dto.proceeds = Some(*proceeds);
        }
        SimEvent::StockDelisted { country_name, company_id, company_name, shares_wiped, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.company_name = Some(company_name.to_string());
            dto.shares_wiped = Some(*shares_wiped);
        }
        SimEvent::StockMarketClose { country_name, total_market_cap, total_turnover, listed_count, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.total_market_cap = Some(*total_market_cap);
            dto.total_turnover = Some(*total_turnover);
            dto.listed_count = Some(*listed_count as u64);
        }
        SimEvent::RatePolicyChange {
            country_name,
            new_rate,
            old_rate,
            inflation_rate,
            unemployment_rate,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.new_rate = Some(*new_rate);
            dto.old_rate = Some(*old_rate);
            dto.inflation_rate = Some(*inflation_rate);
            dto.unemployment_rate = Some(*unemployment_rate);
        }
        SimEvent::BondIssued { country_name, face_value, coupon_rate, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.face_value = Some(*face_value);
            dto.coupon_rate = Some(*coupon_rate);
        }
        SimEvent::CouponPaid { country_name, total_coupon, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.total_coupon = Some(*total_coupon);
        }
        SimEvent::BondMatured { country_name, face_value, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.face_value = Some(*face_value);
        }
        SimEvent::BondDefault { country_name, shortfall, monetized, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.shortfall = Some(*shortfall);
            dto.monetized = Some(*monetized);
        }
        SimEvent::MoneyMinted { country_name, amount, inflation_shortfall, unemployment_gap, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.amount = Some(*amount);
            dto.inflation_shortfall = Some(*inflation_shortfall);
            dto.unemployment_gap = Some(*unemployment_gap);
        }
        SimEvent::StockSecondaryOffering { country_name, company_id, ticker, shares_issued, price, total_raised, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.company_id = Some(*company_id);
            dto.ticker = Some(ticker.to_string());
            dto.shares_issued = Some(*shares_issued);
            dto.price = Some(*price);
            dto.total_raised = Some(*total_raised);
        }
        SimEvent::ForeignDepositOpened {
            home_country_name,
            foreign_country_name,
            person_id,
            bank_id,
            home_amount,
            foreign_amount,
            fx_rate_home_per_foreign,
            annual_rate,
            ..
        } => {
            dto.home_country_name = Some(home_country_name.to_string());
            dto.foreign_country_name = Some(foreign_country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.bank_id = Some(*bank_id);
            dto.home_amount = Some(*home_amount);
            dto.foreign_amount = Some(*foreign_amount);
            dto.fx_rate_home_per_foreign = Some(*fx_rate_home_per_foreign);
            dto.annual_rate = Some(*annual_rate);
        }
        SimEvent::ForeignDepositClosed {
            home_country_name,
            foreign_country_name,
            person_id,
            returned_home_amount,
            returned_foreign_amount,
            fx_rate_home_per_foreign,
            ..
        } => {
            dto.home_country_name = Some(home_country_name.to_string());
            dto.foreign_country_name = Some(foreign_country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.returned_home_amount = Some(*returned_home_amount);
            dto.returned_foreign_amount = Some(*returned_foreign_amount);
            dto.fx_rate_home_per_foreign = Some(*fx_rate_home_per_foreign);
        }
        SimEvent::FxRateChanged {
            country_name,
            currency_code,
            old_rate,
            new_rate,
            pressure,
            reserves_base,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.currency_code = Some(currency_code.to_string());
            dto.old_rate = Some(*old_rate);
            dto.new_rate = Some(*new_rate);
            dto.pressure = Some(*pressure);
            dto.reserves_base = Some(*reserves_base);
        }
        SimEvent::FxReservePressure {
            country_name,
            currency_code,
            reserves_base,
            reserve_gap,
            pressure,
            ..
        } => {
            dto.country_name = Some(country_name.to_string());
            dto.currency_code = Some(currency_code.to_string());
            dto.reserves_base = Some(*reserves_base);
            dto.reserve_gap = Some(*reserve_gap);
            dto.pressure = Some(*pressure);
        }
        SimEvent::DisasterApplied { country_name, disaster_label, intensity_pct, summary, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.disaster_label = Some(disaster_label.to_string());
            dto.intensity_pct = Some(*intensity_pct);
            dto.summary = Some(summary.to_string());
        }
        SimEvent::HomePurchased { country_name, person_id, price, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.price = Some(*price);
        }
        SimEvent::Foreclosure { country_name, person_id, principal_written_off, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.principal_written_off = Some(*principal_written_off);
        }
        SimEvent::MortgagePaidOff { country_name, person_id, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.person_id = Some(*person_id);
        }
        SimEvent::HousingPriceChanged { country_name, new_price_index, change_pct, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.new_price_index = Some(*new_price_index);
            dto.change_pct = Some(*change_pct);
        }
        SimEvent::RentUnpaid { country_name, person_id, amount_due, amount_paid, .. } => {
            dto.country_name = Some(country_name.to_string());
            dto.person_id = Some(*person_id);
            dto.amount_due = Some(*amount_due);
            dto.amount_paid = Some(*amount_paid);
        }
    }

    dto
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

fn blank_event_dto(kind: String, day: u64) -> SimEventDto {
    SimEventDto {
        kind,
        day,
        entity_kind: None,
        bank_kind: None,
        ipo_reason: None,
        country_name: None,
        from_country_name: None,
        to_country_name: None,
        home_country_name: None,
        foreign_country_name: None,
        currency_code: None,
        company_name: None,
        bank_name: None,
        acquired_by_bank_name: None,
        ticker: None,
        disaster_label: None,
        summary: None,
        count: None,
        entity_id: None,
        person_id: None,
        company_id: None,
        bank_id: None,
        acquired_by_bank_id: None,
        from_company_id: None,
        to_company_id: None,
        produced_good_id: None,
        founders: None,
        job_type_id: None,
        intensity_pct: None,
        months_unemployed: None,
        months_on_benefits: None,
        total_recipients: None,
        entities_affected: None,
        migrated_depositors: None,
        migrated_companies: None,
        migrated_loans: None,
        quits: None,
        retirements: None,
        layoffs_non_distress: None,
        replacement_openings: None,
        laid_off_count: None,
        listed_count: None,
        shares_issued: None,
        primary_shares_issued: None,
        secondary_shares_sold: None,
        shares_liquidated: None,
        shares_wiped: None,
        principal: None,
        principal_lost: None,
        unpaid_amount: None,
        total_amount: None,
        total_funding: None,
        treasury_funding: None,
        deficit_funding: None,
        seed_capital: None,
        health: None,
        requested: None,
        haircut_loss: None,
        treasury_support: None,
        amount_lost: None,
        cut_rate: None,
        monthly_salary: None,
        principal_written_off: None,
        price_per_share: None,
        total_raised: None,
        primary_raised: None,
        secondary_raised: None,
        founder_payout_total: None,
        per_share: None,
        proceeds: None,
        total_market_cap: None,
        total_turnover: None,
        new_rate: None,
        old_rate: None,
        inflation_rate: None,
        unemployment_rate: None,
        face_value: None,
        coupon_rate: None,
        total_coupon: None,
        shortfall: None,
        monetized: None,
        amount: None,
        inflation_shortfall: None,
        unemployment_gap: None,
        price: None,
        home_amount: None,
        foreign_amount: None,
        fx_rate_home_per_foreign: None,
        annual_rate: None,
        returned_home_amount: None,
        returned_foreign_amount: None,
        reserves_base: None,
        reserve_gap: None,
        pressure: None,
        new_price_index: None,
        change_pct: None,
        amount_due: None,
        amount_paid: None,
        transferred_reserves: None,
    }
}

fn entity_kind_to_code(kind: &EntityKind) -> u8 {
    match kind {
        EntityKind::Person => 0,
        EntityKind::Company => 1,
        EntityKind::Country => 2,
    }
}

fn bank_kind_to_code(kind: crate::sim::bank::BankKind) -> u8 {
    match kind {
        crate::sim::bank::BankKind::Public => 0,
        crate::sim::bank::BankKind::Private => 1,
    }
}

fn ipo_reason_to_code(reason: &IpoReason) -> u8 {
    match reason {
        IpoReason::Seed => 0,
        IpoReason::Growth => 1,
        IpoReason::Distress => 2,
    }
}

fn sim_event_day(event: &SimEvent) -> u64 {
    match event {
        SimEvent::PopBirth { day, .. }
        | SimEvent::PopDeath { day, .. }
        | SimEvent::LoanEmitted { day, .. }
        | SimEvent::LoanEnded { day, .. }
        | SimEvent::LoanWrittenOff { day, .. }
        | SimEvent::DebtDistress { day, .. }
        | SimEvent::GovTransfer { day, .. }
        | SimEvent::GovCompanyFunded { day, .. }
        | SimEvent::CompanyFounded { day, .. }
        | SimEvent::BankFounded { day, .. }
        | SimEvent::BankRunStarted { day, .. }
        | SimEvent::BankRunWithdrawal { day, .. }
        | SimEvent::BankResolutionStarted { day, .. }
        | SimEvent::BankResolved { day, .. }
        | SimEvent::BankClosed { day, .. }
        | SimEvent::BankDepositHaircut { day, .. }
        | SimEvent::CompanyWageCut { day, .. }
        | SimEvent::CompanyLayoffs { day, .. }
        | SimEvent::LaborChurnMonthly { day, .. }
        | SimEvent::JobSwitched { day, .. }
        | SimEvent::PersonMigratedForWork { day, .. }
        | SimEvent::PersonMigratedForDistress { day, .. }
        | SimEvent::CompanyBankrupt { day, .. }
        | SimEvent::StockIpo { day, .. }
        | SimEvent::StockDividend { day, .. }
        | SimEvent::StockMarginCall { day, .. }
        | SimEvent::StockDelisted { day, .. }
        | SimEvent::StockMarketClose { day, .. }
        | SimEvent::RatePolicyChange { day, .. }
        | SimEvent::BondIssued { day, .. }
        | SimEvent::CouponPaid { day, .. }
        | SimEvent::BondMatured { day, .. }
        | SimEvent::BondDefault { day, .. }
        | SimEvent::MoneyMinted { day, .. }
        | SimEvent::StockSecondaryOffering { day, .. }
        | SimEvent::ForeignDepositOpened { day, .. }
        | SimEvent::ForeignDepositClosed { day, .. }
        | SimEvent::FxRateChanged { day, .. }
        | SimEvent::FxReservePressure { day, .. }
        | SimEvent::DisasterApplied { day, .. }
        | SimEvent::HomePurchased { day, .. }
        | SimEvent::Foreclosure { day, .. }
        | SimEvent::MortgagePaidOff { day, .. }
        | SimEvent::HousingPriceChanged { day, .. }
        | SimEvent::RentUnpaid { day, .. } => *day,
    }
}
