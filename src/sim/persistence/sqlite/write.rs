use std::collections::{BTreeMap, HashMap};

use rusqlite::{Connection, params, types::Value};

use crate::sim::persistence::PersistenceError;
use crate::sim::persistence::dto::{
    BankDto, BankHistoryEntryDto, CompanyDto, CountryDto, EmploymentStatusDto, HousingStatusDto,
    MetricPointDto, PortfolioDto, SimEventDto, StockMarketDto, WorldDto,
};

use super::{
    SCHEMA_VERSION,
    helpers::{
        bool_to_i64, execute_rows, insert_indexed_integer_series, insert_indexed_real_series,
    },
    params_row::insert_country_sim_params,
};

pub(super) fn save_snapshot(
    conn: &Connection,
    save_name: &str,
    world: &WorldDto,
    now: i64,
) -> Result<(), PersistenceError> {
    conn.execute(
        "INSERT INTO save_metadata (id, save_name, schema_version, created_at, updated_at, generation_seed, world_day, country_count)
         VALUES (1, ?1, ?2, ?3, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
           save_name=excluded.save_name,
           schema_version=excluded.schema_version,
           updated_at=excluded.updated_at,
           generation_seed=excluded.generation_seed,
           world_day=excluded.world_day,
           country_count=excluded.country_count",
        params![
            save_name,
            i64::from(SCHEMA_VERSION),
            now,
            world.generation_seed as i64,
            world.day as i64,
            world.countries.len() as i64
        ],
    )?;
    conn.execute(
        "INSERT INTO world_state (id, day, generation_seed, paused, policy_rate) VALUES (1, ?1, ?2, ?3, ?4)",
        params![world.day as i64, world.generation_seed as i64, bool_to_i64(world.paused), world.policy_rate],
    )?;

    for (good_index, tariffs) in world.tariff_rates_by_good.iter().enumerate() {
        for (seller_country_index, tariff_rate) in tariffs.iter().enumerate() {
            conn.execute(
                "INSERT INTO world_tariff_rates (good_index, seller_country_index, tariff_rate) VALUES (?1, ?2, ?3)",
                params![good_index as i64, seller_country_index as i64, *tariff_rate],
            )?;
        }
    }

    let name_to_country_id: HashMap<&str, u16> = world
        .countries
        .iter()
        .map(|country| (country.name.as_str(), country.id))
        .collect();

    for (country_index, country) in world.countries.iter().enumerate() {
        insert_country(conn, country_index, country)?;
        for (event_seq, event) in country.pending_events.iter().enumerate() {
            insert_event(
                conn,
                "country",
                i64::from(country.id),
                event_seq as i64,
                event,
                &name_to_country_id,
            )?;
        }
    }

    insert_metric_points(conn, "world_history", None, &world.world_history)?;
    for (country_index, points) in world.country_history.iter().enumerate() {
        insert_metric_points(conn, "country_history", Some(country_index as i64), points)?;
    }
    for (owner_country_index, source_map) in world.country_fx_source_history.iter().enumerate() {
        for (source_country_index, history) in source_map {
            for (history_index, reserve_base) in history.iter().enumerate() {
                conn.execute(
                    "INSERT INTO country_fx_source_history (owner_country_index, source_country_index, history_index, reserve_base) VALUES (?1, ?2, ?3, ?4)",
                    params![owner_country_index as i64, source_country_index.parse::<i64>().unwrap_or_default(), history_index as i64, *reserve_base],
                )?;
            }
        }
    }
    for BankHistoryEntryDto {
        country_idx,
        bank_id,
        points,
    } in &world.bank_history
    {
        for (history_index, point) in points.iter().enumerate() {
            conn.execute(
                "INSERT INTO bank_history (country_index, bank_id, history_index, month, reserve, deposits, created_money, equity, run_intensity) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![*country_idx as i64, i64::from(*bank_id), history_index as i64, point.month as i64, point.reserve, point.deposits, point.created_money, point.equity, point.run_intensity],
            )?;
        }
    }
    for (event_seq, event) in world.event_log.iter().enumerate() {
        insert_event(
            conn,
            "world",
            0,
            event_seq as i64,
            event,
            &name_to_country_id,
        )?;
    }
    Ok(())
}

fn insert_country(
    conn: &Connection,
    country_index: usize,
    country: &CountryDto,
) -> Result<(), PersistenceError> {
    conn.execute(
        "INSERT INTO countries (country_id, country_index, name, currency_code, currency_name, currency_symbol, day, month, next_startup_serial, next_person_id, birth_carry, last_day_births, last_day_deaths, treasury_balance, treasury_bank_id, monthly_tax_collected, monthly_corporate_tax_collected, monthly_household_spending, monthly_tariff_collected, monthly_goods_import_spending, monthly_goods_import_spending_base, monthly_goods_export_revenue, monthly_goods_export_revenue_base, monthly_unmet_need_events, monthly_severe_unmet_need_events, monthly_chronic_unmet_need_count, monthly_essential_debt_spending, monthly_hires_total, monthly_immigrants, monthly_emigrants, monthly_migrants_for_work, monthly_migrants_for_distress, monthly_private_banks_founded, cumulative_private_banks_founded, monthly_private_banks_closed, cumulative_private_banks_closed, monthly_gov_spending, monthly_budget_balance, monthly_gdp, austerity_active, gov_debt, bonds_outstanding, bond_coupon_rate, bond_months_to_maturity, monthly_coupon_paid, monthly_bonds_issued, monthly_bonds_matured, monthly_savings_interest_paid, monthly_fx_financial_inflows_base, monthly_fx_financial_outflows_base, monthly_fx_net_flow_base, monthly_money_minted, cumulative_money_minted, last_mint_inflation_shortfall, last_mint_unemployment_gap, last_mint_fraction, last_completed_month_tax_collected, last_completed_month_household_spending, last_completed_month_tariff_collected, last_completed_month_gov_spending, last_completed_month_budget_balance, last_completed_month_coupon_paid, last_completed_month_bonds_issued, last_completed_month_bonds_matured, last_completed_month_savings_interest_paid, last_completed_month_fx_net_flow_base, last_completed_month_rent_collected, last_completed_month_mortgage_payments, last_completed_month_home_purchases, last_completed_month_foreclosures, last_completed_month_unmet_need_events, last_completed_month_severe_unmet_need_events, last_completed_month_chronic_unmet_need_count, last_completed_month_essential_debt_spending, last_completed_month_hires_total, last_completed_month_money_minted, policy_rate, fx_rate_local_per_base, fx_reserves_base, fx_pressure, cpi_index, last_month_cpi_index, monthly_inflation_rate, initial_money_supply, household_cash_total, company_cash_total, total_money_supply, expected_money_supply, money_supply_gap, next_bank_id, next_bank_serial, public_bank_id, housing_stock_total, housing_stock_occupied_owned, housing_stock_occupied_rented, housing_stock_vacant, housing_price_index, monthly_rent, monthly_rent_collected, monthly_mortgage_payments, monthly_home_purchases, monthly_foreclosures, monthly_housing_units_built)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82, ?83, ?84, ?85, ?86, ?87, ?88, ?89, ?90, ?91, ?92, ?93, ?94, ?95, ?96, ?97, ?98, ?99, ?100, ?101, ?102, ?103)",
        params![
            i64::from(country.id), country_index as i64, country.name, country.currency.code, country.currency.name, country.currency.symbol, country.day as i64, country.month as i64, country.next_startup_serial as i64, country.next_person_id as i64, country.birth_carry, country.last_day_births as i64, country.last_day_deaths as i64, country.treasury_balance, country.treasury_bank_id.map(i64::from), country.monthly_tax_collected, country.monthly_corporate_tax_collected, country.monthly_household_spending, country.monthly_tariff_collected, country.monthly_goods_import_spending, country.monthly_goods_import_spending_base, country.monthly_goods_export_revenue, country.monthly_goods_export_revenue_base, country.monthly_unmet_need_events as i64, country.monthly_severe_unmet_need_events as i64, country.monthly_chronic_unmet_need_count as i64, country.monthly_essential_debt_spending, country.monthly_hires_total as i64, country.monthly_immigrants as i64, country.monthly_emigrants as i64, country.monthly_migrants_for_work as i64, country.monthly_migrants_for_distress as i64, country.monthly_private_banks_founded as i64, country.cumulative_private_banks_founded as i64, country.monthly_private_banks_closed as i64, country.cumulative_private_banks_closed as i64, country.monthly_gov_spending, country.monthly_budget_balance, country.monthly_gdp, bool_to_i64(country.austerity_active), country.gov_debt, country.bonds_outstanding, country.bond_coupon_rate, i64::from(country.bond_months_to_maturity), country.monthly_coupon_paid, country.monthly_bonds_issued, country.monthly_bonds_matured, country.monthly_savings_interest_paid, country.monthly_fx_financial_inflows_base, country.monthly_fx_financial_outflows_base, country.monthly_fx_net_flow_base, country.monthly_money_minted, country.cumulative_money_minted, country.last_mint_inflation_shortfall, country.last_mint_unemployment_gap, country.last_mint_fraction, country.last_completed_month_tax_collected, country.last_completed_month_household_spending, country.last_completed_month_tariff_collected, country.last_completed_month_gov_spending, country.last_completed_month_budget_balance, country.last_completed_month_coupon_paid, country.last_completed_month_bonds_issued, country.last_completed_month_bonds_matured, country.last_completed_month_savings_interest_paid, country.last_completed_month_fx_net_flow_base, country.last_completed_month_rent_collected, country.last_completed_month_mortgage_payments, country.last_completed_month_home_purchases as i64, country.last_completed_month_foreclosures as i64, country.last_completed_month_unmet_need_events as i64, country.last_completed_month_severe_unmet_need_events as i64, country.last_completed_month_chronic_unmet_need_count as i64, country.last_completed_month_essential_debt_spending, country.last_completed_month_hires_total as i64, country.last_completed_month_money_minted, country.policy_rate, country.fx_rate_local_per_base, country.fx_reserves_base, country.fx_pressure, country.cpi_index, country.last_month_cpi_index, country.monthly_inflation_rate, country.initial_money_supply, country.household_cash_total, country.company_cash_total, country.total_money_supply, country.expected_money_supply, country.money_supply_gap, i64::from(country.next_bank_id), country.next_bank_serial as i64, country.public_bank_id.map(i64::from), country.housing_stock_total as i64, country.housing_stock_occupied_owned as i64, country.housing_stock_occupied_rented as i64, country.housing_stock_vacant as i64, country.housing_price_index, country.monthly_rent, country.monthly_rent_collected, country.monthly_mortgage_payments, country.monthly_home_purchases as i64, country.monthly_foreclosures as i64, country.monthly_housing_units_built as i64,
        ],
    )?;
    insert_country_sim_params(conn, country.id, &country.sim_params.clone().into())?;
    insert_simple_pairs(
        conn,
        "country_export_tariffs",
        country.id,
        &country.export_tariffs_by_good,
    )?;
    insert_simple_pairs(
        conn,
        "country_import_tariffs",
        country.id,
        &country.import_tariffs_by_good,
    )?;
    insert_map_pairs(
        conn,
        "country_fx_reserves_by_source",
        "source_country_index",
        "reserve_amount",
        country.id,
        &country.fx_reserves_by_source,
    )?;
    insert_households(conn, country.id, &country.households)?;
    insert_people(conn, country.id, &country.people)?;
    insert_companies(conn, country.id, &country.companies)?;
    insert_banks(conn, country.id, &country.banks)?;
    insert_stock_market(conn, country.id, &country.stock_market)?;
    Ok(())
}

fn insert_households(
    conn: &Connection,
    country_id: u16,
    households: &[crate::sim::persistence::dto::HouseholdDto],
) -> Result<(), PersistenceError> {
    execute_rows(
        conn,
        "INSERT INTO households (country_id, household_index, household_id, member_count, sheltered) VALUES (?1, ?2, ?3, ?4, ?5)",
        households.iter().enumerate().map(|(i, h)| {
            vec![
                v(country_id),
                Value::from(i as i64),
                v(h.id),
                v(h.member_count),
                Value::from(bool_to_i64(h.sheltered)),
            ]
        }),
    )
}

fn insert_people(
    conn: &Connection,
    country_id: u16,
    people: &[crate::sim::persistence::dto::PersonDto],
) -> Result<(), PersistenceError> {
    for (person_index, person) in people.iter().enumerate() {
        let (employment_kind, employment_job_type_id, employment_company_id, employment_bank_id) =
            match person.employment {
                EmploymentStatusDto::Unemployed => (0_i64, None, None, None),
                EmploymentStatusDto::Employed {
                    job_type_id,
                    employer_company_id,
                } => (
                    1,
                    Some(i64::from(job_type_id)),
                    Some(i64::from(employer_company_id)),
                    None,
                ),
                EmploymentStatusDto::EmployedAtBank {
                    job_type_id,
                    bank_id,
                } => (
                    2,
                    Some(i64::from(job_type_id)),
                    None,
                    Some(i64::from(bank_id)),
                ),
            };
        let (housing_kind, housing_landlord_company_id, housing_landlord_country_id) =
            match &person.housing {
                HousingStatusDto::Unhoused => (0_i64, None, None),
                HousingStatusDto::Renter {
                    landlord_company_id,
                    landlord_country_id,
                } => (
                    1,
                    Some(i64::from(*landlord_company_id)),
                    Some(i64::from(*landlord_country_id)),
                ),
                HousingStatusDto::Owner => (2, None, None),
                HousingStatusDto::Mortgaged { .. } => (3, None, None),
            };
        conn.execute(
            "INSERT INTO people (country_id, person_index, person_id, birth_country_id, current_country_id, employment_kind, employment_job_type_id, employment_company_id, employment_bank_id, employment_start_day, agreed_monthly_salary, cash_balance, deposit_bank_id, household_id, name_seed, age_days, education_track, habilitation, last_month_gross_pay, last_month_tax_paid, last_month_net_pay, last_completed_month_gross_pay, last_completed_month_tax_paid, last_completed_month_net_pay, last_month_spending, last_month_goods_spending, last_month_imported_goods_spending, last_month_goods_units_bought, last_month_needs_met_mask, last_month_unmet_needs_mask, last_month_severe_unmet_needs_mask, consecutive_severe_unmet_months, last_month_chronic_unmet, last_month_loan_payment, last_month_new_loan, last_month_savings_interest, last_month_rent_paid, last_month_mortgage_payment, last_completed_month_spending, last_completed_month_goods_spending, last_completed_month_imported_goods_spending, last_completed_month_goods_units_bought, last_completed_month_loan_payment, last_completed_month_new_loan, last_completed_month_savings_interest, last_completed_month_rent_paid, last_completed_month_mortgage_payment, housing_kind, housing_landlord_company_id, housing_landlord_country_id, months_unemployed, months_on_benefits, lifetime_migrations, last_migration_day) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54)",
            params![i64::from(country_id), person_index as i64, i64::from(person.id), i64::from(person.birth_country_id), i64::from(person.current_country_id), employment_kind, employment_job_type_id, employment_company_id, employment_bank_id, person.employment_start_day.map(|v| v as i64), person.agreed_monthly_salary, person.cash_balance, person.deposit_bank_id.map(i64::from), i64::from(person.household_id), i64::from(person.name_seed), i64::from(person.age_days), i64::from(person.education_track), i64::from(person.habilitation), person.last_month_gross_pay, person.last_month_tax_paid, person.last_month_net_pay, person.last_completed_month_gross_pay, person.last_completed_month_tax_paid, person.last_completed_month_net_pay, person.last_month_spending, person.last_month_goods_spending, person.last_month_imported_goods_spending, i64::from(person.last_month_goods_units_bought), i64::from(person.last_month_needs_met_mask), i64::from(person.last_month_unmet_needs_mask), i64::from(person.last_month_severe_unmet_needs_mask), i64::from(person.consecutive_severe_unmet_months), bool_to_i64(person.last_month_chronic_unmet), person.last_month_loan_payment, person.last_month_new_loan, person.last_month_savings_interest, person.last_month_rent_paid, person.last_month_mortgage_payment, person.last_completed_month_spending, person.last_completed_month_goods_spending, person.last_completed_month_imported_goods_spending, i64::from(person.last_completed_month_goods_units_bought), person.last_completed_month_loan_payment, person.last_completed_month_new_loan, person.last_completed_month_savings_interest, person.last_completed_month_rent_paid, person.last_completed_month_mortgage_payment, housing_kind, housing_landlord_company_id, housing_landlord_country_id, i64::from(person.months_unemployed), i64::from(person.months_on_benefits), i64::from(person.lifetime_migrations), person.last_migration_day.map(|v| v as i64)],
        )?;
        if let Some(loan) = &person.loan {
            conn.execute("INSERT INTO person_loans (country_id, person_id, debt_balance, created_principal_outstanding, monthly_payment, annual_rate, term_months, months_remaining, lender_bank_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", params![i64::from(country_id), i64::from(person.id), loan.debt_balance, loan.created_principal_outstanding, loan.monthly_payment, loan.annual_rate, i64::from(loan.term_months), i64::from(loan.months_remaining), i64::from(loan.lender_bank_id)])?;
        }
        if let HousingStatusDto::Mortgaged { mortgage } = &person.housing {
            conn.execute("INSERT INTO person_mortgages (country_id, person_id, principal_balance, original_principal, monthly_payment, annual_rate, term_months, months_remaining, months_missed, lender_bank_id, collateral_value) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", params![i64::from(country_id), i64::from(person.id), mortgage.principal_balance, mortgage.original_principal, mortgage.monthly_payment, mortgage.annual_rate, i64::from(mortgage.term_months), i64::from(mortgage.months_remaining), i64::from(mortgage.months_missed), i64::from(mortgage.lender_bank_id), mortgage.collateral_value])?;
        }
        if let Some(fd) = &person.foreign_deposit {
            conn.execute("INSERT INTO person_foreign_deposits (country_id, person_id, foreign_country_id, bank_id, balance, home_amount_at_open, opening_fx_rate_home_per_foreign) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![i64::from(country_id), i64::from(person.id), i64::from(fd.country_id), i64::from(fd.bank_id), fd.balance, fd.home_amount_at_open, fd.opening_fx_rate_home_per_foreign])?;
        }
        insert_slot_values(
            conn,
            "person_essential_stock",
            country_id,
            person.id,
            &person.essential_stock_units,
        )?;
        insert_slot_values(
            conn,
            "person_superficial_stock",
            country_id,
            person.id,
            &person.superficial_stock_units,
        )?;
        insert_slot_values(
            conn,
            "person_superficial_carry",
            country_id,
            person.id,
            &person.superficial_stock_consumption_carry,
        )?;
    }
    Ok(())
}

fn insert_companies(
    conn: &Connection,
    country_id: u16,
    companies: &[CompanyDto],
) -> Result<(), PersistenceError> {
    for (company_index, company) in companies.iter().enumerate() {
        conn.execute(
            "INSERT INTO companies (country_id, company_index, company_id, name, is_public_utility, is_essential_backfill, produced_good_id, employee_count, manufacturing_employee_count, inventory_units, unit_price, wage_offer_multiplier, consecutive_missed_loan_payments, missed_loan_payment_this_month, production_effort_carry, cash_balance, deposit_bank_id, monthly_payroll_budget, last_month_payroll_paid, last_completed_month_payroll_paid, last_month_sales, last_month_profit, last_completed_month_profit, consecutive_profitable_months, consecutive_loss_months, last_month_loan_payment, last_completed_month_loan_payment, last_month_new_loan, last_completed_month_new_loan, last_month_savings_interest, last_completed_month_savings_interest, last_month_units_produced, last_month_units_sold, last_month_export_units_sold, last_month_domestic_units_sold, last_month_sell_through, last_completed_month_sales, last_completed_month_units_produced, last_completed_month_units_sold, last_completed_month_export_units_sold, last_completed_month_domestic_units_sold, last_completed_month_sell_through, age_months, ipo_cooldown_months, failed_ipo_attempts, rental_units_owned, last_month_rent_revenue) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47)",
            params![i64::from(country_id), company_index as i64, i64::from(company.id), company.name, bool_to_i64(company.is_public_utility), bool_to_i64(company.is_essential_backfill), i64::from(company.produced_good_id), i64::from(company.employee_count), i64::from(company.manufacturing_employee_count), i64::from(company.inventory_units), company.unit_price, company.wage_offer_multiplier, i64::from(company.consecutive_missed_loan_payments), bool_to_i64(company.missed_loan_payment_this_month), company.production_effort_carry, company.cash_balance, company.deposit_bank_id.map(i64::from), company.monthly_payroll_budget, company.last_month_payroll_paid, company.last_completed_month_payroll_paid, company.last_month_sales, company.last_month_profit, company.last_completed_month_profit, i64::from(company.consecutive_profitable_months), i64::from(company.consecutive_loss_months), company.last_month_loan_payment, company.last_completed_month_loan_payment, company.last_month_new_loan, company.last_completed_month_new_loan, company.last_month_savings_interest, company.last_completed_month_savings_interest, i64::from(company.last_month_units_produced), i64::from(company.last_month_units_sold), i64::from(company.last_month_export_units_sold), i64::from(company.last_month_domestic_units_sold), company.last_month_sell_through, company.last_completed_month_sales, i64::from(company.last_completed_month_units_produced), i64::from(company.last_completed_month_units_sold), i64::from(company.last_completed_month_export_units_sold), i64::from(company.last_completed_month_domestic_units_sold), company.last_completed_month_sell_through, i64::from(company.age_months), i64::from(company.ipo_cooldown_months), i64::from(company.failed_ipo_attempts), i64::from(company.rental_units_owned), company.last_month_rent_revenue],
        )?;
        insert_small_rows(
            conn,
            "company_open_positions",
            country_id,
            company.id,
            company.open_positions.iter().enumerate().map(|(i, op)| {
                vec![
                    v(country_id),
                    v(company.id),
                    Value::from(i as i64),
                    v(op.job_type_id),
                    v(op.slots),
                    Value::from(op.offered_monthly_salary),
                ]
            }),
        )?;
        insert_small_rows(
            conn,
            "company_offered_job_types",
            country_id,
            company.id,
            company
                .offered_job_types
                .iter()
                .enumerate()
                .map(|(i, job_type_id)| {
                    vec![
                        v(country_id),
                        v(company.id),
                        Value::from(i as i64),
                        v(*job_type_id),
                    ]
                }),
        )?;
        insert_small_rows(
            conn,
            "company_founder_stakes",
            country_id,
            company.id,
            company.founder_stakes.iter().enumerate().map(|(i, fs)| {
                vec![
                    v(country_id),
                    v(company.id),
                    Value::from(i as i64),
                    v(fs.person_id),
                    Value::from(fs.shares_owned as i64),
                    Value::from(fs.cash_contributed),
                ]
            }),
        )?;
        insert_small_rows(
            conn,
            "company_historical_ids",
            country_id,
            company.id,
            company
                .historical_ids
                .iter()
                .enumerate()
                .map(|(i, id)| vec![v(country_id), v(company.id), Value::from(i as i64), v(*id)]),
        )?;
        insert_indexed_real_series(
            conn,
            "company_sales_history",
            "company_id",
            country_id,
            company.id,
            &company.sales_history,
        )?;
        insert_indexed_real_series(
            conn,
            "company_profit_history",
            "company_id",
            country_id,
            company.id,
            &company.profit_history,
        )?;
        insert_indexed_integer_series(
            conn,
            "company_sold_units_history",
            "company_id",
            country_id,
            company.id,
            &company.sold_units_history,
        )?;
        insert_indexed_integer_series(
            conn,
            "company_employee_count_history",
            "company_id",
            country_id,
            company.id,
            &company.employee_count_history,
        )?;
        insert_indexed_real_series(
            conn,
            "company_price_history",
            "company_id",
            country_id,
            company.id,
            &company.price_history,
        )?;
        insert_indexed_real_series(
            conn,
            "company_loan_payment_history",
            "company_id",
            country_id,
            company.id,
            &company.loan_payment_history,
        )?;
        insert_indexed_real_series(
            conn,
            "company_cash_balance_history",
            "company_id",
            country_id,
            company.id,
            &company.cash_balance_history,
        )?;
        if let Some(loan) = &company.loan {
            conn.execute("INSERT INTO company_loans (country_id, company_id, debt_balance, created_principal_outstanding, monthly_payment, annual_rate, term_months, months_remaining, lender_bank_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", params![i64::from(country_id), i64::from(company.id), loan.debt_balance, loan.created_principal_outstanding, loan.monthly_payment, loan.annual_rate, i64::from(loan.term_months), i64::from(loan.months_remaining), i64::from(loan.lender_bank_id)])?;
        }
    }
    Ok(())
}

fn insert_banks(
    conn: &Connection,
    country_id: u16,
    banks: &[BankDto],
) -> Result<(), PersistenceError> {
    for (bank_index, bank) in banks.iter().enumerate() {
        conn.execute("INSERT INTO banks (country_id, bank_index, bank_id, name, kind, reserve_balance, created_money_outstanding, total_originated, total_principal_repaid, deposit_liabilities, foreign_deposit_liabilities, equity_capital, run_intensity, is_in_resolution, months_in_resolution, resolution_distress_streak, resolution_stable_streak, employee_count, monthly_payroll_budget, last_month_payroll_paid, wage_offer_multiplier, last_month_interest_income, last_month_deposit_interest_expense, last_month_credit_losses, last_month_profit) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)", params![i64::from(country_id), bank_index as i64, i64::from(bank.id), bank.name, i64::from(bank.kind), bank.reserve_balance, bank.created_money_outstanding, bank.total_originated, bank.total_principal_repaid, bank.deposit_liabilities, bank.foreign_deposit_liabilities, bank.equity_capital, bank.run_intensity, bool_to_i64(bank.is_in_resolution), i64::from(bank.months_in_resolution), i64::from(bank.resolution_distress_streak), i64::from(bank.resolution_stable_streak), i64::from(bank.employee_count), bank.monthly_payroll_budget, bank.last_month_payroll_paid, bank.wage_offer_multiplier, bank.last_month_interest_income, bank.last_month_deposit_interest_expense, bank.last_month_credit_losses, bank.last_month_profit])?;
        insert_small_rows(
            conn,
            "bank_open_positions",
            country_id,
            bank.id,
            bank.open_positions.iter().enumerate().map(|(i, op)| {
                vec![
                    v(country_id),
                    v(bank.id),
                    Value::from(i as i64),
                    v(op.job_type_id),
                    v(op.slots),
                    Value::from(op.offered_monthly_salary),
                ]
            }),
        )?;
        insert_small_rows(
            conn,
            "bank_stock_positions",
            country_id,
            bank.id,
            bank.stock_positions.iter().enumerate().map(|(i, sp)| {
                vec![
                    v(country_id),
                    v(bank.id),
                    Value::from(i as i64),
                    v(sp.security_country_id),
                    v(sp.security_company_id),
                    Value::from(sp.shares as i64),
                    Value::from(sp.avg_cost),
                ]
            }),
        )?;
    }
    Ok(())
}

fn insert_stock_market(
    conn: &Connection,
    country_id: u16,
    market: &StockMarketDto,
) -> Result<(), PersistenceError> {
    conn.execute("INSERT INTO country_stock_markets (country_id, index_base, ipo_raised_mtd, dividends_paid_mtd, last_completed_month_dividends_paid, forced_liquidations_mtd, last_completed_month_ipo_raised, last_completed_month_forced_liquidations, bank_portfolio_realized_pnl_mtd, bank_portfolio_dividends_mtd, bank_portfolio_last_completed_month_dividends, bank_portfolio_last_completed_month_realized_pnl) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)", params![i64::from(country_id), market.index_base, market.ipo_raised_mtd, market.dividends_paid_mtd, market.last_completed_month_dividends_paid, market.forced_liquidations_mtd, market.last_completed_month_ipo_raised, market.last_completed_month_forced_liquidations, market.bank_portfolio.realized_pnl_mtd, market.bank_portfolio.dividends_mtd, market.bank_portfolio.last_completed_month_dividends, market.bank_portfolio.last_completed_month_realized_pnl])?;
    let mut portfolios: BTreeMap<u32, &PortfolioDto> = BTreeMap::new();
    for (person_id, portfolio) in &market.person_portfolios {
        if let Ok(person_id) = person_id.parse::<u32>() {
            portfolios.insert(person_id, portfolio);
        }
    }
    for (person_id, portfolio) in portfolios {
        conn.execute("INSERT INTO stock_market_person_portfolios (country_id, person_id, realized_pnl_mtd, dividends_mtd, last_completed_month_dividends, last_completed_month_realized_pnl) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![i64::from(country_id), i64::from(person_id), portfolio.realized_pnl_mtd, portfolio.dividends_mtd, portfolio.last_completed_month_dividends, portfolio.last_completed_month_realized_pnl])?;
        execute_rows(
            conn,
            "INSERT INTO stock_market_person_positions (country_id, person_id, position_index, security_country_id, security_company_id, shares, avg_cost, pledged_shares) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            portfolio.positions.iter().enumerate().map(|(i, pos)| {
                vec![
                    v(country_id),
                    v(person_id),
                    Value::from(i as i64),
                    v(pos.security_country_id),
                    v(pos.security_company_id),
                    Value::from(pos.shares as i64),
                    Value::from(pos.avg_cost),
                    Value::from(pos.pledged_shares as i64),
                ]
            }),
        )?;
    }
    execute_rows(
        conn,
        "INSERT INTO stock_market_bank_positions (country_id, position_index, security_country_id, security_company_id, shares, avg_cost, pledged_shares) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        market
            .bank_portfolio
            .positions
            .iter()
            .enumerate()
            .map(|(i, pos)| {
                vec![
                    v(country_id),
                    Value::from(i as i64),
                    v(pos.security_country_id),
                    v(pos.security_company_id),
                    Value::from(pos.shares as i64),
                    Value::from(pos.avg_cost),
                    Value::from(pos.pledged_shares as i64),
                ]
            }),
    )?;
    let mut listed: BTreeMap<u16, _> = BTreeMap::new();
    for (listing_company_id, equity) in &market.listed {
        if let Ok(id) = listing_company_id.parse::<u16>() {
            listed.insert(id, equity);
        }
    }
    for (listing_company_id, equity) in listed {
        conn.execute("INSERT INTO stock_market_listed_equities (country_id, listing_company_id, current_company_id, ticker, shares_outstanding, free_float_shares, last_price, prev_close, last_closed_day_pct_change, day_volume, day_turnover, day_trades_count, market_cap, dps_cursor) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)", params![i64::from(country_id), i64::from(listing_company_id), i64::from(equity.current_company_id), equity.ticker, equity.shares_outstanding as i64, equity.free_float_shares as i64, equity.last_price, equity.prev_close, equity.last_closed_day_pct_change, equity.day_volume as i64, equity.day_turnover, equity.day_trades_count as i64, equity.market_cap, i64::from(equity.dps_cursor)])?;
        insert_indexed_real_series(
            conn,
            "stock_market_equity_price_history",
            "listing_company_id",
            country_id,
            listing_company_id,
            &equity.price_history,
        )?;
        execute_rows(
            conn,
            "INSERT INTO stock_market_equity_volume_history (country_id, listing_company_id, history_index, value) VALUES (?1, ?2, ?3, ?4)",
            equity.volume_history.iter().enumerate().map(|(i, value)| {
                vec![
                    v(country_id),
                    v(listing_company_id),
                    Value::from(i as i64),
                    Value::from(*value as i64),
                ]
            }),
        )?;
        insert_indexed_real_series(
            conn,
            "stock_market_equity_market_cap_history",
            "listing_company_id",
            country_id,
            listing_company_id,
            &equity.market_cap_history,
        )?;
        execute_rows(
            conn,
            "INSERT INTO stock_market_equity_monthly_dps (country_id, listing_company_id, slot_index, value) VALUES (?1, ?2, ?3, ?4)",
            equity.monthly_dps.iter().enumerate().map(|(i, value)| {
                vec![
                    v(country_id),
                    v(listing_company_id),
                    Value::from(i as i64),
                    Value::from(*value),
                ]
            }),
        )?;
    }
    execute_rows(
        conn,
        "INSERT INTO stock_market_daily_index_history (country_id, history_index, day, index_value, total_turnover, bank_equity_exposure) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        market.daily_index_history.iter().enumerate().map(|(i, p)| {
            vec![
                v(country_id),
                Value::from(i as i64),
                Value::from(p.day as i64),
                Value::from(p.index_value),
                Value::from(p.total_turnover),
                Value::from(p.bank_equity_exposure),
            ]
        }),
    )?;
    Ok(())
}

fn insert_metric_points(
    conn: &Connection,
    table: &str,
    country_index: Option<i64>,
    points: &[MetricPointDto],
) -> Result<(), PersistenceError> {
    const METRIC_COLUMNS: &str = "history_index, month, fx_rate_local_per_base, fx_reserves_base, fx_net_flow_base, fx_pressure, population, total_money_supply, household_cash_total, company_cash_total, expected_money_supply, money_supply_gap, created_money_outstanding, bank_reserve, treasury_balance, budget_balance, monthly_tax_collected, monthly_gov_spending, gov_debt, monthly_money_minted, cumulative_money_minted, monthly_mint_fraction, monthly_goods_import_spending, monthly_goods_export_revenue, monthly_unmet_need_events, monthly_severe_unmet_need_events, monthly_chronic_unmet_need_count, monthly_essential_debt_spending, cpi_index, inflation_rate, policy_rate, deposit_rate, loan_rate, monthly_savings_interest_paid, bonds_outstanding, stock_market_cap, stock_bank_equity_exposure, housing_price_index, monthly_rent, homeownership_rate, housing_vacancy_rate, monthly_foreclosures, monthly_home_purchases, monthly_immigrants, monthly_emigrants, monthly_migrants_for_work, monthly_migrants_for_distress";
    let parameter_count = if country_index.is_some() { 48 } else { 47 };
    let placeholders = (1..=parameter_count)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let columns = if country_index.is_some() {
        format!("country_index, {METRIC_COLUMNS}")
    } else {
        METRIC_COLUMNS.to_string()
    };
    let sql = format!("INSERT INTO {table} ({columns}) VALUES ({placeholders})");
    execute_rows(
        conn,
        &sql,
        points
            .iter()
            .enumerate()
            .map(|(i, p)| metric_row(country_index, i as i64, p)),
    )
}

fn insert_event(
    conn: &Connection,
    owner_scope: &str,
    owner_country_id: i64,
    event_seq: i64,
    event: &SimEventDto,
    name_to_country_id: &HashMap<&str, u16>,
) -> Result<(), PersistenceError> {
    const EVENT_COLUMNS: &[&str] = &[
        "owner_scope",
        "owner_country_id",
        "event_seq",
        "event_kind",
        "day",
        "country_name",
        "from_country_name",
        "to_country_name",
        "home_country_name",
        "foreign_country_name",
        "company_name",
        "bank_name",
        "acquired_by_bank_name",
        "ticker",
        "currency_code",
        "disaster_label",
        "summary",
        "person_id",
        "company_id",
        "bank_id",
        "entity_id",
        "job_type_id",
        "produced_good_id",
        "acquired_by_bank_id",
        "from_company_id",
        "to_company_id",
        "source_country_id",
        "target_country_id",
        "entity_kind",
        "ipo_reason",
        "bank_kind",
        "count",
        "founders",
        "entities_affected",
        "migrated_depositors",
        "migrated_companies",
        "migrated_loans",
        "laid_off_count",
        "quits",
        "retirements",
        "layoffs_non_distress",
        "replacement_openings",
        "months_unemployed",
        "months_on_benefits",
        "total_recipients",
        "listed_count",
        "intensity_pct",
        "shares_issued",
        "primary_shares_issued",
        "secondary_shares_sold",
        "shares_liquidated",
        "shares_wiped",
        "principal",
        "principal_lost",
        "unpaid_amount",
        "total_amount",
        "total_funding",
        "treasury_funding",
        "deficit_funding",
        "seed_capital",
        "health",
        "requested",
        "haircut_loss",
        "treasury_support",
        "transferred_reserves",
        "amount_lost",
        "cut_rate",
        "monthly_salary",
        "principal_written_off",
        "price_per_share",
        "total_raised",
        "primary_raised",
        "secondary_raised",
        "founder_payout_total",
        "per_share",
        "proceeds",
        "total_market_cap",
        "total_turnover",
        "new_rate",
        "old_rate",
        "inflation_rate",
        "unemployment_rate",
        "face_value",
        "coupon_rate",
        "total_coupon",
        "shortfall",
        "monetized",
        "amount",
        "inflation_shortfall",
        "unemployment_gap",
        "price",
        "home_amount",
        "foreign_amount",
        "fx_rate_home_per_foreign",
        "annual_rate",
        "returned_home_amount",
        "returned_foreign_amount",
        "pressure",
        "reserves_base",
        "reserve_gap",
        "new_price_index",
        "change_pct",
        "amount_due",
        "amount_paid",
    ];
    let placeholders = (1..=EVENT_COLUMNS.len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO events ({}) VALUES ({})",
        EVENT_COLUMNS.join(", "),
        placeholders
    );
    let values = vec![
        Value::Text(owner_scope.to_string()),
        Value::Integer(owner_country_id),
        Value::Integer(event_seq),
        Value::Text(event.kind.clone()),
        Value::Integer(event.day as i64),
        opt_value(event.country_name.clone()),
        opt_value(event.from_country_name.clone()),
        opt_value(event.to_country_name.clone()),
        opt_value(event.home_country_name.clone()),
        opt_value(event.foreign_country_name.clone()),
        opt_value(event.company_name.clone()),
        opt_value(event.bank_name.clone()),
        opt_value(event.acquired_by_bank_name.clone()),
        opt_value(event.ticker.clone()),
        opt_value(event.currency_code.clone()),
        opt_value(event.disaster_label.clone()),
        opt_value(event.summary.clone()),
        opt_value(event.person_id.map(i64::from)),
        opt_value(event.company_id.map(i64::from)),
        opt_value(event.bank_id.map(i64::from)),
        opt_value(event.entity_id.map(i64::from)),
        opt_value(event.job_type_id.map(i64::from)),
        opt_value(event.produced_good_id.map(i64::from)),
        opt_value(event.acquired_by_bank_id.map(i64::from)),
        opt_value(event.from_company_id.map(i64::from)),
        opt_value(event.to_company_id.map(i64::from)),
        opt_value(resolve_country_id(
            event.from_country_name.as_deref(),
            name_to_country_id,
        )),
        opt_value(
            resolve_country_id(event.to_country_name.as_deref(), name_to_country_id).or_else(
                || resolve_country_id(event.foreign_country_name.as_deref(), name_to_country_id),
            ),
        ),
        opt_value(event.entity_kind.map(i64::from)),
        opt_value(event.ipo_reason.map(i64::from)),
        opt_value(event.bank_kind.map(i64::from)),
        opt_value(event.count.map(i64::from)),
        opt_value(event.founders.map(i64::from)),
        opt_value(event.entities_affected.map(i64::from)),
        opt_value(event.migrated_depositors.map(i64::from)),
        opt_value(event.migrated_companies.map(i64::from)),
        opt_value(event.migrated_loans.map(i64::from)),
        opt_value(event.laid_off_count.map(i64::from)),
        opt_value(event.quits.map(i64::from)),
        opt_value(event.retirements.map(i64::from)),
        opt_value(event.layoffs_non_distress.map(i64::from)),
        opt_value(event.replacement_openings.map(i64::from)),
        opt_value(event.months_unemployed.map(i64::from)),
        opt_value(event.months_on_benefits.map(i64::from)),
        opt_value(event.total_recipients.map(i64::from)),
        opt_value(event.listed_count.map(|v| v as i64)),
        opt_value(event.intensity_pct.map(i64::from)),
        opt_value(event.shares_issued.map(|v| v as i64)),
        opt_value(event.primary_shares_issued.map(|v| v as i64)),
        opt_value(event.secondary_shares_sold.map(|v| v as i64)),
        opt_value(event.shares_liquidated.map(|v| v as i64)),
        opt_value(event.shares_wiped.map(|v| v as i64)),
        opt_value(event.principal),
        opt_value(event.principal_lost),
        opt_value(event.unpaid_amount),
        opt_value(event.total_amount),
        opt_value(event.total_funding),
        opt_value(event.treasury_funding),
        opt_value(event.deficit_funding),
        opt_value(event.seed_capital),
        opt_value(event.health),
        opt_value(event.requested),
        opt_value(event.haircut_loss),
        opt_value(event.treasury_support),
        opt_value(event.transferred_reserves),
        opt_value(event.amount_lost),
        opt_value(event.cut_rate),
        opt_value(event.monthly_salary),
        opt_value(event.principal_written_off),
        opt_value(event.price_per_share),
        opt_value(event.total_raised),
        opt_value(event.primary_raised),
        opt_value(event.secondary_raised),
        opt_value(event.founder_payout_total),
        opt_value(event.per_share),
        opt_value(event.proceeds),
        opt_value(event.total_market_cap),
        opt_value(event.total_turnover),
        opt_value(event.new_rate),
        opt_value(event.old_rate),
        opt_value(event.inflation_rate),
        opt_value(event.unemployment_rate),
        opt_value(event.face_value),
        opt_value(event.coupon_rate),
        opt_value(event.total_coupon),
        opt_value(event.shortfall),
        opt_value(event.monetized),
        opt_value(event.amount),
        opt_value(event.inflation_shortfall),
        opt_value(event.unemployment_gap),
        opt_value(event.price),
        opt_value(event.home_amount),
        opt_value(event.foreign_amount),
        opt_value(event.fx_rate_home_per_foreign),
        opt_value(event.annual_rate),
        opt_value(event.returned_home_amount),
        opt_value(event.returned_foreign_amount),
        opt_value(event.pressure),
        opt_value(event.reserves_base),
        opt_value(event.reserve_gap),
        opt_value(event.new_price_index),
        opt_value(event.change_pct),
        opt_value(event.amount_due),
        opt_value(event.amount_paid),
    ];
    debug_assert_eq!(values.len(), EVENT_COLUMNS.len());
    conn.execute(&sql, rusqlite::params_from_iter(values))?;
    Ok(())
}

fn insert_simple_pairs(
    conn: &Connection,
    table: &str,
    country_id: u16,
    values: &[f64],
) -> Result<(), PersistenceError> {
    let sql =
        format!("INSERT INTO {table} (country_id, good_index, tariff_rate) VALUES (?1, ?2, ?3)");
    execute_rows(
        conn,
        &sql,
        values
            .iter()
            .enumerate()
            .map(|(i, value)| vec![v(country_id), Value::from(i as i64), Value::from(*value)]),
    )
}

fn insert_map_pairs(
    conn: &Connection,
    table: &str,
    key_column: &str,
    value_column: &str,
    country_id: u16,
    values: &HashMap<String, f64>,
) -> Result<(), PersistenceError> {
    let sql = format!(
        "INSERT INTO {table} (country_id, {key_column}, {value_column}) VALUES (?1, ?2, ?3)"
    );
    let ordered: BTreeMap<i64, f64> = values
        .iter()
        .filter_map(|(k, value)| k.parse::<i64>().ok().map(|k| (k, *value)))
        .collect();
    execute_rows(
        conn,
        &sql,
        ordered
            .into_iter()
            .map(|(key, value)| vec![v(country_id), Value::from(key), Value::from(value)]),
    )
}

fn insert_small_rows(
    conn: &Connection,
    table: &str,
    country_id: u16,
    entity_id: u16,
    rows: impl IntoIterator<Item = Vec<Value>>,
) -> Result<(), PersistenceError> {
    let sql = match table {
        "company_open_positions" => {
            "INSERT INTO company_open_positions (country_id, company_id, position_index, job_type_id, slots, offered_monthly_salary) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        }
        "company_offered_job_types" => {
            "INSERT INTO company_offered_job_types (country_id, company_id, job_type_index, job_type_id) VALUES (?1, ?2, ?3, ?4)"
        }
        "company_founder_stakes" => {
            "INSERT INTO company_founder_stakes (country_id, company_id, stake_index, person_id, shares_owned, cash_contributed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        }
        "company_historical_ids" => {
            "INSERT INTO company_historical_ids (country_id, company_id, history_index, historical_company_id) VALUES (?1, ?2, ?3, ?4)"
        }
        "bank_open_positions" => {
            "INSERT INTO bank_open_positions (country_id, bank_id, position_index, job_type_id, slots, offered_monthly_salary) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        }
        "bank_stock_positions" => {
            "INSERT INTO bank_stock_positions (country_id, bank_id, position_index, security_country_id, security_company_id, shares, avg_cost) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        }
        _ => return Ok(()),
    };
    let _ = (country_id, entity_id);
    execute_rows(conn, sql, rows)
}

fn insert_slot_values<T: Copy>(
    conn: &Connection,
    table: &str,
    country_id: u16,
    person_id: u32,
    values: &[T],
) -> Result<(), PersistenceError>
where
    Value: From<T>,
{
    let sql = format!(
        "INSERT INTO {table} (country_id, person_id, slot_index, units) VALUES (?1, ?2, ?3, ?4)"
    );
    execute_rows(
        conn,
        &sql,
        values.iter().enumerate().map(|(i, value)| {
            vec![
                Value::Integer(i64::from(country_id)),
                Value::Integer(i64::from(person_id)),
                Value::Integer(i as i64),
                Value::from(*value),
            ]
        }),
    )
}

fn metric_row(country_index: Option<i64>, history_index: i64, p: &MetricPointDto) -> Vec<Value> {
    let mut row = Vec::with_capacity(48);
    if let Some(country_index) = country_index {
        row.push(Value::from(country_index));
    }
    row.extend([
        Value::from(history_index),
        Value::from(p.month as i64),
        Value::from(p.fx_rate_local_per_base),
        Value::from(p.fx_reserves_base),
        Value::from(p.fx_net_flow_base),
        Value::from(p.fx_pressure),
        Value::from(p.population),
        Value::from(p.total_money_supply),
        Value::from(p.household_cash_total),
        Value::from(p.company_cash_total),
        Value::from(p.expected_money_supply),
        Value::from(p.money_supply_gap),
        Value::from(p.created_money_outstanding),
        Value::from(p.bank_reserve),
        Value::from(p.treasury_balance),
        Value::from(p.budget_balance),
        Value::from(p.monthly_tax_collected),
        Value::from(p.monthly_gov_spending),
        Value::from(p.gov_debt),
        Value::from(p.monthly_money_minted),
        Value::from(p.cumulative_money_minted),
        Value::from(p.monthly_mint_fraction),
        Value::from(p.monthly_goods_import_spending),
        Value::from(p.monthly_goods_export_revenue),
        Value::from(p.monthly_unmet_need_events),
        Value::from(p.monthly_severe_unmet_need_events),
        Value::from(p.monthly_chronic_unmet_need_count),
        Value::from(p.monthly_essential_debt_spending),
        Value::from(p.cpi_index),
        Value::from(p.inflation_rate),
        Value::from(p.policy_rate),
        Value::from(p.deposit_rate),
        Value::from(p.loan_rate),
        Value::from(p.monthly_savings_interest_paid),
        Value::from(p.bonds_outstanding),
        Value::from(p.stock_market_cap),
        Value::from(p.stock_bank_equity_exposure),
        Value::from(p.housing_price_index),
        Value::from(p.monthly_rent),
        Value::from(p.homeownership_rate),
        Value::from(p.housing_vacancy_rate),
        Value::from(p.monthly_foreclosures),
        Value::from(p.monthly_home_purchases),
        Value::from(p.monthly_immigrants),
        Value::from(p.monthly_emigrants),
        Value::from(p.monthly_migrants_for_work),
        Value::from(p.monthly_migrants_for_distress),
    ]);
    row
}

fn resolve_country_id(name: Option<&str>, map: &HashMap<&str, u16>) -> Option<i64> {
    name.and_then(|name| map.get(name).copied()).map(i64::from)
}

fn opt_value<T>(value: Option<T>) -> Value
where
    Value: From<T>,
{
    value.map(Value::from).unwrap_or(Value::Null)
}

fn v<T>(value: T) -> Value
where
    Value: From<T>,
{
    Value::from(value)
}
