use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use super::super::theme;
use crate::components::simulation::snapshot::{FxTabSnapshot, compact_num};

const RESERVE_COLUMN_SPACING: u16 = 1;
const MIN_RESERVE_COLUMN_WIDTH: usize = 18;
const RESERVE_COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::LightBlue,
    Color::LightGreen,
];

pub(super) fn draw_fx_summary(frame: &mut Frame, area: Rect, tab: &FxTabSnapshot) {
    let lines: Vec<Line<'_>> = tab
        .selected_fx_summary_lines
        .iter()
        .map(|line| Line::raw(line.as_str()))
        .collect();
    let block = Block::default()
        .title(" Selected FX Summary ")
        .borders(Borders::ALL)
        .border_style(theme::neutral_border())
        .title_style(theme::primary_text());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if tab.selected_reserves_breakdown.is_empty() {
        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::primary_text())
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let summary_height = u16::try_from(tab.selected_fx_summary_lines.len()).unwrap_or(u16::MAX);
    let [summary_area, reserves_area] =
        Layout::vertical([Constraint::Length(summary_height), Constraint::Min(0)]).areas(inner);

    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::primary_text())
            .wrap(Wrap { trim: false }),
        summary_area,
    );

    if reserves_area.height == 0 {
        return;
    }

    let column_count =
        reserve_summary_column_count(reserves_area.width, tab.selected_reserves_breakdown.len());
    let widths = vec![Constraint::Ratio(1, column_count as u32); column_count];
    let rows = tab
        .selected_reserves_breakdown
        .chunks(column_count)
        .enumerate()
        .map(|(row_idx, entries)| {
            let cells = entries
                .iter()
                .enumerate()
                .map(|(col_idx, entry)| {
                    let color =
                        RESERVE_COLORS[(row_idx * column_count + col_idx) % RESERVE_COLORS.len()];
                    Cell::from(Line::styled(
                        format!(
                            "{}: {} {}",
                            entry.currency_code,
                            compact_num(entry.amount_local),
                            entry.currency_symbol,
                        ),
                        Style::default().fg(color),
                    ))
                })
                .chain((entries.len()..column_count).map(|_| Cell::from("")));
            Row::new(cells)
        });

    frame.render_widget(
        Table::new(rows, widths).column_spacing(RESERVE_COLUMN_SPACING),
        reserves_area,
    );
}

pub(super) fn reserve_summary_row_count(summary_width: u16, entry_count: usize) -> usize {
    if entry_count == 0 {
        return 0;
    }

    let columns = reserve_summary_column_count(summary_width, entry_count);
    entry_count.div_ceil(columns)
}

fn reserve_summary_column_count(summary_width: u16, entry_count: usize) -> usize {
    if entry_count == 0 {
        return 1;
    }

    let inner_width = summary_width.saturating_sub(2) as usize;
    let spacing = RESERVE_COLUMN_SPACING as usize;
    let max_columns = ((inner_width + spacing) / (MIN_RESERVE_COLUMN_WIDTH + spacing)).max(1);
    max_columns.min(entry_count)
}
