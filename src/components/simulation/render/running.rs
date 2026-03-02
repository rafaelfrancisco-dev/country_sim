use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::{
    components::simulation::hotkeys,
    components::simulation::snapshot::RunningSnapshot,
    components::simulation::ui_state::{DetailsView, RunningTab},
    seed::format_generation_seed,
    sim::SimulationMode,
};

const HEADER_MAX_LINES: usize = 4;
const HEADER_SEPARATOR: &str = " | ";

struct HeaderChunk {
    spans: Vec<Span<'static>>,
    width: usize,
}

impl Simulation {
    pub(in crate::components::simulation) fn draw_running(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
    ) {
        let block = Block::default()
            .title(" Country Simulation ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .style(Style::default().bg(theme::PANEL_BG))
            .title_style(theme::primary_text());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let header_lines = self.pack_running_header(world, inner.width, HEADER_MAX_LINES);
        let header_height = header_lines.len().clamp(1, HEADER_MAX_LINES) as u16;

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .areas(inner);

        frame.render_widget(Paragraph::new(header_lines), header);

        let mut footer_text = match world.active_tab {
            RunningTab::Main => self.draw_main_tab(frame, body, world),
            RunningTab::Goods => self.draw_goods_tab(frame, body, world),
            RunningTab::Stocks => self.draw_stocks_tab(frame, body, world),
            RunningTab::Charts => self.draw_charts_tab(frame, body, world),
            RunningTab::Events => self.draw_events_tab(frame, body, world),
        };
        if !world.invariant_warnings.is_empty() {
            footer_text.push_str(&format!(
                " | Invariant warnings: {}",
                world.invariant_warnings.len()
            ));
        }

        frame.render_widget(
            Paragraph::new(footer_text)
                .style(theme::muted_text())
                .wrap(Wrap { trim: true }),
            footer,
        );
    }

    fn draw_main_tab(&self, frame: &mut Frame, body: Rect, world: &RunningSnapshot) -> String {
        match world.details_view {
            DetailsView::List => {
                let [countries_area, people_area] =
                    Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .areas(body);

                self.draw_countries_table(frame, countries_area, world);
                self.draw_people_table(frame, people_area, world);

                format!(
                    "Controls: {}  [a/d] focus country/people  [w/s] move row (focused list)  [PgUp/PgDn] page (focused list)  [Enter] details (focused row) | Country Page {}/{} | People Page {}/{}",
                    hotkeys::GLOBAL_RUNNING_HINT,
                    world.country_page + 1,
                    world.total_country_pages,
                    world.people_page + 1,
                    world.total_people_pages,
                )
            }
            DetailsView::Person => {
                self.draw_person_details(frame, body, world);
                format!(
                    "Controls: {}  [Esc] back to people list  [c] open employer company",
                    hotkeys::GLOBAL_RUNNING_HINT
                )
            }
            DetailsView::Country => {
                self.draw_country_details(frame, body, world);
                format!(
                    "Controls: {}  [Esc] back to lists",
                    hotkeys::GLOBAL_RUNNING_HINT
                )
            }
            DetailsView::Company => {
                self.draw_company_details(frame, body, world);
                format!(
                    "Controls: {}  [Esc] back to lists",
                    hotkeys::GLOBAL_RUNNING_HINT
                )
            }
        }
    }

    fn pack_running_header(
        &self,
        world: &RunningSnapshot,
        max_width: u16,
        max_lines: usize,
    ) -> Vec<Line<'static>> {
        let mut chunks = self.running_header_chunks(world).into_iter().peekable();
        let mut lines: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
        let limit = usize::from(max_width.max(1));
        let separator = Span::styled(HEADER_SEPARATOR, theme::muted_text());
        let separator_width = HEADER_SEPARATOR.len();

        for _ in 0..max_lines {
            let Some(first) = chunks.next() else {
                break;
            };
            let mut line_spans = first.spans;
            let mut line_width = first.width;

            while let Some(next) = chunks.next_if(|chunk| {
                line_width
                    .saturating_add(separator_width)
                    .saturating_add(chunk.width)
                    <= limit
            }) {
                line_spans.push(separator.clone());
                line_spans.extend(next.spans);
                line_width = line_width
                    .saturating_add(separator_width)
                    .saturating_add(next.width);
            }

            lines.push((line_spans, line_width));
        }

        if chunks.next().is_some()
            && let Some((spans, _)) = lines.last_mut()
        {
            spans.push(Span::styled(" ...", theme::muted_text()));
        }

        if lines.is_empty() {
            lines.push((
                vec![Span::styled("No running data", theme::muted_text())],
                15,
            ));
        }

        lines
            .into_iter()
            .map(|(spans, _)| Line::from(spans))
            .collect()
    }

    fn running_header_chunks(&self, world: &RunningSnapshot) -> Vec<HeaderChunk> {
        let run_state = if world.paused { "Paused" } else { "Running" };
        let state_style = if world.paused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        };
        let sim_style = match world.sim_mode {
            SimulationMode::Realtime => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            SimulationMode::MaxSpeed => Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        };

        vec![
            Self::header_chunk("Day", world.day.to_string(), theme::primary_text()),
            Self::header_chunk(
                "Month",
                world.selected_country_month.to_string(),
                theme::primary_text(),
            ),
            Self::header_chunk("State", run_state.to_string(), state_style),
            Self::header_chunk("Sim", world.sim_mode.label().to_string(), sim_style),
            Self::header_chunk(
                "View",
                self.active_tab_label(world.active_tab).to_string(),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Tick/s",
                format!("{:.1}", world.ticks_per_second),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Avg Tick (ms)",
                format!("{:.2}", world.avg_tick_duration_ms),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Queue",
                world.queue_depth.to_string(),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Lag (ms)",
                format!("{:.2}", world.tick_queue_lag_ms),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Publishes",
                world.snapshot_publish_count.to_string(),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Snap Build (ms)",
                format!("{:.2}", world.avg_snapshot_build_ms),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Max Snap Age (ms)",
                format!("{:.2}", world.max_snapshot_age_ms),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Selected Country",
                world.selected_country_name.clone(),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Total Citizens",
                world.total_population.clone(),
                theme::primary_text(),
            ),
            Self::header_chunk(
                "Seed",
                format_generation_seed(world.generation_seed),
                theme::primary_text(),
            ),
        ]
    }

    fn header_chunk(label: &str, value: String, value_style: Style) -> HeaderChunk {
        let label_string = format!("{label}: ");
        let width = label_string.len().saturating_add(value.len());
        HeaderChunk {
            spans: vec![
                Span::styled(label_string, theme::muted_text()),
                Span::styled(value, value_style),
            ],
            width,
        }
    }

    fn active_tab_label(&self, active_tab: RunningTab) -> &'static str {
        match active_tab {
            RunningTab::Main => "Main",
            RunningTab::Goods => "Goods",
            RunningTab::Stocks => "Stocks",
            RunningTab::Charts => "Charts",
            RunningTab::Events => "Events",
        }
    }
}
