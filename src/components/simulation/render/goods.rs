use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::{
    snapshot::RunningSnapshot,
    ui_state::{DetailsView, SelectionFocus},
};

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
                "Controls: [Tab/Shift+Tab] switch tab  [Esc] back to goods list  [Space] pause/resume  [n] step day (paused)  [q] quit".to_string()
            }
            _ => {
                let [top, bottom] =
                    Layout::vertical([Constraint::Length(8), Constraint::Min(10)]).areas(body);

                self.draw_goods_summary(frame, top, world);
                self.draw_goods_company_table(frame, bottom, world);

                format!(
                    "Controls: [Tab/Shift+Tab] switch tab  [</>] change country  [←/→] focus country/company  [↑/↓] move row (focused list)  [j/k] page (focused list)  [Enter] company details  [Space] pause/resume  [n] step day (paused)  [q] quit | Country Page {}/{} | Company Page {}/{}",
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
            Cell::from("Inv"),
            Cell::from("Prod"),
            Cell::from("Sold"),
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
                Cell::from(company.inventory.clone()),
                Cell::from(company.produced.clone()),
                Cell::from(company.sold.clone()),
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
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(7),
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
        let details = world
            .company_details_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Company Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }
}
