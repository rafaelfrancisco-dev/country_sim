use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use crate::sim::{
    country::Country,
    event::SimEvent,
    world::{BankMetricPoint, MetricPoint, World},
};

use super::{
    PersistenceError, SaveDescriptor, SaveRepository,
    dto::{BankHistoryEntryDto, WorldDto},
};

mod event_row;
mod from;
mod from_rest;
mod helpers;
mod metric_row;
mod params_row;
mod read;
mod schema;
#[cfg(test)]
mod tests;
mod to;
mod to_rest;
mod write;

const SCHEMA_VERSION: u32 = 3;

pub struct SqliteSeaOrmSaveRepository {
    db_dir: PathBuf,
}

impl SqliteSeaOrmSaveRepository {
    pub fn new(db_dir: PathBuf) -> Self {
        Self { db_dir }
    }

    fn open_db(&self, save_name: &str) -> Result<Connection, PersistenceError> {
        let path = self.save_path(save_name)?;
        Ok(Connection::open(&path)?)
    }

    fn unix_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

fn sanitize_save_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect()
}

impl SaveRepository for SqliteSeaOrmSaveRepository {
    fn list_saves(&self) -> Result<Vec<SaveDescriptor>, PersistenceError> {
        let mut saves = Vec::new();
        let read_dir = match std::fs::read_dir(&self.db_dir) {
            Ok(rd) => rd,
            Err(_) => return Ok(saves),
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }

            let conn = match Connection::open(&path) {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            let result = conn.query_row(
                "SELECT save_name, world_day, country_count, generation_seed, updated_at FROM save_metadata LIMIT 1",
                [],
                |row| {
                    Ok(SaveDescriptor {
                        save_name: row.get(0)?,
                        world_day: row.get::<_, i64>(1)? as u64,
                        country_count: row.get::<_, i64>(2)? as u32,
                        generation_seed: row.get::<_, i64>(3)? as u64,
                        updated_at: row.get::<_, i64>(4)? as u64,
                    })
                },
            );

            if let Ok(desc) = result {
                saves.push(desc);
            }
        }

        saves.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(saves)
    }

    fn save_world(&self, save_name: &str, world: &World) -> Result<(), PersistenceError> {
        std::fs::create_dir_all(&self.db_dir)?;

        let conn = self.open_db(save_name)?;
        schema::create_schema(&conn)?;
        conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;

        let result = (|| {
            schema::clear_snapshot_tables(&conn)?;
            let world_dto = world_to_dto(world);
            write::save_snapshot(&conn, save_name, &world_dto, Self::unix_timestamp() as i64)
        })();

        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(err) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(err)
            }
        }
    }

    fn load_world(&self, save_name: &str) -> Result<World, PersistenceError> {
        let conn = self.open_db(save_name)?;

        let version: i64 = conn
            .query_row(
                "SELECT schema_version FROM save_metadata LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| PersistenceError::NotFound(save_name.to_string()))?;

        if version as u32 != SCHEMA_VERSION {
            return Err(PersistenceError::SchemaMismatch {
                expected: SCHEMA_VERSION,
                found: version as u32,
            });
        }

        conn.execute_batch("BEGIN DEFERRED TRANSACTION")?;
        let result = (|| {
            let world_dto = read::load_snapshot(&conn)?;
            world_from_dto(world_dto)
        })();

        let _ = conn.execute_batch("ROLLBACK");
        result
    }

    fn save_path(&self, save_name: &str) -> Result<PathBuf, PersistenceError> {
        let sanitized = sanitize_save_name(save_name);
        if sanitized.is_empty() {
            return Err(PersistenceError::PathResolution(
                "save name is empty after sanitization".to_string(),
            ));
        }
        Ok(self.db_dir.join(format!("{sanitized}.db")))
    }
}

fn world_to_dto(world: &World) -> WorldDto {
    use self::to::{bank_metric_point_to_dto, country_to_dto, metric_point_to_dto};
    use self::to_rest::sim_event_to_dto;

    let countries = world.countries.iter().map(country_to_dto).collect();
    let world_history = world
        .world_history
        .iter()
        .map(metric_point_to_dto)
        .collect();
    let country_history = world
        .country_history
        .iter()
        .map(|history| history.iter().map(metric_point_to_dto).collect())
        .collect();
    let country_fx_source_history = world
        .country_fx_source_history
        .iter()
        .map(|source_map| {
            source_map
                .iter()
                .map(|(key, values)| (key.to_string(), values.iter().copied().collect()))
                .collect()
        })
        .collect();
    let mut bank_history: Vec<_> = world
        .bank_history
        .iter()
        .map(|((country_idx, bank_id), points)| BankHistoryEntryDto {
            country_idx: *country_idx,
            bank_id: *bank_id,
            points: points.iter().map(bank_metric_point_to_dto).collect(),
        })
        .collect();
    bank_history.sort_by_key(|entry| (entry.country_idx, entry.bank_id));
    let event_log = world.event_log.iter().map(sim_event_to_dto).collect();

    WorldDto {
        day: world.day,
        generation_seed: world.generation_seed,
        paused: world.paused,
        policy_rate: world.policy_rate,
        tariff_rates_by_good: world.tariff_rates_by_good.clone(),
        countries,
        world_history,
        country_history,
        country_fx_source_history,
        bank_history,
        event_log,
    }
}

fn world_from_dto(dto: WorldDto) -> Result<World, PersistenceError> {
    use self::from::{bank_metric_point_from_dto, country_from_dto, metric_point_from_dto};
    use self::from_rest::sim_event_from_dto;

    let countries: Vec<Country> = dto
        .countries
        .into_iter()
        .map(country_from_dto)
        .collect::<Result<_, _>>()?;

    let world_history: VecDeque<MetricPoint> = dto
        .world_history
        .into_iter()
        .map(metric_point_from_dto)
        .collect();
    let country_history: Vec<VecDeque<MetricPoint>> = dto
        .country_history
        .into_iter()
        .map(|history| history.into_iter().map(metric_point_from_dto).collect())
        .collect();
    let country_fx_source_history: Vec<BTreeMap<usize, VecDeque<f64>>> = dto
        .country_fx_source_history
        .into_iter()
        .map(|source_map| {
            source_map
                .into_iter()
                .filter_map(|(key, values)| {
                    key.parse::<usize>().ok().map(|key| (key, values.into()))
                })
                .collect()
        })
        .collect();

    let mut bank_history: HashMap<(usize, u16), VecDeque<BankMetricPoint>> = HashMap::new();
    for entry in dto.bank_history {
        bank_history.insert(
            (entry.country_idx, entry.bank_id),
            entry
                .points
                .into_iter()
                .map(bank_metric_point_from_dto)
                .collect(),
        );
    }

    let event_log: VecDeque<SimEvent> = dto.event_log.into_iter().map(sim_event_from_dto).collect();
    let tariff_rates_by_good = if dto.tariff_rates_by_good.is_empty() {
        World::build_tariff_rates_from_countries(&countries)
    } else {
        dto.tariff_rates_by_good
    };

    let mut countries = countries;
    for country in &mut countries {
        country.rebuild_person_runtime();
    }

    Ok(World::new_for_persistence(
        dto.day,
        dto.generation_seed,
        countries,
        dto.paused,
        dto.policy_rate,
        world_history,
        country_history,
        country_fx_source_history,
        bank_history,
        event_log,
        tariff_rates_by_good,
    ))
}
