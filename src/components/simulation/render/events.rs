use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::super::{Simulation, theme};
use crate::components::simulation::{hotkeys, snapshot::RunningSnapshot};

impl Simulation {
    pub(in crate::components::simulation) fn draw_events_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
    ) -> String {
        let [list_area, search_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(body);

        let total = world.event_log_lines.len();
        let max_scroll = total.saturating_sub(1);
        let scroll = world.events_scroll.min(max_scroll);
        let list_height = list_area.height.saturating_sub(2) as usize;

        let visible: Vec<ListItem> = world
            .event_log_lines
            .iter()
            .skip(scroll)
            .take(list_height)
            .map(|line| {
                let style = event_line_style(line);
                ListItem::new(Line::from(Span::styled(line.as_str(), style)))
            })
            .collect();

        let list_title = if total == 0 {
            " Events (none) ".to_string()
        } else {
            let end = (scroll + visible.len()).min(total);
            format!(" Events ({}-{} of {}) ", scroll + 1, end, total)
        };

        let list_block = Block::default()
            .title(list_title)
            .borders(Borders::ALL)
            .border_style(theme::neutral_border());

        let list = List::new(visible).block(list_block);
        frame.render_widget(list, list_area);

        let search_border_style = if world.events_search_active {
            Style::default().fg(Color::Yellow)
        } else {
            theme::neutral_border()
        };

        let search_display = if world.events_search_active {
            format!("{}_", world.events_search_query)
        } else if world.events_search_query.is_empty() {
            "Press [/] to search".to_string()
        } else {
            format!("Filter: {}", world.events_search_query)
        };

        let search_block = Block::default()
            .title(" Search ")
            .borders(Borders::ALL)
            .border_style(search_border_style);

        frame.render_widget(
            Paragraph::new(search_display).block(search_block),
            search_area,
        );

        let search_hint = if world.events_search_active {
            "Esc: close search  Ctrl+u: clear query"
        } else if !world.events_search_query.is_empty() {
            "/: edit filter"
        } else {
            "/: search"
        };

        format!(
            "Controls: {}  [w/s] scroll  [{}]  [o] toggle sort | Events: {} | Sort: {}",
            hotkeys::GLOBAL_RUNNING_HINT,
            search_hint,
            world.events_total_count,
            world.events_sort_label,
        )
    }
}

fn event_line_style(line: &str) -> Style {
    if line.starts_with("[BIRTH]") {
        Style::default().fg(Color::Green)
    } else if line.starts_with("[DEATH]") || line.starts_with("[LOSS]") {
        Style::default().fg(Color::Red)
    } else if line.starts_with("[BANKR]") {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if line.starts_with("[LOAN+]") {
        Style::default().fg(Color::Yellow)
    } else if line.starts_with("[LOAN=]") {
        Style::default().fg(Color::Cyan)
    } else if line.starts_with("[DEBT!]") {
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("[FOUND]") {
        Style::default().fg(Color::LightGreen)
    } else if line.starts_with("[WAGE-]") {
        Style::default().fg(Color::Yellow)
    } else if line.starts_with("[LAYOF]") {
        Style::default().fg(Color::LightRed)
    } else if line.starts_with("[SWITCH]") {
        Style::default().fg(Color::Blue)
    } else if line.starts_with("[IPO]") {
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("[DIV]") {
        Style::default().fg(Color::Green)
    } else if line.starts_with("[MARGN]") {
        Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("[SEIZE]") || line.starts_with("[DELIST]") {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if line.starts_with("[STK]") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}
