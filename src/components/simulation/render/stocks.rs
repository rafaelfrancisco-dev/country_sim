use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
};

use super::super::{Simulation, theme};
use super::disaster_markers;
use crate::components::simulation::{
    hotkeys,
    snapshot::{RunningSnapshot, StocksTabSnapshot},
};

struct StockChart<'a> {
    title: &'a str,
    data: &'a [f64],
    color: Color,
    as_money: bool,
}

struct StockChartContext<'a> {
    world_day: u64,
    disaster_days: &'a [u64],
    currency_symbol: &'a str,
}

impl Simulation {
    pub(in crate::components::simulation) fn draw_stocks_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
        tab: &StocksTabSnapshot,
    ) -> String {
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(body);

        let [summary_area, table_area] =
            Layout::horizontal([Constraint::Length(36), Constraint::Min(0)]).areas(top);

        self.draw_stock_summary(frame, summary_area, tab);
        self.draw_stock_table(frame, table_area, tab);
        self.draw_stock_charts(
            frame,
            bottom,
            tab,
            world.day,
            world.currency_symbol.as_str(),
        );

        format!(
            "Controls: {}  [,/.] cycle country  [a/d] focus country/table  [w/s] move row  [PgUp/PgDn] page | Listed: {}",
            hotkeys::GLOBAL_RUNNING_HINT,
            tab.stock_rows.len()
        )
    }

    fn draw_stock_summary(&self, frame: &mut Frame, area: Rect, tab: &StocksTabSnapshot) {
        let block = Block::default()
            .title(" Market Summary ")
            .borders(Borders::ALL)
            .border_style(theme::neutral_border());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<Line> = tab
            .stock_summary_lines
            .iter()
            .map(|l| Line::from(Span::styled(l.as_str(), theme::primary_text())))
            .collect();
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn draw_stock_table(&self, frame: &mut Frame, area: Rect, tab: &StocksTabSnapshot) {
        let total_rows = tab.stock_rows.len();
        let visible_height = area.height.saturating_sub(3) as usize;
        let selected_idx = tab
            .stock_rows
            .iter()
            .position(|r| r.is_selected)
            .unwrap_or(0);
        let view_scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));

        let start = if total_rows == 0 { 0 } else { view_scroll + 1 };
        let end = (view_scroll + visible_height).min(total_rows);
        let title = if total_rows == 0 {
            " Listed Companies ".to_string()
        } else {
            format!(" Listed Companies ({}-{} of {}) ", start, end, total_rows)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme::neutral_border());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if tab.stock_rows.is_empty() {
            frame.render_widget(
                Paragraph::new("No listed companies yet. IPOs occur when distressed companies cannot secure debt financing.")
                    .style(theme::muted_text())
                    .wrap(Wrap { trim: true }),
                inner,
            );
            return;
        }

        let header = Row::new(vec![
            "Ticker", "Company", "Price", "Day%", "Mkt Cap", "Vol", "Float", "Bank%",
        ])
        .style(
            Style::default()
                .fg(theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        );

        let rows: Vec<Row> = tab
            .stock_rows
            .iter()
            .skip(view_scroll)
            .take(visible_height)
            .map(|r| {
                let day_style = if r.day_pct.starts_with('+') {
                    Style::default().fg(Color::Green)
                } else if r.day_pct.starts_with('-') {
                    Style::default().fg(Color::Red)
                } else {
                    theme::primary_text()
                };

                let row_style = if r.is_selected {
                    Style::default()
                        .bg(theme::HIGHLIGHT_BG)
                        .fg(theme::TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    ratatui::text::Text::from(r.ticker.as_str()),
                    ratatui::text::Text::from(r.company_name.as_str()),
                    ratatui::text::Text::from(r.last_price.as_str()),
                    ratatui::text::Text::styled(r.day_pct.as_str(), day_style),
                    ratatui::text::Text::from(r.market_cap.as_str()),
                    ratatui::text::Text::from(r.volume.as_str()),
                    ratatui::text::Text::from(r.float_shares.as_str()),
                    ratatui::text::Text::from(r.bank_pct.as_str()),
                ])
                .style(row_style)
            })
            .collect();

        let widths = [
            Constraint::Length(6),
            Constraint::Min(12),
            Constraint::Length(8),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(6),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default());
        frame.render_widget(table, inner);

        if total_rows > visible_height {
            let mut sb_state = ScrollbarState::new(total_rows.saturating_sub(visible_height))
                .position(view_scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut sb_state,
            );
        }
    }

    fn draw_stock_charts(
        &self,
        frame: &mut Frame,
        area: Rect,
        tab: &StocksTabSnapshot,
        world_day: u64,
        currency_symbol: &str,
    ) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);

        let [price_area, volume_area] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(left);

        let [index_area, bank_area] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(right);

        let context = StockChartContext {
            world_day,
            disaster_days: tab.disaster_days.as_ref(),
            currency_symbol,
        };

        self.draw_stock_chart_f64(
            frame,
            price_area,
            StockChart {
                title: " Selected Stock Price ",
                data: tab.stock_selected_index_price_history.as_ref(),
                color: Color::Cyan,
                as_money: true,
            },
            &context,
        );
        self.draw_stock_chart_f64(
            frame,
            volume_area,
            StockChart {
                title: " Selected Stock Volume ",
                data: tab.stock_selected_index_volume_history.as_ref(),
                color: Color::Yellow,
                as_money: false,
            },
            &context,
        );
        self.draw_stock_chart_f64(
            frame,
            index_area,
            StockChart {
                title: " Country Stock Index ",
                data: tab.stock_country_index_history.as_ref(),
                color: Color::Green,
                as_money: false,
            },
            &context,
        );
        self.draw_stock_chart_f64(
            frame,
            bank_area,
            StockChart {
                title: " Bank Equity Exposure ",
                data: tab.stock_bank_exposure_history.as_ref(),
                color: Color::Magenta,
                as_money: true,
            },
            &context,
        );
    }

    fn draw_stock_chart_f64(
        &self,
        frame: &mut Frame,
        area: Rect,
        chart: StockChart<'_>,
        context: &StockChartContext<'_>,
    ) {
        let StockChart {
            title,
            data,
            color,
            as_money,
        } = chart;
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme::neutral_border());

        if data.is_empty() {
            frame.render_widget(
                Paragraph::new("No data yet")
                    .style(theme::muted_text())
                    .block(block),
                area,
            );
            return;
        }

        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let padding = ((max - min) * 0.1).max(1.0);
        let y_min = (min - padding).max(0.0);
        let y_max = max + padding;

        let points: Vec<(f64, f64)> = data
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect();

        let dataset = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(color))
            .data(&points);
        let start_day = context
            .world_day
            .saturating_sub(data.len() as u64)
            .saturating_add(1);
        let marker_positions =
            disaster_markers::disaster_relative_day_positions(context.disaster_days, start_day);
        let marker_segments = disaster_markers::marker_segments(
            &marker_positions,
            0.0,
            data.len() as f64,
            y_min,
            y_max,
        );
        let mut datasets = Vec::with_capacity(1 + marker_segments.len());
        datasets.push(dataset);
        datasets.extend(disaster_markers::marker_datasets(&marker_segments));

        let x_labels = vec![
            ratatui::text::Span::raw("0"),
            ratatui::text::Span::raw(data.len().to_string()),
        ];
        let y_labels = if as_money {
            vec![
                ratatui::text::Span::raw(Self::money(y_min, context.currency_symbol)),
                ratatui::text::Span::raw(Self::money(y_max, context.currency_symbol)),
            ]
        } else {
            vec![
                ratatui::text::Span::raw(if y_min.abs() >= 10_000.0 {
                    Self::compact_num(y_min)
                } else {
                    format!("{y_min:.0}")
                }),
                ratatui::text::Span::raw(if y_max.abs() >= 10_000.0 {
                    Self::compact_num(y_max)
                } else {
                    format!("{y_max:.0}")
                }),
            ]
        };

        let chart = Chart::new(datasets)
            .block(block)
            .x_axis(
                Axis::default()
                    .bounds([0.0, data.len() as f64])
                    .labels(x_labels),
            )
            .y_axis(Axis::default().bounds([y_min, y_max]).labels(y_labels));

        frame.render_widget(chart, area);
    }
}
