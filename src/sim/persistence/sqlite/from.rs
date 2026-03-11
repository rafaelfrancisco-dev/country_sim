use std::sync::Arc;

use crate::sim::{
    bank::LoanAccount,
    country::Country,
    currency::CurrencySpec,
    essentials::Household,
    good::GOODS_COUNT,
    person::{
        EducationTrack, EmploymentStatus, ForeignDeposit, HabilitationLevel, HousingStatus, Person,
        ESSENTIAL_STOCK_SLOT_COUNT, SUPERFICIAL_GOOD_SLOT_COUNT,
    },
    world::{BankMetricPoint, MetricPoint},
};

use super::super::{
    PersistenceError,
    dto::{
        CountryDto, EmploymentStatusDto, HousingStatusDto, LoanAccountDto,
        MetricPointDto, BankMetricPointDto, PersonDto,
    },
};

pub(super) fn metric_point_from_dto(p: MetricPointDto) -> MetricPoint {
    MetricPoint {
        month: p.month,
        fx_rate_local_per_base: p.fx_rate_local_per_base,
        fx_reserves_base: p.fx_reserves_base,
        fx_net_flow_base: p.fx_net_flow_base,
        fx_pressure: p.fx_pressure,
        population: p.population,
        total_money_supply: p.total_money_supply,
        household_cash_total: p.household_cash_total,
        company_cash_total: p.company_cash_total,
        expected_money_supply: p.expected_money_supply,
        money_supply_gap: p.money_supply_gap,
        created_money_outstanding: p.created_money_outstanding,
        bank_reserve: p.bank_reserve,
        treasury_balance: p.treasury_balance,
        budget_balance: p.budget_balance,
        monthly_tax_collected: p.monthly_tax_collected,
        monthly_gov_spending: p.monthly_gov_spending,
        gov_debt: p.gov_debt,
        monthly_money_minted: p.monthly_money_minted,
        cumulative_money_minted: p.cumulative_money_minted,
        monthly_mint_fraction: p.monthly_mint_fraction,
        monthly_goods_import_spending: p.monthly_goods_import_spending,
        monthly_goods_export_revenue: p.monthly_goods_export_revenue,
        monthly_unmet_need_events: p.monthly_unmet_need_events,
        monthly_severe_unmet_need_events: p.monthly_severe_unmet_need_events,
        monthly_chronic_unmet_need_count: p.monthly_chronic_unmet_need_count,
        monthly_essential_debt_spending: p.monthly_essential_debt_spending,
        cpi_index: p.cpi_index,
        inflation_rate: p.inflation_rate,
        policy_rate: p.policy_rate,
        deposit_rate: p.deposit_rate,
        loan_rate: p.loan_rate,
        monthly_savings_interest_paid: p.monthly_savings_interest_paid,
        bonds_outstanding: p.bonds_outstanding,
        stock_market_cap: p.stock_market_cap,
        stock_bank_equity_exposure: p.stock_bank_equity_exposure,
        housing_price_index: p.housing_price_index,
        monthly_rent: p.monthly_rent,
        homeownership_rate: p.homeownership_rate,
        housing_vacancy_rate: p.housing_vacancy_rate,
        monthly_foreclosures: p.monthly_foreclosures,
        monthly_home_purchases: p.monthly_home_purchases,
        monthly_immigrants: p.monthly_immigrants,
        monthly_emigrants: p.monthly_emigrants,
        monthly_migrants_for_work: p.monthly_migrants_for_work,
        monthly_migrants_for_distress: p.monthly_migrants_for_distress,
    }
}

pub(super) fn bank_metric_point_from_dto(p: BankMetricPointDto) -> BankMetricPoint {
    BankMetricPoint {
        month: p.month,
        reserve: p.reserve,
        deposits: p.deposits,
        created_money: p.created_money,
        equity: p.equity,
        run_intensity: p.run_intensity,
    }
}

pub(super) fn country_from_dto(dto: CountryDto) -> Result<Country, PersistenceError> {
    use super::from_rest::{bank_from_dto, company_from_dto, household_from_dto, sim_event_from_dto, stock_market_from_dto};
    use std::collections::HashMap;

    let currency = CurrencySpec {
        code: Arc::from(dto.currency.code.as_str()),
        name: Arc::from(dto.currency.name.as_str()),
        symbol: Arc::from(dto.currency.symbol.as_str()),
    };

    let people: Vec<Person> = dto.people.into_iter().map(person_from_dto).collect();
    let companies = dto.companies.into_iter().map(company_from_dto).collect();
    let banks = dto.banks.into_iter().map(bank_from_dto).collect();
    let households: Vec<Household> = dto.households.into_iter().map(household_from_dto).collect();
    let stock_market = stock_market_from_dto(dto.stock_market);

    let fx_reserves_by_source: HashMap<usize, f64> = dto
        .fx_reserves_by_source
        .into_iter()
        .filter_map(|(k, v)| k.parse::<usize>().ok().map(|ki| (ki, v)))
        .collect();

    let mut export_tariffs_by_good = [0.0_f64; GOODS_COUNT];
    for (i, v) in dto.export_tariffs_by_good.into_iter().enumerate() {
        if i < GOODS_COUNT {
            export_tariffs_by_good[i] = v;
        }
    }

    let pending_events = dto
        .pending_events
        .into_iter()
        .map(sim_event_from_dto)
        .collect();

    // Build Country using pub(crate) direct struct construction
    let mut country = Country {
        id: dto.id,
        name: Arc::from(dto.name.as_str()),
        currency,
        people,
        companies,
        households,
        day: dto.day,
        month: dto.month,
        next_startup_serial: dto.next_startup_serial,
        next_person_id: dto.next_person_id,
        birth_carry: dto.birth_carry,
        last_day_births: dto.last_day_births,
        last_day_deaths: dto.last_day_deaths,
        sim_params: dto.sim_params.into(),
        treasury_balance: dto.treasury_balance,
        treasury_bank_id: dto.treasury_bank_id,
        monthly_tax_collected: dto.monthly_tax_collected,
        monthly_corporate_tax_collected: dto.monthly_corporate_tax_collected,
        monthly_household_spending: dto.monthly_household_spending,
        monthly_tariff_collected: dto.monthly_tariff_collected,
        monthly_goods_import_spending: dto.monthly_goods_import_spending,
        monthly_goods_import_spending_base: dto.monthly_goods_import_spending_base,
        monthly_goods_export_revenue: dto.monthly_goods_export_revenue,
        monthly_goods_export_revenue_base: dto.monthly_goods_export_revenue_base,
        monthly_unmet_need_events: dto.monthly_unmet_need_events,
        monthly_severe_unmet_need_events: dto.monthly_severe_unmet_need_events,
        monthly_chronic_unmet_need_count: dto.monthly_chronic_unmet_need_count,
        monthly_essential_debt_spending: dto.monthly_essential_debt_spending,
        monthly_hires_total: dto.monthly_hires_total,
        monthly_immigrants: dto.monthly_immigrants,
        monthly_emigrants: dto.monthly_emigrants,
        monthly_migrants_for_work: dto.monthly_migrants_for_work,
        monthly_migrants_for_distress: dto.monthly_migrants_for_distress,
        monthly_private_banks_founded: dto.monthly_private_banks_founded,
        cumulative_private_banks_founded: dto.cumulative_private_banks_founded,
        monthly_private_banks_closed: dto.monthly_private_banks_closed,
        cumulative_private_banks_closed: dto.cumulative_private_banks_closed,
        monthly_gov_spending: dto.monthly_gov_spending,
        monthly_budget_balance: dto.monthly_budget_balance,
        monthly_gdp: dto.monthly_gdp,
        austerity_active: dto.austerity_active,
        gov_debt: dto.gov_debt,
        bonds_outstanding: dto.bonds_outstanding,
        bond_coupon_rate: dto.bond_coupon_rate,
        bond_months_to_maturity: dto.bond_months_to_maturity,
        monthly_coupon_paid: dto.monthly_coupon_paid,
        monthly_bonds_issued: dto.monthly_bonds_issued,
        monthly_bonds_matured: dto.monthly_bonds_matured,
        monthly_savings_interest_paid: dto.monthly_savings_interest_paid,
        monthly_fx_financial_inflows_base: dto.monthly_fx_financial_inflows_base,
        monthly_fx_financial_outflows_base: dto.monthly_fx_financial_outflows_base,
        monthly_fx_net_flow_base: dto.monthly_fx_net_flow_base,
        monthly_money_minted: dto.monthly_money_minted,
        cumulative_money_minted: dto.cumulative_money_minted,
        last_mint_inflation_shortfall: dto.last_mint_inflation_shortfall,
        last_mint_unemployment_gap: dto.last_mint_unemployment_gap,
        last_mint_fraction: dto.last_mint_fraction,
        policy_rate: dto.policy_rate,
        fx_rate_local_per_base: dto.fx_rate_local_per_base,
        fx_reserves_base: dto.fx_reserves_base,
        fx_reserves_by_source,
        fx_pressure: dto.fx_pressure,
        cpi_index: dto.cpi_index,
        last_month_cpi_index: dto.last_month_cpi_index,
        monthly_inflation_rate: dto.monthly_inflation_rate,
        stock_market,
        export_tariffs_by_good,
        pending_events,
        initial_money_supply: dto.initial_money_supply,
        household_cash_total: dto.household_cash_total,
        company_cash_total: dto.company_cash_total,
        total_money_supply: dto.total_money_supply,
        expected_money_supply: dto.expected_money_supply,
        money_supply_gap: dto.money_supply_gap,
        banks,
        next_bank_id: dto.next_bank_id,
        next_bank_serial: dto.next_bank_serial,
        public_bank_id: dto.public_bank_id,
        housing_stock_total: dto.housing_stock_total,
        housing_stock_occupied_owned: dto.housing_stock_occupied_owned,
        housing_stock_occupied_rented: dto.housing_stock_occupied_rented,
        housing_stock_vacant: dto.housing_stock_vacant,
        housing_price_index: dto.housing_price_index,
        monthly_rent: dto.monthly_rent,
        monthly_rent_collected: dto.monthly_rent_collected,
        monthly_mortgage_payments: dto.monthly_mortgage_payments,
        monthly_home_purchases: dto.monthly_home_purchases,
        monthly_foreclosures: dto.monthly_foreclosures,
        monthly_housing_units_built: dto.monthly_housing_units_built,
        use_inner_parallelism: false,
        runtime: Default::default(),
    };

    // Rebuild runtime indices from people
    country.rebuild_person_runtime();

    Ok(country)
}

pub(super) fn person_from_dto(dto: PersonDto) -> Person {
    use crate::sim::bank::MortgageAccount;

    let employment = match dto.employment {
        EmploymentStatusDto::Unemployed => EmploymentStatus::Unemployed,
        EmploymentStatusDto::Employed { job_type_id, employer_company_id } => {
            EmploymentStatus::Employed { job_type_id, employer_company_id }
        }
        EmploymentStatusDto::EmployedAtBank { job_type_id, bank_id } => {
            EmploymentStatus::EmployedAtBank { job_type_id, bank_id }
        }
    };

    let housing = match dto.housing {
        HousingStatusDto::Unhoused => HousingStatus::Unhoused,
        HousingStatusDto::Renter { landlord_company_id, landlord_country_id } => {
            HousingStatus::Renter { landlord_company_id, landlord_country_id }
        }
        HousingStatusDto::Owner => HousingStatus::Owner,
        HousingStatusDto::Mortgaged { mortgage } => HousingStatus::Mortgaged {
            mortgage: MortgageAccount {
                principal_balance: mortgage.principal_balance,
                original_principal: mortgage.original_principal,
                monthly_payment: mortgage.monthly_payment,
                annual_rate: mortgage.annual_rate,
                term_months: mortgage.term_months,
                months_remaining: mortgage.months_remaining,
                months_missed: mortgage.months_missed,
                lender_bank_id: mortgage.lender_bank_id,
                collateral_value: mortgage.collateral_value,
            },
        },
    };

    let education_track = match dto.education_track {
        1 => EducationTrack::HighSchool,
        2 => EducationTrack::Bachelor,
        3 => EducationTrack::Doctor,
        _ => EducationTrack::BasicOnly,
    };

    let habilitation = match dto.habilitation {
        1 => HabilitationLevel::BasicSchool,
        2 => HabilitationLevel::HighSchool,
        3 => HabilitationLevel::HigherLearning,
        _ => HabilitationLevel::NoSchoolYet,
    };

    let loan = dto.loan.map(loan_account_from_dto);

    let foreign_deposit = dto.foreign_deposit.map(|fd| ForeignDeposit {
        country_id: fd.country_id,
        bank_id: fd.bank_id,
        balance: fd.balance,
        home_amount_at_open: fd.home_amount_at_open,
        opening_fx_rate_home_per_foreign: fd.opening_fx_rate_home_per_foreign,
    });

    // Reconstruct fixed-size arrays from Vecs
    let mut essential_stock_units = [0.0f32; ESSENTIAL_STOCK_SLOT_COUNT];
    for (i, v) in dto.essential_stock_units.into_iter().enumerate() {
        if i < ESSENTIAL_STOCK_SLOT_COUNT {
            essential_stock_units[i] = v;
        }
    }

    let mut superficial_stock_units = [0u16; SUPERFICIAL_GOOD_SLOT_COUNT];
    for (i, v) in dto.superficial_stock_units.into_iter().enumerate() {
        if i < SUPERFICIAL_GOOD_SLOT_COUNT {
            superficial_stock_units[i] = v;
        }
    }

    let mut superficial_stock_consumption_carry = [0.0f32; SUPERFICIAL_GOOD_SLOT_COUNT];
    for (i, v) in dto.superficial_stock_consumption_carry.into_iter().enumerate() {
        if i < SUPERFICIAL_GOOD_SLOT_COUNT {
            superficial_stock_consumption_carry[i] = v;
        }
    }

    Person {
        id: dto.id,
        birth_country_id: dto.birth_country_id,
        current_country_id: dto.current_country_id,
        employment,
        employment_start_day: dto.employment_start_day,
        agreed_monthly_salary: dto.agreed_monthly_salary,
        cash_balance: dto.cash_balance,
        deposit_bank_id: dto.deposit_bank_id,
        household_id: dto.household_id,
        name_seed: dto.name_seed,
        age_days: dto.age_days,
        education_track,
        habilitation,
        loan,
        last_month_gross_pay: dto.last_month_gross_pay,
        last_month_tax_paid: dto.last_month_tax_paid,
        last_month_net_pay: dto.last_month_net_pay,
        last_completed_month_gross_pay: dto.last_completed_month_gross_pay,
        last_completed_month_tax_paid: dto.last_completed_month_tax_paid,
        last_completed_month_net_pay: dto.last_completed_month_net_pay,
        last_month_spending: dto.last_month_spending,
        last_month_goods_spending: dto.last_month_goods_spending,
        last_month_imported_goods_spending: dto.last_month_imported_goods_spending,
        last_month_goods_units_bought: dto.last_month_goods_units_bought,
        last_month_needs_met_mask: dto.last_month_needs_met_mask,
        last_month_unmet_needs_mask: dto.last_month_unmet_needs_mask,
        last_month_severe_unmet_needs_mask: dto.last_month_severe_unmet_needs_mask,
        consecutive_severe_unmet_months: dto.consecutive_severe_unmet_months,
        last_month_chronic_unmet: dto.last_month_chronic_unmet,
        last_month_loan_payment: dto.last_month_loan_payment,
        last_month_new_loan: dto.last_month_new_loan,
        last_month_savings_interest: dto.last_month_savings_interest,
        last_month_rent_paid: dto.last_month_rent_paid,
        last_month_mortgage_payment: dto.last_month_mortgage_payment,
        housing,
        months_unemployed: dto.months_unemployed,
        months_on_benefits: dto.months_on_benefits,
        lifetime_migrations: dto.lifetime_migrations,
        last_migration_day: dto.last_migration_day,
        essential_stock_units,
        superficial_stock_units,
        superficial_stock_consumption_carry,
        foreign_deposit,
    }
}

pub(super) fn loan_account_from_dto(dto: LoanAccountDto) -> LoanAccount {
    LoanAccount {
        debt_balance: dto.debt_balance,
        created_principal_outstanding: dto.created_principal_outstanding,
        monthly_payment: dto.monthly_payment,
        annual_rate: dto.annual_rate,
        term_months: dto.term_months,
        months_remaining: dto.months_remaining,
        lender_bank_id: dto.lender_bank_id,
    }
}
