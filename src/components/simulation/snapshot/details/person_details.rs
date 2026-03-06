use super::*;
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
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

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
        format!(
            "Monthly Salary: {}",
            money(person.monthly_salary(), currency_symbol)
        ),
        format!(
            "Cash Balance: {}",
            money(person.cash_balance, currency_symbol)
        ),
        format!(
            "Loan Debt: {}",
            money(
                person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance),
                currency_symbol
            )
        ),
        String::new(),
        format!(
            "Last Month Gross Pay: {}",
            money(person.last_completed_month_gross_pay, currency_symbol)
        ),
        format!(
            "Last Month Tax Paid: {}",
            money(person.last_completed_month_tax_paid, currency_symbol)
        ),
        format!(
            "Last Month Net Pay: {}",
            money(person.last_completed_month_net_pay, currency_symbol)
        ),
        format!(
            "MTD Spending: {}",
            money(person.last_month_spending, currency_symbol)
        ),
        format!(
            "MTD Loan Payment: {}",
            money(person.last_month_loan_payment, currency_symbol)
        ),
        format!(
            "MTD New Loan: {}",
            money(person.last_month_new_loan, currency_symbol)
        ),
        format!(
            "MTD Svgs Int: {}",
            money(person.last_month_savings_interest, currency_symbol)
        ),
        String::new(),
        "Housing:".to_string(),
        format!("Status: {}", person.housing.label()),
    ];

    // Housing-specific detail lines
    use crate::sim::person::HousingStatus;
    match &person.housing {
        HousingStatus::Mortgaged { mortgage } => {
            lines.push(format!(
                "Mortgage Balance: {}",
                money(mortgage.principal_balance, currency_symbol)
            ));
            lines.push(format!(
                "Monthly Payment: {}",
                money(mortgage.monthly_payment, currency_symbol)
            ));
            lines.push(format!(
                "Rate: {:.2}% | {} months left",
                mortgage.annual_rate * 100.0,
                mortgage.months_remaining
            ));
            if mortgage.months_missed > 0 {
                lines.push(format!("Months Missed: {}", mortgage.months_missed));
            }
        }
        HousingStatus::Renter {
            landlord_company_id, ..
        } => {
            if let Some(name) = country.company_name(*landlord_company_id) {
                lines.push(format!("Landlord: {name}"));
            }
        }
        _ => {}
    }

    if person.last_month_rent_paid > 0.0 {
        lines.push(format!(
            "MTD Rent Paid: {}",
            money(person.last_month_rent_paid, currency_symbol)
        ));
    }
    if person.last_month_mortgage_payment > 0.0 {
        lines.push(format!(
            "MTD Mortgage Payment: {}",
            money(person.last_month_mortgage_payment, currency_symbol)
        ));
    }

    lines.push(String::new());
    lines.push(format!(
        "MTD Goods Spending: {}",
        money(person.last_month_goods_spending, currency_symbol)
    ));
    lines.push(format!(
        "MTD Imported Goods Spending: {}",
        money(person.last_month_imported_goods_spending, currency_symbol)
    ));
    lines.push(format!(
        "MTD Goods Units Bought: {}",
        person.last_month_goods_units_bought
    ));
    lines.push(format!(
        "Needs Met: {}",
        goods_mask_names(person.last_month_needs_met_mask)
    ));
    lines.push(format!(
        "Needs Unmet: {}",
        goods_mask_names(person.last_month_unmet_needs_mask)
    ));

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
    lines.extend(person_stock_holdings_lines(
        world,
        country,
        person.id,
        currency_symbol,
    ));
    lines.push(String::new());
    lines.push("Press [c] to open employer company details in Goods tab.".to_string());

    lines
}
