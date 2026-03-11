use rusqlite::{Connection, OptionalExtension, Params, params, params_from_iter, types::Value};

use crate::sim::persistence::PersistenceError;

pub(super) fn bool_to_i64(value: bool) -> i64 {
    i64::from(value)
}

pub(super) fn i64_to_bool(value: i64) -> bool {
    value != 0
}

pub(super) fn insert_indexed_real_series(
    conn: &Connection,
    table: &str,
    id_column: &str,
    country_id: u16,
    entity_id: u16,
    values: &[f64],
) -> Result<(), PersistenceError> {
    let sql = format!(
        "INSERT INTO {table} (country_id, {id_column}, history_index, value) VALUES (?1, ?2, ?3, ?4)"
    );
    let mut stmt = conn.prepare(&sql)?;
    for (history_index, value) in values.iter().enumerate() {
        stmt.execute(params![
            i64::from(country_id),
            i64::from(entity_id),
            history_index as i64,
            value,
        ])?;
    }
    Ok(())
}

pub(super) fn insert_indexed_integer_series<T>(
    conn: &Connection,
    table: &str,
    id_column: &str,
    country_id: u16,
    entity_id: u16,
    values: &[T],
) -> Result<(), PersistenceError>
where
    T: Copy + Into<i64>,
{
    let sql = format!(
        "INSERT INTO {table} (country_id, {id_column}, history_index, value) VALUES (?1, ?2, ?3, ?4)"
    );
    let mut stmt = conn.prepare(&sql)?;
    for (history_index, value) in values.iter().enumerate() {
        stmt.execute(params![
            i64::from(country_id),
            i64::from(entity_id),
            history_index as i64,
            (*value).into(),
        ])?;
    }
    Ok(())
}

pub(super) fn load_real_series(
    conn: &Connection,
    query: &str,
    bind: impl Params,
) -> Result<Vec<f64>, PersistenceError> {
    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map(bind, |row| row.get::<_, f64>(0))?;
    let mut values = Vec::new();
    for value in rows {
        values.push(value?);
    }
    Ok(values)
}

pub(super) fn load_i64_series(
    conn: &Connection,
    query: &str,
    bind: impl Params,
) -> Result<Vec<i64>, PersistenceError> {
    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map(bind, |row| row.get::<_, i64>(0))?;
    let mut values = Vec::new();
    for value in rows {
        values.push(value?);
    }
    Ok(values)
}

pub(super) fn load_optional_row<T, F>(
    conn: &Connection,
    sql: &str,
    bind: impl Params,
    map: F,
) -> Result<Option<T>, PersistenceError>
where
    F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    Ok(conn.query_row(sql, bind, map).optional()?)
}

pub(super) fn execute_rows(
    conn: &Connection,
    sql: &str,
    rows: impl IntoIterator<Item = Vec<Value>>,
) -> Result<(), PersistenceError> {
    let mut stmt = conn.prepare(sql)?;
    for row in rows {
        stmt.execute(params_from_iter(row))?;
    }
    Ok(())
}
