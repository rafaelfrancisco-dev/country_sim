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
            selected_reserves_breakdown: Vec::new(),
            selected_fx_rate_history: Arc::default(),
            selected_fx_reserves_by_currency: Vec::new(),
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

    let reserves_breakdown = build_reserves_breakdown(world, ui.selected_country);

    RunningTabSnapshot::Fx(Box::new(FxTabSnapshot {
        selected_country_name: selected_country.name().to_string(),
        selected_currency_code: selected_country.currency_code().to_string(),
        selected_currency_name: selected_country.currency_name().to_string(),
        selected_currency_symbol: selected_country.currency_symbol().to_string(),
        selected_fx_summary_lines: selected_fx_summary_lines(
            selected_country,
            month_move,
            &reserves_breakdown,
        ),
        pair_board_lines: pair_board_lines(selected_country, &fx_rows),
        fx_rows,
        selected_reserves_breakdown: reserves_breakdown,
        selected_fx_rate_history: selected_history
            .iter()
            .map(|point| point.fx_rate_local_per_base)
            .collect::<Vec<_>>()
            .into(),
        selected_fx_reserves_by_currency: build_reserves_currency_history(
            world,
            ui.selected_country,
            &selected_history,
        ),
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

/// Build per-currency reserves time series for the chart.
/// Each entry is (currency_code, local-currency history) sorted by source country index.
/// Historical base amounts are converted to local using the rate at each point.
fn build_reserves_currency_history(
    world: &World,
    selected_idx: usize,
    selected_history: &[MetricPoint],
) -> Vec<(String, Arc<[f64]>)> {
    let Some(src_hist) = world.country_fx_source_history.get(selected_idx) else {
        return Vec::new();
    };

    src_hist
        .iter()
        .filter_map(|(&src_idx, series)| {
            let src = world.countries().get(src_idx)?;
            // Convert base amounts to local using the historical rate at each point.
            let data: Arc<[f64]> = series
                .iter()
                .zip(
                    selected_history
                        .iter()
                        .map(|p| p.fx_rate_local_per_base)
                        .chain(std::iter::repeat(1.0)),
                )
                .map(|(&base_amt, rate)| base_amt * rate)
                .collect::<Vec<_>>()
                .into();
            // Skip series that were always zero.
            if data.iter().all(|&v| v < 0.01) {
                return None;
            }
            Some((src.currency_code().to_string(), data))
        })
        .collect()
}

/// Build the per-currency reserves breakdown for the selected country.
fn build_reserves_breakdown(world: &World, selected_idx: usize) -> Vec<FxReserveEntry> {
    let Some(selected) = world.countries().get(selected_idx) else {
        return Vec::new();
    };

    let attributed_total_base = selected
        .fx_reserves_by_source()
        .iter()
        .filter(|(source_idx, _)| **source_idx != selected_idx)
        .map(|(_, amount_base)| *amount_base)
        .sum::<f64>();
    let unattributed_local = (selected.fx_reserves_base() - attributed_total_base).max(0.0)
        * selected.fx_rate_local_per_base();

    let mut entries = vec![FxReserveEntry {
        is_unattributed: true,
        currency_symbol: selected.currency_symbol().to_string(),
        currency_code: "Unattributed".to_string(),
        amount_local: unattributed_local,
    }];

    entries.extend(
        world
            .countries()
            .iter()
            .enumerate()
            .filter(|(source_idx, _)| *source_idx != selected_idx)
            .map(|(source_idx, source)| FxReserveEntry {
                is_unattributed: false,
                currency_symbol: source.currency_symbol().to_string(),
                currency_code: source.currency_code().to_string(),
                amount_local: selected
                    .fx_reserves_by_source()
                    .get(&source_idx)
                    .copied()
                    .unwrap_or(0.0)
                    * source.fx_rate_local_per_base(),
            }),
    );
    entries
}

fn fx_rows(world: &World, selected_country_idx: usize) -> Vec<FxRowSnapshot> {
    let selected_rate = world
        .countries()
        .get(selected_country_idx)
        .map(|c| c.fx_rate_local_per_base())
        .unwrap_or(1.0)
        .max(0.0001);

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

            // Cross-rate to selected country: how many of this country's currency
            // per one unit of the selected country's currency.
            let peer_rate = country.fx_rate_local_per_base().max(0.0001);
            let cross_rate = peer_rate / selected_rate;

            // Reserves and net flow shown in the country's own local currency.
            let reserves_local = country.fx_reserves_base() * country.fx_rate_local_per_base();
            let net_flow_local =
                country.monthly_fx_net_flow_base() * country.fx_rate_local_per_base();

            FxRowSnapshot {
                country: country.name().to_string(),
                currency_code: country.currency_code().to_string(),
                current_fx_rate: if idx == selected_country_idx {
                    "—".to_string()
                } else {
                    format!("{cross_rate:.4}")
                },
                fx_rate_raw: country.fx_rate_local_per_base(),
                month_move_pct: month_move
                    .map(|value| format!("{value:+.2}%"))
                    .unwrap_or_else(|| "N/A".to_string()),
                reserves: money_sym(reserves_local, country.currency_symbol()),
                net_flow: money_sym(net_flow_local, country.currency_symbol()),
                pressure: format!("{:.2}", country.fx_pressure()),
                export_import_ratio: format_export_import_ratio(export_import_ratio),
                is_selected: idx == selected_country_idx,
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        right
            .fx_rate_raw
            .partial_cmp(&left.fx_rate_raw)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.country.cmp(&right.country))
    });
    rows
}

fn selected_fx_summary_lines(
    country: &Country,
    month_move: Option<f64>,
    _breakdown: &[FxReserveEntry],
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "Currency: {} ({}) {}",
            country.currency_name(),
            country.currency_code(),
            country.currency_symbol()
        ),
        format!(
            "Last Monthly Move: {}",
            month_move
                .map(|value| format!("{value:+.2}%"))
                .unwrap_or_else(|| "N/A".to_string())
        ),
        format!(
            "FX Pressure: {:.2} ({})",
            country.fx_pressure(),
            pressure_label(country.fx_pressure())
        ),
        format!(
            "Goods Exports / Imports: {} / {}",
            money_sym(
                country.monthly_goods_export_revenue_base() * country.fx_rate_local_per_base(),
                country.currency_symbol()
            ),
            money_sym(
                country.monthly_goods_import_spending_base() * country.fx_rate_local_per_base(),
                country.currency_symbol()
            ),
        ),
        format!(
            "Financial In / Out: {} / {}",
            money_sym(
                country.monthly_fx_financial_inflows_base() * country.fx_rate_local_per_base(),
                country.currency_symbol()
            ),
            money_sym(
                country.monthly_fx_financial_outflows_base() * country.fx_rate_local_per_base(),
                country.currency_symbol()
            ),
        ),
        format!(
            "Net FX Flow: {}",
            money_sym(
                country.monthly_fx_net_flow_base() * country.fx_rate_local_per_base(),
                country.currency_symbol()
            )
        ),
    ];

    lines.push("FX Reserves:".to_string());

    lines
}

fn pair_board_lines(selected_country: &Country, fx_rows: &[FxRowSnapshot]) -> Vec<String> {
    let mut lines = Vec::new();

    for row in fx_rows
        .iter()
        .filter(|row| row.country != selected_country.name())
        .take(5)
    {
        let peer_rate = parse_rate(&row.current_fx_rate).max(0.0001);
        // current_fx_rate is already the cross-rate (peer / selected), so:
        // 1 {selected_ccy} = peer_rate {peer_ccy}
        let relative = if peer_rate > 1.0 {
            "weaker"
        } else if peer_rate < 1.0 {
            "stronger"
        } else {
            "flat"
        };
        lines.push(format!(
            "{} ({}) 1 {} = {:.4} {} | {} vs selected",
            row.country,
            row.currency_code,
            selected_country.currency_code(),
            peer_rate,
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

fn money_sym(amount: f64, symbol: &str) -> String {
    super::money(amount, symbol)
}

#[cfg(test)]
mod tests;
