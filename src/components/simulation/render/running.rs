use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

use super::super::{Simulation, theme};
use crate::sim::world::{DetailsView, RunningTab};

impl Simulation {
    pub(in crate::components::simulation) fn draw_running(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let block = Block::default()
            .title(" Country Simulation ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .style(Style::default().bg(theme::PANEL_BG))
            .title_style(theme::primary_text());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .areas(inner);
        let [stats, tabs_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Length(2)]).areas(header);

        let selected_country_name = world
            .selected_country()
            .map(|country| country.name.as_str())
            .unwrap_or("N/A");

        let run_state = if world.paused { "Paused" } else { "Running" };
        let selected_country_month = world.selected_country().map_or(0, |country| country.month);
        let state_style = if world.paused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        };
        let header_line = Line::from(vec![
            Span::styled("Day: ", theme::muted_text()),
            Span::styled(world.day.to_string(), theme::primary_text()),
            Span::styled(" | Month: ", theme::muted_text()),
            Span::styled(selected_country_month.to_string(), theme::primary_text()),
            Span::styled(" | State: ", theme::muted_text()),
            Span::styled(run_state, state_style),
            Span::styled(" | Selected Country: ", theme::muted_text()),
            Span::styled(selected_country_name, theme::primary_text()),
            Span::styled(" | Total Citizens: ", theme::muted_text()),
            Span::styled(world.total_population().to_string(), theme::primary_text()),
        ]);
        frame.render_widget(Paragraph::new(header_line), stats);
        self.draw_running_tabs(frame, tabs_area, world.active_tab);

        let footer_text = match world.active_tab {
            RunningTab::Main => self.draw_main_tab(frame, body),
            RunningTab::Charts => self.draw_charts_tab(frame, body),
        };

        frame.render_widget(
            Paragraph::new(footer_text)
                .style(theme::muted_text())
                .wrap(Wrap { trim: true }),
            footer,
        );
    }

    fn draw_running_tabs(&self, frame: &mut Frame, area: Rect, active_tab: RunningTab) {
        let selected = match active_tab {
            RunningTab::Main => 0,
            RunningTab::Charts => 1,
        };

        let tabs = Tabs::new(vec!["Main", "Charts"])
            .select(selected)
            .highlight_style(
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .bg(theme::HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD),
            )
            .style(theme::muted_text())
            .block(
                Block::default()
                    .title(" Running Tabs ")
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text()),
            );
        frame.render_widget(tabs, area);
    }

    fn draw_main_tab(&self, frame: &mut Frame, body: Rect) -> String {
        let Some(world) = self.engine.world() else {
            return String::new();
        };

        match world.details_view {
            DetailsView::List => {
                let [countries_area, people_area] =
                    Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .areas(body);

                self.draw_countries_table(frame, countries_area);
                self.draw_people_table(frame, people_area);

                format!(
                    "Controls: [Tab/Shift+Tab] switch tab  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [↑/↓] move row (focused list)  [j/k] page (focused list)  [Enter] details (focused row)  [q] quit | Country Page {}/{} | People Page {}/{}",
                    world.country_page() + 1,
                    world.total_country_pages(),
                    world.people_page + 1,
                    world.total_people_pages(),
                )
            }
            DetailsView::Person => {
                self.draw_person_details(frame, body);
                "Controls: [Tab/Shift+Tab] switch tab  [Esc] back to people list  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [j/k] page (focused list)  [q] quit".to_string()
            }
            DetailsView::Country => {
                self.draw_country_details(frame, body);
                "Controls: [Tab/Shift+Tab] switch tab  [Esc] back to lists  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [j/k] page (focused list)  [q] quit".to_string()
            }
        }
    }
}
