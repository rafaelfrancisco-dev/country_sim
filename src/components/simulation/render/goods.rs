use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Line,
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Wrap},
};

use super::super::{Simulation, theme};
use super::disaster_markers;
use super::events::event_search_hint;
use crate::components::simulation::{
    hotkeys,
    snapshot::{GoodsTabSnapshot, RunningSnapshot},
    ui_state::{DetailsView, SelectionFocus},
};

fn compact_count_f64(n: f64) -> String {
    let abs = n.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{n:.0}");
    };
    let scaled = n / divisor;
    let r = (scaled * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}{suffix}")
    } else {
        format!("{r:.1}{suffix}")
    }
}

struct CompanyHistoryChart<'a> {
    history: &'a [f64],
    title: &'a str,
    color: Color,
    as_money: bool,
}
struct CompanyHistoryContext<'a> {
    currency_symbol: &'a str,
    selected_country_month: u64,
    disaster_days: &'a [u64],
}

impl Simulation {
    pub(super) fn draw_goods_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
        tab: &GoodsTabSnapshot,
    ) -> String {
        match world.details_view {
            DetailsView::Company => {
                self.draw_company_details(frame, body, world, tab);
                match world.details_page {
                    0 => format!(
                        "Controls: {}  [Esc] back to goods list  [PgUp/PgDn] details page (1/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    1 => format!(
                        "Controls: {}  [Esc] back to goods list  [PgUp/PgDn] details page (2/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    2 => format!(
                        "Controls: {}  [Esc] back to goods list  [PgUp/PgDn] details page (3/4)  [w/s] scroll  [{}]  [o] toggle sort  [f] category | Events: {} | Sort: {} | Category: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        event_search_hint(
                            world.events_search_active,
                            &tab.company_related_events.events_search_query
                        ),
                        tab.company_related_events.events_total_count,
                        tab.company_related_events.events_sort_label,
                        tab.company_related_events.events_category_label,
                    ),
                    _ => format!(
                        "Controls: {}  [Esc] back to goods list  [PgUp/PgDn] details page (4/4)  [w/s] scroll workers | Employees: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        tab.company_worker_rows.len(),
                    ),
                }
            }
            _ => {
                let [top, bottom] =
                    Layout::vertical([Constraint::Length(8), Constraint::Min(10)]).areas(body);

                self.draw_goods_summary(frame, top, tab);
                self.draw_goods_company_table(frame, bottom, world, tab);

                format!(
                    "Controls: {}  [,/.] change country  [a/d] focus country/company  [w/s] move row (focused list)  [PgUp/PgDn] page (focused list)  [Enter] company details | Country Page {}/{} | Company Page {}/{}",
                    hotkeys::GLOBAL_RUNNING_HINT,
                    world.country_page + 1,
                    world.total_country_pages,
                    world.company_page + 1,
                    world.total_company_pages,
                )
            }
        }
    }

    fn draw_goods_summary(&self, frame: &mut Frame, area: Rect, tab: &GoodsTabSnapshot) {
        let lines = tab
            .country_goods_summary_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Goods Summary ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    fn draw_goods_company_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &GoodsTabSnapshot,
    ) {
        let avg_label = format!("Avg {}", world.currency_symbol);
        let header = Row::new([
            Cell::from("ID"),
            Cell::from("Company"),
            Cell::from("Good"),
            Cell::from("Price"),
            Cell::from(avg_label),
            Cell::from("Inv"),
            Cell::from("Prod"),
            Cell::from("Sold"),
            Cell::from("Tot Sold"),
            Cell::from("Export"),
            Cell::from("D/X"),
            Cell::from("Sales"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );

        let total_rows = tab.company_rows.len();
        let visible_height = area.height.saturating_sub(3) as usize;
        let selected_idx = tab.company_rows.iter().position(|r| r.is_selected).unwrap_or(0);
        let view_scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));

        let rows = tab.company_rows.iter().skip(view_scroll).take(visible_height).map(|company| {
            let style = if company.is_selected && world.selection_focus == SelectionFocus::Company {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(theme::HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD)
            } else if company.is_even {
                Style::default().fg(theme::TEXT_PRIMARY)
            } else {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(Color::Rgb(23, 31, 48))
            };

            Row::new([
                Cell::from(company.company_id.clone()),
                Cell::from(company.name.clone()),
                Cell::from(company.good.clone()),
                Cell::from(company.price.clone()),
                Cell::from(company.avg_price.clone()),
                Cell::from(company.inventory.clone()),
                Cell::from(company.produced.clone()),
                Cell::from(company.sold.clone()),
                Cell::from(company.total_sold.clone()),
                Cell::from(company.export_sold.clone()),
                Cell::from(company.domestic_export_split.clone()),
                Cell::from(company.sales.clone()),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(4),
            Constraint::Length(18),
            Constraint::Length(18),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(12),
        ];

        let start = if total_rows == 0 { 0 } else { view_scroll + 1 };
        let end = (view_scroll + visible_height).min(total_rows);
        let title = if total_rows == 0 {
            " Companies By Goods ".to_string()
        } else {
            format!(" Companies By Goods ({}-{} of {}) ", start, end, total_rows)
        };

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if world.selection_focus == SelectionFocus::Company {
                        theme::active_border()
                    } else {
                        theme::neutral_border()
                    })
                    .title_style(theme::primary_text()),
            );

        frame.render_widget(table, area);
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

    pub(super) fn draw_company_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &GoodsTabSnapshot,
    ) {
        match world.details_page {
            0 => {
                let details = tab
                    .company_details_lines
                    .iter()
                    .map(|line| Line::from(line.as_str()))
                    .collect::<Vec<_>>();

                frame.render_widget(
                    Paragraph::new(details)
                        .style(theme::primary_text().bg(theme::PANEL_BG))
                        .wrap(Wrap { trim: true })
                        .block(
                            Block::default()
                                .title(" Company Details (Page 1/4) ")
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
                let [sold_area, employee_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_top);
                let [price_area, loan_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_mid);
                let [money_area, spare_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_bottom);

                let chart_context = CompanyHistoryContext {
                    currency_symbol: world.currency_symbol.as_str(),
                    selected_country_month: world.selected_country_month,
                    disaster_days: tab.disaster_days.as_ref(),
                };

                self.draw_company_history_chart(
                    frame,
                    sold_area,
                    CompanyHistoryChart {
                        history: tab.company_sold_history.as_ref(),
                        title: "Sold Units History (Page 2/4)",
                        color: Color::Cyan,
                        as_money: false,
                    },
                    &chart_context,
                );
                self.draw_company_history_chart(
                    frame,
                    employee_area,
                    CompanyHistoryChart {
                        history: tab.company_employee_history.as_ref(),
                        title: "Employees History (Page 2/4)",
                        color: Color::Green,
                        as_money: false,
                    },
                    &chart_context,
                );
                self.draw_company_history_chart(
                    frame,
                    price_area,
                    CompanyHistoryChart {
                        history: tab.company_price_history.as_ref(),
                        title: "Price History (Page 2/4)",
                        color: Color::Yellow,
                        as_money: true,
                    },
                    &chart_context,
                );
                self.draw_company_history_chart(
                    frame,
                    loan_area,
                    CompanyHistoryChart {
                        history: tab.company_loan_payment_history.as_ref(),
                        title: "Loan Payments History (Page 2/4)",
                        color: Color::LightMagenta,
                        as_money: true,
                    },
                    &chart_context,
                );
                self.draw_company_history_chart(
                    frame,
                    money_area,
                    CompanyHistoryChart {
                        history: tab.company_money_history.as_ref(),
                        title: "Money History (Page 2/4)",
                        color: Color::LightGreen,
                        as_money: true,
                    },
                    &chart_context,
                );
                frame.render_widget(
                    Paragraph::new("")
                        .style(theme::muted_text().bg(theme::PANEL_BG))
                        .block(
                            Block::default()
                                .title(" Company Charts (Page 2/4) ")
                                .borders(Borders::ALL)
                                .border_style(theme::neutral_border())
                                .style(Style::default().bg(theme::PANEL_BG)),
                        ),
                    spare_area,
                );
            }
            2 => {
                self.draw_event_log_panel(
                    frame,
                    area,
                    world,
                    &tab.company_related_events,
                    "Company Related Events (Page 3/4)",
                );
            }
            _ => self.draw_company_workers_table(frame, area, world, tab),
        }
    }

    fn draw_company_workers_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &GoodsTabSnapshot,
    ) {
        let total = tab.company_worker_rows.len();
        let max_scroll = total.saturating_sub(1);
        let scroll = world.company_workers_scroll.min(max_scroll);
        let list_height = usize::from(area.height.saturating_sub(3));
        let visible_rows = tab
            .company_worker_rows
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
                    Cell::from(row.name.clone()),
                    Cell::from(row.job.clone()),
                    Cell::from(row.tenure_months.clone()),
                    Cell::from(row.salary.clone()),
                ])
                .style(style)
            });

        let start = if total == 0 { 0 } else { scroll + 1 };
        let end = if total == 0 {
            0
        } else {
            (scroll + list_height).min(total)
        };
        let pay_label = format!("Pay ({})", world.currency_symbol);
        let header = Row::new([
            Cell::from("Name"),
            Cell::from("Job"),
            Cell::from("Tenure (mo)"),
            Cell::from(pay_label),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );
        let widths = [
            Constraint::Length(28),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(16),
        ];

        let table = Table::new(visible_rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(format!(
                        " Company Employees (Page 4/4) ({}-{} of {}) ",
                        start, end, total
                    ))
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text()),
            );

        frame.render_widget(table, area);
    }

    fn draw_company_history_chart(
        &self,
        frame: &mut Frame,
        area: Rect,
        chart: CompanyHistoryChart<'_>,
        context: &CompanyHistoryContext<'_>,
    ) {
        let CompanyHistoryChart {
            history,
            title,
            color,
            as_money,
        } = chart;
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
            .map(|(idx, value)| (idx as f64 + 1.0, *value))
            .collect::<Vec<_>>();

        let x_min = 1.0;
        let mut x_max = history.len() as f64;
        if (x_max - x_min).abs() < f64::EPSILON {
            x_max = x_min + 1.0;
        }

        let (value_min, value_max) = history
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(*value), max.max(*value))
            });
        let (y_min, y_max) = Self::company_chart_axis_bounds(value_min, value_max);
        let y_labels = if as_money {
            vec![
                Self::money(y_min, context.currency_symbol),
                Self::money(y_max, context.currency_symbol),
            ]
        } else {
            vec![compact_count_f64(y_min), compact_count_f64(y_max)]
        };

        let dataset = Dataset::default()
            .graph_type(GraphType::Line)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(color).bg(theme::PANEL_BG))
            .data(&points);
        let start_month = context
            .selected_country_month
            .saturating_add(1)
            .saturating_sub(history.len() as u64);
        let marker_positions =
            disaster_markers::disaster_relative_month_positions(context.disaster_days, start_month);
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

    fn company_chart_axis_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        if (max - min).abs() < f64::EPSILON {
            let padding = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.10
            };
            let lower = (min - padding).max(0.0);
            let upper = max + padding;
            return (lower, upper);
        }

        let span = max - min;
        let padding = span * 0.10;
        let lower = (min - padding).max(0.0);
        let upper = max + padding;
        (lower, upper)
    }
}
