use super::*;
use std::cmp::Ordering;

type CompanyHistorySeries = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

pub(in crate::components::simulation) fn selected_company_histories(
    world: &World,
    ui: &UiState,
) -> CompanyHistorySeries {
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

pub(in crate::components::simulation) fn country_goods_summary_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

    vec![
        format!("Country: {} (Month {})", country.name, country.month),
        format!(
            "MTD Household Goods Spending: {}",
            money(country.monthly_household_spending, currency_symbol)
        ),
        format!(
            "MTD Imported Goods Spending: {}",
            money(country.monthly_goods_import_spending, currency_symbol)
        ),
        format!(
            "MTD Goods Export Revenue: {}",
            money(country.monthly_goods_export_revenue, currency_symbol)
        ),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected, currency_symbol)
        ),
        format!(
            "MTD Essential Debt Spending: {}",
            money(country.monthly_essential_debt_spending, currency_symbol)
        ),
        format!(
            "MTD Unmet Need Events: {}",
            compact_count(country.monthly_unmet_need_events as f64)
        ),
    ]
}

pub(in crate::components::simulation) fn selected_company_worker_rows(
    world: &World,
    ui: &UiState,
    currency_symbol: &str,
) -> Vec<CompanyWorkerRowSnapshot> {
    let Some(country) = ui.selected_country(world) else {
        return Vec::new();
    };
    let Some(company) = ui.selected_company_on_page(world) else {
        return Vec::new();
    };

    let mut rows = country
        .people
        .iter()
        .filter_map(|person| {
            if person.employer_company_id() != Some(company.id) {
                return None;
            }

            let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
            let name = person_name_from_seed(name_seed);
            let job = person
                .job_type_id()
                .and_then(job_by_id)
                .map_or("Unknown".to_string(), |it| it.name.to_string());
            let tenure_months = person
                .employment_start_day
                .map_or(0, |start_day| world.day.saturating_sub(start_day) / 30);
            Some((name, job, tenure_months, person.monthly_salary()))
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then_with(|| b.3.partial_cmp(&a.3).unwrap_or(Ordering::Equal))
            .then_with(|| a.0.cmp(&b.0))
    });

    rows.into_iter()
        .map(
            |(name, job, tenure_months, salary)| CompanyWorkerRowSnapshot {
                name,
                job,
                tenure_months: tenure_months.to_string(),
                salary: money(salary, currency_symbol),
            },
        )
        .collect()
}
