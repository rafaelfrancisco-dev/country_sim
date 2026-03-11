use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};

use crate::sim::{
    bank::{Bank, BankKind, BankStockPosition, LoanAccount, MortgageAccount},
    company::{Company, FounderStake},
    country::Country,
    currency::CurrencySpec,
    essentials::Household,
    event::SimEvent,
    good::GOODS_COUNT,
    person::{
        EducationTrack, EmploymentStatus, ForeignDeposit, HabilitationLevel, HousingStatus, Person,
        ESSENTIAL_STOCK_SLOT_COUNT, SUPERFICIAL_GOOD_SLOT_COUNT,
    },
    stock_market::{DailyIndexPoint, ListedEquity, Portfolio, Position, StockMarket, StockSecurityId},
    world::{BankMetricPoint, MetricPoint, World},
};

use super::{
    PersistenceError, SaveDescriptor, SaveRepository,
    dto::{
        BankDto, BankHistoryEntryDto, BankJobOpeningDto, BankMetricPointDto, BankStockPositionDto,
        CompanyDto, CountryDto, CurrencySpecDto, DailyIndexPointDto, EmploymentStatusDto,
        ForeignDepositDto, FounderStakeDto, HouseholdDto, HousingStatusDto, JobOpeningDto,
        ListedEquityDto, LoanAccountDto, MetricPointDto, MortgageAccountDto, PersonDto,
        PortfolioDto, PositionDto, SimEventDto, StockMarketDto, WorldDto,
    },
};

mod from;
mod from_rest;
mod to;
mod to_rest;

const SCHEMA_VERSION: u32 = 1;

pub struct SqliteSeaOrmSaveRepository {
    db_dir: PathBuf,
}

impl SqliteSeaOrmSaveRepository {
    pub fn new(db_dir: PathBuf) -> Self {
        Self { db_dir }
    }

    fn open_db(&self, save_name: &str) -> Result<Connection, PersistenceError> {
        let path = self.save_path(save_name)?;
        let conn = Connection::open(&path)?;
        Ok(conn)
    }

    fn create_schema(conn: &Connection) -> Result<(), PersistenceError> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS save_metadata (
                id INTEGER PRIMARY KEY,
                save_name TEXT NOT NULL,
                schema_version INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                generation_seed INTEGER NOT NULL,
                world_day INTEGER NOT NULL,
                country_count INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS world_json (
                id INTEGER PRIMARY KEY,
                data TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
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
                Ok(c) => c,
                Err(_) => continue,
            };

            let result: rusqlite::Result<SaveDescriptor> = conn.query_row(
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
        // Ensure directory exists
        std::fs::create_dir_all(&self.db_dir)?;

        let conn = self.open_db(save_name)?;
        Self::create_schema(&conn)?;

        let world_dto = world_to_dto(world);
        let json = serde_json::to_string(&world_dto)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let now = Self::unix_timestamp() as i64;
        let country_count = world_dto.countries.len() as i64;
        let world_day = world_dto.day as i64;
        let seed = world_dto.generation_seed as i64;

        // Upsert metadata
        let existing: i64 = conn
            .query_row("SELECT COUNT(*) FROM save_metadata", [], |r| r.get(0))
            .unwrap_or(0);

        if existing == 0 {
            conn.execute(
                "INSERT INTO save_metadata (save_name, schema_version, created_at, updated_at, generation_seed, world_day, country_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![save_name, SCHEMA_VERSION as i64, now, now, seed, world_day, country_count],
            )?;
        } else {
            conn.execute(
                "UPDATE save_metadata SET save_name=?1, updated_at=?2, generation_seed=?3, world_day=?4, country_count=?5",
                params![save_name, now, seed, world_day, country_count],
            )?;
        }

        // Upsert world json
        let json_existing: i64 = conn
            .query_row("SELECT COUNT(*) FROM world_json", [], |r| r.get(0))
            .unwrap_or(0);

        if json_existing == 0 {
            conn.execute("INSERT INTO world_json (data) VALUES (?1)", params![json])?;
        } else {
            conn.execute("UPDATE world_json SET data=?1", params![json])?;
        }

        Ok(())
    }

    fn load_world(&self, save_name: &str) -> Result<World, PersistenceError> {
        let conn = self.open_db(save_name)?;

        // Check schema version
        let version: i64 = conn
            .query_row(
                "SELECT schema_version FROM save_metadata LIMIT 1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| PersistenceError::NotFound(save_name.to_string()))?;

        if version as u32 != SCHEMA_VERSION {
            return Err(PersistenceError::SchemaMismatch {
                expected: SCHEMA_VERSION,
                found: version as u32,
            });
        }

        let json: String = conn
            .query_row("SELECT data FROM world_json LIMIT 1", [], |r| r.get(0))
            .map_err(|_| PersistenceError::NotFound(save_name.to_string()))?;

        let world_dto: WorldDto = serde_json::from_str(&json)
            .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        let world = world_from_dto(world_dto)?;
        Ok(world)
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

// ---------------------------------------------------------------------------
// World → WorldDto
// ---------------------------------------------------------------------------

fn world_to_dto(world: &World) -> WorldDto {
    use self::to::{bank_metric_point_to_dto, country_to_dto, metric_point_to_dto};
    use self::to_rest::sim_event_to_dto;

    let countries: Vec<CountryDto> = world.countries.iter().map(country_to_dto).collect();

    let world_history: Vec<MetricPointDto> = world
        .world_history
        .iter()
        .map(metric_point_to_dto)
        .collect();

    let country_history: Vec<Vec<MetricPointDto>> = world
        .country_history
        .iter()
        .map(|hist| hist.iter().map(metric_point_to_dto).collect())
        .collect();

    let country_fx_source_history: Vec<HashMap<String, Vec<f64>>> = world
        .country_fx_source_history
        .iter()
        .map(|btree| {
            btree
                .iter()
                .map(|(k, v)| (k.to_string(), v.iter().copied().collect()))
                .collect()
        })
        .collect();

    let bank_history: Vec<BankHistoryEntryDto> = world
        .bank_history
        .iter()
        .map(|((country_idx, bank_id), points)| BankHistoryEntryDto {
            country_idx: *country_idx,
            bank_id: *bank_id,
            points: points.iter().map(bank_metric_point_to_dto).collect(),
        })
        .collect();

    let event_log: Vec<SimEventDto> = world.event_log.iter().map(sim_event_to_dto).collect();

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

// ---------------------------------------------------------------------------
// WorldDto → World reconstruction
// ---------------------------------------------------------------------------

fn world_from_dto(dto: WorldDto) -> Result<World, PersistenceError> {
    use self::from::{bank_metric_point_from_dto, country_from_dto, metric_point_from_dto};
    use self::from_rest::sim_event_from_dto;

    let countries: Vec<Country> = dto
        .countries
        .into_iter()
        .map(country_from_dto)
        .collect::<Result<_, _>>()?;

    let world_history: VecDeque<MetricPoint> =
        dto.world_history.into_iter().map(metric_point_from_dto).collect();

    let country_history: Vec<VecDeque<MetricPoint>> = dto
        .country_history
        .into_iter()
        .map(|hist| hist.into_iter().map(metric_point_from_dto).collect())
        .collect();

    let country_fx_source_history: Vec<BTreeMap<usize, VecDeque<f64>>> = dto
        .country_fx_source_history
        .into_iter()
        .map(|hm| {
            hm.into_iter()
                .filter_map(|(k, v)| k.parse::<usize>().ok().map(|ki| (ki, v.into())))
                .collect()
        })
        .collect();

    let mut bank_history: HashMap<(usize, u16), VecDeque<BankMetricPoint>> = HashMap::new();
    for entry in dto.bank_history {
        let points: VecDeque<BankMetricPoint> =
            entry.points.into_iter().map(bank_metric_point_from_dto).collect();
        bank_history.insert((entry.country_idx, entry.bank_id), points);
    }

    let event_log: VecDeque<SimEvent> = dto.event_log.into_iter().map(sim_event_from_dto).collect();

    let tariff_rates_by_good = if dto.tariff_rates_by_good.is_empty() {
        World::build_tariff_rates_from_countries(&countries)
    } else {
        dto.tariff_rates_by_good
    };

    // Rebuild ephemeral per-country runtime caches before constructing the World.
    let mut countries = countries;
    for country in &mut countries {
        country.rebuild_person_runtime();
    }

    let world = World::new_for_persistence(
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
    );

    Ok(world)
}
