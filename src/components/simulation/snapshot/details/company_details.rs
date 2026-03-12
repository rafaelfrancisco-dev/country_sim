use super::*;
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
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

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
                    money(opening.offered_monthly_salary, currency_symbol)
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let opening_overflow = company.open_positions.len().saturating_sub(4);
    let mut active_employee_count = 0_u64;
    let mut active_monthly_payroll = 0.0_f64;
    for person in &country.people {
        if person.employer_company_id() == Some(company.id) {
            active_employee_count = active_employee_count.saturating_add(1);
            active_monthly_payroll += person.monthly_salary();
        }
    }
    let average_monthly_pay = if active_employee_count > 0 {
        active_monthly_payroll / active_employee_count as f64
    } else {
        0.0
    };
    let output_target_units = company_target_daily_output(country, company);
    let production_readiness = production_readiness_label(country, company, output_target_units);
    let recipe_description = recipe_description(company);
    let input_coverage_description = input_coverage_description(company, output_target_units);
    let inventory_runway_months = if company.last_completed_month_units_sold > 0 {
        company.inventory_units as f64 / company.last_completed_month_units_sold as f64
    } else if company.inventory_units > 0 {
        f64::INFINITY
    } else {
        0.0
    };
    let domestic_export_total = company
        .last_completed_month_domestic_units_sold
        .saturating_add(company.last_completed_month_export_units_sold);
    let domestic_export_mix = if domestic_export_total > 0 {
        format!(
            "{:.1}% / {:.1}%",
            company.last_completed_month_domestic_units_sold as f64 / domestic_export_total as f64
                * 100.0,
            company.last_completed_month_export_units_sold as f64 / domestic_export_total as f64
                * 100.0
        )
    } else {
        "N/A".to_string()
    };
    let listed_equity = country.stock_market.equity_by_company_id(company.id);
    let listed_label = listed_equity.map_or("No", |_| "Yes");
    let ticker_label = listed_equity.map_or("N/A".to_string(), |equity| equity.ticker.to_string());
    let listing_price = listed_equity.map_or(0.0, |equity| equity.last_price);
    let listing_day_pct = listed_equity.map_or(0.0, |equity| equity.closed_day_pct_change());
    let listing_market_cap = listed_equity.map_or(0.0, |equity| equity.market_cap);
    let listing_float_shares = listed_equity.map_or(0_u64, |equity| equity.free_float_shares);
    let bank_held_pct = listed_equity.map_or(0.0, |equity| {
        let bank_shares = country
            .stock_market
            .bank_portfolio
            .position(crate::sim::stock_market::StockSecurityId {
                country_id: country.id,
                company_id: equity.company_id,
            })
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
            money(listing_price, currency_symbol),
            listing_day_pct * 100.0
        ),
        format!(
            "Market Cap / Float Shares: {} / {}",
            money(listing_market_cap, currency_symbol),
            compact_count(listing_float_shares as f64)
        ),
        format!("Bank-held Shares: {:.2}%", bank_held_pct),
        String::new(),
        "Hiring:".to_string(),
        format!("Employees: {}", company.employee_count),
        format!(
            "Active Employees (People Registry): {}",
            compact_count(active_employee_count as f64)
        ),
        format!(
            "Production Employees: {}",
            company.manufacturing_employee_count
        ),
        format!(
            "Current Monthly Payroll (Employees): {}",
            money(active_monthly_payroll, currency_symbol)
        ),
        format!(
            "Avg Monthly Pay / Employee: {}",
            money(average_monthly_pay, currency_symbol)
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
        format!("Unit Price: {}", money(company.unit_price, currency_symbol)),
        format!(
            "Output Inventory Units: {}",
            compact_count(company.inventory_units)
        ),
        format!(
            "Daily Target Output: {}",
            compact_count(output_target_units as f64)
        ),
        format!("Production Readiness: {production_readiness}"),
        format!("Input Recipe: {recipe_description}"),
        format!("Input Coverage: {input_coverage_description}"),
        format!(
            "Facility Housing Units / Target: {}/{}",
            compact_count(company.facility_housing_units as f64),
            compact_count(company.target_facility_housing_units() as f64)
        ),
        format!(
            "Last Month Produced Units: {}",
            compact_count(company.last_completed_month_units_produced)
        ),
        format!(
            "Last Month Sold Units: {}",
            compact_count(company.last_completed_month_units_sold)
        ),
        format!(
            "Last Month Domestic/Export Units: {}/{}",
            compact_count(company.last_completed_month_domestic_units_sold),
            compact_count(company.last_completed_month_export_units_sold)
        ),
        format!("Last Month Domestic/Export Mix: {domestic_export_mix}"),
        format!(
            "Last Month Sell Through: {:.2}%",
            company.last_completed_month_sell_through * 100.0
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
            money(company.average_monthly_revenue(), currency_symbol)
        ),
        String::new(),
        "Finance & Stress:".to_string(),
        format!(
            "Last Month Sales: {}",
            money(company.last_completed_month_sales, currency_symbol)
        ),
        format!(
            "Last Month Payroll Paid: {}",
            money(company.last_completed_month_payroll_paid, currency_symbol)
        ),
        format!(
            "Last Month Input Spend: {}",
            money(company.last_completed_month_input_spending, currency_symbol)
        ),
        format!(
            "Last Month Facility Spend: {}",
            money(
                company.last_completed_month_facility_spending,
                currency_symbol
            )
        ),
        format!(
            "Last Month Profit: {}",
            money(company.last_completed_month_profit, currency_symbol)
        ),
        format!(
            "Last Month Loan Payment: {}",
            money(company.last_completed_month_loan_payment, currency_symbol)
        ),
        format!(
            "Last Month New Loan: {}",
            money(company.last_completed_month_new_loan, currency_symbol)
        ),
        format!(
            "Last Month Svgs Int: {}",
            money(
                company.last_completed_month_savings_interest,
                currency_symbol
            )
        ),
        format!(
            "Current Money: {}",
            money(company.cash_balance, currency_symbol)
        ),
        format!(
            "Loan Debt: {}",
            money(
                company.loan.as_ref().map_or(0.0, |loan| loan.debt_balance),
                currency_symbol
            )
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

fn company_target_daily_output(
    country: &crate::sim::country::Country,
    company: &crate::sim::company::Company,
) -> u32 {
    let Some(good) = good_by_id(company.produced_good_id) else {
        return 0;
    };
    if good.effort_per_unit <= 0.0
        || company.manufacturing_employee_count == 0
        || company_output_inventory_throttled(country, company)
    {
        return 0;
    }

    let daily_effort_per_worker = country.sim_params().goods_market.worker_effort_per_month / 30.0;
    ((company.production_effort_carry
        + company.manufacturing_employee_count as f64 * daily_effort_per_worker)
        / good.effort_per_unit)
        .floor()
        .max(0.0) as u32
}

fn production_readiness_label(
    country: &crate::sim::country::Country,
    company: &crate::sim::company::Company,
    output_target_units: u32,
) -> String {
    if company.manufacturing_employee_count == 0 {
        return "No production labor".to_string();
    }
    if company_output_inventory_throttled(country, company) {
        return "Throttled by excess output inventory".to_string();
    }
    if output_target_units == 0 {
        return "Insufficient labor effort carry for next unit".to_string();
    }
    if company.recipe_for_output_good().inputs().count() == 0 {
        return "Ready (no material inputs required)".to_string();
    }
    if company.can_produce_one_unit() {
        "Ready to produce".to_string()
    } else {
        "Waiting on material inputs".to_string()
    }
}

fn recipe_description(company: &crate::sim::company::Company) -> String {
    let inputs = company
        .recipe_for_output_good()
        .inputs()
        .map(|(good_id, required)| {
            let label = good_by_id(good_id).map_or("Unknown", |good| good.name);
            format!("{label} x{required}")
        })
        .collect::<Vec<_>>();
    if inputs.is_empty() {
        "No material inputs".to_string()
    } else {
        inputs.join(", ")
    }
}

fn input_coverage_description(
    company: &crate::sim::company::Company,
    output_target_units: u32,
) -> String {
    let inputs = company
        .recipe_for_output_good()
        .inputs()
        .map(|(good_id, required_per_unit)| {
            let required_units = output_target_units.saturating_mul(u32::from(required_per_unit));
            let on_hand = company.input_inventory_units(good_id);
            let label = good_by_id(good_id).map_or("Unknown", |good| good.name);
            if required_units == 0 {
                format!("{label}: {on_hand} on hand")
            } else {
                format!("{label}: {on_hand}/{required_units}")
            }
        })
        .collect::<Vec<_>>();
    if inputs.is_empty() {
        "N/A".to_string()
    } else {
        inputs.join(" | ")
    }
}

fn company_output_inventory_throttled(
    country: &crate::sim::country::Country,
    company: &crate::sim::company::Company,
) -> bool {
    let Some(&last_sold) = company.sold_units_history.back() else {
        return false;
    };
    if last_sold == 0 {
        return false;
    }

    let months_of_inventory = company.inventory_units as f64 / last_sold as f64;
    months_of_inventory > country.sim_params().goods_market.excess_inventory_months
}
