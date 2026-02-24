use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::RunningSnapshot;

impl Simulation {
    pub(super) fn draw_person_details(&self, frame: &mut Frame, area: Rect, world: &RunningSnapshot) {
        let details = world
            .person_details_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Person Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    pub(super) fn draw_country_details(&self, frame: &mut Frame, area: Rect, world: &RunningSnapshot) {
        let details = world
            .country_details_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Country Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }
}
