use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::SetupSnapshot;

impl Simulation {
    pub(in crate::components::simulation) fn draw_setup(
        &self,
        frame: &mut Frame,
        area: Rect,
        setup: &SetupSnapshot,
    ) {
        let block = Block::default()
            .title(" Country Simulation Setup ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .style(Style::default().bg(theme::PANEL_BG))
            .title_style(theme::primary_text());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [intro, countries, millions, seed, status, help] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .areas(inner);

        frame.render_widget(
            Paragraph::new("Configure world size. Inputs are constrained to 1-50 countries and 0.1-20.0 million people per country.")
                .style(theme::primary_text())
                .wrap(Wrap { trim: true }),
            intro,
        );

        let active_style = Style::default()
            .fg(theme::TEXT_PRIMARY)
            .bg(theme::HIGHLIGHT_BG)
            .add_modifier(Modifier::BOLD);
        let inactive_style = theme::muted_text();

        let country_style = if setup.active_field == crate::sim::setup::SetupField::CountryCount {
            active_style
        } else {
            inactive_style
        };

        let million_style =
            if setup.active_field == crate::sim::setup::SetupField::MillionsPerCountry {
                active_style
            } else {
                inactive_style
            };
        let seed_style = if setup.active_field == crate::sim::setup::SetupField::GenerationSeed {
            active_style
        } else {
            inactive_style
        };

        let country_text = if setup.country_count_input.is_empty() {
            "<enter number>".to_string()
        } else {
            setup.country_count_input.clone()
        };

        let million_text = if setup.millions_per_country_input.is_empty() {
            "<enter number>".to_string()
        } else {
            setup.millions_per_country_input.clone()
        };
        let seed_text = if setup.generation_seed_input.is_empty() {
            "<enter decimal or 0xhex>".to_string()
        } else {
            setup.generation_seed_input.clone()
        };

        frame.render_widget(
            Paragraph::new(country_text)
                .block(
                    Block::default()
                        .title(" Countries ")
                        .borders(Borders::ALL)
                        .border_style(
                            if setup.active_field == crate::sim::setup::SetupField::CountryCount {
                                theme::active_border()
                            } else {
                                theme::neutral_border()
                            },
                        )
                        .title_style(theme::primary_text()),
                )
                .style(country_style),
            countries,
        );

        frame.render_widget(
            Paragraph::new(million_text)
                .block(
                    Block::default()
                        .title(" Millions per Country ")
                        .borders(Borders::ALL)
                        .border_style(
                            if setup.active_field
                                == crate::sim::setup::SetupField::MillionsPerCountry
                            {
                                theme::active_border()
                            } else {
                                theme::neutral_border()
                            },
                        )
                        .title_style(theme::primary_text()),
                )
                .style(million_style),
            millions,
        );

        frame.render_widget(
            Paragraph::new(seed_text)
                .block(
                    Block::default()
                        .title(" Generation Seed ")
                        .borders(Borders::ALL)
                        .border_style(
                            if setup.active_field == crate::sim::setup::SetupField::GenerationSeed {
                                theme::active_border()
                            } else {
                                theme::neutral_border()
                            },
                        )
                        .title_style(theme::primary_text()),
                )
                .style(seed_style),
            seed,
        );

        let mut status_lines = Vec::new();
        if let Some(error) = &setup.validation_error {
            status_lines.push(Line::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(warning) = &setup.memory_warning {
            status_lines.push(Line::styled(
                warning.clone(),
                Style::default().fg(Color::Yellow),
            ));
        }
        if status_lines.is_empty() {
            let default_status = if setup.is_generating {
                "Generating world... please wait."
            } else {
                "Ready. Press Enter to generate world."
            };
            status_lines.push(Line::styled(default_status, theme::primary_text()));
        }

        frame.render_widget(
            Paragraph::new(status_lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true }),
            status,
        );

        frame.render_widget(
            Paragraph::new(
                "Keys: [Tab/Shift+Tab] change field  [0-9/.] edit numeric fields  [0-9/a-f/x/_] edit seed  [Backspace] erase  [Enter] start simulation",
            )
            .style(theme::muted_text()),
            help,
        );
    }
}
