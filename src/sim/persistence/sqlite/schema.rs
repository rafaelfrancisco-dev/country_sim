use rusqlite::Connection;

use crate::sim::persistence::PersistenceError;

const SCHEMA_SQL: &str = include_str!("schema.sql");

const TABLES_TO_CLEAR: &[&str] = &[
    "events",
    "bank_history",
    "country_fx_source_history",
    "country_history",
    "world_history",
    "stock_market_daily_index_history",
    "stock_market_equity_monthly_dps",
    "stock_market_equity_market_cap_history",
    "stock_market_equity_volume_history",
    "stock_market_equity_price_history",
    "stock_market_listed_equities",
    "stock_market_bank_positions",
    "stock_market_person_positions",
    "stock_market_person_portfolios",
    "country_stock_markets",
    "bank_stock_positions",
    "bank_fx_reserve_sources",
    "bank_open_positions",
    "banks",
    "company_loans",
    "company_cash_balance_history",
    "company_loan_payment_history",
    "company_price_history",
    "company_employee_count_history",
    "company_sold_units_history",
    "company_profit_history",
    "company_sales_history",
    "company_historical_ids",
    "company_founder_stakes",
    "company_offered_job_types",
    "company_open_positions",
    "companies",
    "person_superficial_carry",
    "person_superficial_stock",
    "person_essential_stock",
    "person_foreign_deposits",
    "person_mortgages",
    "person_loans",
    "people",
    "households",
    "country_fx_reserves_by_source",
    "country_export_tariffs",
    "country_import_tariffs",
    "country_sim_params",
    "countries",
    "world_tariff_rates",
    "world_state",
];

pub(super) fn create_schema(conn: &Connection) -> Result<(), PersistenceError> {
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}

pub(super) fn clear_snapshot_tables(conn: &Connection) -> Result<(), PersistenceError> {
    for table in TABLES_TO_CLEAR {
        conn.execute(&format!("DELETE FROM {table}"), [])?;
    }
    Ok(())
}
