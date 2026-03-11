use std::collections::HashMap;

use rusqlite::{Connection, params};

use crate::sim::persistence::PersistenceError;
use crate::sim::persistence::dto::{
    BankDto, BankHistoryEntryDto, BankJobOpeningDto, BankStockPositionDto, CompanyDto, CountryDto,
    CurrencySpecDto, DailyIndexPointDto, EmploymentStatusDto, ForeignDepositDto, FounderStakeDto,
    HouseholdDto, HousingStatusDto, JobOpeningDto, ListedEquityDto, LoanAccountDto,
    MortgageAccountDto, PersonDto, PortfolioDto, PositionDto, StockMarketDto, WorldDto,
};

use super::{
    event_row::load_events,
    helpers::{i64_to_bool, load_i64_series, load_optional_row, load_real_series},
    metric_row::{load_bank_metric_points, load_metric_points},
    params_row::load_country_sim_params,
};

pub(super) fn load_snapshot(conn: &Connection) -> Result<WorldDto, PersistenceError> {
    let (day, generation_seed, paused, policy_rate) = conn.query_row(
        "SELECT day, generation_seed, paused, policy_rate FROM world_state WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)? as u64,
                row.get::<_, i64>(1)? as u64,
                i64_to_bool(row.get(2)?),
                row.get(3)?,
            ))
        },
    )?;

    let countries = load_countries(conn)?;
    let tariff_rates_by_good = load_tariff_rates(conn, countries.len())?;
    let world_history = load_metric_points(
        conn,
        "SELECT month, fx_rate_local_per_base, fx_reserves_base, fx_net_flow_base, fx_pressure,
                population, total_money_supply, household_cash_total, company_cash_total,
                expected_money_supply, money_supply_gap, created_money_outstanding, bank_reserve,
                treasury_balance, budget_balance, monthly_tax_collected, monthly_gov_spending,
                gov_debt, monthly_money_minted, cumulative_money_minted, monthly_mint_fraction,
                monthly_goods_import_spending, monthly_goods_export_revenue, monthly_unmet_need_events,
                monthly_severe_unmet_need_events, monthly_chronic_unmet_need_count,
                monthly_essential_debt_spending, cpi_index, inflation_rate, policy_rate,
                deposit_rate, loan_rate, monthly_savings_interest_paid, bonds_outstanding,
                stock_market_cap, stock_bank_equity_exposure, housing_price_index, monthly_rent,
                homeownership_rate, housing_vacancy_rate, monthly_foreclosures, monthly_home_purchases,
                monthly_immigrants, monthly_emigrants, monthly_migrants_for_work,
                monthly_migrants_for_distress
         FROM world_history ORDER BY history_index",
        [],
    )?;

    let mut country_history = Vec::with_capacity(countries.len());
    let mut country_fx_source_history = Vec::with_capacity(countries.len());
    for country_index in 0..countries.len() {
        country_history.push(load_metric_points(
            conn,
            "SELECT month, fx_rate_local_per_base, fx_reserves_base, fx_net_flow_base, fx_pressure,
                    population, total_money_supply, household_cash_total, company_cash_total,
                    expected_money_supply, money_supply_gap, created_money_outstanding, bank_reserve,
                    treasury_balance, budget_balance, monthly_tax_collected, monthly_gov_spending,
                    gov_debt, monthly_money_minted, cumulative_money_minted, monthly_mint_fraction,
                    monthly_goods_import_spending, monthly_goods_export_revenue, monthly_unmet_need_events,
                    monthly_severe_unmet_need_events, monthly_chronic_unmet_need_count,
                    monthly_essential_debt_spending, cpi_index, inflation_rate, policy_rate,
                    deposit_rate, loan_rate, monthly_savings_interest_paid, bonds_outstanding,
                    stock_market_cap, stock_bank_equity_exposure, housing_price_index, monthly_rent,
                    homeownership_rate, housing_vacancy_rate, monthly_foreclosures, monthly_home_purchases,
                    monthly_immigrants, monthly_emigrants, monthly_migrants_for_work,
                    monthly_migrants_for_distress
             FROM country_history WHERE country_index=?1 ORDER BY history_index",
            [country_index as i64],
        )?);
        country_fx_source_history.push(load_country_fx_source_history(conn, country_index)?);
    }

    let bank_history = load_bank_history(conn)?;
    let event_log = load_events(conn, "world", 0)?;

    Ok(WorldDto {
        day,
        generation_seed,
        paused,
        policy_rate,
        tariff_rates_by_good,
        countries,
        world_history,
        country_history,
        country_fx_source_history,
        bank_history,
        event_log,
    })
}

fn load_countries(conn: &Connection) -> Result<Vec<CountryDto>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT country_id, country_index, name, currency_code, currency_name, currency_symbol, day, month,
                next_startup_serial, next_person_id, birth_carry, last_day_births, last_day_deaths,
                treasury_balance, treasury_bank_id, monthly_tax_collected, monthly_corporate_tax_collected,
                monthly_household_spending, monthly_tariff_collected, monthly_goods_import_spending,
                monthly_goods_import_spending_base, monthly_goods_export_revenue, monthly_goods_export_revenue_base,
                monthly_unmet_need_events, monthly_severe_unmet_need_events, monthly_chronic_unmet_need_count,
                monthly_essential_debt_spending, monthly_hires_total, monthly_immigrants, monthly_emigrants,
                monthly_migrants_for_work, monthly_migrants_for_distress, monthly_private_banks_founded,
                cumulative_private_banks_founded, monthly_private_banks_closed, cumulative_private_banks_closed,
                monthly_gov_spending, monthly_budget_balance, monthly_gdp, austerity_active, gov_debt,
                bonds_outstanding, bond_coupon_rate, bond_months_to_maturity, monthly_coupon_paid,
                monthly_bonds_issued, monthly_bonds_matured, monthly_savings_interest_paid,
                monthly_fx_financial_inflows_base, monthly_fx_financial_outflows_base, monthly_fx_net_flow_base,
                monthly_money_minted, cumulative_money_minted, last_mint_inflation_shortfall,
                last_mint_unemployment_gap, last_mint_fraction, last_completed_month_tax_collected,
                last_completed_month_household_spending, last_completed_month_tariff_collected,
                last_completed_month_gov_spending, last_completed_month_budget_balance,
                last_completed_month_coupon_paid, last_completed_month_bonds_issued,
                last_completed_month_bonds_matured, last_completed_month_savings_interest_paid,
                last_completed_month_fx_net_flow_base, last_completed_month_rent_collected,
                last_completed_month_mortgage_payments, last_completed_month_home_purchases,
                last_completed_month_foreclosures, last_completed_month_unmet_need_events,
                last_completed_month_severe_unmet_need_events,
                last_completed_month_chronic_unmet_need_count,
                last_completed_month_essential_debt_spending, last_completed_month_hires_total,
                last_completed_month_money_minted, policy_rate, fx_rate_local_per_base,
                fx_reserves_base, fx_pressure, cpi_index, last_month_cpi_index, monthly_inflation_rate, smoothed_monthly_cpi_change,
                initial_money_supply, household_cash_total, company_cash_total, total_money_supply,
                expected_money_supply, money_supply_gap, next_bank_id, next_bank_serial, public_bank_id,
                housing_stock_total, housing_stock_occupied_owned, housing_stock_occupied_rented,
                housing_stock_vacant, housing_price_index, monthly_rent, monthly_rent_collected,
                monthly_mortgage_payments, monthly_home_purchases, monthly_foreclosures, monthly_housing_units_built
         FROM countries ORDER BY country_index",
    )?;
    let mut countries = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let country_id = row.get::<_, u16>(0)?;
        countries.push(CountryDto {
            id: country_id,
            name: row.get(2)?,
            currency: CurrencySpecDto {
                code: row.get(3)?,
                name: row.get(4)?,
                symbol: row.get(5)?,
            },
            day: row.get::<_, i64>(6)? as u64,
            month: row.get::<_, i64>(7)? as u64,
            next_startup_serial: row.get::<_, i64>(8)? as u32,
            next_person_id: row.get::<_, i64>(9)? as u32,
            birth_carry: row.get(10)?,
            last_day_births: row.get::<_, i64>(11)? as u32,
            last_day_deaths: row.get::<_, i64>(12)? as u32,
            sim_params: load_country_sim_params(conn, country_id)?.into(),
            treasury_balance: row.get(13)?,
            treasury_bank_id: row.get(14)?,
            monthly_tax_collected: row.get(15)?,
            monthly_corporate_tax_collected: row.get(16)?,
            monthly_household_spending: row.get(17)?,
            monthly_tariff_collected: row.get(18)?,
            monthly_goods_import_spending: row.get(19)?,
            monthly_goods_import_spending_base: row.get(20)?,
            monthly_goods_export_revenue: row.get(21)?,
            monthly_goods_export_revenue_base: row.get(22)?,
            monthly_unmet_need_events: row.get::<_, i64>(23)? as u64,
            monthly_severe_unmet_need_events: row.get::<_, i64>(24)? as u64,
            monthly_chronic_unmet_need_count: row.get::<_, i64>(25)? as u64,
            monthly_essential_debt_spending: row.get(26)?,
            monthly_hires_total: row.get::<_, i64>(27)? as u64,
            monthly_immigrants: row.get::<_, i64>(28)? as u64,
            monthly_emigrants: row.get::<_, i64>(29)? as u64,
            monthly_migrants_for_work: row.get::<_, i64>(30)? as u64,
            monthly_migrants_for_distress: row.get::<_, i64>(31)? as u64,
            monthly_private_banks_founded: row.get::<_, i64>(32)? as u64,
            cumulative_private_banks_founded: row.get::<_, i64>(33)? as u64,
            monthly_private_banks_closed: row.get::<_, i64>(34)? as u64,
            cumulative_private_banks_closed: row.get::<_, i64>(35)? as u64,
            monthly_gov_spending: row.get(36)?,
            monthly_budget_balance: row.get(37)?,
            monthly_gdp: row.get(38)?,
            austerity_active: i64_to_bool(row.get(39)?),
            gov_debt: row.get(40)?,
            bonds_outstanding: row.get(41)?,
            bond_coupon_rate: row.get(42)?,
            bond_months_to_maturity: row.get(43)?,
            monthly_coupon_paid: row.get(44)?,
            monthly_bonds_issued: row.get(45)?,
            monthly_bonds_matured: row.get(46)?,
            monthly_savings_interest_paid: row.get(47)?,
            monthly_fx_financial_inflows_base: row.get(48)?,
            monthly_fx_financial_outflows_base: row.get(49)?,
            monthly_fx_net_flow_base: row.get(50)?,
            monthly_money_minted: row.get(51)?,
            cumulative_money_minted: row.get(52)?,
            last_mint_inflation_shortfall: row.get(53)?,
            last_mint_unemployment_gap: row.get(54)?,
            last_mint_fraction: row.get(55)?,
            last_completed_month_tax_collected: row.get(56)?,
            last_completed_month_household_spending: row.get(57)?,
            last_completed_month_tariff_collected: row.get(58)?,
            last_completed_month_gov_spending: row.get(59)?,
            last_completed_month_budget_balance: row.get(60)?,
            last_completed_month_coupon_paid: row.get(61)?,
            last_completed_month_bonds_issued: row.get(62)?,
            last_completed_month_bonds_matured: row.get(63)?,
            last_completed_month_savings_interest_paid: row.get(64)?,
            last_completed_month_fx_net_flow_base: row.get(65)?,
            last_completed_month_rent_collected: row.get(66)?,
            last_completed_month_mortgage_payments: row.get(67)?,
            last_completed_month_home_purchases: row.get::<_, i64>(68)? as u32,
            last_completed_month_foreclosures: row.get::<_, i64>(69)? as u32,
            last_completed_month_unmet_need_events: row.get::<_, i64>(70)? as u64,
            last_completed_month_severe_unmet_need_events: row.get::<_, i64>(71)? as u64,
            last_completed_month_chronic_unmet_need_count: row.get::<_, i64>(72)? as u64,
            last_completed_month_essential_debt_spending: row.get(73)?,
            last_completed_month_hires_total: row.get::<_, i64>(74)? as u64,
            last_completed_month_money_minted: row.get(75)?,
            policy_rate: row.get(76)?,
            fx_rate_local_per_base: row.get(77)?,
            fx_reserves_base: row.get(78)?,
            fx_pressure: row.get(79)?,
            cpi_index: row.get(80)?,
            last_month_cpi_index: row.get(81)?,
            monthly_inflation_rate: row.get(82)?,
            smoothed_monthly_cpi_change: row.get(83)?,
            stock_market: load_stock_market(conn, country_id)?,
            export_tariffs_by_good: load_country_export_tariffs(conn, country_id)?,
            import_tariffs_by_good: load_country_import_tariffs(conn, country_id)?,
            pending_events: load_events(conn, "country", i64::from(country_id))?,
            initial_money_supply: row.get(84)?,
            household_cash_total: row.get(85)?,
            company_cash_total: row.get(86)?,
            total_money_supply: row.get(87)?,
            expected_money_supply: row.get(88)?,
            money_supply_gap: row.get(89)?,
            next_bank_id: row.get(90)?,
            next_bank_serial: row.get::<_, i64>(91)? as u32,
            public_bank_id: row.get(92)?,
            housing_stock_total: row.get::<_, i64>(93)? as u32,
            housing_stock_occupied_owned: row.get::<_, i64>(94)? as u32,
            housing_stock_occupied_rented: row.get::<_, i64>(95)? as u32,
            housing_stock_vacant: row.get::<_, i64>(96)? as u32,
            housing_price_index: row.get(97)?,
            monthly_rent: row.get(98)?,
            monthly_rent_collected: row.get(99)?,
            monthly_mortgage_payments: row.get(100)?,
            monthly_home_purchases: row.get::<_, i64>(101)? as u32,
            monthly_foreclosures: row.get::<_, i64>(102)? as u32,
            monthly_housing_units_built: row.get::<_, i64>(103)? as u32,
            people: load_people(conn, country_id)?,
            companies: load_companies(conn, country_id)?,
            banks: load_banks(conn, country_id)?,
            households: load_households(conn, country_id)?,
            fx_reserves_by_source: load_country_fx_reserves(conn, country_id)?,
        });
    }
    Ok(countries)
}

fn load_tariff_rates(
    conn: &Connection,
    country_count: usize,
) -> Result<Vec<Vec<f64>>, PersistenceError> {
    let mut stmt = conn.prepare("SELECT good_index, seller_country_index, tariff_rate FROM world_tariff_rates ORDER BY good_index, seller_country_index")?;
    let mut matrix: Vec<Vec<f64>> = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)? as usize,
            row.get::<_, i64>(1)? as usize,
            row.get::<_, f64>(2)?,
        ))
    })?;
    for row in rows {
        let (good_index, seller_country_index, tariff_rate) = row?;
        if matrix.len() <= good_index {
            matrix.resize_with(good_index + 1, || vec![0.0; country_count]);
        }
        if matrix[good_index].len() < country_count {
            matrix[good_index].resize(country_count, 0.0);
        }
        if seller_country_index < country_count {
            matrix[good_index][seller_country_index] = tariff_rate;
        }
    }
    Ok(matrix)
}

fn load_country_export_tariffs(
    conn: &Connection,
    country_id: u16,
) -> Result<Vec<f64>, PersistenceError> {
    load_real_series(
        conn,
        "SELECT tariff_rate FROM country_export_tariffs WHERE country_id=?1 ORDER BY good_index",
        [i64::from(country_id)],
    )
}

fn load_country_import_tariffs(
    conn: &Connection,
    country_id: u16,
) -> Result<Vec<f64>, PersistenceError> {
    load_real_series(
        conn,
        "SELECT tariff_rate FROM country_import_tariffs WHERE country_id=?1 ORDER BY good_index",
        [i64::from(country_id)],
    )
}

fn load_country_fx_reserves(
    conn: &Connection,
    country_id: u16,
) -> Result<HashMap<String, f64>, PersistenceError> {
    let mut stmt = conn.prepare("SELECT source_country_index, reserve_amount FROM country_fx_reserves_by_source WHERE country_id=?1 ORDER BY source_country_index")?;
    let rows = stmt.query_map([i64::from(country_id)], |row| {
        Ok((row.get::<_, i64>(0)?.to_string(), row.get::<_, f64>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row?;
        map.insert(k, v);
    }
    Ok(map)
}

fn load_country_fx_source_history(
    conn: &Connection,
    owner_country_index: usize,
) -> Result<HashMap<String, Vec<f64>>, PersistenceError> {
    let mut stmt = conn.prepare("SELECT source_country_index, reserve_base FROM country_fx_source_history WHERE owner_country_index=?1 ORDER BY source_country_index, history_index")?;
    let rows = stmt.query_map([owner_country_index as i64], |row| {
        Ok((row.get::<_, i64>(0)?.to_string(), row.get::<_, f64>(1)?))
    })?;
    let mut map: HashMap<String, Vec<f64>> = HashMap::new();
    for row in rows {
        let (source, value) = row?;
        map.entry(source).or_default().push(value);
    }
    Ok(map)
}

fn load_bank_history(conn: &Connection) -> Result<Vec<BankHistoryEntryDto>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT country_index, bank_id FROM bank_history ORDER BY country_index, bank_id",
    )?;
    let keys = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)? as usize, row.get::<_, u16>(1)?))
    })?;
    let mut entries = Vec::new();
    for key in keys {
        let (country_idx, bank_id) = key?;
        let points = load_bank_metric_points(
            conn,
            "SELECT month, reserve, deposits, created_money, equity, run_intensity FROM bank_history WHERE country_index=?1 AND bank_id=?2 ORDER BY history_index",
            params![country_idx as i64, i64::from(bank_id)],
        )?;
        entries.push(BankHistoryEntryDto {
            country_idx,
            bank_id,
            points,
        });
    }
    Ok(entries)
}

fn load_households(
    conn: &Connection,
    country_id: u16,
) -> Result<Vec<HouseholdDto>, PersistenceError> {
    let mut stmt = conn.prepare("SELECT household_id, member_count, sheltered FROM households WHERE country_id=?1 ORDER BY household_index")?;
    let rows = stmt.query_map([i64::from(country_id)], |row| {
        Ok(HouseholdDto {
            id: row.get::<_, i64>(0)? as u32,
            member_count: row.get(1)?,
            sheltered: i64_to_bool(row.get(2)?),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_people(conn: &Connection, country_id: u16) -> Result<Vec<PersonDto>, PersistenceError> {
    let mut stmt = conn.prepare("SELECT person_id, birth_country_id, current_country_id, employment_kind, employment_job_type_id, employment_company_id, employment_bank_id, employment_start_day, agreed_monthly_salary, cash_balance, deposit_bank_id, household_id, name_seed, age_days, education_track, habilitation, last_month_gross_pay, last_month_tax_paid, last_month_net_pay, last_completed_month_gross_pay, last_completed_month_tax_paid, last_completed_month_net_pay, last_month_spending, last_month_goods_spending, last_month_imported_goods_spending, last_month_goods_units_bought, last_month_needs_met_mask, last_month_unmet_needs_mask, last_month_severe_unmet_needs_mask, consecutive_severe_unmet_months, last_month_chronic_unmet, last_month_loan_payment, last_month_new_loan, last_month_savings_interest, last_month_rent_paid, last_month_mortgage_payment, last_completed_month_spending, last_completed_month_goods_spending, last_completed_month_imported_goods_spending, last_completed_month_goods_units_bought, last_completed_month_loan_payment, last_completed_month_new_loan, last_completed_month_savings_interest, last_completed_month_rent_paid, last_completed_month_mortgage_payment, housing_kind, housing_landlord_company_id, housing_landlord_country_id, months_unemployed, months_on_benefits, lifetime_migrations, last_migration_day FROM people WHERE country_id=?1 ORDER BY person_index")?;
    let mut people = Vec::new();
    let mut rows = stmt.query([i64::from(country_id)])?;
    while let Some(row) = rows.next()? {
        let person_id = row.get::<_, i64>(0)? as u32;
        people.push(PersonDto {
            id: person_id,
            birth_country_id: row.get(1)?,
            current_country_id: row.get(2)?,
            employment: load_employment(row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?),
            employment_start_day: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
            agreed_monthly_salary: row.get(8)?,
            cash_balance: row.get(9)?,
            deposit_bank_id: row.get(10)?,
            household_id: row.get::<_, i64>(11)? as u32,
            name_seed: row.get::<_, i64>(12)? as u32,
            age_days: row.get(13)?,
            education_track: row.get(14)?,
            habilitation: row.get(15)?,
            loan: load_optional_row(conn, "SELECT debt_balance, created_principal_outstanding, monthly_payment, annual_rate, term_months, months_remaining, lender_bank_id FROM person_loans WHERE country_id=?1 AND person_id=?2", params![i64::from(country_id), i64::from(person_id)], |r| Ok(LoanAccountDto { debt_balance: r.get(0)?, created_principal_outstanding: r.get(1)?, monthly_payment: r.get(2)?, annual_rate: r.get(3)?, term_months: r.get(4)?, months_remaining: r.get(5)?, lender_bank_id: r.get(6)? }))?,
            last_month_gross_pay: row.get(16)?,
            last_month_tax_paid: row.get(17)?,
            last_month_net_pay: row.get(18)?,
            last_completed_month_gross_pay: row.get(19)?,
            last_completed_month_tax_paid: row.get(20)?,
            last_completed_month_net_pay: row.get(21)?,
            last_month_spending: row.get(22)?,
            last_month_goods_spending: row.get(23)?,
            last_month_imported_goods_spending: row.get(24)?,
            last_month_goods_units_bought: row.get::<_, i64>(25)? as u32,
            last_month_needs_met_mask: row.get(26)?,
            last_month_unmet_needs_mask: row.get(27)?,
            last_month_severe_unmet_needs_mask: row.get(28)?,
            consecutive_severe_unmet_months: row.get(29)?,
            last_month_chronic_unmet: i64_to_bool(row.get(30)?),
            last_month_loan_payment: row.get(31)?,
            last_month_new_loan: row.get(32)?,
            last_month_savings_interest: row.get(33)?,
            last_month_rent_paid: row.get(34)?,
            last_month_mortgage_payment: row.get(35)?,
            last_completed_month_spending: row.get(36)?,
            last_completed_month_goods_spending: row.get(37)?,
            last_completed_month_imported_goods_spending: row.get(38)?,
            last_completed_month_goods_units_bought: row.get::<_, i64>(39)? as u32,
            last_completed_month_loan_payment: row.get(40)?,
            last_completed_month_new_loan: row.get(41)?,
            last_completed_month_savings_interest: row.get(42)?,
            last_completed_month_rent_paid: row.get(43)?,
            last_completed_month_mortgage_payment: row.get(44)?,
            housing: load_housing(conn, country_id, person_id, row.get(45)?, row.get(46)?, row.get(47)?)?,
            months_unemployed: row.get::<_, i64>(48)? as u32,
            months_on_benefits: row.get::<_, i64>(49)? as u32,
            lifetime_migrations: row.get::<_, i64>(50)? as u32,
            last_migration_day: row.get::<_, Option<i64>>(51)?.map(|v| v as u64),
            essential_stock_units: load_real_series(conn, "SELECT units FROM person_essential_stock WHERE country_id=?1 AND person_id=?2 ORDER BY slot_index", params![i64::from(country_id), i64::from(person_id)])?.into_iter().map(|v| v as f32).collect(),
            superficial_stock_units: load_i64_series(conn, "SELECT units FROM person_superficial_stock WHERE country_id=?1 AND person_id=?2 ORDER BY slot_index", params![i64::from(country_id), i64::from(person_id)])?.into_iter().map(|v| v as u16).collect(),
            superficial_stock_consumption_carry: load_real_series(conn, "SELECT units FROM person_superficial_carry WHERE country_id=?1 AND person_id=?2 ORDER BY slot_index", params![i64::from(country_id), i64::from(person_id)])?.into_iter().map(|v| v as f32).collect(),
            foreign_deposit: load_optional_row(conn, "SELECT foreign_country_id, bank_id, balance, home_amount_at_open, opening_fx_rate_home_per_foreign FROM person_foreign_deposits WHERE country_id=?1 AND person_id=?2", params![i64::from(country_id), i64::from(person_id)], |r| Ok(ForeignDepositDto { country_id: r.get(0)?, bank_id: r.get(1)?, balance: r.get(2)?, home_amount_at_open: r.get(3)?, opening_fx_rate_home_per_foreign: r.get(4)? }))?,
        });
    }
    Ok(people)
}

fn load_employment(
    kind: i64,
    job_type_id: Option<u8>,
    company_id: Option<u16>,
    bank_id: Option<u16>,
) -> EmploymentStatusDto {
    match kind {
        1 => EmploymentStatusDto::Employed {
            job_type_id: job_type_id.unwrap_or(0),
            employer_company_id: company_id.unwrap_or(0),
        },
        2 => EmploymentStatusDto::EmployedAtBank {
            job_type_id: job_type_id.unwrap_or(0),
            bank_id: bank_id.unwrap_or(0),
        },
        _ => EmploymentStatusDto::Unemployed,
    }
}

fn load_housing(
    conn: &Connection,
    country_id: u16,
    person_id: u32,
    kind: i64,
    landlord_company_id: Option<u16>,
    landlord_country_id: Option<u16>,
) -> Result<HousingStatusDto, PersistenceError> {
    Ok(match kind {
        1 => HousingStatusDto::Renter { landlord_company_id: landlord_company_id.unwrap_or(0), landlord_country_id: landlord_country_id.unwrap_or(0) },
        2 => HousingStatusDto::Owner,
        3 => HousingStatusDto::Mortgaged { mortgage: load_optional_row(conn, "SELECT principal_balance, original_principal, monthly_payment, annual_rate, term_months, months_remaining, months_missed, lender_bank_id, collateral_value FROM person_mortgages WHERE country_id=?1 AND person_id=?2", params![i64::from(country_id), i64::from(person_id)], |r| Ok(MortgageAccountDto { principal_balance: r.get(0)?, original_principal: r.get(1)?, monthly_payment: r.get(2)?, annual_rate: r.get(3)?, term_months: r.get(4)?, months_remaining: r.get(5)?, months_missed: r.get(6)?, lender_bank_id: r.get(7)?, collateral_value: r.get(8)? }))?.unwrap_or(MortgageAccountDto { principal_balance: 0.0, original_principal: 0.0, monthly_payment: 0.0, annual_rate: 0.0, term_months: 0, months_remaining: 0, months_missed: 0, lender_bank_id: 0, collateral_value: 0.0 }) },
        _ => HousingStatusDto::Unhoused,
    })
}

fn load_companies(conn: &Connection, country_id: u16) -> Result<Vec<CompanyDto>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT company_id, name, is_public_utility, is_essential_backfill, produced_good_id,
                employee_count, manufacturing_employee_count, inventory_units, unit_price,
                wage_offer_multiplier, consecutive_missed_loan_payments,
                missed_loan_payment_this_month, production_effort_carry, cash_balance,
                deposit_bank_id, monthly_payroll_budget, last_month_payroll_paid,
                last_completed_month_payroll_paid, last_month_sales, last_month_profit,
                last_completed_month_profit, consecutive_profitable_months,
                consecutive_loss_months, last_month_loan_payment, last_completed_month_loan_payment,
                last_month_new_loan, last_completed_month_new_loan, last_month_savings_interest,
                last_completed_month_savings_interest, last_month_units_produced,
                last_month_units_sold, last_month_export_units_sold,
                last_month_domestic_units_sold, last_month_sell_through, last_completed_month_sales,
                last_completed_month_units_produced, last_completed_month_units_sold,
                last_completed_month_export_units_sold, last_completed_month_domestic_units_sold,
                last_completed_month_sell_through, age_months, ipo_cooldown_months, failed_ipo_attempts, rental_units_owned,
                last_month_rent_revenue
         FROM companies WHERE country_id=?1 ORDER BY company_index",
    )?;
    let mut companies = Vec::new();
    let mut rows = stmt.query([i64::from(country_id)])?;
    while let Some(row) = rows.next()? {
        let company_id = row.get::<_, u16>(0)?;
        companies.push(CompanyDto {
            id: company_id,
            historical_ids: load_i64_series(conn, "SELECT historical_company_id FROM company_historical_ids WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?.into_iter().map(|v| v as u16).collect(),
            name: row.get(1)?,
            is_public_utility: i64_to_bool(row.get(2)?),
            is_essential_backfill: i64_to_bool(row.get(3)?),
            offered_job_types: load_i64_series(conn, "SELECT job_type_id FROM company_offered_job_types WHERE country_id=?1 AND company_id=?2 ORDER BY job_type_index", params![i64::from(country_id), i64::from(company_id)])?.into_iter().map(|v| v as u8).collect(),
            produced_good_id: row.get(4)?,
            open_positions: {
                let mut stmt = conn.prepare("SELECT job_type_id, slots, offered_monthly_salary FROM company_open_positions WHERE country_id=?1 AND company_id=?2 ORDER BY position_index")?;
                let rows = stmt.query_map(params![i64::from(country_id), i64::from(company_id)], |r| Ok(JobOpeningDto { job_type_id: r.get(0)?, slots: r.get(1)?, offered_monthly_salary: r.get(2)? }))?;
                rows.collect::<Result<Vec<_>, _>>()?
            },
            founder_stakes: {
                let mut stmt = conn.prepare("SELECT person_id, shares_owned, cash_contributed FROM company_founder_stakes WHERE country_id=?1 AND company_id=?2 ORDER BY stake_index")?;
                let rows = stmt.query_map(params![i64::from(country_id), i64::from(company_id)], |r| Ok(FounderStakeDto { person_id: r.get::<_, i64>(0)? as u32, shares_owned: r.get::<_, i64>(1)? as u64, cash_contributed: r.get(2)? }))?;
                rows.collect::<Result<Vec<_>, _>>()?
            },
            employee_count: row.get::<_, i64>(5)? as u32,
            manufacturing_employee_count: row.get::<_, i64>(6)? as u32,
            inventory_units: row.get::<_, i64>(7)? as u32,
            unit_price: row.get(8)?,
            wage_offer_multiplier: row.get(9)?,
            consecutive_missed_loan_payments: row.get(10)?,
            missed_loan_payment_this_month: i64_to_bool(row.get(11)?),
            production_effort_carry: row.get(12)?,
            cash_balance: row.get(13)?,
            deposit_bank_id: row.get(14)?,
            monthly_payroll_budget: row.get(15)?,
            last_month_payroll_paid: row.get(16)?,
            last_completed_month_payroll_paid: row.get(17)?,
            last_month_sales: row.get(18)?,
            last_month_profit: row.get(19)?,
            last_completed_month_profit: row.get(20)?,
            consecutive_profitable_months: row.get(21)?,
            consecutive_loss_months: row.get(22)?,
            last_month_loan_payment: row.get(23)?,
            last_completed_month_loan_payment: row.get(24)?,
            last_month_new_loan: row.get(25)?,
            last_completed_month_new_loan: row.get(26)?,
            last_month_savings_interest: row.get(27)?,
            last_completed_month_savings_interest: row.get(28)?,
            last_month_units_produced: row.get::<_, i64>(29)? as u32,
            last_month_units_sold: row.get::<_, i64>(30)? as u32,
            last_month_export_units_sold: row.get::<_, i64>(31)? as u32,
            last_month_domestic_units_sold: row.get::<_, i64>(32)? as u32,
            last_month_sell_through: row.get(33)?,
            last_completed_month_sales: row.get(34)?,
            last_completed_month_units_produced: row.get::<_, i64>(35)? as u32,
            last_completed_month_units_sold: row.get::<_, i64>(36)? as u32,
            last_completed_month_export_units_sold: row.get::<_, i64>(37)? as u32,
            last_completed_month_domestic_units_sold: row.get::<_, i64>(38)? as u32,
            last_completed_month_sell_through: row.get(39)?,
            sales_history: load_real_series(conn, "SELECT value FROM company_sales_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?,
            profit_history: load_real_series(conn, "SELECT value FROM company_profit_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?,
            sold_units_history: load_i64_series(conn, "SELECT value FROM company_sold_units_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?.into_iter().map(|v| v as u32).collect(),
            employee_count_history: load_i64_series(conn, "SELECT value FROM company_employee_count_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?.into_iter().map(|v| v as u32).collect(),
            price_history: load_real_series(conn, "SELECT value FROM company_price_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?,
            loan_payment_history: load_real_series(conn, "SELECT value FROM company_loan_payment_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?,
            cash_balance_history: load_real_series(conn, "SELECT value FROM company_cash_balance_history WHERE country_id=?1 AND company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(company_id)])?,
            age_months: row.get(40)?,
            ipo_cooldown_months: row.get(41)?,
            failed_ipo_attempts: row.get(42)?,
            loan: load_optional_row(conn, "SELECT debt_balance, created_principal_outstanding, monthly_payment, annual_rate, term_months, months_remaining, lender_bank_id FROM company_loans WHERE country_id=?1 AND company_id=?2", params![i64::from(country_id), i64::from(company_id)], |r| Ok(LoanAccountDto { debt_balance: r.get(0)?, created_principal_outstanding: r.get(1)?, monthly_payment: r.get(2)?, annual_rate: r.get(3)?, term_months: r.get(4)?, months_remaining: r.get(5)?, lender_bank_id: r.get(6)? }))?,
            rental_units_owned: row.get::<_, i64>(43)? as u32,
            last_month_rent_revenue: row.get(44)?,
        });
    }
    Ok(companies)
}

fn load_banks(conn: &Connection, country_id: u16) -> Result<Vec<BankDto>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT bank_id, name, kind, reserve_balance, created_money_outstanding, total_originated,
                total_principal_repaid, deposit_liabilities, foreign_deposit_liabilities,
                equity_capital, run_intensity, is_in_resolution, months_in_resolution,
                resolution_distress_streak, resolution_stable_streak, employee_count,
                monthly_payroll_budget, last_month_payroll_paid, wage_offer_multiplier,
                last_month_interest_income, last_month_deposit_interest_expense,
                last_month_credit_losses, last_month_profit
         FROM banks WHERE country_id=?1 ORDER BY bank_index",
    )?;
    let rows = stmt.query_map([i64::from(country_id)], |row| {
        let bank_id = row.get::<_, u16>(0)?;
        Ok(BankDto {
            id: bank_id,
            name: row.get(1)?,
            kind: row.get(2)?,
            reserve_balance: row.get(3)?,
            created_money_outstanding: row.get(4)?,
            total_originated: row.get(5)?,
            total_principal_repaid: row.get(6)?,
            deposit_liabilities: row.get(7)?,
            foreign_deposit_liabilities: row.get(8)?,
            equity_capital: row.get(9)?,
            run_intensity: row.get(10)?,
            is_in_resolution: i64_to_bool(row.get(11)?),
            months_in_resolution: row.get(12)?,
            resolution_distress_streak: row.get(13)?,
            resolution_stable_streak: row.get(14)?,
            open_positions: {
                let mut stmt = conn.prepare("SELECT job_type_id, slots, offered_monthly_salary FROM bank_open_positions WHERE country_id=?1 AND bank_id=?2 ORDER BY position_index")?;
                let rows = stmt.query_map(params![i64::from(country_id), i64::from(bank_id)], |r| Ok(BankJobOpeningDto { job_type_id: r.get(0)?, slots: r.get(1)?, offered_monthly_salary: r.get(2)? }))?;
                rows.collect::<Result<Vec<_>, _>>()?
            },
            employee_count: row.get::<_, i64>(15)? as u32,
            monthly_payroll_budget: row.get(16)?,
            last_month_payroll_paid: row.get(17)?,
            wage_offer_multiplier: row.get(18)?,
            last_month_interest_income: row.get(19)?,
            last_month_deposit_interest_expense: row.get(20)?,
            last_month_credit_losses: row.get(21)?,
            last_month_profit: row.get(22)?,
            stock_positions: {
                let mut stmt = conn.prepare("SELECT security_country_id, security_company_id, shares, avg_cost FROM bank_stock_positions WHERE country_id=?1 AND bank_id=?2 ORDER BY position_index")?;
                let rows = stmt.query_map(params![i64::from(country_id), i64::from(bank_id)], |r| Ok(BankStockPositionDto { security_country_id: r.get(0)?, security_company_id: r.get(1)?, shares: r.get::<_, i64>(2)? as u64, avg_cost: r.get(3)? }))?;
                rows.collect::<Result<Vec<_>, _>>()?
            },
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_stock_market(
    conn: &Connection,
    country_id: u16,
) -> Result<StockMarketDto, PersistenceError> {
    let (index_base, ipo_raised_mtd, dividends_paid_mtd, last_completed_month_dividends_paid, forced_liquidations_mtd, last_completed_month_ipo_raised, last_completed_month_forced_liquidations, bank_portfolio_realized_pnl_mtd, bank_portfolio_dividends_mtd, bank_portfolio_last_completed_month_dividends, bank_portfolio_last_completed_month_realized_pnl) = conn.query_row(
        "SELECT index_base, ipo_raised_mtd, dividends_paid_mtd, last_completed_month_dividends_paid, forced_liquidations_mtd, last_completed_month_ipo_raised, last_completed_month_forced_liquidations, bank_portfolio_realized_pnl_mtd, bank_portfolio_dividends_mtd, bank_portfolio_last_completed_month_dividends, bank_portfolio_last_completed_month_realized_pnl FROM country_stock_markets WHERE country_id=?1",
        [i64::from(country_id)],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?)),
    )?;

    let mut person_portfolios = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT person_id, realized_pnl_mtd, dividends_mtd, last_completed_month_dividends, last_completed_month_realized_pnl FROM stock_market_person_portfolios WHERE country_id=?1 ORDER BY person_id")?;
        let rows = stmt.query_map([i64::from(country_id)], |row| {
            let person_id = row.get::<_, i64>(0)? as u32;
            Ok((
                person_id.to_string(),
                PortfolioDto {
                    positions: {
                        let mut stmt = conn.prepare("SELECT security_country_id, security_company_id, shares, avg_cost, pledged_shares FROM stock_market_person_positions WHERE country_id=?1 AND person_id=?2 ORDER BY position_index")?;
                        let rows = stmt.query_map(params![i64::from(country_id), i64::from(person_id)], |r| Ok(PositionDto { security_country_id: r.get(0)?, security_company_id: r.get(1)?, shares: r.get::<_, i64>(2)? as u64, avg_cost: r.get(3)?, pledged_shares: r.get::<_, i64>(4)? as u64 }))?;
                        rows.collect::<Result<Vec<_>, _>>()?
                    },
                    realized_pnl_mtd: row.get(1)?,
                    dividends_mtd: row.get(2)?,
                    last_completed_month_dividends: row.get(3)?,
                    last_completed_month_realized_pnl: row.get(4)?,
                },
            ))
        })?;
        for row in rows {
            let (person_id, portfolio) = row?;
            person_portfolios.insert(person_id, portfolio);
        }
    }

    let bank_portfolio = PortfolioDto {
        positions: {
            let mut stmt = conn.prepare("SELECT security_country_id, security_company_id, shares, avg_cost, pledged_shares FROM stock_market_bank_positions WHERE country_id=?1 ORDER BY position_index")?;
            let rows = stmt.query_map([i64::from(country_id)], |r| {
                Ok(PositionDto {
                    security_country_id: r.get(0)?,
                    security_company_id: r.get(1)?,
                    shares: r.get::<_, i64>(2)? as u64,
                    avg_cost: r.get(3)?,
                    pledged_shares: r.get::<_, i64>(4)? as u64,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        },
        realized_pnl_mtd: bank_portfolio_realized_pnl_mtd,
        dividends_mtd: bank_portfolio_dividends_mtd,
        last_completed_month_dividends: bank_portfolio_last_completed_month_dividends,
        last_completed_month_realized_pnl: bank_portfolio_last_completed_month_realized_pnl,
    };

    let mut listed = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT listing_company_id, current_company_id, ticker, shares_outstanding, free_float_shares, last_price, prev_close, last_closed_day_pct_change, day_volume, day_turnover, day_trades_count, market_cap, dps_cursor FROM stock_market_listed_equities WHERE country_id=?1 ORDER BY listing_company_id")?;
        let mut rows = stmt.query([i64::from(country_id)])?;
        while let Some(row) = rows.next()? {
            let listing_company_id = row.get::<_, u16>(0)?;
            listed.insert(
                listing_company_id.to_string(),
                ListedEquityDto {
                    company_id: listing_company_id,
                    current_company_id: row.get(1)?,
                    ticker: row.get(2)?,
                    shares_outstanding: row.get::<_, i64>(3)? as u64,
                    free_float_shares: row.get::<_, i64>(4)? as u64,
                    last_price: row.get(5)?,
                    prev_close: row.get(6)?,
                    last_closed_day_pct_change: row.get(7)?,
                    day_volume: row.get::<_, i64>(8)? as u64,
                    day_turnover: row.get(9)?,
                    day_trades_count: row.get::<_, i64>(10)? as u64,
                    market_cap: row.get(11)?,
                    price_history: load_real_series(conn, "SELECT value FROM stock_market_equity_price_history WHERE country_id=?1 AND listing_company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(listing_company_id)])?,
                    volume_history: load_i64_series(conn, "SELECT value FROM stock_market_equity_volume_history WHERE country_id=?1 AND listing_company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(listing_company_id)])?.into_iter().map(|v| v as u64).collect(),
                    market_cap_history: load_real_series(conn, "SELECT value FROM stock_market_equity_market_cap_history WHERE country_id=?1 AND listing_company_id=?2 ORDER BY history_index", params![i64::from(country_id), i64::from(listing_company_id)])?,
                    monthly_dps: load_real_series(conn, "SELECT value FROM stock_market_equity_monthly_dps WHERE country_id=?1 AND listing_company_id=?2 ORDER BY slot_index", params![i64::from(country_id), i64::from(listing_company_id)])?,
                    dps_cursor: row.get(12)?,
                },
            );
        }
    }

    let daily_index_history = {
        let mut stmt = conn.prepare("SELECT day, index_value, total_turnover, bank_equity_exposure FROM stock_market_daily_index_history WHERE country_id=?1 ORDER BY history_index")?;
        let rows = stmt.query_map([i64::from(country_id)], |row| {
            Ok(DailyIndexPointDto {
                day: row.get::<_, i64>(0)? as u64,
                index_value: row.get(1)?,
                total_turnover: row.get(2)?,
                bank_equity_exposure: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    Ok(StockMarketDto {
        person_portfolios,
        bank_portfolio,
        listed,
        daily_index_history,
        index_base,
        ipo_raised_mtd,
        dividends_paid_mtd,
        last_completed_month_dividends_paid,
        forced_liquidations_mtd,
        last_completed_month_ipo_raised,
        last_completed_month_forced_liquidations,
    })
}
