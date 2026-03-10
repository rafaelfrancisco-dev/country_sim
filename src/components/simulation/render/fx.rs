use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Line,
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
};

use super::super::{Simulation, theme};
use crate::components::simulation::{
    hotkeys,
    snapshot::{FxRowSnapshot, FxTabSnapshot, RunningSnapshot},
};

struct FxChart<'a> {
    title: &'a str,
    data: &'a [f64],
    color: Color,
    format: FxAxisFormat,
}

#[derive(Copy, Clone)]
enum FxAxisFormat {
    Rate,
    Money,
    Pressure,
}

impl Simulation {
    pub(in crate::components::simulation) fn draw_fx_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        _world: &RunningSnapshot,
        tab: &FxTabSnapshot,
    ) -> String {
        let [top, middle, bottom] = Layout::vertical([
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(12),
        ])
        .areas(body);

        let [summary_area, pairs_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(top);
        let [c1, c2, c3, c4] = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .areas(bottom);

        self.draw_fx_summary(frame, summary_area, tab);
        self.draw_fx_pairs(frame, pairs_area, tab);
        self.draw_fx_table(frame, middle, tab);
        self.draw_fx_chart(
            frame,
            c1,
            FxChart {
                title: " FX Rate (index) ",
                data: tab.selected_fx_rate_history.as_ref(),
                color: Color::Cyan,
                format: FxAxisFormat::Rate,
            },
        );
        self.draw_fx_chart(
            frame,
            c2,
            FxChart {
                title: " FX Net Flow ",
                data: tab.selected_fx_net_flow_history.as_ref(),
                color: Color::Green,
                format: FxAxisFormat::Money,
            },
        );
        self.draw_fx_reserves_chart(frame, c3, tab);
        self.draw_fx_chart(
            frame,
            c4,
            FxChart {
                title: " FX Pressure ",
                data: tab.selected_fx_pressure_history.as_ref(),
                color: Color::Magenta,
                format: FxAxisFormat::Pressure,
            },
        );

        format!(
            "Controls: {}  [,/.] change country | FX Monitor: {} ({}) | Countries: {}",
            hotkeys::GLOBAL_RUNNING_HINT,
            tab.selected_country_name,
            tab.selected_currency_code,
            tab.fx_rows.len()
        )
    }

    fn draw_fx_summary(&self, frame: &mut Frame, area: Rect, tab: &FxTabSnapshot) {
        let mut lines: Vec<Line<'_>> = tab
            .selected_fx_summary_lines
            .iter()
            .map(|line| Line::raw(line.as_str()))
            .collect();

        // Append per-currency reserves breakdown as styled entries.
        if !tab.selected_reserves_breakdown.is_empty() {
            let colors = [
                Color::Cyan,
                Color::Green,
                Color::Yellow,
                Color::Magenta,
                Color::LightBlue,
                Color::LightGreen,
            ];
            for (i, entry) in tab.selected_reserves_breakdown.iter().enumerate() {
                let color = colors[i % colors.len()];
                lines.push(Line::styled(
                    format!(
                        "  {} {}",
                        crate::components::simulation::snapshot::compact_num(entry.amount_local),
                        entry.currency_symbol,
                    ),
                    Style::default().fg(color),
                ));
            }
        }

        let block = Block::default()
            .title(" Selected FX Summary ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .title_style(theme::primary_text());
        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
    }

    fn draw_fx_pairs(&self, frame: &mut Frame, area: Rect, tab: &FxTabSnapshot) {
        let lines = tab
            .pair_board_lines
            .iter()
            .map(|line| Line::raw(line.as_str()))
            .collect::<Vec<_>>();
        let block = Block::default()
            .title(" Pair Board ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .title_style(theme::primary_text());
        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
    }

    fn draw_fx_reserves_chart(&self, frame: &mut Frame, area: Rect, tab: &FxTabSnapshot) {
        const COLORS: [Color; 6] = [
            Color::Yellow,
            Color::Cyan,
            Color::Green,
            Color::Magenta,
            Color::LightBlue,
            Color::LightRed,
        ];

        let series = &tab.selected_fx_reserves_by_currency;

        if series.is_empty() {
            frame.render_widget(
                Paragraph::new("No cross-border flows yet.")
                    .style(theme::muted_text())
                    .block(
                        Block::default()
                            .title(" FX Reserves by Currency ")
                            .borders(Borders::ALL)
                            .border_style(theme::neutral_border())
                            .title_style(theme::primary_text()),
                    ),
                area,
            );
            return;
        }

        // Build point vecs for each series (must outlive the Dataset borrows).
        let all_points: Vec<Vec<(f64, f64)>> = series
            .iter()
            .map(|(_, data)| {
                data.iter()
                    .enumerate()
                    .map(|(i, &v)| (i as f64, v))
                    .collect()
            })
            .collect();

        let x_max = all_points
            .iter()
            .map(|pts| pts.len())
            .max()
            .unwrap_or(1)
            .saturating_sub(1)
            .max(1) as f64;

        let (min, max) = all_points
            .iter()
            .flat_map(|pts| pts.iter().map(|(_, v)| *v))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), v| {
                (mn.min(v), mx.max(v))
            });
        let (y_min, y_max) = Self::fx_axis_bounds(min, max);

        let datasets: Vec<Dataset<'_>> = all_points
            .iter()
            .zip(series.iter())
            .enumerate()
            .map(|(i, (pts, (label, _)))| {
                Dataset::default()
                    .name(label.as_str())
                    .graph_type(GraphType::Line)
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(COLORS[i % COLORS.len()]))
                    .data(pts)
            })
            .collect();

        frame.render_widget(
            Chart::new(datasets)
                .x_axis(
                    Axis::default()
                        .title("Month")
                        .style(theme::primary_text())
                        .bounds([0.0, x_max])
                        .labels(vec![
                            Line::raw("0"),
                            Line::raw(format!("{}", x_max as usize)),
                        ]),
                )
                .y_axis(
                    Axis::default()
                        .style(theme::primary_text())
                        .bounds([y_min, y_max])
                        .labels(vec![
                            Self::fx_axis_label(FxAxisFormat::Money, y_min),
                            Self::fx_axis_label(FxAxisFormat::Money, y_max),
                        ]),
                )
                .block(
                    Block::default()
                        .title(format!(
                            " FX Reserves by Currency ({}) ",
                            tab.selected_currency_code
                        ))
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    fn draw_fx_table(&self, frame: &mut Frame, area: Rect, tab: &FxTabSnapshot) {
        let total_rows = tab.fx_rows.len();
        let visible_height = area.height.saturating_sub(3) as usize;
        let selected_idx = tab
            .fx_rows
            .iter()
            .position(|row| row.is_selected)
            .unwrap_or(0);
        let view_scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));
        let start = if total_rows == 0 { 0 } else { view_scroll + 1 };
        let end = (view_scroll + visible_height).min(total_rows);
        let title = if total_rows == 0 {
            " FX Leaderboard ".to_string()
        } else {
            format!(" FX Leaderboard ({}-{} of {}) ", start, end, total_rows)
        };

        let header = Row::new([
            "Country",
            "CCY",
            "Rate vs Sel",
            "MoM",
            "Reserves",
            "Net Flow",
            "Pressure",
            "X/M",
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );

        let rows = tab
            .fx_rows
            .iter()
            .skip(view_scroll)
            .take(visible_height)
            .map(Self::fx_table_row);

        let widths = [
            Constraint::Length(16),
            Constraint::Length(6),
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(9),
            Constraint::Length(7),
        ];

        frame.render_widget(
            Table::new(rows, widths)
                .header(header)
                .column_spacing(1)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );

        if total_rows > visible_height {
            let mut sb_state = ScrollbarState::new(total_rows.saturating_sub(visible_height))
                .position(view_scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut sb_state,
            );
        }
    }

    fn fx_table_row(row: &FxRowSnapshot) -> Row<'static> {
        let base_style = if row.is_selected {
            Style::default()
                .fg(theme::TEXT_PRIMARY)
                .bg(theme::HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_PRIMARY)
        };
        let move_style = if row.month_move_pct.starts_with('+') {
            Style::default().fg(Color::Green)
        } else if row.month_move_pct.starts_with('-') {
            Style::default().fg(Color::Red)
        } else {
            base_style
        };

        Row::new(vec![
            ratatui::text::Text::from(row.country.clone()),
            ratatui::text::Text::from(row.currency_code.clone()),
            ratatui::text::Text::from(row.current_fx_rate.clone()),
            ratatui::text::Text::styled(row.month_move_pct.clone(), move_style),
            ratatui::text::Text::from(row.reserves.clone()),
            ratatui::text::Text::from(row.net_flow.clone()),
            ratatui::text::Text::from(row.pressure.clone()),
            ratatui::text::Text::from(row.export_import_ratio.clone()),
        ])
        .style(base_style)
    }

    fn draw_fx_chart(&self, frame: &mut Frame, area: Rect, chart: FxChart<'_>) {
        if chart.data.is_empty() {
            frame.render_widget(
                Paragraph::new("No history data yet.")
                    .style(theme::muted_text())
                    .block(
                        Block::default()
                            .title(chart.title)
                            .borders(Borders::ALL)
                            .border_style(theme::neutral_border())
                            .title_style(theme::primary_text()),
                    ),
                area,
            );
            return;
        }

        let points = chart
            .data
            .iter()
            .enumerate()
            .map(|(idx, value)| (idx as f64, *value))
            .collect::<Vec<_>>();
        let x_max = points.len().saturating_sub(1).max(1) as f64;
        let (min, max) = points
            .iter()
            .map(|(_, value)| *value)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        let (y_min, y_max) = Self::fx_axis_bounds(min, max);

        frame.render_widget(
            Chart::new(vec![
                Dataset::default()
                    .graph_type(GraphType::Line)
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(chart.color))
                    .data(&points),
            ])
            .x_axis(
                Axis::default()
                    .title("Month")
                    .style(theme::primary_text())
                    .bounds([0.0, x_max])
                    .labels(vec![
                        Line::raw("0"),
                        Line::raw(format!("{}", x_max as usize)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(theme::primary_text())
                    .bounds([y_min, y_max])
                    .labels(vec![
                        Self::fx_axis_label(chart.format, y_min),
                        Self::fx_axis_label(chart.format, y_max),
                    ]),
            )
            .block(
                Block::default()
                    .title(chart.title)
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text()),
            ),
            area,
        );
    }

    fn fx_axis_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }
        if (max - min).abs() < f64::EPSILON {
            let padding = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.1
            };
            return (min - padding, max + padding);
        }
        let padding = (max - min).abs() * 0.1;
        (min - padding, max + padding)
    }

    fn fx_axis_label(format: FxAxisFormat, value: f64) -> Line<'static> {
        match format {
            FxAxisFormat::Rate => Line::raw(format!("{value:.2}")),
            FxAxisFormat::Money => {
                Line::raw(crate::components::simulation::snapshot::compact_num(value))
            }
            FxAxisFormat::Pressure => Line::raw(format!("{value:.2}")),
        }
    }
}
