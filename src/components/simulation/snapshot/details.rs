use super::*;

impl SetupSnapshot {
    pub(in crate::components::simulation) fn from_setup_form(
        setup: &crate::sim::setup::SetupForm,
        is_generating: bool,
    ) -> Self {
        Self {
            country_count_input: setup.country_count_input.clone(),
            millions_per_country_input: setup.millions_per_country_input.clone(),
            generation_seed_input: setup.generation_seed_input.clone(),
            active_field: setup.active_field,
            validation_error: setup.validation_error.clone(),
            memory_warning: setup.memory_warning.clone(),
            is_generating,
        }
    }
}

pub(in crate::components::simulation) fn person_details_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let Some(person) = ui.selected_person_on_page(world) else {
        return vec![
            "No person available for the current selection. Press [Esc] to return.".to_string(),
        ];
    };

    let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
    let name = person_name_from_seed(name_seed);
    let job_label = person
        .job_type_id()
        .and_then(job_by_id)
        .map_or("Unemployed".to_string(), |job| job.name.to_string());
    let employer_label = person
        .employer_company_id()
        .and_then(|company_id| country.company_name(company_id))
        .unwrap_or("N/A")
        .to_string();
    let employment_label = if person.is_employed() {
        "Employed"
    } else {
        "Unemployed"
    };

    vec![
        format!("Name: {name}"),
        format!("Citizen ID: {}", person.id),
        format!("Country: {}", country.name),
        format!("Day: {}", world.day),
        format!("Age: {}", person.age_years()),
        format!("Education Track: {}", person.education_track.label()),
        format!("Habilitation: {}", person.habilitation.label()),
        String::new(),
        format!("Employment: {employment_label}"),
        format!("Job: {job_label}"),
        format!("Employer: {employer_label}"),
        format!("Monthly Salary: {}", money(person.monthly_salary())),
        format!("Cash Balance: {}", money(person.cash_balance)),
        format!(
            "Loan Debt: {}",
            money(person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance))
        ),
        String::new(),
        format!("MTD Gross Pay: {}", money(person.last_month_gross_pay)),
        format!("MTD Tax Paid: {}", money(person.last_month_tax_paid)),
        format!("MTD Net Pay: {}", money(person.last_month_net_pay)),
        format!("MTD Spending: {}", money(person.last_month_spending)),
        format!(
            "MTD Loan Payment: {}",
            money(person.last_month_loan_payment)
        ),
        format!("MTD New Loan: {}", money(person.last_month_new_loan)),
        String::new(),
        format!(
            "MTD Goods Spending: {}",
            money(person.last_month_goods_spending)
        ),
        format!(
            "MTD Imported Goods Spending: {}",
            money(person.last_month_imported_goods_spending)
        ),
        format!(
            "MTD Goods Units Bought: {}",
            person.last_month_goods_units_bought
        ),
        format!(
            "Needs Met: {}",
            goods_mask_names(person.last_month_needs_met_mask)
        ),
        format!(
            "Needs Unmet: {}",
            goods_mask_names(person.last_month_unmet_needs_mask)
        ),
        "Press [c] to open employer company details in Goods tab.".to_string(),
    ]
}

pub(in crate::components::simulation) fn country_details_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    vec![
        format!("Name: {}", country.name),
        format!("Country ID: {}", country.id),
        format!("Day: {}", world.day),
        format!("Month: {}", country.month),
        String::new(),
        format!("Population: {}", compact_count(country.population() as f64)),
        format!("Household Cash: {}", money(country.household_cash_total)),
        format!("Company Cash: {}", money(country.company_cash_total)),
        format!("Treasury: {}", money(country.treasury_balance)),
        format!("Bank Reserve: {}", money(country.bank.reserve_balance)),
        format!(
            "Loans Outstanding: {}",
            money(country.bank.created_money_outstanding)
        ),
        format!("Money Supply: {}", money(country.total_money_supply)),
        format!("Expected Supply: {}", money(country.expected_money_supply)),
        format!("Supply Gap: {}", money(country.money_supply_gap)),
        format!(
            "MTD Tax Collected: {}",
            money(country.monthly_tax_collected)
        ),
        format!("MTD Gov Spending: {}", money(country.monthly_gov_spending)),
        format!(
            "MTD Budget Balance: {}",
            money(country.monthly_budget_balance)
        ),
        format!("Monetized Debt: {}", money(country.gov_debt)),
        format!(
            "Bonds Outstanding: {}",
            money(country.bonds_outstanding)
        ),
        format!(
            "Bond Coupon Rate: {:.2}%",
            country.bond_coupon_rate * 100.0
        ),
        format!(
            "Bond Months to Maturity: {}",
            country.bond_months_to_maturity
        ),
        format!("MTD Coupon Paid: {}", money(country.monthly_coupon_paid)),
        format!("MTD Bonds Issued: {}", money(country.monthly_bonds_issued)),
        format!(
            "MTD Bonds Matured: {}",
            money(country.monthly_bonds_matured)
        ),
        String::new(),
        format!("CPI Index: {:.4}", country.cpi_index),
        format!(
            "Inflation Rate (ann.): {:.2}%",
            country.monthly_inflation_rate * 100.0
        ),
        format!("Policy Rate: {:.2}%", country.policy_rate * 100.0),
        format!("Bank Loan Rate: {:.2}%", country.bank_loan_rate() * 100.0),
        String::new(),
        format!(
            "MTD Household Spending: {}",
            money(country.monthly_household_spending)
        ),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected)
        ),
        format!(
            "People Page: {}/{}",
            ui.people_page + 1,
            ui.total_people_pages(world)
        ),
    ]
}

pub(in crate::components::simulation) fn company_details_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let Some(company) = ui.selected_company_on_page(world) else {
        return vec!["No company available for the current selection.".to_string()];
    };

    let good_label = good_by_id(company.produced_good_id)
        .map_or("Unknown", |good| good.name)
        .to_string();

    vec![
        format!("Country: {}", country.name),
        format!("Company: {}", company.name),
        format!("Company ID: {}", company.id),
        format!("Produced Good: {}", good_label),
        String::new(),
        format!("Employees: {}", company.employee_count),
        format!(
            "Manufacturing Employees: {}",
            company.manufacturing_employee_count
        ),
        format!("Open Positions: {}", company.open_positions.len()),
        format!("Unit Price: {}", money(company.unit_price)),
        format!(
            "Inventory Units: {}",
            compact_count(company.inventory_units)
        ),
        format!(
            "MTD Produced Units: {}",
            compact_count(company.last_month_units_produced)
        ),
        format!(
            "MTD Sold Units: {}",
            compact_count(company.last_month_units_sold)
        ),
        format!(
            "MTD Domestic/Export Units: {}/{}",
            compact_count(company.last_month_domestic_units_sold),
            compact_count(company.last_month_export_units_sold)
        ),
        format!(
            "MTD Sell Through: {:.2}%",
            company.last_month_sell_through * 100.0
        ),
        String::new(),
        format!("MTD Sales: {}", money(company.last_month_sales)),
        format!(
            "MTD Payroll Paid: {}",
            money(company.last_month_payroll_paid)
        ),
        format!("MTD Profit: {}", money(company.last_month_profit)),
        format!(
            "MTD Loan Payment: {}",
            money(company.last_month_loan_payment)
        ),
        format!("MTD New Loan: {}", money(company.last_month_new_loan)),
        format!("Current Money: {}", money(company.cash_balance)),
        format!(
            "Loan Debt: {}",
            money(company.loan.as_ref().map_or(0.0, |loan| loan.debt_balance))
        ),
    ]
}

pub(in crate::components::simulation) fn selected_company_histories(
    world: &World,
    ui: &UiState,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let Some(company) = ui.selected_company_on_page(world) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };

    (
        company
            .sold_units_history
            .iter()
            .map(|units| f64::from(*units))
            .collect(),
        company
            .employee_count_history
            .iter()
            .map(|count| f64::from(*count))
            .collect(),
        company.price_history.clone(),
        company.loan_payment_history.clone(),
        company.cash_balance_history.clone(),
    )
}

pub(in crate::components::simulation) fn country_goods_summary_lines(world: &World, ui: &UiState) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    vec![
        format!("Country: {} (Month {})", country.name, country.month),
        format!(
            "MTD Household Goods Spending: {}",
            money(country.monthly_household_spending)
        ),
        format!(
            "MTD Imported Goods Spending: {}",
            money(country.monthly_goods_import_spending)
        ),
        format!(
            "MTD Goods Export Revenue: {}",
            money(country.monthly_goods_export_revenue)
        ),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected)
        ),
        format!(
            "MTD Essential Debt Spending: {}",
            money(country.monthly_essential_debt_spending)
        ),
        format!(
            "MTD Unmet Need Events: {}",
            country.monthly_unmet_need_events
        ),
    ]
}


