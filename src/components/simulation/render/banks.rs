use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Line,
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table, Wrap},
};

use super::super::{Simulation, theme};
use super::events::event_search_hint;
use crate::components::simulation::{
    hotkeys,
    snapshot::{BanksTabSnapshot, RunningSnapshot},
    ui_state::{DetailsView, SelectionFocus},
};

impl Simulation {
    pub(super) fn draw_banks_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
        tab: &BanksTabSnapshot,
    ) -> String {
        match world.details_view {
            DetailsView::Bank => {
                self.draw_bank_details(frame, body, world, tab);
                match world.details_page {
                    0 => format!(
                        "Controls: {}  [Esc] back to banks list  [PgUp/PgDn] details page (1/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    1 => format!(
                        "Controls: {}  [Esc] back to banks list  [PgUp/PgDn] details page (2/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    2 => format!(
                        "Controls: {}  [Esc] back to banks list  [PgUp/PgDn] details page (3/4)  [w/s] scroll  [{}]  [o] toggle sort  [f] category | Events: {} | Sort: {} | Category: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        event_search_hint(
                            world.events_search_active,
                            &tab.bank_related_events.events_search_query
                        ),
                        tab.bank_related_events.events_total_count,
                        tab.bank_related_events.events_sort_label,
                        tab.bank_related_events.events_category_label,
                    ),
                    _ => format!(
                        "Controls: {}  [Esc] back to banks list  [PgUp/PgDn] details page (4/4)  [w/s] scroll depositors | Depositors: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        tab.bank_depositor_rows.len(),
                    ),
                }
            }
            _ => {
                let [top, bottom] =
                    Layout::vertical([Constraint::Length(7), Constraint::Min(10)]).areas(body);
                self.draw_banks_summary(frame, top, tab);
                self.draw_banks_table(frame, bottom, world, tab);
                format!(
                    "Controls: {}  [,/.] change country  [a/d] focus country/bank  [w/s] move row (focused list)  [PgUp/PgDn] page (focused list)  [Enter] bank details | Country Page {}/{} | Bank Page {}/{}",
                    hotkeys::GLOBAL_RUNNING_HINT,
                    world.country_page + 1,
                    world.total_country_pages,
                    world.bank_page + 1,
                    world.total_bank_pages,
                )
            }
        }
    }

    fn draw_banks_summary(&self, frame: &mut Frame, area: Rect, tab: &BanksTabSnapshot) {
        let lines = tab
            .country_banks_summary_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Banking Summary ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    fn draw_banks_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &BanksTabSnapshot,
    ) {
        let header = Row::new([
            Cell::from("ID"),
            Cell::from("Bank"),
            Cell::from("Type"),
            Cell::from("Reserve"),
            Cell::from("Deposits"),
            Cell::from("Loans"),
            Cell::from("Health"),
            Cell::from("Run"),
            Cell::from("Status"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );

        let rows = tab.bank_rows.iter().map(|bank| {
            let style = if bank.is_selected && world.selection_focus == SelectionFocus::Bank {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(theme::HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD)
            } else if bank.is_even {
                Style::default().fg(theme::TEXT_PRIMARY)
            } else {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(Color::Rgb(23, 31, 48))
            };

            Row::new([
                Cell::from(bank.bank_id.clone()),
                Cell::from(bank.name.clone()),
                Cell::from(bank.kind.clone()),
                Cell::from(bank.reserve.clone()),
                Cell::from(bank.deposits.clone()),
                Cell::from(bank.created_money.clone()),
                Cell::from(bank.health.clone()),
                Cell::from(bank.run_intensity.clone()),
                Cell::from(bank.resolution.clone()),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(4),
            Constraint::Length(24),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(14),
        ];

        frame.render_widget(
            Table::new(rows, widths)
                .header(header)
                .column_spacing(1)
                .block(
                    Block::default()
                        .title(" Banks ")
                        .borders(Borders::ALL)
                        .border_style(if world.selection_focus == SelectionFocus::Bank {
                            theme::active_border()
                        } else {
                            theme::neutral_border()
                        })
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    pub(super) fn draw_bank_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &BanksTabSnapshot,
    ) {
        match world.details_page {
            0 => {
                let details = tab
                    .bank_details_lines
                    .iter()
                    .map(|line| Line::from(line.as_str()))
                    .collect::<Vec<_>>();
                frame.render_widget(
                    Paragraph::new(details)
                        .style(theme::primary_text().bg(theme::PANEL_BG))
                        .wrap(Wrap { trim: true })
                        .block(
                            Block::default()
                                .title(" Bank Details (Page 1/4) ")
                                .borders(Borders::ALL)
                                .border_style(theme::neutral_border())
                                .title_style(theme::primary_text())
                                .style(Style::default().bg(theme::PANEL_BG)),
                        ),
                    area,
                );
            }
            1 => {
                let [charts_top, charts_mid, charts_bottom] = Layout::vertical([
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ])
                .areas(area);
                let [reserve_area, deposit_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_top);
                let [created_area, equity_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_mid);
                let [run_area, spare_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_bottom);

                self.draw_bank_history_chart(
                    frame,
                    reserve_area,
                    tab.bank_reserve_history.as_ref(),
                    "Reserve History (Page 2/4)",
                    Color::Cyan,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                );
                self.draw_bank_history_chart(
                    frame,
                    deposit_area,
                    tab.bank_deposit_history.as_ref(),
                    "Deposits History (Page 2/4)",
                    Color::Green,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                );
                self.draw_bank_history_chart(
                    frame,
                    created_area,
                    tab.bank_created_money_history.as_ref(),
                    "Created Money History (Page 2/4)",
                    Color::Yellow,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                );
                self.draw_bank_history_chart(
                    frame,
                    equity_area,
                    tab.bank_equity_history.as_ref(),
                    "Equity History (Page 2/4)",
                    Color::LightMagenta,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                );
                self.draw_bank_history_chart(
                    frame,
                    run_area,
                    tab.bank_run_intensity_history.as_ref(),
                    "Run Intensity History (Page 2/4)",
                    Color::LightRed,
                    true,
                    true,
                    world.currency_symbol.as_str(),
                );
                frame.render_widget(
                    Paragraph::new("")
                        .style(theme::muted_text().bg(theme::PANEL_BG))
                        .block(
                            Block::default()
                                .title(" Bank Charts (Page 2/4) ")
                                .borders(Borders::ALL)
                                .border_style(theme::neutral_border())
                                .style(Style::default().bg(theme::PANEL_BG)),
                        ),
                    spare_area,
                );
            }
            2 => self.draw_event_log_panel(
                frame,
                area,
                world,
                &tab.bank_related_events,
                "Bank Related Events (Page 3/4)",
            ),
            _ => self.draw_bank_depositors_table(frame, area, world, tab),
        }
    }

    fn draw_bank_depositors_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &BanksTabSnapshot,
    ) {
        let total = tab.bank_depositor_rows.len();
        let max_scroll = total.saturating_sub(1);
        let scroll = world.bank_depositors_scroll.min(max_scroll);
        let list_height = usize::from(area.height.saturating_sub(3));
        let visible_rows = tab
            .bank_depositor_rows
            .iter()
            .skip(scroll)
            .take(list_height)
            .enumerate()
            .map(|(idx, row)| {
                let style = if idx.is_multiple_of(2) {
                    Style::default().fg(theme::TEXT_PRIMARY)
                } else {
                    Style::default()
                        .fg(theme::TEXT_PRIMARY)
                        .bg(Color::Rgb(23, 31, 48))
                };
                Row::new([
                    Cell::from(row.entity_kind.clone()),
                    Cell::from(row.entity.clone()),
                    Cell::from(row.balance.clone()),
                    Cell::from(row.share.clone()),
                ])
                .style(style)
            });

        let start = if total == 0 { 0 } else { scroll + 1 };
        let end = if total == 0 {
            0
        } else {
            (scroll + list_height).min(total)
        };
        let header = Row::new([
            Cell::from("Type"),
            Cell::from("Entity"),
            Cell::from(format!("Balance ({})", world.currency_symbol)),
            Cell::from("Share"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );
        let widths = [
            Constraint::Length(10),
            Constraint::Length(36),
            Constraint::Length(18),
            Constraint::Length(10),
        ];

        frame.render_widget(
            Table::new(visible_rows, widths)
                .header(header)
                .column_spacing(1)
                .block(
                    Block::default()
                        .title(format!(
                            " Bank Depositors (Page 4/4) ({}-{} of {}) ",
                            start, end, total
                        ))
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "chart rendering keeps call sites explicit for each history line"
    )]
    fn draw_bank_history_chart(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &[f64],
        title: &str,
        color: Color,
        as_percent: bool,
        clamp_percent_range: bool,
        currency_symbol: &str,
    ) {
        if history.is_empty() {
            frame.render_widget(
                Paragraph::new("No monthly history yet.")
                    .wrap(Wrap { trim: true })
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
            .enumerate()
            .map(|(idx, value)| {
                let y = if as_percent { *value * 100.0 } else { *value };
                (idx as f64 + 1.0, y)
            })
            .collect::<Vec<_>>();

        let x_min = 1.0;
        let mut x_max = history.len() as f64;
        if (x_max - x_min).abs() < f64::EPSILON {
            x_max = x_min + 1.0;
        }

        let (value_min, value_max) = points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), point| {
                (min.min(point.1), max.max(point.1))
            });
        let (mut y_min, mut y_max) = Self::bank_chart_axis_bounds(value_min, value_max);
        if clamp_percent_range {
            y_min = y_min.clamp(0.0, 100.0);
            y_max = y_max.clamp(0.0, 100.0);
            if (y_max - y_min).abs() < f64::EPSILON {
                y_max = (y_min + 1.0).min(100.0);
            }
        }

        let y_labels = if as_percent {
            vec![format!("{y_min:.1}%"), format!("{y_max:.1}%")]
        } else {
            vec![
                Self::money(y_min, currency_symbol),
                Self::money(y_max, currency_symbol),
            ]
        };

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
                    .labels(vec!["1".to_string(), history.len().to_string()]),
            )
            .y_axis(
                Axis::default()
                    .style(theme::primary_text().bg(theme::PANEL_BG))
                    .bounds([y_min, y_max])
                    .labels(y_labels),
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

    fn bank_chart_axis_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }
        if (max - min).abs() < f64::EPSILON {
            let padding = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.10
            };
            return (min - padding, max + padding);
        }
        let span = max - min;
        let padding = span * 0.10;
        (min - padding, max + padding)
    }
}
