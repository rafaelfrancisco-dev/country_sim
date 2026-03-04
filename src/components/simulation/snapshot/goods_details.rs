use super::*;

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
