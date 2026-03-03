use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::{hotkeys, snapshot::RunningSnapshot};
use crate::sim::world::MetricPoint;

impl Simulation {
    pub(super) fn draw_charts_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
    ) -> String {
        let scope_label = world.chart_scope_label.as_str();
        let history: &[MetricPoint] = world.chart_history.as_slice();

        if history.is_empty() {
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
            let [row1, row2, row3, row4] = Layout::vertical([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .areas(body);
            let [population_area, supply_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row1);
            let [created_money_area, reserve_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row2);
            let [treasury_area, budget_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row3);
            let [cpi_area, policy_rate_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row4);

            self.draw_metric_chart(
                frame,
                population_area,
                history,
                &format!("Population ({scope_label})"),
                Color::Cyan,
                |point| point.population,
            );
            self.draw_metric_chart(
                frame,
                supply_area,
                history,
                &format!("Total Money Supply ({scope_label})"),
                Color::Green,
                |point| point.total_money_supply,
            );
            self.draw_metric_chart(
                frame,
                created_money_area,
                history,
                &format!("Created Money Outstanding ({scope_label})"),
                Color::Yellow,
                |point| point.created_money_outstanding,
            );
            self.draw_metric_chart(
                frame,
                reserve_area,
                history,
                &format!("Bank Reserve ({scope_label})"),
                Color::Magenta,
                |point| point.bank_reserve,
            );
            self.draw_metric_chart(
                frame,
                treasury_area,
                history,
                &format!("Treasury Balance ({scope_label})"),
                Color::LightBlue,
                |point| point.treasury_balance,
            );
            self.draw_metric_chart(
                frame,
                budget_area,
                history,
                &format!("Budget Balance ({scope_label})"),
                Color::LightRed,
                |point| point.budget_balance,
            );
            self.draw_metric_chart(
                frame,
                cpi_area,
                history,
                &format!("CPI Index ({scope_label})"),
                Color::LightYellow,
                |point| point.cpi_index,
            );
            self.draw_metric_chart(
                frame,
                policy_rate_area,
                history,
                &format!("Policy Rate % ({scope_label})"),
                Color::LightCyan,
                |point| point.policy_rate * 100.0,
            );
        }

        format!(
            "Controls: {}  [t] toggle scope  [,/.] cycle country",
            hotkeys::GLOBAL_RUNNING_HINT
        )
    }

    fn draw_metric_chart<F>(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &[MetricPoint],
        title: &str,
        color: Color,
        value_fn: F,
    ) where
        F: Fn(&MetricPoint) -> f64,
    {
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

        let chart = Chart::new(vec![dataset])
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
                    .labels(vec![Self::money(y_min), Self::money(y_max)]),
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
