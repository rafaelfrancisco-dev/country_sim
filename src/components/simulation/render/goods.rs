use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Line,
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::{
    snapshot::RunningSnapshot,
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

impl Simulation {
    pub(super) fn draw_goods_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
    ) -> String {
        match world.details_view {
            DetailsView::Company => {
                self.draw_company_details(frame, body, world);
                "Controls: [Tab/Shift+Tab] switch tab  [Esc] back to goods list  [Space] pause/resume  [n] step day (paused)  [m] toggle sim mode  [q] quit".to_string()
            }
            _ => {
                let [top, bottom] =
                    Layout::vertical([Constraint::Length(8), Constraint::Min(10)]).areas(body);

                self.draw_goods_summary(frame, top, world);
                self.draw_goods_company_table(frame, bottom, world);

                format!(
                    "Controls: [Tab/Shift+Tab] switch tab  [</>] change country  [←/→] focus country/company  [↑/↓] move row (focused list)  [j/k] page (focused list)  [Enter] company details  [Space] pause/resume  [n] step day (paused)  [m] toggle sim mode  [q] quit | Country Page {}/{} | Company Page {}/{}",
                    world.country_page + 1,
                    world.total_country_pages,
                    world.company_page + 1,
                    world.total_company_pages,
                )
            }
        }
    }

    fn draw_goods_summary(&self, frame: &mut Frame, area: Rect, world: &RunningSnapshot) {
        let lines = world
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

    fn draw_goods_company_table(&self, frame: &mut Frame, area: Rect, world: &RunningSnapshot) {
        let header = Row::new([
            Cell::from("ID"),
            Cell::from("Company"),
            Cell::from("Good"),
            Cell::from("Price"),
            Cell::from("Avg $"),
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

        let rows = world.company_rows.iter().map(|company| {
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

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(" Companies By Goods ")
                    .borders(Borders::ALL)
                    .border_style(if world.selection_focus == SelectionFocus::Company {
                        theme::active_border()
                    } else {
                        theme::neutral_border()
                    })
                    .title_style(theme::primary_text()),
            );

        frame.render_widget(table, area);
    }

    pub(super) fn draw_company_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
    ) {
        let [details_area, charts_area] =
            Layout::vertical([Constraint::Length(16), Constraint::Min(10)]).areas(area);

        let details = world
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
                        .title(" Company Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text())
                        .style(Style::default().bg(theme::PANEL_BG)),
                ),
            details_area,
        );

        let [charts_top, charts_bottom] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(charts_area);
        let [sold_area, employee_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(charts_top);
        let [price_area, loan_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(charts_bottom);

        self.draw_company_history_chart(
            frame,
            sold_area,
            world.company_sold_history.as_slice(),
            "Sold Units History",
            Color::Cyan,
            false,
        );
        self.draw_company_history_chart(
            frame,
            employee_area,
            world.company_employee_history.as_slice(),
            "Employees History",
            Color::Green,
            false,
        );
        self.draw_company_history_chart(
            frame,
            price_area,
            world.company_price_history.as_slice(),
            "Price History",
            Color::Yellow,
            true,
        );
        self.draw_company_history_chart(
            frame,
            loan_area,
            world.company_loan_payment_history.as_slice(),
            "Loan Payments History",
            Color::LightMagenta,
            true,
        );
    }

    fn draw_company_history_chart(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &[f64],
        title: &str,
        color: Color,
        as_money: bool,
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
            vec![Self::money(y_min), Self::money(y_max)]
        } else {
            vec![compact_count_f64(y_min), compact_count_f64(y_max)]
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
