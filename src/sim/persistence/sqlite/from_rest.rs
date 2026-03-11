use std::{collections::{HashMap, VecDeque}, sync::Arc};

use crate::sim::{
    bank::{Bank, BankKind, BankStockPosition},
    company::Company,
    essentials::Household,
    event::SimEvent,
    stock_market::{DailyIndexPoint, ListedEquity, Portfolio, Position, StockMarket, StockSecurityId},
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

    Bank {
        id: dto.id,
        name: Arc::from(dto.name.as_str()),
        kind,
        reserve_balance: dto.reserve_balance,
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
        .filter_map(|(k, v)| k.parse::<u16>().ok().map(|ki| (ki, listed_equity_from_dto(v))))
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
    // For round-trip, reconstruct events from payload.
    // Simple approach: reconstruct from known fields using json Value.
    // For events we can't reconstruct, fall back to a PopBirth placeholder.
    let p = &dto.payload;
    let get_str = |key: &str| -> Arc<str> {
        p.get(key)
            .and_then(|v| v.as_str())
            .map(|s| Arc::from(s))
            .unwrap_or_else(|| Arc::from(""))
    };
    let get_u64 = |key: &str| -> u64 {
        p.get(key).and_then(|v| v.as_u64()).unwrap_or(0)
    };
    let get_u32 = |key: &str| -> u32 { get_u64(key) as u32 };
    let get_u16 = |key: &str| -> u16 { get_u64(key) as u16 };
    let get_u8 = |key: &str| -> u8 { get_u64(key) as u8 };
    let get_f64 = |key: &str| -> f64 {
        p.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
    };

    match dto.kind.as_str() {
        "PopBirth" => SimEvent::PopBirth {
            country_name: get_str("country_name"),
            count: get_u32("count"),
            day: get_u64("day"),
        },
        "PopDeath" => SimEvent::PopDeath {
            country_name: get_str("country_name"),
            count: get_u32("count"),
            day: get_u64("day"),
        },
        "GovTransfer" => SimEvent::GovTransfer {
            country_name: get_str("country_name"),
            total_recipients: get_u32("total_recipients"),
            total_amount: get_f64("total_amount"),
            day: get_u64("day"),
        },
        "CompanyWageCut" => SimEvent::CompanyWageCut {
            country_name: get_str("country_name"),
            company_id: get_u16("company_id"),
            cut_rate: get_f64("cut_rate"),
            day: get_u64("day"),
        },
        "CompanyLayoffs" => SimEvent::CompanyLayoffs {
            country_name: get_str("country_name"),
            company_id: get_u16("company_id"),
            laid_off_count: get_u16("laid_off_count"),
            day: get_u64("day"),
        },
        "BondIssued" => SimEvent::BondIssued {
            country_name: get_str("country_name"),
            face_value: get_f64("face_value"),
            coupon_rate: get_f64("coupon_rate"),
            day: get_u64("day"),
        },
        "CouponPaid" => SimEvent::CouponPaid {
            country_name: get_str("country_name"),
            total_coupon: get_f64("total_coupon"),
            day: get_u64("day"),
        },
        "BondMatured" => SimEvent::BondMatured {
            country_name: get_str("country_name"),
            face_value: get_f64("face_value"),
            day: get_u64("day"),
        },
        "BondDefault" => SimEvent::BondDefault {
            country_name: get_str("country_name"),
            shortfall: get_f64("shortfall"),
            monetized: get_f64("monetized"),
            day: get_u64("day"),
        },
        "MoneyMinted" => SimEvent::MoneyMinted {
            country_name: get_str("country_name"),
            amount: get_f64("amount"),
            inflation_shortfall: get_f64("inflation_shortfall"),
            unemployment_gap: get_f64("unemployment_gap"),
            day: get_u64("day"),
        },
        "RatePolicyChange" => SimEvent::RatePolicyChange {
            country_name: get_str("country_name"),
            new_rate: get_f64("new_rate"),
            old_rate: get_f64("old_rate"),
            inflation_rate: get_f64("inflation_rate"),
            unemployment_rate: get_f64("unemployment_rate"),
            day: get_u64("day"),
        },
        "FxRateChanged" => SimEvent::FxRateChanged {
            country_name: get_str("country_name"),
            currency_code: get_str("currency_code"),
            old_rate: get_f64("old_rate"),
            new_rate: get_f64("new_rate"),
            pressure: get_f64("pressure"),
            reserves_base: get_f64("reserves_base"),
            day: get_u64("day"),
        },
        "FxReservePressure" => SimEvent::FxReservePressure {
            country_name: get_str("country_name"),
            currency_code: get_str("currency_code"),
            reserves_base: get_f64("reserves_base"),
            reserve_gap: get_f64("reserve_gap"),
            pressure: get_f64("pressure"),
            day: get_u64("day"),
        },
        "HomePurchased" => SimEvent::HomePurchased {
            country_name: get_str("country_name"),
            person_id: get_u32("person_id"),
            price: get_f64("price"),
            day: get_u64("day"),
        },
        "MortgagePaidOff" => SimEvent::MortgagePaidOff {
            country_name: get_str("country_name"),
            person_id: get_u32("person_id"),
            day: get_u64("day"),
        },
        "HousingPriceChanged" => SimEvent::HousingPriceChanged {
            country_name: get_str("country_name"),
            new_price_index: get_f64("new_price_index"),
            change_pct: get_f64("change_pct"),
            day: get_u64("day"),
        },
        "RentUnpaid" => SimEvent::RentUnpaid {
            country_name: get_str("country_name"),
            person_id: get_u32("person_id"),
            amount_due: get_f64("amount_due"),
            amount_paid: get_f64("amount_paid"),
            day: get_u64("day"),
        },
        "StockDividend" => SimEvent::StockDividend {
            country_name: get_str("country_name"),
            company_id: get_u16("company_id"),
            ticker: get_str("ticker"),
            total_amount: get_f64("total_amount"),
            per_share: get_f64("per_share"),
            day: get_u64("day"),
        },
        "StockMarginCall" => SimEvent::StockMarginCall {
            country_name: get_str("country_name"),
            person_id: get_u32("person_id"),
            shares_liquidated: get_u64("shares_liquidated"),
            proceeds: get_f64("proceeds"),
            day: get_u64("day"),
        },
        "StockDelisted" => SimEvent::StockDelisted {
            country_name: get_str("country_name"),
            company_id: get_u16("company_id"),
            company_name: get_str("company_name"),
            shares_wiped: get_u64("shares_wiped"),
            day: get_u64("day"),
        },
        "StockMarketClose" => SimEvent::StockMarketClose {
            country_name: get_str("country_name"),
            day: get_u64("day"),
            total_market_cap: get_f64("total_market_cap"),
            total_turnover: get_f64("total_turnover"),
            listed_count: get_u64("listed_count") as usize,
        },
        "LaborChurnMonthly" => SimEvent::LaborChurnMonthly {
            country_name: get_str("country_name"),
            quits: get_u32("quits"),
            retirements: get_u32("retirements"),
            layoffs_non_distress: get_u32("layoffs_non_distress"),
            replacement_openings: get_u32("replacement_openings"),
            day: get_u64("day"),
        },
        "JobSwitched" => SimEvent::JobSwitched {
            country_name: get_str("country_name"),
            person_id: get_u32("person_id"),
            from_company_id: get_u16("from_company_id"),
            to_company_id: get_u16("to_company_id"),
            monthly_salary: get_f64("monthly_salary"),
            day: get_u64("day"),
        },
        "CompanyBankrupt" => SimEvent::CompanyBankrupt {
            country_name: get_str("country_name"),
            company_id: get_u16("company_id"),
            company_name: get_str("company_name"),
            laid_off_count: get_u16("laid_off_count"),
            principal_written_off: get_f64("principal_written_off"),
            day: get_u64("day"),
        },
        "LoanWrittenOff" => SimEvent::LoanWrittenOff {
            country_name: get_str("country_name"),
            entity_id: get_u32("entity_id"),
            principal_lost: get_f64("principal_lost"),
            day: get_u64("day"),
        },
        "DisasterApplied" => SimEvent::DisasterApplied {
            country_name: get_str("country_name"),
            disaster_label: get_str("disaster_label"),
            intensity_pct: get_u8("intensity_pct"),
            summary: get_str("summary"),
            day: get_u64("day"),
        },
        // For remaining complex events, fall back to a simple placeholder
        _ => SimEvent::PopBirth {
            country_name: Arc::from("restored"),
            count: 0,
            day: 0,
        },
    }
}
