use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use crate::sim::{
    bank::BankKind,
    event::{IpoReason, SimEvent},
    generator::{GenerationConfig, generate_countries},
    persistence::{PersistenceError, SaveRepository, dto::*},
    world::World,
};

use super::{
    SCHEMA_VERSION, SqliteSeaOrmSaveRepository, read, schema, to_rest, world_to_dto, write,
};

fn sample_world() -> World {
    let seed = 0x5eed_c0de;
    let config = GenerationConfig::with_defaults(2, 32, seed);
    let countries = generate_countries(&config);
    World::new_with_generation_seed(countries, seed)
}

fn sample_metric_point(month: u64, base: f64) -> MetricPointDto {
    MetricPointDto {
        month,
        fx_rate_local_per_base: base + 0.1,
        fx_reserves_base: base + 1.0,
        fx_net_flow_base: base + 2.0,
        fx_pressure: base + 3.0,
        population: base + 4.0,
        total_money_supply: base + 5.0,
        household_cash_total: base + 6.0,
        company_cash_total: base + 7.0,
        expected_money_supply: base + 8.0,
        money_supply_gap: base + 9.0,
        created_money_outstanding: base + 10.0,
        bank_reserve: base + 11.0,
        treasury_balance: base + 12.0,
        budget_balance: base + 13.0,
        monthly_tax_collected: base + 14.0,
        monthly_gov_spending: base + 15.0,
        gov_debt: base + 16.0,
        monthly_money_minted: base + 17.0,
        cumulative_money_minted: base + 18.0,
        monthly_mint_fraction: base + 19.0,
        monthly_goods_import_spending: base + 20.0,
        monthly_goods_export_revenue: base + 21.0,
        monthly_unmet_need_events: base + 22.0,
        monthly_severe_unmet_need_events: base + 23.0,
        monthly_chronic_unmet_need_count: base + 24.0,
        monthly_essential_debt_spending: base + 25.0,
        cpi_index: base + 26.0,
        inflation_rate: base + 27.0,
        policy_rate: base + 28.0,
        deposit_rate: base + 29.0,
        loan_rate: base + 30.0,
        monthly_savings_interest_paid: base + 31.0,
        bonds_outstanding: base + 32.0,
        stock_market_cap: base + 33.0,
        stock_bank_equity_exposure: base + 34.0,
        housing_price_index: base + 35.0,
        monthly_rent: base + 36.0,
        homeownership_rate: base + 37.0,
        housing_vacancy_rate: base + 38.0,
        monthly_foreclosures: base + 39.0,
        monthly_home_purchases: base + 40.0,
        monthly_immigrants: base + 41.0,
        monthly_emigrants: base + 42.0,
        monthly_migrants_for_work: base + 43.0,
        monthly_migrants_for_distress: base + 44.0,
    }
}

fn sample_bank_metric_point(month: u64, base: f64) -> BankMetricPointDto {
    BankMetricPointDto {
        month,
        reserve: base + 1.0,
        deposits: base + 2.0,
        created_money: base + 3.0,
        equity: base + 4.0,
        run_intensity: base + 5.0,
    }
}

fn rich_world_dto() -> WorldDto {
    let mut dto = world_to_dto(&sample_world());
    dto.day = 77;
    dto.paused = true;
    dto.policy_rate = 0.043;
    dto.tariff_rates_by_good = vec![vec![0.11, 0.22], vec![0.33, 0.44], vec![0.55, 0.66]];
    dto.world_history = vec![sample_metric_point(1, 10.0), sample_metric_point(2, 20.0)];
    dto.country_history = vec![
        vec![sample_metric_point(1, 100.0)],
        vec![sample_metric_point(1, 200.0), sample_metric_point(2, 210.0)],
    ];
    dto.country_fx_source_history = vec![
        HashMap::from([("1".to_string(), vec![10.0, 11.0, 12.0])]),
        HashMap::from([("0".to_string(), vec![20.0, 21.0])]),
    ];

    let country0_name = dto.countries[0].name.clone();
    let country1_name = dto.countries[1].name.clone();
    let country1_id = dto.countries[1].id;
    let country1_bank0_id = dto.countries[1].banks[0].id;
    let company0_id = dto.countries[0].companies[0].id;
    let bank0_id = dto.countries[0].banks[0].id;
    let person0_id = dto.countries[0].people[0].id;

    dto.bank_history = vec![BankHistoryEntryDto {
        country_idx: 0,
        bank_id: bank0_id,
        points: vec![sample_bank_metric_point(1, 30.0), sample_bank_metric_point(2, 40.0)],
    }];

    let country0 = &mut dto.countries[0];
    country0.sim_params.demographic.births_per_1000_per_year = 23.5;
    country0.sim_params.labor.seek_base = 0.125;
    country0.sim_params.monetary.credit_spread = 0.027;
    country0.export_tariffs_by_good = vec![0.15, 0.25, 0.35];
    country0.fx_reserves_by_source = HashMap::from([("1".to_string(), 9876.5)]);
    country0.pending_events = vec![
        to_rest::sim_event_to_dto(&SimEvent::JobSwitched {
            country_name: Arc::from(country0_name.as_str()),
            person_id: person0_id,
            from_company_id: company0_id,
            to_company_id: company0_id.saturating_add(1),
            monthly_salary: 4321.0,
            day: 77,
        }),
        to_rest::sim_event_to_dto(&SimEvent::BankClosed {
            country_name: Arc::from(country0_name.as_str()),
            bank_id: bank0_id,
            bank_name: Arc::from("Public Bank"),
            acquired_by_bank_id: bank0_id,
            acquired_by_bank_name: Arc::from("Bridge Bank"),
            migrated_depositors: 12,
            migrated_companies: 3,
            migrated_loans: 4,
            transferred_reserves: 555.0,
            day: 78,
        }),
    ];

    let person0 = &mut country0.people[0];
    person0.loan = Some(LoanAccountDto {
        debt_balance: 3210.0,
        created_principal_outstanding: 3500.0,
        monthly_payment: 175.0,
        annual_rate: 0.09,
        term_months: 36,
        months_remaining: 30,
        lender_bank_id: bank0_id,
    });
    person0.housing = HousingStatusDto::Mortgaged {
        mortgage: MortgageAccountDto {
            principal_balance: 90_000.0,
            original_principal: 100_000.0,
            monthly_payment: 850.0,
            annual_rate: 0.045,
            term_months: 360,
            months_remaining: 332,
            months_missed: 1,
            lender_bank_id: bank0_id,
            collateral_value: 120_000.0,
        },
    };
    person0.foreign_deposit = Some(ForeignDepositDto {
        country_id: country1_id,
        bank_id: country1_bank0_id,
        balance: 2500.0,
        home_amount_at_open: 2400.0,
        opening_fx_rate_home_per_foreign: 0.96,
    });
    person0.essential_stock_units = vec![1.25, 2.5, 3.75];
    person0.superficial_stock_units = vec![5, 6, 7, 8, 9];
    person0.superficial_stock_consumption_carry = vec![0.5, 0.6, 0.7, 0.8, 0.9];

    let company0 = &mut country0.companies[0];
    company0.historical_ids = vec![company0.id.saturating_add(100)];
    company0.offered_job_types = vec![1, 2, 3];
    company0.open_positions = vec![
        JobOpeningDto { job_type_id: 1, slots: 2, offered_monthly_salary: 4100.0 },
        JobOpeningDto { job_type_id: 2, slots: 1, offered_monthly_salary: 5200.0 },
    ];
    company0.founder_stakes = vec![FounderStakeDto {
        person_id: person0_id,
        shares_owned: 10_000,
        cash_contributed: 80_000.0,
    }];
    company0.sales_history = vec![11.0, 12.0, 13.0];
    company0.profit_history = vec![1.0, 2.0, 3.0];
    company0.sold_units_history = vec![4, 5, 6];
    company0.employee_count_history = vec![7, 8, 9];
    company0.price_history = vec![10.0, 10.5];
    company0.loan_payment_history = vec![100.0, 101.0];
    company0.cash_balance_history = vec![5000.0, 5100.0];
    company0.loan = Some(LoanAccountDto {
        debt_balance: 12_000.0,
        created_principal_outstanding: 14_000.0,
        monthly_payment: 600.0,
        annual_rate: 0.08,
        term_months: 48,
        months_remaining: 36,
        lender_bank_id: bank0_id,
    });

    let bank0 = &mut country0.banks[0];
    bank0.kind = match BankKind::Private {
        BankKind::Public => 0,
        BankKind::Private => 1,
    };
    bank0.is_in_resolution = true;
    bank0.months_in_resolution = 5;
    bank0.open_positions = vec![BankJobOpeningDto {
        job_type_id: 4,
        slots: 2,
        offered_monthly_salary: 3900.0,
    }];
    bank0.stock_positions = vec![BankStockPositionDto {
        security_country_id: country0.id,
        security_company_id: company0.id,
        shares: 150,
        avg_cost: 12.75,
    }];

    country0.stock_market.person_portfolios = HashMap::from([(
        person0_id.to_string(),
        PortfolioDto {
            positions: vec![PositionDto {
                security_country_id: country0.id,
                security_company_id: company0.id,
                shares: 25,
                avg_cost: 11.5,
                pledged_shares: 5,
            }],
            realized_pnl_mtd: 88.0,
            dividends_mtd: 9.5,
            last_completed_month_dividends: 12.0,
        },
    )]);
    country0.stock_market.bank_portfolio = PortfolioDto {
        positions: vec![PositionDto {
            security_country_id: country0.id,
            security_company_id: company0.id,
            shares: 50,
            avg_cost: 10.25,
            pledged_shares: 0,
        }],
        realized_pnl_mtd: 15.0,
        dividends_mtd: 1.5,
        last_completed_month_dividends: 2.5,
    };
    country0.stock_market.listed = HashMap::from([(
        company0.id.to_string(),
        ListedEquityDto {
            company_id: company0.id,
            current_company_id: company0.id,
            ticker: "ACME".to_string(),
            shares_outstanding: 100_000,
            free_float_shares: 60_000,
            last_price: 12.5,
            prev_close: 12.0,
            last_closed_day_pct_change: 0.041,
            day_volume: 1_234,
            day_turnover: 15_432.5,
            day_trades_count: 67,
            market_cap: 1_250_000.0,
            price_history: vec![11.0, 11.5, 12.5],
            volume_history: vec![900, 1000, 1234],
            market_cap_history: vec![1_100_000.0, 1_150_000.0, 1_250_000.0],
            monthly_dps: vec![0.1, 0.2, 0.3],
            dps_cursor: 2,
        },
    )]);
    country0.stock_market.daily_index_history = vec![
        DailyIndexPointDto { day: 75, index_value: 1002.0, total_turnover: 22_000.0, bank_equity_exposure: 50_000.0 },
        DailyIndexPointDto { day: 76, index_value: 1007.0, total_turnover: 24_000.0, bank_equity_exposure: 51_000.0 },
    ];

    dto.event_log = vec![
        to_rest::sim_event_to_dto(&SimEvent::StockIpo {
            country_name: Arc::from(country0_name.as_str()),
            company_id: company0_id,
            company_name: Arc::from("Acme"),
            ticker: Arc::from("ACME"),
            reason: IpoReason::Growth,
            shares_issued: 100_000,
            primary_shares_issued: 80_000,
            secondary_shares_sold: 20_000,
            price_per_share: 12.5,
            total_raised: 1_250_000.0,
            primary_raised: 1_000_000.0,
            secondary_raised: 250_000.0,
            founder_payout_total: 250_000.0,
            day: 76,
        }),
        to_rest::sim_event_to_dto(&SimEvent::ForeignDepositOpened {
            home_country_name: Arc::from(country0_name.as_str()),
            foreign_country_name: Arc::from(country1_name.as_str()),
            person_id: person0_id,
            bank_id: country1_bank0_id,
            home_amount: 2000.0,
            foreign_amount: 2100.0,
            fx_rate_home_per_foreign: 0.95,
            annual_rate: 0.03,
            day: 77,
        }),
        to_rest::sim_event_to_dto(&SimEvent::DisasterApplied {
            country_name: Arc::from(country0_name.as_str()),
            disaster_label: Arc::from("Flood"),
            intensity_pct: 45,
            summary: Arc::from("River overflowed industrial district"),
            day: 78,
        }),
    ];

    dto
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    path.push(format!("country-sim-{name}-{suffix}"));
    std::fs::create_dir_all(&path).expect("temp dir must be creatable");
    path
}

#[test]
fn sqlite_snapshot_round_trip_preserves_rich_world_dto() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite must open");
    schema::create_schema(&conn).expect("schema creation must succeed");

    let expected = rich_world_dto();
    write::save_snapshot(&conn, "roundtrip", &expected, 1234).expect("save must succeed");
    let loaded = read::load_snapshot(&conn).expect("load must succeed");

    assert_eq!(loaded, expected);
}

#[test]
fn schema_creation_uses_normalized_tables_and_no_world_json() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite must open");
    schema::create_schema(&conn).expect("schema creation must succeed");

    let world_json_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='world_json'",
            [],
            |row| row.get(0),
        )
        .expect("sqlite_master query must succeed");
    assert_eq!(world_json_count, 0);

    for table in ["save_metadata", "world_state", "countries", "country_sim_params", "events"] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .expect("table presence query must succeed");
        assert_eq!(count, 1, "missing table {table}");
    }
}

#[test]
fn repository_round_trip_preserves_generated_world() {
    let dir = temp_dir("repo-roundtrip");
    let repo = SqliteSeaOrmSaveRepository::new(dir.clone());
    let world = sample_world();

    repo.save_world("roundtrip", &world).expect("save must succeed");
    let loaded = repo.load_world("roundtrip").expect("load must succeed");

    assert_eq!(world_to_dto(&loaded), world_to_dto(&world));

    std::fs::remove_dir_all(dir).expect("temp dir cleanup must succeed");
}

#[test]
fn repository_rejects_v1_schema_database() {
    let dir = temp_dir("v1-schema");
    let repo = SqliteSeaOrmSaveRepository::new(dir.clone());
    let db_path = repo.save_path("legacy").expect("save path must resolve");
    let conn = Connection::open(&db_path).expect("sqlite file must open");

    conn.execute_batch(
        "
        CREATE TABLE save_metadata (
            id INTEGER PRIMARY KEY,
            save_name TEXT NOT NULL,
            schema_version INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            generation_seed INTEGER NOT NULL,
            world_day INTEGER NOT NULL,
            country_count INTEGER NOT NULL
        );
        CREATE TABLE world_json (
            id INTEGER PRIMARY KEY,
            payload_json TEXT NOT NULL
        );
        INSERT INTO save_metadata
            (id, save_name, schema_version, created_at, updated_at, generation_seed, world_day, country_count)
        VALUES
            (1, 'legacy', 1, 0, 0, 123, 45, 2);
        INSERT INTO world_json (id, payload_json) VALUES (1, '{}');
        ",
    )
    .expect("legacy schema should be creatable");

    let err = repo.load_world("legacy").expect_err("v1 database must be rejected");
    match err {
        PersistenceError::SchemaMismatch { expected, found } => {
            assert_eq!(expected, SCHEMA_VERSION);
            assert_eq!(found, 1);
        }
        other => panic!("expected schema mismatch, got {other:?}"),
    }

    std::fs::remove_dir_all(dir).expect("temp dir cleanup must succeed");
}
