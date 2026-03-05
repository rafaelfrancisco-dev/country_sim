use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tui_checkbox::Checkbox;

use super::super::{Simulation, theme};
use crate::components::simulation::{
    hotkeys,
    snapshot::{DisasterRowSnapshot, DisastersTabSnapshot, RunningSnapshot},
};

impl Simulation {
    pub(in crate::components::simulation) fn draw_disasters_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        _world: &RunningSnapshot,
        tab: &DisastersTabSnapshot,
    ) -> String {
        let [summary_area, rows_area, status_area] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(body);

        self.draw_disaster_summary(frame, summary_area, tab);
        self.draw_disaster_rows(frame, rows_area, tab);
        self.draw_disaster_status(frame, status_area, tab);

        format!(
            "Controls: {}  [,/.] change country  [w/s] select disaster  [a/d] intensity  [x] toggle enabled  [Enter] apply",
            hotkeys::GLOBAL_RUNNING_HINT
        )
    }

    fn draw_disaster_summary(&self, frame: &mut Frame, area: Rect, tab: &DisastersTabSnapshot) {
        let lines = vec![
            Line::from(vec![
                Span::styled("Country: ", theme::muted_text()),
                Span::styled(tab.selected_country_name.as_str(), theme::primary_text()),
            ]),
            Line::from(vec![
                Span::styled("Population: ", theme::muted_text()),
                Span::styled(tab.population.as_str(), theme::primary_text()),
            ]),
            Line::from(vec![
                Span::styled("Companies: ", theme::muted_text()),
                Span::styled(tab.companies.as_str(), theme::primary_text()),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title(" Disaster Scope ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                )
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn draw_disaster_rows(&self, frame: &mut Frame, area: Rect, tab: &DisastersTabSnapshot) {
        let block = Block::default()
            .title(" Disaster Controls ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .title_style(theme::primary_text());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if tab.rows.is_empty() {
            frame.render_widget(
                Paragraph::new("No disasters configured.")
                    .style(theme::muted_text())
                    .wrap(Wrap { trim: true }),
                inner,
            );
            return;
        }

        let row_constraints: Vec<Constraint> =
            tab.rows.iter().map(|_| Constraint::Length(2)).collect();
        let row_areas = Layout::vertical(row_constraints).split(inner);
        for (row_area, row) in row_areas.into_iter().zip(tab.rows.iter()) {
            self.draw_disaster_row(frame, *row_area, row);
        }
    }

    fn draw_disaster_row(&self, frame: &mut Frame, area: Rect, row: &DisasterRowSnapshot) {
        let [name_area, i1, i2, i3, i4] = Layout::horizontal([
            Constraint::Length(44),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
        ])
        .areas(area);

        let selected_style = if row.is_selected {
            Style::default()
                .bg(theme::HIGHLIGHT_BG)
                .fg(theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::primary_text()
        };
        let checkbox_style = if row.enabled {
            Style::default().fg(Color::LightGreen)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        frame.render_widget(
            Checkbox::new(row.label.as_str(), row.enabled)
                .style(selected_style)
                .checkbox_style(checkbox_style)
                .checked_symbol("[X] ")
                .unchecked_symbol("[ ] "),
            name_area,
        );

        let intensity_areas = [i1, i2, i3, i4];
        for (idx, intensity_area) in intensity_areas.into_iter().enumerate() {
            let pct = row.intensity_options[idx];
            let is_checked = idx == row.selected_intensity_idx;
            let intensity_style = if row.is_selected && is_checked {
                Style::default()
                    .bg(theme::HIGHLIGHT_BG)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_checked {
                Style::default().fg(Color::Yellow)
            } else if row.is_selected {
                Style::default()
                    .bg(theme::HIGHLIGHT_BG)
                    .fg(theme::TEXT_PRIMARY)
            } else {
                theme::primary_text()
            };

            frame.render_widget(
                Checkbox::new(format!("{pct}%"), is_checked)
                    .style(intensity_style)
                    .checked_symbol("(*) ")
                    .unchecked_symbol("( ) "),
                intensity_area,
            );
        }
    }

    fn draw_disaster_status(&self, frame: &mut Frame, area: Rect, tab: &DisastersTabSnapshot) {
        let status = tab
            .last_apply_status
            .as_deref()
            .unwrap_or("Select disasters and press Enter to apply.");
        frame.render_widget(
            Paragraph::new(status)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Apply Status ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }
}
