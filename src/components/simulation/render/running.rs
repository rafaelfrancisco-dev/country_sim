use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::sync::Arc;

use super::super::{Simulation, theme};
use super::events::event_search_hint;
use crate::{
    components::simulation::hotkeys,
    components::simulation::snapshot::{
        BanksTabSnapshot, GoodsTabSnapshot, MainTabSnapshot, RunningSnapshot, RunningTabSnapshot,
    },
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

        let mut footer_text = match (world.active_tab, &world.tab) {
            (RunningTab::Main, RunningTabSnapshot::Main(main_tab)) => {
                self.draw_main_tab(frame, body, world, main_tab)
            }
            (RunningTab::Goods, RunningTabSnapshot::Goods(goods_tab)) => {
                self.draw_goods_tab(frame, body, world, goods_tab)
            }
            (RunningTab::Banks, RunningTabSnapshot::Banks(banks_tab)) => {
                self.draw_banks_tab(frame, body, world, banks_tab)
            }
            (RunningTab::Stocks, RunningTabSnapshot::Stocks(stocks_tab)) => {
                self.draw_stocks_tab(frame, body, world, stocks_tab)
            }
            (RunningTab::Charts, RunningTabSnapshot::Charts(charts_tab)) => {
                self.draw_charts_tab(frame, body, world, charts_tab)
            }
            (RunningTab::Events, RunningTabSnapshot::Events(events_tab)) => {
                self.draw_events_tab(frame, body, world, events_tab)
            }
            _ => "Snapshot/render tab mismatch".to_string(),
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

    fn draw_main_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
        tab: &MainTabSnapshot,
    ) -> String {
        match world.details_view {
            DetailsView::List => {
                let [countries_area, people_area] =
                    Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .areas(body);

                self.draw_countries_table(frame, countries_area, world, tab);
                self.draw_people_table(frame, people_area, world, tab);

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
                self.draw_person_details(frame, body, world, tab);
                if world.details_page == 0 {
                    format!(
                        "Controls: {}  [Esc] back to people list  [PgUp/PgDn] details page (1/2)  [c] open employer company",
                        hotkeys::GLOBAL_RUNNING_HINT
                    )
                } else {
                    format!(
                        "Controls: {}  [Esc] back to people list  [PgUp/PgDn] details page (2/2)  [w/s] scroll  [{}]  [o] toggle sort  [f] category | Events: {} | Sort: {} | Category: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        event_search_hint(
                            world.events_search_active,
                            &tab.person_related_events.events_search_query
                        ),
                        tab.person_related_events.events_total_count,
                        tab.person_related_events.events_sort_label,
                        tab.person_related_events.events_category_label,
                    )
                }
            }
            DetailsView::Country => {
                self.draw_country_details(frame, body, world, tab);
                format!(
                    "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page  [o] policy overrides",
                    hotkeys::GLOBAL_RUNNING_HINT
                )
            }
            DetailsView::CountryPolicy => {
                self.draw_country_policy_details(frame, body, tab);
                if world.country_policy_is_editing {
                    format!(
                        "Controls: {}  [Enter] save field  [Esc] cancel edit",
                        hotkeys::GLOBAL_RUNNING_HINT
                    )
                } else {
                    format!(
                        "Controls: {}  [w/s] field  [Enter] edit field  [r] reset managed values  [Esc] back to country details",
                        hotkeys::GLOBAL_RUNNING_HINT
                    )
                }
            }
            DetailsView::Company => {
                let empty_goods = GoodsTabSnapshot {
                    company_rows: Vec::new(),
                    company_details_lines: Vec::new(),
                    company_worker_rows: Vec::new(),
                    company_related_events:
                        crate::components::simulation::snapshot::EventsTabSnapshot {
                            event_log_lines: Vec::new(),
                            events_total_count: 0,
                            events_scroll: 0,
                            events_sort_label: tab.person_related_events.events_sort_label,
                            events_category_label: tab.person_related_events.events_category_label,
                            events_search_query: tab
                                .person_related_events
                                .events_search_query
                                .clone(),
                        },
                    company_sold_history: Arc::default(),
                    company_employee_history: Arc::default(),
                    company_price_history: Arc::default(),
                    company_loan_payment_history: Arc::default(),
                    company_money_history: Arc::default(),
                    country_goods_summary_lines: Vec::new(),
                };
                self.draw_company_details(frame, body, world, &empty_goods);
                match world.details_page {
                    0 => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (1/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    1 => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (2/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    2 => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (3/4)  [w/s] scroll  [{}]  [o] toggle sort  [f] category | Events: {} | Sort: {} | Category: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        event_search_hint(
                            world.events_search_active,
                            &tab.person_related_events.events_search_query
                        ),
                        0,
                        tab.person_related_events.events_sort_label,
                        tab.person_related_events.events_category_label,
                    ),
                    _ => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (4/4)  [w/s] scroll workers | Employees: 0",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                }
            }
            DetailsView::Bank => {
                let empty_banks = BanksTabSnapshot {
                    bank_rows: Vec::new(),
                    bank_details_lines: Vec::new(),
                    bank_related_events:
                        crate::components::simulation::snapshot::EventsTabSnapshot {
                            event_log_lines: Vec::new(),
                            events_total_count: 0,
                            events_scroll: 0,
                            events_sort_label: tab.person_related_events.events_sort_label,
                            events_category_label: tab.person_related_events.events_category_label,
                            events_search_query: tab
                                .person_related_events
                                .events_search_query
                                .clone(),
                        },
                    bank_depositor_rows: Vec::new(),
                    bank_reserve_history: Arc::default(),
                    bank_deposit_history: Arc::default(),
                    bank_created_money_history: Arc::default(),
                    bank_equity_history: Arc::default(),
                    bank_run_intensity_history: Arc::default(),
                    country_banks_summary_lines: Vec::new(),
                };
                self.draw_bank_details(frame, body, world, &empty_banks);
                match world.details_page {
                    0 => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (1/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    1 => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (2/4)",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                    2 => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (3/4)  [w/s] scroll  [{}]  [o] toggle sort  [f] category | Events: {} | Sort: {} | Category: {}",
                        hotkeys::GLOBAL_RUNNING_HINT,
                        event_search_hint(
                            world.events_search_active,
                            &tab.person_related_events.events_search_query
                        ),
                        0,
                        tab.person_related_events.events_sort_label,
                        tab.person_related_events.events_category_label,
                    ),
                    _ => format!(
                        "Controls: {}  [Esc] back to lists  [PgUp/PgDn] details page (4/4)  [w/s] scroll depositors | Depositors: 0",
                        hotkeys::GLOBAL_RUNNING_HINT
                    ),
                }
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
            RunningTab::Banks => "Banks",
            RunningTab::Stocks => "Stocks",
            RunningTab::Charts => "Charts",
            RunningTab::Events => "Events",
        }
    }
}
