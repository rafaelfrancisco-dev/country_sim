use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use super::disaster_markers;
use crate::components::simulation::{
    hotkeys,
    snapshot::{ChartsTabSnapshot, RunningSnapshot},
};
use crate::sim::world::MetricPoint;
#[path = "charts_metrics.rs"]
mod metrics;
use metrics::AxisValueFormat;

impl Simulation {
    pub(super) fn draw_charts_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
        tab: &ChartsTabSnapshot,
    ) -> String {
        let scope_label = tab.chart_scope_label.as_str();
        let history: &[MetricPoint] = tab.chart_history.as_ref();
        let total_pages = tab.total_chart_pages.max(1);
        let page_idx = tab.chart_page.min(total_pages.saturating_sub(1));
        let currency_symbol = world.currency_symbol.as_str();
        let disaster_days = tab.disaster_days.as_ref();

        if page_idx >= 5 {
            self.draw_pie_charts_page(frame, body, &tab.pie_page, page_idx - 5);
        } else if history.is_empty() {
            let placeholder = Paragraph::new(
                "No history data yet. Advance the simulation by month to collect snapshots.",
            )
            .style(theme::primary_text().bg(theme::PANEL_BG))
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Charts ")
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text())
                    .style(Style::default().bg(theme::PANEL_BG)),
            );
            frame.render_widget(placeholder, body);
        } else {
            let chart_areas = Self::chart_grid_areas(body);
            let chart_specs = Self::chart_specs_for_page(page_idx);

            for (area, spec) in chart_areas.into_iter().zip(chart_specs.into_iter()) {
                self.draw_metric_chart(
                    frame,
                    area,
                    history,
                    &format!("{} ({scope_label})", spec.title),
                    spec.color,
                    spec.value_fn,
                    spec.axis_format,
                    currency_symbol,
                    disaster_days,
                );
            }
        }

        format!(
            "Controls: {}  [t] toggle scope  [,/.] cycle country  [PgUp/PgDn] chart page | {}",
            hotkeys::GLOBAL_RUNNING_HINT,
            Self::chart_page_label(page_idx, total_pages)
        )
    }

    fn chart_page_label(page_idx: usize, total_pages: usize) -> String {
        match page_idx {
            4 => format!(
                "Chart Page {}/{} (Housing Market)",
                page_idx + 1,
                total_pages
            ),
            5 => format!(
                "Chart Page {}/{} (Pie 1/2: World)",
                page_idx + 1,
                total_pages
            ),
            6 => format!(
                "Chart Page {}/{} (Pie 2/2: Selected Country)",
                page_idx + 1,
                total_pages
            ),
            _ => format!("Chart Page {}/{}", page_idx + 1, total_pages),
        }
    }

    pub(super) fn chart_grid_areas(area: Rect) -> [Rect; 8] {
        let [row1, row2, row3, row4] = Layout::vertical([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .areas(area);
        let [a1, a2] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(row1);
        let [a3, a4] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(row2);
        let [a5, a6] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(row3);
        let [a7, a8] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(row4);
        [a1, a2, a3, a4, a5, a6, a7, a8]
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "chart rendering requires explicit visual + accessor parameters"
    )]
    fn draw_metric_chart(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &[MetricPoint],
        title: &str,
        color: Color,
        value_fn: fn(&MetricPoint) -> f64,
        axis_format: AxisValueFormat,
        currency_symbol: &str,
        disaster_days: &[u64],
    ) {
        if history.is_empty() {
            frame.render_widget(
                Paragraph::new("No data.")
                    .style(theme::muted_text().bg(theme::PANEL_BG))
                    .block(
                        Block::default()
                            .title(format!(" {title} "))
                            .borders(Borders::ALL)
                            .border_style(theme::neutral_border())
                            .title_style(theme::primary_text())
                            .style(Style::default().bg(theme::PANEL_BG)),
                    ),
                area,
            );
            return;
        }

        let points = history
            .iter()
            .map(|point| (point.month as f64, value_fn(point)))
            .collect::<Vec<_>>();

        let first_month = history.first().map_or(0, |point| point.month);
        let last_month = history.last().map_or(first_month, |point| point.month);
        let x_min = first_month as f64;
        let mut x_max = last_month as f64;
        if (x_max - x_min).abs() < f64::EPSILON {
            x_max = x_min + 1.0;
        }

        let (value_min, value_max) = points
            .iter()
            .map(|(_, value)| *value)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        let (y_min, y_max) = Self::expanded_axis_bounds(value_min, value_max);

        let dataset = Dataset::default()
            .graph_type(GraphType::Line)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(color).bg(theme::PANEL_BG))
            .data(&points);
        let marker_positions = disaster_markers::disaster_month_positions(disaster_days);
        let marker_segments =
            disaster_markers::marker_segments(&marker_positions, x_min, x_max, y_min, y_max);
        let mut datasets = Vec::with_capacity(1 + marker_segments.len());
        datasets.push(dataset);
        datasets.extend(disaster_markers::marker_datasets(&marker_segments));

        let chart = Chart::new(datasets)
            .style(Style::default().bg(theme::PANEL_BG))
            .x_axis(
                Axis::default()
                    .title("Month")
                    .style(theme::primary_text().bg(theme::PANEL_BG))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        first_month.to_string(),
                        last_month.max(first_month + 1).to_string(),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(theme::primary_text().bg(theme::PANEL_BG))
                    .bounds([y_min, y_max])
                    .labels(vec![
                        Self::format_axis_label(axis_format, y_min, currency_symbol),
                        Self::format_axis_label(axis_format, y_max, currency_symbol),
                    ]),
            )
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text())
                    .style(Style::default().bg(theme::PANEL_BG)),
            );

        frame.render_widget(chart, area);
    }

    fn format_axis_label(
        axis_format: AxisValueFormat,
        value: f64,
        currency_symbol: &str,
    ) -> String {
        match axis_format {
            AxisValueFormat::Money => Self::money(value, currency_symbol),
            AxisValueFormat::Percent => format!("{value:.2}%"),
            AxisValueFormat::Index => format!("{value:.3}"),
            AxisValueFormat::Count => {
                if value.abs() >= 10_000.0 {
                    Self::compact_num(value)
                } else {
                    format!("{value:.0}")
                }
            }
        }
    }

    fn expanded_axis_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }
        if (max - min).abs() < f64::EPSILON {
            let padding = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.10
            };
            let lower = (min - padding).min(min);
            let upper = (max + padding).max(max);
            return (lower, upper);
        }
        let span = max - min;
        let padding = span * 0.10;
        let lower = if min >= 0.0 {
            (min - padding).max(0.0)
        } else {
            min - padding
        };
        let upper = max + padding;
        (lower, upper)
    }
}
