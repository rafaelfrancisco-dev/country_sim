use rusqlite::Connection;

use crate::sim::persistence::PersistenceError;
use crate::sim::persistence::dto::{BankMetricPointDto, MetricPointDto};

pub(super) fn load_metric_points(
    conn: &Connection,
    sql: &str,
    bind: impl rusqlite::Params,
) -> Result<Vec<MetricPointDto>, PersistenceError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(bind, metric_point_from_row)?;
    let mut points = Vec::new();
    for row in rows {
        points.push(row?);
    }
    Ok(points)
}

pub(super) fn load_bank_metric_points(
    conn: &Connection,
    sql: &str,
    bind: impl rusqlite::Params,
) -> Result<Vec<BankMetricPointDto>, PersistenceError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(bind, |row| {
        Ok(BankMetricPointDto {
            month: row.get::<_, i64>(0)? as u64,
            reserve: row.get(1)?,
            deposits: row.get(2)?,
            created_money: row.get(3)?,
            equity: row.get(4)?,
            run_intensity: row.get(5)?,
        })
    })?;
    let mut points = Vec::new();
    for row in rows {
        points.push(row?);
    }
    Ok(points)
}

pub(super) fn metric_point_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetricPointDto> {
    Ok(MetricPointDto {
        month: row.get::<_, i64>(0)? as u64,
        fx_rate_local_per_base: row.get(1)?,
        fx_reserves_base: row.get(2)?,
        fx_net_flow_base: row.get(3)?,
        fx_pressure: row.get(4)?,
        population: row.get(5)?,
        total_money_supply: row.get(6)?,
        household_cash_total: row.get(7)?,
        company_cash_total: row.get(8)?,
        expected_money_supply: row.get(9)?,
        money_supply_gap: row.get(10)?,
        created_money_outstanding: row.get(11)?,
        bank_reserve: row.get(12)?,
        treasury_balance: row.get(13)?,
        budget_balance: row.get(14)?,
        monthly_tax_collected: row.get(15)?,
        monthly_gov_spending: row.get(16)?,
        gov_debt: row.get(17)?,
        monthly_money_minted: row.get(18)?,
        cumulative_money_minted: row.get(19)?,
        monthly_mint_fraction: row.get(20)?,
        monthly_goods_import_spending: row.get(21)?,
        monthly_goods_export_revenue: row.get(22)?,
        company_input_spending: row.get(23)?,
        monthly_unmet_need_events: row.get(24)?,
        monthly_severe_unmet_need_events: row.get(25)?,
        monthly_chronic_unmet_need_count: row.get(26)?,
        monthly_essential_debt_spending: row.get(27)?,
        cpi_index: row.get(28)?,
        inflation_rate: row.get(29)?,
        policy_rate: row.get(30)?,
        deposit_rate: row.get(31)?,
        loan_rate: row.get(32)?,
        monthly_savings_interest_paid: row.get(33)?,
        bonds_outstanding: row.get(34)?,
        stock_market_cap: row.get(35)?,
        stock_bank_equity_exposure: row.get(36)?,
        housing_price_index: row.get(37)?,
        monthly_rent: row.get(38)?,
        homeownership_rate: row.get(39)?,
        housing_vacancy_rate: row.get(40)?,
        company_facility_units: row.get(41)?,
        monthly_foreclosures: row.get(42)?,
        monthly_home_purchases: row.get(43)?,
        monthly_immigrants: row.get(44)?,
        monthly_emigrants: row.get(45)?,
        monthly_migrants_for_work: row.get(46)?,
        monthly_migrants_for_distress: row.get(47)?,
    })
}
