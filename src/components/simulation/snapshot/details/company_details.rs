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
    let listing_day_pct = listed_equity.map_or(0.0, |equity| equity.closed_day_pct_change());
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
