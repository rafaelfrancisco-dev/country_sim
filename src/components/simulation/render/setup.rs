use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::SetupSnapshot;

impl Simulation {
    pub(in crate::components::simulation) fn draw_setup(
        &self,
        frame: &mut Frame,
        area: Rect,
        setup: &SetupSnapshot,
        is_generating: bool,
    ) {
        let block = Block::default()
            .title(" Country Simulation Setup ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .style(Style::default().bg(theme::PANEL_BG))
            .title_style(theme::primary_text());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let saves_height = if setup.save_list.is_empty() {
            0
        } else {
            (setup.save_list.len().min(5) as u16) + 2
        };

        let [
            intro,
            countries,
            thousands,
            seed,
            save_name_area,
            saves_area,
            status,
            help,
        ] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(saves_height),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(inner);

        frame.render_widget(
            Paragraph::new("Configure world size. Inputs are constrained to 1-50 countries and 10-20000 thousand people per country. Countries will be assigned currencies from a built-in real-world pool.")
                .style(theme::primary_text())
                .wrap(Wrap { trim: true }),
            intro,
        );

        let active_style = Style::default()
            .fg(theme::TEXT_PRIMARY)
            .bg(theme::HIGHLIGHT_BG)
            .add_modifier(Modifier::BOLD);
        let inactive_style = theme::muted_text();

        let field_style = |f: crate::sim::setup::SetupField| {
            if setup.active_field == f {
                active_style
            } else {
                inactive_style
            }
        };
        let field_border = |f: crate::sim::setup::SetupField| {
            if setup.active_field == f {
                theme::active_border()
            } else {
                theme::neutral_border()
            }
        };

        let country_style = field_style(crate::sim::setup::SetupField::CountryCount);
        let thousand_style = field_style(crate::sim::setup::SetupField::ThousandsPerCountry);
        let seed_style = field_style(crate::sim::setup::SetupField::GenerationSeed);

        let country_text = if setup.country_count_input.is_empty() {
            "<enter number>".to_string()
        } else {
            setup.country_count_input.clone()
        };

        let thousand_text = if setup.thousands_per_country_input.is_empty() {
            "<enter number>".to_string()
        } else {
            setup.thousands_per_country_input.clone()
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
                        .border_style(field_border(crate::sim::setup::SetupField::CountryCount))
                        .title_style(theme::primary_text()),
                )
                .style(country_style),
            countries,
        );

        frame.render_widget(
            Paragraph::new(thousand_text)
                .block(
                    Block::default()
                        .title(" Thousands per Country ")
                        .borders(Borders::ALL)
                        .border_style(field_border(
                            crate::sim::setup::SetupField::ThousandsPerCountry,
                        ))
                        .title_style(theme::primary_text()),
                )
                .style(thousand_style),
            thousands,
        );

        frame.render_widget(
            Paragraph::new(seed_text)
                .block(
                    Block::default()
                        .title(" Generation Seed ")
                        .borders(Borders::ALL)
                        .border_style(field_border(crate::sim::setup::SetupField::GenerationSeed))
                        .title_style(theme::primary_text()),
                )
                .style(seed_style),
            seed,
        );

        // Save Name field
        let save_name_text = if setup.save_name_input.is_empty() {
            "<enter save name>".to_string()
        } else {
            setup.save_name_input.clone()
        };
        frame.render_widget(
            Paragraph::new(save_name_text)
                .block(
                    Block::default()
                        .title(" Save Name ")
                        .borders(Borders::ALL)
                        .border_style(field_border(crate::sim::setup::SetupField::SaveName))
                        .title_style(theme::primary_text()),
                )
                .style(field_style(crate::sim::setup::SetupField::SaveName)),
            save_name_area,
        );

        // Existing Saves list
        if !setup.save_list.is_empty() {
            let saves_focused = setup.active_field == crate::sim::setup::SetupField::ExistingSaves;
            let items: Vec<ListItem> = setup
                .save_list
                .iter()
                .enumerate()
                .map(|(i, desc)| {
                    let selected_marker = if i == setup.selected_save_row && saves_focused {
                        "▶ "
                    } else {
                        "  "
                    };
                    let label = format!(
                        "{selected_marker}{} (day {}, seed {:#x})",
                        desc.save_name, desc.world_day, desc.generation_seed
                    );
                    let style = if i == setup.selected_save_row {
                        if saves_focused {
                            active_style
                        } else {
                            Style::default().fg(theme::TEXT_PRIMARY)
                        }
                    } else {
                        inactive_style
                    };
                    ListItem::new(Span::styled(label, style))
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(setup.selected_save_row));

            frame.render_stateful_widget(
                List::new(items).block(
                    Block::default()
                        .title(" Existing Saves ")
                        .borders(Borders::ALL)
                        .border_style(field_border(crate::sim::setup::SetupField::ExistingSaves))
                        .title_style(theme::primary_text()),
                ),
                saves_area,
                &mut list_state,
            );
        }

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
        if let Some(persist_msg) = &setup.persistence_status {
            let color = if persist_msg.starts_with("Load error")
                || persist_msg.starts_with("Save error")
                || persist_msg.starts_with("Save failed")
            {
                Color::Red
            } else {
                Color::Green
            };
            status_lines.push(Line::styled(
                persist_msg.clone(),
                Style::default().fg(color),
            ));
        }
        if status_lines.is_empty() {
            let default_status = if is_generating {
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
                "Keys: [Tab/Shift+Tab] change field  [0-9] edit  [Backspace] erase  [Enter] start (or load when Existing Saves focused)  [w/s] navigate saves  [Shift+S] save while running",
            )
            .style(theme::muted_text())
            .wrap(Wrap { trim: true }),
            help,
        );
    }
}
