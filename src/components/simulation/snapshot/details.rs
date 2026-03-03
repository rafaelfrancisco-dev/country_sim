use super::*;

impl SetupSnapshot {
    pub(in crate::components::simulation) fn from_setup_form(
        setup: &crate::sim::setup::SetupForm,
        is_generating: bool,
    ) -> Self {
        Self {
            country_count_input: setup.country_count_input.clone(),
            thousands_per_country_input: setup.thousands_per_country_input.clone(),
            generation_seed_input: setup.generation_seed_input.clone(),
            active_field: setup.active_field,
            validation_error: setup.validation_error.clone(),
            memory_warning: setup.memory_warning.clone(),
            is_generating,
        }
    }
}

pub(in crate::components::simulation) fn person_details_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
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
    let superficial_good_ids: [u8; 5] = [10, 11, 12, 13, 14];
    let mut lines = vec![
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
    ];

    lines.push(String::new());
    lines.push("Current Goods On Hand:".to_string());
    let mut total_superficial_units = 0_u64;
    for (slot_idx, good_id) in superficial_good_ids.into_iter().enumerate() {
        let units = person
            .superficial_stock_units
            .get(slot_idx)
            .copied()
            .unwrap_or_default();
        total_superficial_units = total_superficial_units.saturating_add(u64::from(units));
        let good_label = good_by_id(good_id).map_or("Unknown", |good| good.name);
        lines.push(format!("{good_label}: {} unit(s)", compact_count(units)));
    }
    lines.push(format!(
        "Total Superficial Units: {}",
        compact_count(total_superficial_units as f64)
    ));
    lines.push(String::new());
    lines.extend(person_stock_holdings_lines(country, person.id));
    lines.push(String::new());
    lines.push("Press [c] to open employer company details in Goods tab.".to_string());

    lines
}

pub(in crate::components::simulation) fn country_details_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let company_count = country.companies.len();
    let public_utility_count = country
        .companies
        .iter()
        .filter(|company| company.is_public_utility)
        .count();
    let essential_backfill_count = country
        .companies
        .iter()
        .filter(|company| company.is_essential_backfill)
        .count();
    let total_open_slots: u64 = country
        .companies
        .iter()
        .flat_map(|company| company.open_positions.iter())
        .map(|opening| u64::from(opening.slots))
        .sum();
    let listed_count = country.stock_market.listed_count();

    let trade_balance =
        country.monthly_goods_export_revenue - country.monthly_goods_import_spending;
    let avg_export_tariff = if country.export_tariffs_by_good.is_empty() {
        0.0
    } else {
        country.export_tariffs_by_good.iter().sum::<f64>()
            / country.export_tariffs_by_good.len() as f64
    };
    let (max_tariff_good_id, max_export_tariff) = country
        .export_tariffs_by_good
        .iter()
        .enumerate()
        .max_by(|(_, lhs), (_, rhs)| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(good_idx, tariff)| (good_idx as u8, *tariff))
        .unwrap_or((0, 0.0));
    let max_tariff_good_label = good_by_id(max_tariff_good_id).map_or("Unknown", |good| good.name);

    let market_cap = country.stock_market.total_market_cap();
    let index_value = country.stock_market.index_value();
    let bank_equity_exposure = country.stock_market.bank_equity_exposure();
    let daily_turnover = country.stock_market.total_daily_turnover();

    vec![
        format!("Name: {}", country.name),
        format!("Country ID: {}", country.id),
        format!("Day: {}", world.day),
        format!("Month: {}", country.month),
        String::new(),
        "Structure:".to_string(),
        format!("Population: {}", compact_count(country.population() as f64)),
        format!("Companies: {}", compact_count(company_count as f64)),
        format!(
            "Public Utility / Essential Backfill: {}/{}",
            compact_count(public_utility_count as f64),
            compact_count(essential_backfill_count as f64)
        ),
        format!(
            "Open Positions (slots): {}",
            compact_count(total_open_slots as f64)
        ),
        format!("Listed Companies: {}", compact_count(listed_count as f64)),
        String::new(),
        "Money & Fiscal:".to_string(),
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
        format!("Bonds Outstanding: {}", money(country.bonds_outstanding)),
        format!("Bond Coupon Rate: {:.2}%", country.bond_coupon_rate * 100.0),
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
        "Prices & Rates:".to_string(),
        format!("CPI Index: {:.4}", country.cpi_index),
        format!(
            "Inflation Rate (ann.): {:.2}%",
            country.monthly_inflation_rate * 100.0
        ),
        format!("Policy Rate: {:.2}%", country.policy_rate * 100.0),
        format!("Bank Loan Rate: {:.2}%", country.bank_loan_rate() * 100.0),
        String::new(),
        "Trade & Tariffs:".to_string(),
        format!(
            "MTD Household Spending: {}",
            money(country.monthly_household_spending)
        ),
        format!(
            "MTD Imports / Exports: {} / {}",
            money(country.monthly_goods_import_spending),
            money(country.monthly_goods_export_revenue)
        ),
        format!("MTD Trade Balance: {}", money(trade_balance)),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected)
        ),
        format!("Average Export Tariff: {:.2}%", avg_export_tariff * 100.0),
        format!(
            "Max Export Tariff: {:.2}% ({max_tariff_good_label})",
            max_export_tariff * 100.0
        ),
        String::new(),
        "Market Risk:".to_string(),
        format!("Stock Market Cap: {}", money(market_cap)),
        format!("Stock Index: {:.2}", index_value),
        format!("Bank Equity Exposure: {}", money(bank_equity_exposure)),
        format!("Daily Turnover: {}", money(daily_turnover)),
        format!(
            "Active Traders / Buy / Sell Orders: {} / {} / {}",
            compact_count(country.stock_market.day_active_traders_count as f64),
            compact_count(country.stock_market.day_buy_orders_count as f64),
            compact_count(country.stock_market.day_sell_orders_count as f64)
        ),
        String::new(),
        "Social Pressure:".to_string(),
        format!(
            "MTD Unmet Need Events: {}",
            compact_count(country.monthly_unmet_need_events as f64)
        ),
        format!(
            "MTD Essential Debt Spending: {}",
            money(country.monthly_essential_debt_spending)
        ),
        format!(
            "MTD Hires: {}",
            compact_count(country.monthly_hires_total as f64)
        ),
        format!(
            "Last Day Births / Deaths: {} / {}",
            compact_count(country.last_day_births as f64),
            compact_count(country.last_day_deaths as f64)
        ),
        String::new(),
        format!(
            "People Page: {}/{}",
            ui.people_page + 1,
            ui.total_people_pages(world)
        ),
    ]
}

pub(in crate::components::simulation) fn company_details_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let Some(company) = ui.selected_company_on_page(world) else {
        return vec!["No company available for the current selection.".to_string()];
    };

    let good_label = good_by_id(company.produced_good_id)
        .map_or("Unknown", |good| good.name)
        .to_string();
    let total_open_slots: u64 = company
        .open_positions
        .iter()
        .map(|opening| u64::from(opening.slots))
        .sum();
    let opening_breakdown = if company.open_positions.is_empty() {
        "None".to_string()
    } else {
        company
            .open_positions
            .iter()
            .take(4)
            .map(|opening| {
                let job_label = job_by_id(opening.job_type_id).map_or("Unknown", |job| job.name);
                format!(
                    "{job_label}: {} @ {}",
                    opening.slots,
                    money(opening.offered_monthly_salary)
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let opening_overflow = company.open_positions.len().saturating_sub(4);
    let inventory_runway_months = if company.last_month_units_sold > 0 {
        company.inventory_units as f64 / company.last_month_units_sold as f64
    } else if company.inventory_units > 0 {
        f64::INFINITY
    } else {
        0.0
    };
    let domestic_export_total = company
        .last_month_domestic_units_sold
        .saturating_add(company.last_month_export_units_sold);
    let domestic_export_mix = if domestic_export_total > 0 {
        format!(
            "{:.1}% / {:.1}%",
            company.last_month_domestic_units_sold as f64 / domestic_export_total as f64 * 100.0,
            company.last_month_export_units_sold as f64 / domestic_export_total as f64 * 100.0
        )
    } else {
        "N/A".to_string()
    };
    let listed_equity = country.stock_market.listed.get(&company.id);
    let listed_label = listed_equity.map_or("No", |_| "Yes");
    let ticker_label = listed_equity.map_or("N/A".to_string(), |equity| equity.ticker.clone());
    let listing_price = listed_equity.map_or(0.0, |equity| equity.last_price);
    let listing_day_pct = listed_equity.map_or(0.0, |equity| equity.day_pct_change());
    let listing_market_cap = listed_equity.map_or(0.0, |equity| equity.market_cap);
    let listing_float_shares = listed_equity.map_or(0_u64, |equity| equity.free_float_shares);
    let bank_held_pct = listed_equity.map_or(0.0, |equity| {
        let bank_shares = country
            .stock_market
            .bank_portfolio
            .position(company.id)
            .map_or(0_u64, |position| position.shares);
        if equity.shares_outstanding > 0 {
            bank_shares as f64 / equity.shares_outstanding as f64 * 100.0
        } else {
            0.0
        }
    });

    vec![
        format!("Country: {}", country.name),
        format!("Company: {}", company.name),
        format!("Company ID: {}", company.id),
        format!("Produced Good: {}", good_label),
        String::new(),
        "Status & Market:".to_string(),
        format!(
            "Public Utility / Essential Backfill: {}/{}",
            if company.is_public_utility {
                "Yes"
            } else {
                "No"
            },
            if company.is_essential_backfill {
                "Yes"
            } else {
                "No"
            }
        ),
        format!("Listed: {listed_label}"),
        format!("Ticker: {ticker_label}"),
        format!(
            "Stock Price / Day%: {} / {:+.2}%",
            money(listing_price),
            listing_day_pct * 100.0
        ),
        format!(
            "Market Cap / Float Shares: {} / {}",
            money(listing_market_cap),
            compact_count(listing_float_shares as f64)
        ),
        format!("Bank-held Shares: {:.2}%", bank_held_pct),
        String::new(),
        "Hiring:".to_string(),
        format!("Employees: {}", company.employee_count),
        format!(
            "Manufacturing Employees: {}",
            company.manufacturing_employee_count
        ),
        format!("Open Positions: {}", company.open_positions.len()),
        format!(
            "Open Position Slots: {}",
            compact_count(total_open_slots as f64)
        ),
        format!("Openings: {opening_breakdown}"),
        format!("Openings +More: {}", compact_count(opening_overflow as f64)),
        String::new(),
        "Operations:".to_string(),
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
        format!("MTD Domestic/Export Mix: {domestic_export_mix}"),
        format!(
            "MTD Sell Through: {:.2}%",
            company.last_month_sell_through * 100.0
        ),
        format!(
            "Inventory Runway: {}",
            if inventory_runway_months.is_finite() {
                format!("{inventory_runway_months:.2} months")
            } else {
                "Infinite (no recent sales)".to_string()
            }
        ),
        format!(
            "Avg Monthly Revenue (3M): {}",
            money(company.average_monthly_revenue())
        ),
        String::new(),
        "Finance & Stress:".to_string(),
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
        format!(
            "Consecutive Missed Loan Payments: {}",
            company.consecutive_missed_loan_payments
        ),
        format!(
            "Missed Loan Payment This Month: {}",
            if company.missed_loan_payment_this_month {
                "Yes"
            } else {
                "No"
            }
        ),
    ]
}
