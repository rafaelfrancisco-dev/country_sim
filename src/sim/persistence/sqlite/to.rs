use crate::sim::{
    bank::LoanAccount,
    company::Company,
    country::Country,
    currency::CurrencySpec,
    person::{EducationTrack, EmploymentStatus, HabilitationLevel, HousingStatus, Person},
    world::{BankMetricPoint, MetricPoint},
};

use super::super::dto::{
    BankMetricPointDto, CompanyDto, CountryDto, CurrencySpecDto, EmploymentStatusDto,
    ForeignDepositDto, FounderStakeDto, HousingStatusDto, JobOpeningDto, LoanAccountDto,
    MetricPointDto, MortgageAccountDto, PersonDto, SimEventDto, SimulationParamsDto,
};

pub(super) fn metric_point_to_dto(p: &MetricPoint) -> MetricPointDto {
    MetricPointDto {
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

pub(super) fn bank_metric_point_to_dto(p: &BankMetricPoint) -> BankMetricPointDto {
    BankMetricPointDto {
        month: p.month,
        reserve: p.reserve,
        deposits: p.deposits,
        created_money: p.created_money,
        equity: p.equity,
        run_intensity: p.run_intensity,
    }
}

pub(super) fn country_to_dto(country: &Country) -> CountryDto {
    use super::to_rest::{bank_to_dto, household_to_dto, sim_event_to_dto, stock_market_to_dto};
    use std::collections::HashMap;

    let fx_reserves_by_source: HashMap<String, f64> = country
        .fx_reserves_by_source
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let export_tariffs_by_good: Vec<f64> = country.export_tariffs_by_good.to_vec();
    let import_tariffs_by_good: Vec<f64> = country.import_tariffs_by_good.to_vec();
    let pending_events: Vec<SimEventDto> = country
        .pending_events
        .iter()
        .map(sim_event_to_dto)
        .collect();

    let people: Vec<PersonDto> = country.people.iter().map(person_to_dto).collect();
    let companies: Vec<CompanyDto> = country.companies.iter().map(company_to_dto).collect();
    let banks = country.banks.iter().map(bank_to_dto).collect();
    let households = country.households.iter().map(household_to_dto).collect();
    let stock_market = stock_market_to_dto(&country.stock_market);

    CountryDto {
        id: country.id,
        name: country.name.to_string(),
        currency: currency_spec_to_dto(&country.currency),
        day: country.day,
        month: country.month,
        next_startup_serial: country.next_startup_serial,
        next_person_id: country.next_person_id,
        birth_carry: country.birth_carry,
        last_day_births: country.last_day_births,
        last_day_deaths: country.last_day_deaths,
        sim_params: SimulationParamsDto::from(country.sim_params.clone()),
        treasury_balance: country.treasury_balance,
        treasury_bank_id: country.treasury_bank_id,
        monthly_tax_collected: country.monthly_tax_collected,
        monthly_corporate_tax_collected: country.monthly_corporate_tax_collected,
        monthly_household_spending: country.monthly_household_spending,
        monthly_tariff_collected: country.monthly_tariff_collected,
        monthly_goods_import_spending: country.monthly_goods_import_spending,
        monthly_goods_import_spending_base: country.monthly_goods_import_spending_base,
        monthly_goods_export_revenue: country.monthly_goods_export_revenue,
        monthly_goods_export_revenue_base: country.monthly_goods_export_revenue_base,
        monthly_unmet_need_events: country.monthly_unmet_need_events,
        monthly_severe_unmet_need_events: country.monthly_severe_unmet_need_events,
        monthly_chronic_unmet_need_count: country.monthly_chronic_unmet_need_count,
        monthly_essential_debt_spending: country.monthly_essential_debt_spending,
        monthly_hires_total: country.monthly_hires_total,
        monthly_immigrants: country.monthly_immigrants,
        monthly_emigrants: country.monthly_emigrants,
        monthly_migrants_for_work: country.monthly_migrants_for_work,
        monthly_migrants_for_distress: country.monthly_migrants_for_distress,
        monthly_private_banks_founded: country.monthly_private_banks_founded,
        cumulative_private_banks_founded: country.cumulative_private_banks_founded,
        monthly_private_banks_closed: country.monthly_private_banks_closed,
        cumulative_private_banks_closed: country.cumulative_private_banks_closed,
        monthly_gov_spending: country.monthly_gov_spending,
        monthly_budget_balance: country.monthly_budget_balance,
        monthly_gdp: country.monthly_gdp,
        austerity_active: country.austerity_active,
        gov_debt: country.gov_debt,
        bonds_outstanding: country.bonds_outstanding,
        bond_coupon_rate: country.bond_coupon_rate,
        bond_months_to_maturity: country.bond_months_to_maturity,
        monthly_coupon_paid: country.monthly_coupon_paid,
        monthly_bonds_issued: country.monthly_bonds_issued,
        monthly_bonds_matured: country.monthly_bonds_matured,
        monthly_savings_interest_paid: country.monthly_savings_interest_paid,
        monthly_fx_financial_inflows_base: country.monthly_fx_financial_inflows_base,
        monthly_fx_financial_outflows_base: country.monthly_fx_financial_outflows_base,
        monthly_fx_net_flow_base: country.monthly_fx_net_flow_base,
        monthly_money_minted: country.monthly_money_minted,
        cumulative_money_minted: country.cumulative_money_minted,
        last_mint_inflation_shortfall: country.last_mint_inflation_shortfall,
        last_mint_unemployment_gap: country.last_mint_unemployment_gap,
        last_mint_fraction: country.last_mint_fraction,
        policy_rate: country.policy_rate,
        fx_rate_local_per_base: country.fx_rate_local_per_base,
        fx_reserves_base: country.fx_reserves_base,
        fx_reserves_by_source,
        fx_pressure: country.fx_pressure,
        cpi_index: country.cpi_index,
        last_month_cpi_index: country.last_month_cpi_index,
        monthly_inflation_rate: country.monthly_inflation_rate,
        stock_market,
        export_tariffs_by_good,
        import_tariffs_by_good,
        pending_events,
        initial_money_supply: country.initial_money_supply,
        household_cash_total: country.household_cash_total,
        company_cash_total: country.company_cash_total,
        total_money_supply: country.total_money_supply,
        expected_money_supply: country.expected_money_supply,
        money_supply_gap: country.money_supply_gap,
        next_bank_id: country.next_bank_id,
        next_bank_serial: country.next_bank_serial,
        public_bank_id: country.public_bank_id,
        housing_stock_total: country.housing_stock_total,
        housing_stock_occupied_owned: country.housing_stock_occupied_owned,
        housing_stock_occupied_rented: country.housing_stock_occupied_rented,
        housing_stock_vacant: country.housing_stock_vacant,
        housing_price_index: country.housing_price_index,
        monthly_rent: country.monthly_rent,
        monthly_rent_collected: country.monthly_rent_collected,
        monthly_mortgage_payments: country.monthly_mortgage_payments,
        monthly_home_purchases: country.monthly_home_purchases,
        monthly_foreclosures: country.monthly_foreclosures,
        monthly_housing_units_built: country.monthly_housing_units_built,
        people,
        companies,
        banks,
        households,
    }
}

pub(super) fn currency_spec_to_dto(c: &CurrencySpec) -> CurrencySpecDto {
    CurrencySpecDto {
        code: c.code.to_string(),
        name: c.name.to_string(),
        symbol: c.symbol.to_string(),
    }
}

pub(super) fn person_to_dto(p: &Person) -> PersonDto {
    let employment = match p.employment {
        EmploymentStatus::Unemployed => EmploymentStatusDto::Unemployed,
        EmploymentStatus::Employed {
            job_type_id,
            employer_company_id,
        } => EmploymentStatusDto::Employed {
            job_type_id,
            employer_company_id,
        },
        EmploymentStatus::EmployedAtBank {
            job_type_id,
            bank_id,
        } => EmploymentStatusDto::EmployedAtBank {
            job_type_id,
            bank_id,
        },
    };

    let housing = match &p.housing {
        HousingStatus::Unhoused => HousingStatusDto::Unhoused,
        HousingStatus::Renter {
            landlord_company_id,
            landlord_country_id,
        } => HousingStatusDto::Renter {
            landlord_company_id: *landlord_company_id,
            landlord_country_id: *landlord_country_id,
        },
        HousingStatus::Owner => HousingStatusDto::Owner,
        HousingStatus::Mortgaged { mortgage } => HousingStatusDto::Mortgaged {
            mortgage: MortgageAccountDto {
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

    let education_track = match p.education_track {
        EducationTrack::BasicOnly => 0,
        EducationTrack::HighSchool => 1,
        EducationTrack::Bachelor => 2,
        EducationTrack::Doctor => 3,
    };

    let habilitation = match p.habilitation {
        HabilitationLevel::NoSchoolYet => 0,
        HabilitationLevel::BasicSchool => 1,
        HabilitationLevel::HighSchool => 2,
        HabilitationLevel::HigherLearning => 3,
    };

    let loan = p.loan.as_ref().map(loan_account_to_dto);

    let foreign_deposit = p.foreign_deposit.as_ref().map(|fd| ForeignDepositDto {
        country_id: fd.country_id,
        bank_id: fd.bank_id,
        balance: fd.balance,
        home_amount_at_open: fd.home_amount_at_open,
        opening_fx_rate_home_per_foreign: fd.opening_fx_rate_home_per_foreign,
    });

    PersonDto {
        id: p.id,
        birth_country_id: p.birth_country_id,
        current_country_id: p.current_country_id,
        employment,
        employment_start_day: p.employment_start_day,
        agreed_monthly_salary: p.agreed_monthly_salary,
        cash_balance: p.cash_balance,
        deposit_bank_id: p.deposit_bank_id,
        household_id: p.household_id,
        name_seed: p.name_seed,
        age_days: p.age_days,
        education_track,
        habilitation,
        loan,
        last_month_gross_pay: p.last_month_gross_pay,
        last_month_tax_paid: p.last_month_tax_paid,
        last_month_net_pay: p.last_month_net_pay,
        last_completed_month_gross_pay: p.last_completed_month_gross_pay,
        last_completed_month_tax_paid: p.last_completed_month_tax_paid,
        last_completed_month_net_pay: p.last_completed_month_net_pay,
        last_month_spending: p.last_month_spending,
        last_month_goods_spending: p.last_month_goods_spending,
        last_month_imported_goods_spending: p.last_month_imported_goods_spending,
        last_month_goods_units_bought: p.last_month_goods_units_bought,
        last_month_needs_met_mask: p.last_month_needs_met_mask,
        last_month_unmet_needs_mask: p.last_month_unmet_needs_mask,
        last_month_severe_unmet_needs_mask: p.last_month_severe_unmet_needs_mask,
        consecutive_severe_unmet_months: p.consecutive_severe_unmet_months,
        last_month_chronic_unmet: p.last_month_chronic_unmet,
        last_month_loan_payment: p.last_month_loan_payment,
        last_month_new_loan: p.last_month_new_loan,
        last_month_savings_interest: p.last_month_savings_interest,
        last_month_rent_paid: p.last_month_rent_paid,
        last_month_mortgage_payment: p.last_month_mortgage_payment,
        housing,
        months_unemployed: p.months_unemployed,
        months_on_benefits: p.months_on_benefits,
        lifetime_migrations: p.lifetime_migrations,
        last_migration_day: p.last_migration_day,
        essential_stock_units: p.essential_stock_units.to_vec(),
        superficial_stock_units: p.superficial_stock_units.to_vec(),
        superficial_stock_consumption_carry: p.superficial_stock_consumption_carry.to_vec(),
        foreign_deposit,
    }
}

pub(super) fn loan_account_to_dto(l: &LoanAccount) -> LoanAccountDto {
    LoanAccountDto {
        debt_balance: l.debt_balance,
        created_principal_outstanding: l.created_principal_outstanding,
        monthly_payment: l.monthly_payment,
        annual_rate: l.annual_rate,
        term_months: l.term_months,
        months_remaining: l.months_remaining,
        lender_bank_id: l.lender_bank_id,
    }
}

pub(super) fn company_to_dto(c: &Company) -> CompanyDto {
    let open_positions = c
        .open_positions
        .iter()
        .map(|op| JobOpeningDto {
            job_type_id: op.job_type_id,
            slots: op.slots,
            offered_monthly_salary: op.offered_monthly_salary,
        })
        .collect();

    let founder_stakes = c
        .founder_stakes
        .iter()
        .map(|fs| FounderStakeDto {
            person_id: fs.person_id,
            shares_owned: fs.shares_owned,
            cash_contributed: fs.cash_contributed,
        })
        .collect();

    CompanyDto {
        id: c.id,
        historical_ids: c.historical_ids.clone(),
        name: c.name.to_string(),
        is_public_utility: c.is_public_utility,
        is_essential_backfill: c.is_essential_backfill,
        offered_job_types: c.offered_job_types.clone(),
        produced_good_id: c.produced_good_id,
        open_positions,
        founder_stakes,
        employee_count: c.employee_count,
        manufacturing_employee_count: c.manufacturing_employee_count,
        inventory_units: c.inventory_units,
        unit_price: c.unit_price,
        wage_offer_multiplier: c.wage_offer_multiplier,
        consecutive_missed_loan_payments: c.consecutive_missed_loan_payments,
        missed_loan_payment_this_month: c.missed_loan_payment_this_month,
        production_effort_carry: c.production_effort_carry,
        cash_balance: c.cash_balance,
        deposit_bank_id: c.deposit_bank_id,
        monthly_payroll_budget: c.monthly_payroll_budget,
        last_month_payroll_paid: c.last_month_payroll_paid,
        last_completed_month_payroll_paid: c.last_completed_month_payroll_paid,
        last_month_sales: c.last_month_sales,
        last_month_profit: c.last_month_profit,
        last_completed_month_profit: c.last_completed_month_profit,
        consecutive_profitable_months: c.consecutive_profitable_months,
        consecutive_loss_months: c.consecutive_loss_months,
        last_month_loan_payment: c.last_month_loan_payment,
        last_completed_month_loan_payment: c.last_completed_month_loan_payment,
        last_month_new_loan: c.last_month_new_loan,
        last_completed_month_new_loan: c.last_completed_month_new_loan,
        last_month_savings_interest: c.last_month_savings_interest,
        last_completed_month_savings_interest: c.last_completed_month_savings_interest,
        last_month_units_produced: c.last_month_units_produced,
        last_month_units_sold: c.last_month_units_sold,
        last_month_export_units_sold: c.last_month_export_units_sold,
        last_month_domestic_units_sold: c.last_month_domestic_units_sold,
        last_month_sell_through: c.last_month_sell_through,
        sales_history: c.sales_history.iter().copied().collect(),
        profit_history: c.profit_history.iter().copied().collect(),
        sold_units_history: c.sold_units_history.iter().copied().collect(),
        employee_count_history: c.employee_count_history.iter().copied().collect(),
        price_history: c.price_history.iter().copied().collect(),
        loan_payment_history: c.loan_payment_history.iter().copied().collect(),
        cash_balance_history: c.cash_balance_history.iter().copied().collect(),
        age_months: c.age_months,
        ipo_cooldown_months: c.ipo_cooldown_months,
        failed_ipo_attempts: c.failed_ipo_attempts,
        loan: c.loan.as_ref().map(loan_account_to_dto),
        rental_units_owned: c.rental_units_owned,
        last_month_rent_revenue: c.last_month_rent_revenue,
    }
}
