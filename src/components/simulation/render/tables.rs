use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::super::{Simulation, theme};
use crate::sim::{job::job_by_id, names::person_name_from_seed, world::SelectionFocus};

impl Simulation {
    pub(super) fn draw_countries_table(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let header = Row::new([
            Cell::from("Country"),
            Cell::from("Population"),
            Cell::from("Households"),
            Cell::from("Companies"),
            Cell::from("Treasury"),
            Cell::from("Loans"),
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );

        let country_page_start = world.country_page() * crate::sim::world::COUNTRY_PAGE_SIZE;
        let rows = world
            .selected_countries_page()
            .iter()
            .enumerate()
            .map(|(page_idx, country)| {
                let idx = country_page_start + page_idx;
                let style = if idx == world.selected_country
                    && world.selection_focus == SelectionFocus::Country
                {
                    Style::default()
                        .fg(theme::TEXT_PRIMARY)
                        .bg(theme::HIGHLIGHT_BG)
                        .add_modifier(Modifier::BOLD)
                } else if idx % 2 == 0 {
                    Style::default().fg(theme::TEXT_PRIMARY)
                } else {
                    Style::default()
                        .fg(theme::TEXT_PRIMARY)
                        .bg(Color::Rgb(23, 31, 48))
                };

                Row::new([
                    Cell::from(country.name.clone()),
                    Cell::from(country.population().to_string()),
                    Cell::from(Self::money(country.household_cash_total)),
                    Cell::from(Self::money(country.company_cash_total)),
                    Cell::from(Self::money(country.treasury_balance)),
                    Cell::from(Self::money(country.bank.created_money_outstanding)),
                ])
                .style(style)
            });

        let widths = [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Length(12),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(" Countries ")
                    .borders(Borders::ALL)
                    .border_style(if world.selection_focus == SelectionFocus::Country {
                        theme::active_border()
                    } else {
                        theme::neutral_border()
                    })
                    .title_style(theme::primary_text()),
            );

        frame.render_widget(table, area);
    }

    pub(super) fn draw_people_table(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let header = Row::new([
            Cell::from("Citizen ID"),
            Cell::from("Name"),
            Cell::from("Job"),
            Cell::from("Employer"),
            Cell::from("Salary"),
            Cell::from("Cash"),
            Cell::from("Loan"),
        ])
        .style(
            Style::default()
                .fg(Color::Green)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );

        let Some(country) = world.selected_country() else {
            let placeholder = Paragraph::new("No countries generated yet.")
                .style(theme::primary_text())
                .block(
                    Block::default()
                        .title(" People ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                );
            frame.render_widget(placeholder, area);
            return;
        };

        let page_people = world.selected_people_page().unwrap_or(&[]);
        let rows = page_people.iter().enumerate().map(|(row_idx, person)| {
            let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
            let name = person_name_from_seed(name_seed);
            let style = if row_idx == world.selected_person_row
                && world.selection_focus == SelectionFocus::People
            {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(theme::HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD)
            } else if row_idx % 2 == 0 {
                Style::default().fg(theme::TEXT_PRIMARY)
            } else {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(Color::Rgb(23, 31, 48))
            };

            Row::new([
                Cell::from(person.id.to_string()),
                Cell::from(name),
                Cell::from(
                    job_by_id(person.job_type_id)
                        .map_or("Unknown".to_string(), |job| job.name.to_string()),
                ),
                Cell::from(
                    country
                        .company_name(person.employer_company_id)
                        .unwrap_or("N/A")
                        .to_string(),
                ),
                Cell::from(Self::money(person.monthly_salary())),
                Cell::from(Self::money(person.cash_balance)),
                Cell::from(Self::money(
                    person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance),
                )),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(22),
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(" People (sample rows) ")
                    .borders(Borders::ALL)
                    .border_style(if world.selection_focus == SelectionFocus::People {
                        theme::active_border()
                    } else {
                        theme::neutral_border()
                    })
                    .title_style(theme::primary_text()),
            );

        frame.render_widget(table, area);
    }
}
