use super::*;
use crate::sim::{country::Country, world::MetricPoint};
use std::cmp::Ordering;

pub(in crate::components::simulation) fn build_fx_tab_snapshot(
    world: &World,
    ui: &UiState,
) -> RunningTabSnapshot {
    let Some(selected_country) = ui.selected_country(world) else {
        return RunningTabSnapshot::Fx(Box::new(FxTabSnapshot {
            selected_country_name: "N/A".to_string(),
            selected_currency_code: "N/A".to_string(),
            selected_currency_name: "N/A".to_string(),
            selected_currency_symbol: String::new(),
            selected_fx_summary_lines: vec!["No country selected.".to_string()],
            pair_board_lines: vec!["No FX peers available.".to_string()],
            fx_rows: Vec::new(),
            selected_fx_rate_history: Arc::default(),
            world_avg_fx_rate_history: world
                .world_history()
                .iter()
                .map(|point| point.fx_rate_local_per_base)
                .collect::<Vec<_>>()
                .into(),
            selected_fx_reserves_history: Arc::default(),
            selected_fx_net_flow_history: Arc::default(),
            selected_fx_pressure_history: Arc::default(),
        }));
    };

    let fx_rows = fx_rows(world, ui.selected_country);
    let selected_history = world
        .country_history()
        .get(ui.selected_country)
        .map(|history| history.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    let month_move = monthly_move_pct(&selected_history);

    RunningTabSnapshot::Fx(Box::new(FxTabSnapshot {
        selected_country_name: selected_country.name().to_string(),
        selected_currency_code: selected_country.currency_code().to_string(),
        selected_currency_name: selected_country.currency_name().to_string(),
        selected_currency_symbol: selected_country.currency_symbol().to_string(),
        selected_fx_summary_lines: selected_fx_summary_lines(selected_country, month_move),
        pair_board_lines: pair_board_lines(selected_country, &fx_rows),
        fx_rows,
        selected_fx_rate_history: selected_history
            .iter()
            .map(|point| point.fx_rate_local_per_base)
            .collect::<Vec<_>>()
            .into(),
        world_avg_fx_rate_history: world
            .world_history()
            .iter()
            .map(|point| point.fx_rate_local_per_base)
            .collect::<Vec<_>>()
            .into(),
        selected_fx_reserves_history: selected_history
            .iter()
            .map(|point| point.fx_reserves_base)
            .collect::<Vec<_>>()
            .into(),
        selected_fx_net_flow_history: selected_history
            .iter()
            .map(|point| point.fx_net_flow_base)
            .collect::<Vec<_>>()
            .into(),
        selected_fx_pressure_history: selected_history
            .iter()
            .map(|point| point.fx_pressure)
            .collect::<Vec<_>>()
            .into(),
    }))
}

fn fx_rows(world: &World, selected_country_idx: usize) -> Vec<FxRowSnapshot> {
    let mut rows = world
        .countries()
        .iter()
        .enumerate()
        .map(|(idx, country)| {
            let history = world
                .country_history()
                .get(idx)
                .map(|points| points.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();
            let month_move = monthly_move_pct(&history);
            let export_import_ratio = export_import_ratio_value(
                country.monthly_goods_export_revenue_base(),
                country.monthly_goods_import_spending_base(),
            );

            FxRowSnapshot {
                country: country.name().to_string(),
                currency_code: country.currency_code().to_string(),
                current_fx_rate: format!("{:.4}", country.fx_rate_local_per_base()),
                month_move_pct: month_move
                    .map(|value| format!("{value:+.2}%"))
                    .unwrap_or_else(|| "N/A".to_string()),
                reserves_base: base_money(country.fx_reserves_base()),
                net_flow_base: base_money(country.monthly_fx_net_flow_base()),
                pressure: format!("{:.2}", country.fx_pressure()),
                export_import_ratio: format_export_import_ratio(export_import_ratio),
                is_selected: idx == selected_country_idx,
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        let left_rate = parse_rate(&left.current_fx_rate);
        let right_rate = parse_rate(&right.current_fx_rate);
        right_rate
            .partial_cmp(&left_rate)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.country.cmp(&right.country))
    });
    rows
}

fn selected_fx_summary_lines(country: &Country, month_move: Option<f64>) -> Vec<String> {
    vec![
        format!(
            "Currency: {} ({}) {}",
            country.currency_name(),
            country.currency_code(),
            country.currency_symbol()
        ),
        format!("FX Rate vs Base: {:.4}", country.fx_rate_local_per_base()),
        format!(
            "Last Monthly Move: {}",
            month_move
                .map(|value| format!("{value:+.2}%"))
                .unwrap_or_else(|| "N/A".to_string())
        ),
        format!(
            "FX Reserves (base): {}",
            base_money(country.fx_reserves_base())
        ),
        format!(
            "FX Pressure: {:.2} ({})",
            country.fx_pressure(),
            pressure_label(country.fx_pressure())
        ),
        format!(
            "Goods Exports / Imports (base): {} / {}",
            base_money(country.monthly_goods_export_revenue_base()),
            base_money(country.monthly_goods_import_spending_base())
        ),
        format!(
            "Financial Inflows / Outflows (base): {} / {}",
            base_money(country.monthly_fx_financial_inflows_base()),
            base_money(country.monthly_fx_financial_outflows_base())
        ),
        format!(
            "Net FX Flow (base): {}",
            base_money(country.monthly_fx_net_flow_base())
        ),
    ]
}

fn pair_board_lines(selected_country: &Country, fx_rows: &[FxRowSnapshot]) -> Vec<String> {
    let selected_rate = selected_country.fx_rate_local_per_base().max(0.0001);
    let mut lines = Vec::new();

    for row in fx_rows
        .iter()
        .filter(|row| row.country != selected_country.name())
        .take(5)
    {
        let peer_rate = parse_rate(&row.current_fx_rate).max(0.0001);
        let implied_pair = peer_rate / selected_rate;
        let relative = if peer_rate > selected_rate {
            "stronger"
        } else if peer_rate < selected_rate {
            "weaker"
        } else {
            "flat"
        };
        lines.push(format!(
            "{} ({}) 1 {} = {:.4} {} | {} than selected",
            row.country,
            row.currency_code,
            selected_country.currency_code(),
            implied_pair,
            row.currency_code,
            relative
        ));
    }

    if lines.is_empty() {
        lines.push("No FX peers available.".to_string());
    }

    lines
}

fn monthly_move_pct(history: &[MetricPoint]) -> Option<f64> {
    if history.len() < 2 {
        return None;
    }

    let prev = history[history.len().saturating_sub(2)].fx_rate_local_per_base;
    let current = history[history.len().saturating_sub(1)].fx_rate_local_per_base;
    if prev.abs() < f64::EPSILON {
        return None;
    }

    Some(((current - prev) / prev) * 100.0)
}

fn export_import_ratio_value(exports_base: f64, imports_base: f64) -> Option<f64> {
    if imports_base <= 0.0 {
        return None;
    }
    Some(exports_base / imports_base)
}

fn format_export_import_ratio(value: Option<f64>) -> String {
    value
        .map(|ratio| format!("{ratio:.2}x"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn pressure_label(pressure: f64) -> &'static str {
    if pressure >= 0.35 {
        "Stress"
    } else if pressure >= 0.10 {
        "Warm"
    } else if pressure <= -0.35 {
        "Relief"
    } else if pressure <= -0.10 {
        "Cool"
    } else {
        "Stable"
    }
}

fn parse_rate(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(0.0)
}
