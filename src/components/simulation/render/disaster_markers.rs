use super::super::theme;
use crate::sim::country::DAYS_PER_MONTH;
use ratatui::{
    style::{Color, Style},
    symbols,
    widgets::{Dataset, GraphType},
};

const MARKER_COLOR: Color = Color::LightRed;
const DEDUP_EPSILON: f64 = 1.0e-6;

pub(super) fn disaster_month_positions(disaster_days: &[u64]) -> Vec<f64> {
    disaster_days
        .iter()
        .map(|day| *day as f64 / DAYS_PER_MONTH as f64)
        .collect()
}

pub(super) fn disaster_relative_month_positions(
    disaster_days: &[u64],
    start_month: u64,
) -> Vec<f64> {
    disaster_days
        .iter()
        .map(|day| (*day as f64 / DAYS_PER_MONTH as f64) - start_month as f64 + 1.0)
        .collect()
}

pub(super) fn disaster_relative_day_positions(disaster_days: &[u64], start_day: u64) -> Vec<f64> {
    disaster_days
        .iter()
        .map(|day| day.saturating_sub(start_day) as f64)
        .collect()
}

pub(super) fn marker_segments(
    marker_positions: &[f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> Vec<Vec<(f64, f64)>> {
    if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
        return Vec::new();
    }
    let mut visible = marker_positions
        .iter()
        .copied()
        .filter(|x| x.is_finite() && *x >= x_min && *x <= x_max)
        .collect::<Vec<_>>();
    visible.sort_by(f64::total_cmp);
    visible.dedup_by(|a, b| (*a - *b).abs() <= DEDUP_EPSILON);
    visible
        .into_iter()
        .map(|x| vec![(x, y_min), (x, y_max)])
        .collect()
}

pub(super) fn marker_datasets<'a>(segments: &'a [Vec<(f64, f64)>]) -> Vec<Dataset<'a>> {
    segments
        .iter()
        .map(|segment| {
            Dataset::default()
                .graph_type(GraphType::Line)
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(MARKER_COLOR).bg(theme::PANEL_BG))
                .data(segment.as_slice())
        })
        .collect()
}
