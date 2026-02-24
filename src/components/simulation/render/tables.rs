use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::RunningSnapshot;
use crate::components::simulation::ui_state::SelectionFocus;

impl Simulation {
    pub(super) fn draw_countries_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
    ) {
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

        let rows = world.country_rows.iter().map(|country| {
            let style = if country.is_selected && world.selection_focus == SelectionFocus::Country {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(theme::HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD)
            } else if country.is_even {
                Style::default().fg(theme::TEXT_PRIMARY)
            } else {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(Color::Rgb(23, 31, 48))
            };

            Row::new([
                Cell::from(country.name.clone()),
                Cell::from(country.population.clone()),
                Cell::from(country.household_cash.clone()),
                Cell::from(country.company_cash.clone()),
                Cell::from(country.treasury.clone()),
                Cell::from(country.loans.clone()),
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

    pub(super) fn draw_people_table(&self, frame: &mut Frame, area: Rect, world: &RunningSnapshot) {
        let header = Row::new([
            Cell::from("Citizen ID"),
            Cell::from("Name"),
            Cell::from("Age"),
            Cell::from("Education"),
            Cell::from("Employment"),
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

        if !world.has_selected_country {
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
        }

        let rows = world.people_rows.iter().map(|person| {
            let style = if person.is_selected && world.selection_focus == SelectionFocus::People {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(theme::HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD)
            } else if person.is_even {
                Style::default().fg(theme::TEXT_PRIMARY)
            } else {
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(Color::Rgb(23, 31, 48))
            };

            Row::new([
                Cell::from(person.citizen_id.clone()),
                Cell::from(person.name.clone()),
                Cell::from(person.age.clone()),
                Cell::from(person.education.clone()),
                Cell::from(person.employment.clone()),
                Cell::from(person.employer.clone()),
                Cell::from(person.salary.clone()),
                Cell::from(person.cash.clone()),
                Cell::from(person.loan.clone()),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Length(5),
            Constraint::Length(15),
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
