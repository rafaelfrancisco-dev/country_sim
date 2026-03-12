use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use crate::sim::{
    bank::{Bank, BankKind, BankStockPosition},
    company::Company,
    essentials::Household,
    event::{EntityKind, IpoReason, SimEvent},
    stock_market::{
        DailyIndexPoint, ListedEquity, Portfolio, Position, StockMarket, StockSecurityId,
    },
};

use super::super::dto::{
    BankDto, CompanyDto, HouseholdDto, ListedEquityDto, PortfolioDto, SimEventDto, StockMarketDto,
};

pub(super) fn company_from_dto(dto: CompanyDto) -> Company {
    use super::from::loan_account_from_dto;

    let open_positions = dto
        .open_positions
        .into_iter()
        .map(|op| crate::sim::company::JobOpening {
            job_type_id: op.job_type_id,
            slots: op.slots,
            offered_monthly_salary: op.offered_monthly_salary,
        })
        .collect();

    let founder_stakes = dto
        .founder_stakes
        .into_iter()
        .map(|fs| crate::sim::company::FounderStake {
            person_id: fs.person_id,
            shares_owned: fs.shares_owned,
            cash_contributed: fs.cash_contributed,
        })
        .collect();

    Company {
        id: dto.id,
        historical_ids: dto.historical_ids,
        name: Arc::from(dto.name.as_str()),
        is_public_utility: dto.is_public_utility,
        is_essential_backfill: dto.is_essential_backfill,
        offered_job_types: dto.offered_job_types,
        produced_good_id: dto.produced_good_id,
        open_positions,
        founder_stakes,
        employee_count: dto.employee_count,
        manufacturing_employee_count: dto.manufacturing_employee_count,
        inventory_units: dto.inventory_units,
        unit_price: dto.unit_price,
        wage_offer_multiplier: dto.wage_offer_multiplier,
        consecutive_missed_loan_payments: dto.consecutive_missed_loan_payments,
        missed_loan_payment_this_month: dto.missed_loan_payment_this_month,
        production_effort_carry: dto.production_effort_carry,
        cash_balance: dto.cash_balance,
        deposit_bank_id: dto.deposit_bank_id,
        monthly_payroll_budget: dto.monthly_payroll_budget,
        last_month_payroll_paid: dto.last_month_payroll_paid,
        last_completed_month_payroll_paid: dto.last_completed_month_payroll_paid,
        last_month_sales: dto.last_month_sales,
        last_month_profit: dto.last_month_profit,
        last_completed_month_profit: dto.last_completed_month_profit,
        consecutive_profitable_months: dto.consecutive_profitable_months,
        consecutive_loss_months: dto.consecutive_loss_months,
        last_month_loan_payment: dto.last_month_loan_payment,
        last_completed_month_loan_payment: dto.last_completed_month_loan_payment,
        last_month_new_loan: dto.last_month_new_loan,
        last_completed_month_new_loan: dto.last_completed_month_new_loan,
        last_month_savings_interest: dto.last_month_savings_interest,
        last_completed_month_savings_interest: dto.last_completed_month_savings_interest,
        last_month_units_produced: dto.last_month_units_produced,
        last_month_units_sold: dto.last_month_units_sold,
        last_month_export_units_sold: dto.last_month_export_units_sold,
        last_month_domestic_units_sold: dto.last_month_domestic_units_sold,
        last_month_sell_through: dto.last_month_sell_through,
        last_completed_month_sales: dto.last_completed_month_sales,
        last_completed_month_units_produced: dto.last_completed_month_units_produced,
        last_completed_month_units_sold: dto.last_completed_month_units_sold,
        last_completed_month_export_units_sold: dto.last_completed_month_export_units_sold,
        last_completed_month_domestic_units_sold: dto.last_completed_month_domestic_units_sold,
        last_completed_month_sell_through: dto.last_completed_month_sell_through,
        sales_history: VecDeque::from(dto.sales_history),
        profit_history: VecDeque::from(dto.profit_history),
        sold_units_history: VecDeque::from(dto.sold_units_history),
        employee_count_history: VecDeque::from(dto.employee_count_history),
        price_history: VecDeque::from(dto.price_history),
        loan_payment_history: VecDeque::from(dto.loan_payment_history),
        cash_balance_history: VecDeque::from(dto.cash_balance_history),
        age_months: dto.age_months,
        ipo_cooldown_months: dto.ipo_cooldown_months,
        failed_ipo_attempts: dto.failed_ipo_attempts,
        loan: dto.loan.map(loan_account_from_dto),
        rental_units_owned: dto.rental_units_owned,
        last_month_rent_revenue: dto.last_month_rent_revenue,
    }
}

pub(super) fn bank_from_dto(dto: BankDto) -> Bank {
    let kind = match dto.kind {
        1 => BankKind::Private,
        _ => BankKind::Public,
    };

    let open_positions = dto
        .open_positions
        .into_iter()
        .map(|op| crate::sim::bank::JobOpening {
            job_type_id: op.job_type_id,
            slots: op.slots,
            offered_monthly_salary: op.offered_monthly_salary,
        })
        .collect();

    let stock_positions = dto
        .stock_positions
        .into_iter()
        .map(|sp| BankStockPosition {
            security_id: StockSecurityId {
                country_id: sp.security_country_id,
                company_id: sp.security_company_id,
            },
            shares: sp.shares,
            avg_cost: sp.avg_cost,
        })
        .collect();

    let fx_reserve_assets_by_source = dto
        .fx_reserve_assets_by_source
        .into_iter()
        .filter_map(|(country_idx, amount)| {
            country_idx
                .parse::<usize>()
                .ok()
                .map(|country_idx| (country_idx, amount))
        })
        .collect();

    Bank {
        id: dto.id,
        name: Arc::from(dto.name.as_str()),
        kind,
        reserve_balance: dto.reserve_balance,
        fx_reserve_assets_base: dto.fx_reserve_assets_base,
        fx_reserve_assets_by_source,
        central_bank_liquidity_liabilities_local: dto.central_bank_liquidity_liabilities_local,
        created_money_outstanding: dto.created_money_outstanding,
        total_originated: dto.total_originated,
        total_principal_repaid: dto.total_principal_repaid,
        deposit_liabilities: dto.deposit_liabilities,
        foreign_deposit_liabilities: dto.foreign_deposit_liabilities,
        equity_capital: dto.equity_capital,
        run_intensity: dto.run_intensity,
        is_in_resolution: dto.is_in_resolution,
        months_in_resolution: dto.months_in_resolution,
        resolution_distress_streak: dto.resolution_distress_streak,
        resolution_stable_streak: dto.resolution_stable_streak,
        open_positions,
        employee_count: dto.employee_count,
        monthly_payroll_budget: dto.monthly_payroll_budget,
        last_month_payroll_paid: dto.last_month_payroll_paid,
        wage_offer_multiplier: dto.wage_offer_multiplier,
        last_month_interest_income: dto.last_month_interest_income,
        last_month_deposit_interest_expense: dto.last_month_deposit_interest_expense,
        last_month_credit_losses: dto.last_month_credit_losses,
        last_month_profit: dto.last_month_profit,
        stock_positions,
    }
}

pub(super) fn household_from_dto(dto: HouseholdDto) -> Household {
    Household {
        id: dto.id,
        member_count: dto.member_count,
        sheltered: dto.sheltered,
    }
}

pub(super) fn stock_market_from_dto(dto: StockMarketDto) -> StockMarket {
    let person_portfolios: HashMap<u32, Portfolio> = dto
        .person_portfolios
        .into_iter()
        .filter_map(|(k, v)| k.parse::<u32>().ok().map(|ki| (ki, portfolio_from_dto(v))))
        .collect();

    let listed: HashMap<u16, ListedEquity> = dto
        .listed
        .into_iter()
        .filter_map(|(k, v)| {
            k.parse::<u16>()
                .ok()
                .map(|ki| (ki, listed_equity_from_dto(v)))
        })
        .collect();

    let daily_index_history: VecDeque<DailyIndexPoint> = dto
        .daily_index_history
        .into_iter()
        .map(|p| DailyIndexPoint {
            day: p.day,
            index_value: p.index_value,
            total_turnover: p.total_turnover,
            bank_equity_exposure: p.bank_equity_exposure,
        })
        .collect();

    StockMarket {
        person_portfolios,
        bank_portfolio: portfolio_from_dto(dto.bank_portfolio),
        listed,
        daily_index_history,
        index_base: dto.index_base,
        ipo_raised_mtd: dto.ipo_raised_mtd,
        dividends_paid_mtd: dto.dividends_paid_mtd,
        last_completed_month_dividends_paid: dto.last_completed_month_dividends_paid,
        forced_liquidations_mtd: dto.forced_liquidations_mtd,
        last_completed_month_ipo_raised: dto.last_completed_month_ipo_raised,
        last_completed_month_forced_liquidations: dto.last_completed_month_forced_liquidations,
        day_active_traders_count: 0,
        day_buy_orders_count: 0,
        day_sell_orders_count: 0,
    }
}

pub(super) fn portfolio_from_dto(dto: PortfolioDto) -> Portfolio {
    Portfolio {
        positions: dto
            .positions
            .into_iter()
            .map(|p| Position {
                security_id: StockSecurityId {
                    country_id: p.security_country_id,
                    company_id: p.security_company_id,
                },
                shares: p.shares,
                avg_cost: p.avg_cost,
                pledged_shares: p.pledged_shares,
            })
            .collect(),
        realized_pnl_mtd: dto.realized_pnl_mtd,
        dividends_mtd: dto.dividends_mtd,
        last_completed_month_dividends: dto.last_completed_month_dividends,
        last_completed_month_realized_pnl: dto.last_completed_month_realized_pnl,
    }
}

pub(super) fn listed_equity_from_dto(dto: ListedEquityDto) -> ListedEquity {
    let mut monthly_dps = [0.0f64; 12];
    for (i, v) in dto.monthly_dps.into_iter().enumerate() {
        if i < 12 {
            monthly_dps[i] = v;
        }
    }

    ListedEquity {
        company_id: dto.company_id,
        current_company_id: dto.current_company_id,
        ticker: Arc::from(dto.ticker.as_str()),
        shares_outstanding: dto.shares_outstanding,
        free_float_shares: dto.free_float_shares,
        last_price: dto.last_price,
        prev_close: dto.prev_close,
        last_closed_day_pct_change: dto.last_closed_day_pct_change,
        day_volume: dto.day_volume,
        day_turnover: dto.day_turnover,
        day_trades_count: dto.day_trades_count,
        market_cap: dto.market_cap,
        price_history: VecDeque::from(dto.price_history),
        volume_history: VecDeque::from(dto.volume_history),
        market_cap_history: VecDeque::from(dto.market_cap_history),
        monthly_dps,
        dps_cursor: dto.dps_cursor,
    }
}

pub(super) fn sim_event_from_dto(dto: SimEventDto) -> SimEvent {
    match dto.kind.as_str() {
        "PopBirth" => SimEvent::PopBirth {
            country_name: opt_arc(dto.country_name),
            count: dto.count.unwrap_or(0),
            day: dto.day,
        },
        "PopDeath" => SimEvent::PopDeath {
            country_name: opt_arc(dto.country_name),
            count: dto.count.unwrap_or(0),
            day: dto.day,
        },
        "LoanEmitted" => SimEvent::LoanEmitted {
            country_name: opt_arc(dto.country_name),
            entity_kind: entity_kind_from_code(dto.entity_kind),
            entity_id: dto.entity_id.unwrap_or(0),
            principal: dto.principal.unwrap_or(0.0),
            day: dto.day,
        },
        "LoanEnded" => SimEvent::LoanEnded {
            country_name: opt_arc(dto.country_name),
            entity_kind: entity_kind_from_code(dto.entity_kind),
            entity_id: dto.entity_id.unwrap_or(0),
            day: dto.day,
        },
        "LoanWrittenOff" => SimEvent::LoanWrittenOff {
            country_name: opt_arc(dto.country_name),
            entity_id: dto.entity_id.unwrap_or(0),
            principal_lost: dto.principal_lost.unwrap_or(0.0),
            day: dto.day,
        },
        "DebtDistress" => SimEvent::DebtDistress {
            country_name: opt_arc(dto.country_name),
            entity_kind: entity_kind_from_code(dto.entity_kind),
            entity_id: dto.entity_id.unwrap_or(0),
            unpaid_amount: dto.unpaid_amount.unwrap_or(0.0),
            day: dto.day,
        },
        "GovTransfer" => SimEvent::GovTransfer {
            country_name: opt_arc(dto.country_name),
            total_recipients: dto.total_recipients.unwrap_or(0),
            total_amount: dto.total_amount.unwrap_or(0.0),
            day: dto.day,
        },
        "GovCompanyFunded" => SimEvent::GovCompanyFunded {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            produced_good_id: dto.produced_good_id.unwrap_or(0),
            total_funding: dto.total_funding.unwrap_or(0.0),
            treasury_funding: dto.treasury_funding.unwrap_or(0.0),
            deficit_funding: dto.deficit_funding.unwrap_or(0.0),
            day: dto.day,
        },
        "CompanyFounded" => SimEvent::CompanyFounded {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            company_name: opt_arc(dto.company_name),
            founders: dto.founders.unwrap_or(0),
            job_type_id: dto.job_type_id.unwrap_or(0),
            produced_good_id: dto.produced_good_id.unwrap_or(0),
            day: dto.day,
        },
        "BankFounded" => SimEvent::BankFounded {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            bank_name: opt_arc(dto.bank_name),
            kind: bank_kind_from_code(dto.bank_kind),
            seed_capital: dto.seed_capital.unwrap_or(0.0),
            day: dto.day,
        },
        "BankRunStarted" => SimEvent::BankRunStarted {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            bank_name: opt_arc(dto.bank_name),
            health: dto.health.unwrap_or(0.0),
            day: dto.day,
        },
        "BankRunWithdrawal" => SimEvent::BankRunWithdrawal {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            entities_affected: dto.entities_affected.unwrap_or(0),
            requested: dto.requested.unwrap_or(0.0),
            haircut_loss: dto.haircut_loss.unwrap_or(0.0),
            day: dto.day,
        },
        "BankResolutionStarted" => SimEvent::BankResolutionStarted {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            treasury_support: dto.treasury_support.unwrap_or(0.0),
            day: dto.day,
        },
        "BankResolved" => SimEvent::BankResolved {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            bank_name: opt_arc(dto.bank_name),
            day: dto.day,
        },
        "BankClosed" => SimEvent::BankClosed {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            bank_name: opt_arc(dto.bank_name),
            acquired_by_bank_id: dto.acquired_by_bank_id.unwrap_or(0),
            acquired_by_bank_name: opt_arc(dto.acquired_by_bank_name),
            migrated_depositors: dto.migrated_depositors.unwrap_or(0),
            migrated_companies: dto.migrated_companies.unwrap_or(0),
            migrated_loans: dto.migrated_loans.unwrap_or(0),
            transferred_reserves: dto.transferred_reserves.unwrap_or(0.0),
            day: dto.day,
        },
        "BankDepositHaircut" => SimEvent::BankDepositHaircut {
            country_name: opt_arc(dto.country_name),
            bank_id: dto.bank_id.unwrap_or(0),
            entity_kind: entity_kind_from_code(dto.entity_kind),
            entity_id: dto.entity_id.unwrap_or(0),
            amount_lost: dto.amount_lost.unwrap_or(0.0),
            day: dto.day,
        },
        "CompanyWageCut" => SimEvent::CompanyWageCut {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            cut_rate: dto.cut_rate.unwrap_or(0.0),
            day: dto.day,
        },
        "CompanyLayoffs" => SimEvent::CompanyLayoffs {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            laid_off_count: dto.laid_off_count.unwrap_or(0),
            day: dto.day,
        },
        "BondIssued" => SimEvent::BondIssued {
            country_name: opt_arc(dto.country_name),
            face_value: dto.face_value.unwrap_or(0.0),
            coupon_rate: dto.coupon_rate.unwrap_or(0.0),
            day: dto.day,
        },
        "CouponPaid" => SimEvent::CouponPaid {
            country_name: opt_arc(dto.country_name),
            total_coupon: dto.total_coupon.unwrap_or(0.0),
            day: dto.day,
        },
        "BondMatured" => SimEvent::BondMatured {
            country_name: opt_arc(dto.country_name),
            face_value: dto.face_value.unwrap_or(0.0),
            day: dto.day,
        },
        "BondDefault" => SimEvent::BondDefault {
            country_name: opt_arc(dto.country_name),
            shortfall: dto.shortfall.unwrap_or(0.0),
            monetized: dto.monetized.unwrap_or(0.0),
            day: dto.day,
        },
        "MoneyMinted" => SimEvent::MoneyMinted {
            country_name: opt_arc(dto.country_name),
            amount: dto.amount.unwrap_or(0.0),
            inflation_shortfall: dto.inflation_shortfall.unwrap_or(0.0),
            unemployment_gap: dto.unemployment_gap.unwrap_or(0.0),
            day: dto.day,
        },
        "RatePolicyChange" => SimEvent::RatePolicyChange {
            country_name: opt_arc(dto.country_name),
            new_rate: dto.new_rate.unwrap_or(0.0),
            old_rate: dto.old_rate.unwrap_or(0.0),
            inflation_rate: dto.inflation_rate.unwrap_or(0.0),
            unemployment_rate: dto.unemployment_rate.unwrap_or(0.0),
            day: dto.day,
        },
        "FxRateChanged" => SimEvent::FxRateChanged {
            country_name: opt_arc(dto.country_name),
            currency_code: opt_arc(dto.currency_code),
            old_rate: dto.old_rate.unwrap_or(0.0),
            new_rate: dto.new_rate.unwrap_or(0.0),
            pressure: dto.pressure.unwrap_or(0.0),
            reserves_base: dto.reserves_base.unwrap_or(0.0),
            day: dto.day,
        },
        "FxReservePressure" => SimEvent::FxReservePressure {
            country_name: opt_arc(dto.country_name),
            currency_code: opt_arc(dto.currency_code),
            reserves_base: dto.reserves_base.unwrap_or(0.0),
            reserve_gap: dto.reserve_gap.unwrap_or(0.0),
            pressure: dto.pressure.unwrap_or(0.0),
            day: dto.day,
        },
        "HomePurchased" => SimEvent::HomePurchased {
            country_name: opt_arc(dto.country_name),
            person_id: dto.person_id.unwrap_or(0),
            price: dto.price.unwrap_or(0.0),
            day: dto.day,
        },
        "Foreclosure" => SimEvent::Foreclosure {
            country_name: opt_arc(dto.country_name),
            person_id: dto.person_id.unwrap_or(0),
            principal_written_off: dto.principal_written_off.unwrap_or(0.0),
            day: dto.day,
        },
        "MortgagePaidOff" => SimEvent::MortgagePaidOff {
            country_name: opt_arc(dto.country_name),
            person_id: dto.person_id.unwrap_or(0),
            day: dto.day,
        },
        "HousingPriceChanged" => SimEvent::HousingPriceChanged {
            country_name: opt_arc(dto.country_name),
            new_price_index: dto.new_price_index.unwrap_or(0.0),
            change_pct: dto.change_pct.unwrap_or(0.0),
            day: dto.day,
        },
        "RentUnpaid" => SimEvent::RentUnpaid {
            country_name: opt_arc(dto.country_name),
            person_id: dto.person_id.unwrap_or(0),
            amount_due: dto.amount_due.unwrap_or(0.0),
            amount_paid: dto.amount_paid.unwrap_or(0.0),
            day: dto.day,
        },
        "LaborChurnMonthly" => SimEvent::LaborChurnMonthly {
            country_name: opt_arc(dto.country_name),
            quits: dto.quits.unwrap_or(0),
            retirements: dto.retirements.unwrap_or(0),
            layoffs_non_distress: dto.layoffs_non_distress.unwrap_or(0),
            replacement_openings: dto.replacement_openings.unwrap_or(0),
            day: dto.day,
        },
        "JobSwitched" => SimEvent::JobSwitched {
            country_name: opt_arc(dto.country_name),
            person_id: dto.person_id.unwrap_or(0),
            from_company_id: dto.from_company_id.unwrap_or(0),
            to_company_id: dto.to_company_id.unwrap_or(0),
            monthly_salary: dto.monthly_salary.unwrap_or(0.0),
            day: dto.day,
        },
        "PersonMigratedForWork" => SimEvent::PersonMigratedForWork {
            from_country_name: opt_arc(dto.from_country_name),
            to_country_name: opt_arc(dto.to_country_name),
            person_id: dto.person_id.unwrap_or(0),
            job_type_id: dto.job_type_id.unwrap_or(0),
            monthly_salary: dto.monthly_salary.unwrap_or(0.0),
            day: dto.day,
        },
        "PersonMigratedForDistress" => SimEvent::PersonMigratedForDistress {
            from_country_name: opt_arc(dto.from_country_name),
            to_country_name: opt_arc(dto.to_country_name),
            person_id: dto.person_id.unwrap_or(0),
            months_unemployed: dto.months_unemployed.unwrap_or(0),
            months_on_benefits: dto.months_on_benefits.unwrap_or(0),
            day: dto.day,
        },
        "CompanyBankrupt" => SimEvent::CompanyBankrupt {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            company_name: opt_arc(dto.company_name),
            laid_off_count: dto.laid_off_count.unwrap_or(0),
            principal_written_off: dto.principal_written_off.unwrap_or(0.0),
            day: dto.day,
        },
        "StockIpo" => SimEvent::StockIpo {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            company_name: opt_arc(dto.company_name),
            ticker: opt_arc(dto.ticker),
            reason: ipo_reason_from_code(dto.ipo_reason),
            shares_issued: dto.shares_issued.unwrap_or(0),
            primary_shares_issued: dto.primary_shares_issued.unwrap_or(0),
            secondary_shares_sold: dto.secondary_shares_sold.unwrap_or(0),
            price_per_share: dto.price_per_share.unwrap_or(0.0),
            total_raised: dto.total_raised.unwrap_or(0.0),
            primary_raised: dto.primary_raised.unwrap_or(0.0),
            secondary_raised: dto.secondary_raised.unwrap_or(0.0),
            founder_payout_total: dto.founder_payout_total.unwrap_or(0.0),
            day: dto.day,
        },
        "StockDividend" => SimEvent::StockDividend {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            ticker: opt_arc(dto.ticker),
            total_amount: dto.total_amount.unwrap_or(0.0),
            per_share: dto.per_share.unwrap_or(0.0),
            day: dto.day,
        },
        "StockMarginCall" => SimEvent::StockMarginCall {
            country_name: opt_arc(dto.country_name),
            person_id: dto.person_id.unwrap_or(0),
            shares_liquidated: dto.shares_liquidated.unwrap_or(0),
            proceeds: dto.proceeds.unwrap_or(0.0),
            day: dto.day,
        },
        "StockDelisted" => SimEvent::StockDelisted {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            company_name: opt_arc(dto.company_name),
            shares_wiped: dto.shares_wiped.unwrap_or(0),
            day: dto.day,
        },
        "StockMarketClose" => SimEvent::StockMarketClose {
            country_name: opt_arc(dto.country_name),
            day: dto.day,
            total_market_cap: dto.total_market_cap.unwrap_or(0.0),
            total_turnover: dto.total_turnover.unwrap_or(0.0),
            listed_count: dto.listed_count.unwrap_or(0) as usize,
        },
        "StockSecondaryOffering" => SimEvent::StockSecondaryOffering {
            country_name: opt_arc(dto.country_name),
            company_id: dto.company_id.unwrap_or(0),
            ticker: opt_arc(dto.ticker),
            shares_issued: dto.shares_issued.unwrap_or(0),
            price: dto.price.unwrap_or(0.0),
            total_raised: dto.total_raised.unwrap_or(0.0),
            day: dto.day,
        },
        "ForeignDepositOpened" => SimEvent::ForeignDepositOpened {
            home_country_name: opt_arc(dto.home_country_name),
            foreign_country_name: opt_arc(dto.foreign_country_name),
            person_id: dto.person_id.unwrap_or(0),
            bank_id: dto.bank_id.unwrap_or(0),
            home_amount: dto.home_amount.unwrap_or(0.0),
            foreign_amount: dto.foreign_amount.unwrap_or(0.0),
            fx_rate_home_per_foreign: dto.fx_rate_home_per_foreign.unwrap_or(0.0),
            annual_rate: dto.annual_rate.unwrap_or(0.0),
            day: dto.day,
        },
        "ForeignDepositClosed" => SimEvent::ForeignDepositClosed {
            home_country_name: opt_arc(dto.home_country_name),
            foreign_country_name: opt_arc(dto.foreign_country_name),
            person_id: dto.person_id.unwrap_or(0),
            returned_home_amount: dto.returned_home_amount.unwrap_or(0.0),
            returned_foreign_amount: dto.returned_foreign_amount.unwrap_or(0.0),
            fx_rate_home_per_foreign: dto.fx_rate_home_per_foreign.unwrap_or(0.0),
            day: dto.day,
        },
        "DisasterApplied" => SimEvent::DisasterApplied {
            country_name: opt_arc(dto.country_name),
            disaster_label: opt_arc(dto.disaster_label),
            intensity_pct: dto.intensity_pct.unwrap_or(0),
            summary: opt_arc(dto.summary),
            day: dto.day,
        },
        _ => SimEvent::PopBirth {
            country_name: Arc::from("restored"),
            count: 0,
            day: 0,
        },
    }
}

fn opt_arc(value: Option<String>) -> Arc<str> {
    value.map(Arc::<str>::from).unwrap_or_else(|| Arc::from(""))
}

fn entity_kind_from_code(code: Option<u8>) -> EntityKind {
    match code.unwrap_or(0) {
        1 => EntityKind::Company,
        2 => EntityKind::Country,
        _ => EntityKind::Person,
    }
}

fn bank_kind_from_code(code: Option<u8>) -> BankKind {
    match code.unwrap_or(0) {
        1 => BankKind::Private,
        _ => BankKind::Public,
    }
}

fn ipo_reason_from_code(code: Option<u8>) -> IpoReason {
    match code.unwrap_or(0) {
        1 => IpoReason::Growth,
        2 => IpoReason::Distress,
        _ => IpoReason::Seed,
    }
}
