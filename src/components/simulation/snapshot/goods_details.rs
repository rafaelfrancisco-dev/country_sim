use super::*;
use crate::sim::country::DAYS_PER_MONTH;
use std::cmp::Ordering;

type CompanyHistorySeries = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

#[derive(Debug, Clone, Copy, Default)]
struct GoodAggregate {
    producer_count: usize,
    total_price: f64,
    inventory_units: u32,
    produced_units: u32,
    sold_units: u32,
    export_units: u32,
}

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
        company.price_history.iter().copied().collect(),
        company.loan_payment_history.iter().copied().collect(),
        company.cash_balance_history.iter().copied().collect(),
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
        format!(
            "MTD Severe Unmet Events: {}",
            compact_count(country.monthly_severe_unmet_need_events as f64)
        ),
        format!(
            "MTD Chronic Unmet People: {}",
            compact_count(country.monthly_chronic_unmet_need_count as f64)
        ),
    ]
}

pub(in crate::components::simulation) fn production_demand_summary_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };

    let day_in_month = world.day % DAYS_PER_MONTH + 1;
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);
    let aggregates = goods_production_demand_aggregates(country);
    let total_modeled_daily_demand = common_goods()
        .iter()
        .map(|good| country.population() as f64 * good.monthly_demand_factor.max(0.0) / 30.0)
        .sum::<f64>();
    let total_modeled_mtd_demand = total_modeled_daily_demand * day_in_month as f64;
    let zero_producer_goods = aggregates
        .iter()
        .filter(|row| row.producer_count == 0)
        .count();
    let total_inventory_units = aggregates
        .iter()
        .map(|row| u64::from(row.inventory_units))
        .sum::<u64>();
    let total_produced_units = aggregates
        .iter()
        .map(|row| u64::from(row.produced_units))
        .sum::<u64>();
    let total_sold_units = aggregates
        .iter()
        .map(|row| u64::from(row.sold_units))
        .sum::<u64>();

    vec![
        format!(
            "Country: {} (Month {}, Day {day_in_month})",
            country.name, country.month
        ),
        format!("Population: {}", compact_count(country.population() as f64)),
        format!(
            "Modeled Daily Demand: {} units",
            compact_count(total_modeled_daily_demand)
        ),
        format!(
            "Modeled MTD Demand: {} units",
            compact_count(total_modeled_mtd_demand)
        ),
        format!(
            "MTD Produced / Sold: {} / {} units",
            compact_count(total_produced_units as f64),
            compact_count(total_sold_units as f64)
        ),
        format!(
            "Inventory Units / Zero-Producer Goods: {} / {}",
            compact_count(total_inventory_units as f64),
            zero_producer_goods
        ),
        format!(
            "Avg Local Household Spend: {}",
            money(country.monthly_household_spending, currency_symbol)
        ),
    ]
}

pub(in crate::components::simulation) fn production_demand_rows(
    world: &World,
    ui: &UiState,
    currency_symbol: &str,
) -> Vec<GoodsProductionDemandRowSnapshot> {
    let Some(country) = ui.selected_country(world) else {
        return Vec::new();
    };

    let day_in_month = world.day % DAYS_PER_MONTH + 1;
    let aggregates = goods_production_demand_aggregates(country);
    common_goods()
        .iter()
        .enumerate()
        .map(|(idx, good)| {
            let aggregate = aggregates.get(idx).copied().unwrap_or_default();
            let avg_price = if aggregate.producer_count == 0 {
                "N/A".to_string()
            } else {
                money(
                    aggregate.total_price / aggregate.producer_count as f64,
                    currency_symbol,
                )
            };
            let est_daily_demand =
                country.population() as f64 * good.monthly_demand_factor.max(0.0) / 30.0;
            let est_mtd_demand = est_daily_demand * day_in_month as f64;

            GoodsProductionDemandRowSnapshot {
                good: good.name.to_string(),
                producer_count: aggregate.producer_count.to_string(),
                avg_price,
                inventory_units: compact_count(aggregate.inventory_units),
                produced_units: compact_count(aggregate.produced_units),
                sold_units: compact_count(aggregate.sold_units),
                export_units: compact_count(aggregate.export_units),
                est_daily_demand_units: compact_count(est_daily_demand),
                est_mtd_demand_units: compact_count(est_mtd_demand),
            }
        })
        .collect()
}

fn goods_production_demand_aggregates(
    country: &crate::sim::country::Country,
) -> Vec<GoodAggregate> {
    let mut aggregates = vec![GoodAggregate::default(); common_goods().len()];
    for company in &country.companies {
        let Some(row) = aggregates.get_mut(company.produced_good_id as usize) else {
            continue;
        };
        row.producer_count += 1;
        row.total_price += company.unit_price;
        row.inventory_units = row.inventory_units.saturating_add(company.inventory_units);
        row.produced_units = row
            .produced_units
            .saturating_add(company.last_month_units_produced);
        row.sold_units = row.sold_units.saturating_add(company.last_month_units_sold);
        row.export_units = row
            .export_units
            .saturating_add(company.last_month_export_units_sold);
    }
    aggregates
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
