use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::{MainTabSnapshot, RunningSnapshot};
use crate::components::simulation::ui_state::SelectionFocus;

impl Simulation {
    pub(super) fn draw_countries_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &MainTabSnapshot,
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

        let total_rows = tab.country_rows.len();
        let visible_height = area.height.saturating_sub(3) as usize;
        let selected_idx = tab
            .country_rows
            .iter()
            .position(|r| r.is_selected)
            .unwrap_or(0);
        let view_scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));

        let rows = tab
            .country_rows
            .iter()
            .skip(view_scroll)
            .take(visible_height)
            .map(|country| {
                let style =
                    if country.is_selected && world.selection_focus == SelectionFocus::Country {
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

        let start = if total_rows == 0 { 0 } else { view_scroll + 1 };
        let end = (view_scroll + visible_height).min(total_rows);
        let title = if total_rows == 0 {
            " Countries ".to_string()
        } else {
            format!(" Countries ({}-{} of {}) ", start, end, total_rows)
        };

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if world.selection_focus == SelectionFocus::Country {
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

    pub(super) fn draw_people_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &MainTabSnapshot,
    ) {
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

        if !tab.has_selected_country {
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

        let total_rows = tab.people_rows.len();
        let visible_height = area.height.saturating_sub(3) as usize;
        let selected_idx = tab
            .people_rows
            .iter()
            .position(|r| r.is_selected)
            .unwrap_or(0);
        let view_scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));

        let rows = tab
            .people_rows
            .iter()
            .skip(view_scroll)
            .take(visible_height)
            .map(|person| {
                let style = if person.is_selected && world.selection_focus == SelectionFocus::People
                {
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

        let start = if total_rows == 0 { 0 } else { view_scroll + 1 };
        let end = (view_scroll + visible_height).min(total_rows);
        let title = format!(" People ({}-{} of {}) ", start, end, total_rows);

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if world.selection_focus == SelectionFocus::People {
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
}
