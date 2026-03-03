use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::RunningSnapshot;

pub(super) fn paginate_detail_lines<'a>(
    lines: &'a [String],
    requested_page: usize,
    area: Rect,
) -> (&'a [String], usize, usize) {
    if lines.is_empty() {
        return (&lines[0..0], 0, 1);
    }

    let page_size = usize::from(area.height.saturating_sub(2).max(1));
    let total_pages = lines.len().div_ceil(page_size).max(1);
    let page_idx = requested_page.min(total_pages.saturating_sub(1));
    let start = page_idx.saturating_mul(page_size);
    let end = (start + page_size).min(lines.len());

    (&lines[start..end], page_idx, total_pages)
}

impl Simulation {
    pub(super) fn draw_person_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
    ) {
        let (page_lines, page_idx, total_pages) =
            paginate_detail_lines(&world.person_details_lines, world.details_page, area);
        let details = page_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(format!(
                            " Person Details (Page {}/{}) ",
                            page_idx + 1,
                            total_pages
                        ))
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    pub(super) fn draw_country_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
    ) {
        let (page_lines, page_idx, total_pages) =
            paginate_detail_lines(&world.country_details_lines, world.details_page, area);
        let details = page_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(format!(
                            " Country Details (Page {}/{}) ",
                            page_idx + 1,
                            total_pages
                        ))
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    pub(super) fn draw_country_policy_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
    ) {
        let details = world
            .country_policy_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Country Policy Overrides ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginate_detail_lines_slices_and_clamps_requested_page() {
        let lines = (0..9).map(|idx| format!("line-{idx}")).collect::<Vec<_>>();
        let area = Rect::new(0, 0, 80, 6);
        let (slice, page_idx, total_pages) = paginate_detail_lines(&lines, 7, area);

        assert_eq!(page_idx, 2);
        assert_eq!(total_pages, 3);
        assert_eq!(slice, &lines[8..9]);
    }

    #[test]
    fn paginate_detail_lines_handles_empty_lines() {
        let lines = Vec::<String>::new();
        let area = Rect::new(0, 0, 80, 6);
        let (slice, page_idx, total_pages) = paginate_detail_lines(&lines, 0, area);

        assert!(slice.is_empty());
        assert_eq!(page_idx, 0);
        assert_eq!(total_pages, 1);
    }
}
