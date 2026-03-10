use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, Wrap,
    },
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::{GoodsTabSnapshot, RunningSnapshot};
use crate::components::simulation::ui_state::SelectionFocus;

impl Simulation {
    pub(super) fn draw_goods_summary(&self, frame: &mut Frame, area: Rect, tab: &GoodsTabSnapshot) {
        let lines = tab
            .country_goods_summary_lines
            .iter()
            .map(|line| ratatui::text::Line::from(line.as_str()))
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

    pub(super) fn draw_goods_company_table(
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
        let selected_idx = tab
            .company_rows
            .iter()
            .position(|r| r.is_selected)
            .unwrap_or(0);
        let view_scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));

        let rows = tab
            .company_rows
            .iter()
            .skip(view_scroll)
            .take(visible_height)
            .map(|company| {
                let style =
                    if company.is_selected && world.selection_focus == SelectionFocus::Company {
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

        frame.render_widget(
            Table::new(rows, widths)
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

    pub(super) fn draw_goods_production_overview(
        &self,
        frame: &mut Frame,
        body: Rect,
        _world: &RunningSnapshot,
        tab: &GoodsTabSnapshot,
    ) -> String {
        let [summary_area, table_area] =
            Layout::vertical([Constraint::Length(9), Constraint::Min(10)]).areas(body);

        self.draw_goods_production_summary(frame, summary_area, tab);
        self.draw_goods_production_table(frame, table_area, tab);

        format!(
            "Controls: {}  [,/.] change country  [g] switch goods page  [u/i] page goods overview | Goods Page 2/2 | Overview Page {}/{}",
            crate::components::simulation::hotkeys::GLOBAL_RUNNING_HINT,
            tab.goods_production_page + 1,
            tab.total_goods_production_pages,
        )
    }

    fn draw_goods_production_summary(&self, frame: &mut Frame, area: Rect, tab: &GoodsTabSnapshot) {
        let lines = tab
            .production_demand_summary_lines
            .iter()
            .map(|line| ratatui::text::Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Production & Demand Summary ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    fn draw_goods_production_table(&self, frame: &mut Frame, area: Rect, tab: &GoodsTabSnapshot) {
        let start_idx = tab.goods_production_page
            * crate::components::simulation::ui_state::GOODS_PRODUCTION_PAGE_SIZE;
        let rows = tab
            .production_demand_rows
            .iter()
            .skip(start_idx)
            .take(crate::components::simulation::ui_state::GOODS_PRODUCTION_PAGE_SIZE)
            .enumerate()
            .map(|(row_idx, row)| {
                let style = if row_idx.is_multiple_of(2) {
                    Style::default().fg(theme::TEXT_PRIMARY)
                } else {
                    Style::default()
                        .fg(theme::TEXT_PRIMARY)
                        .bg(Color::Rgb(23, 31, 48))
                };

                Row::new([
                    Cell::from(row.good.clone()),
                    Cell::from(row.producer_count.clone()),
                    Cell::from(row.avg_price.clone()),
                    Cell::from(row.inventory_units.clone()),
                    Cell::from(row.produced_units.clone()),
                    Cell::from(row.sold_units.clone()),
                    Cell::from(row.export_units.clone()),
                    Cell::from(row.est_daily_demand_units.clone()),
                    Cell::from(row.est_mtd_demand_units.clone()),
                ])
                .style(style)
            })
            .collect::<Vec<_>>();

        let end_idx = (start_idx
            + crate::components::simulation::ui_state::GOODS_PRODUCTION_PAGE_SIZE)
            .min(tab.production_demand_rows.len());
        let title = if tab.production_demand_rows.is_empty() {
            " Production & Demand ".to_string()
        } else {
            format!(
                " Production & Demand ({}-{} of {}) ",
                start_idx + 1,
                end_idx,
                tab.production_demand_rows.len()
            )
        };
        let header = Row::new([
            "Good",
            "Prods",
            "Avg Price",
            "Inv",
            "MTD Prod",
            "MTD Sold",
            "Export",
            "Daily Dem",
            "MTD Dem",
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );
        let widths = [
            Constraint::Min(16),
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(9),
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
    }
}
