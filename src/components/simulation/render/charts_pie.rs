use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tui_piechart::{PieChart, PieSlice};

use super::super::{Simulation, theme};
use crate::components::simulation::snapshot::{PieChartData, PieChartsPageSnapshot, PieSliceData};

const COUNTRY_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::LightBlue,
    Color::LightRed,
    Color::LightGreen,
    Color::LightCyan,
];

impl Simulation {
    pub(super) fn draw_pie_charts_page(
        &self,
        frame: &mut Frame,
        body: Rect,
        pie_page: &PieChartsPageSnapshot,
        pie_page_idx: usize,
    ) {
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(body);
        let [a1, a2] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(top);
        let [a3, a4] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(bottom);
        let chart_areas = [a1, a2, a3, a4];
        let charts = match pie_page_idx {
            0 => [
                &pie_page.population_by_country,
                &pie_page.money_supply_by_country,
                &pie_page.stock_cap_by_country,
                &pie_page.treasury_by_country,
            ],
            _ => [
                &pie_page.education_tracks,
                &pie_page.employment_status,
                &pie_page.age_brackets,
                &pie_page.wealth_breakdown,
            ],
        };

        for (area, chart) in chart_areas.into_iter().zip(charts.into_iter()) {
            self.draw_single_pie_chart(frame, area, chart);
        }
    }

    fn draw_single_pie_chart(&self, frame: &mut Frame, area: Rect, data: &PieChartData) {
        let block = Block::default()
            .title(format!(" {} ", data.title))
            .borders(Borders::ALL)
            .border_style(theme::neutral_border())
            .title_style(theme::primary_text())
            .style(Style::default().bg(theme::PANEL_BG));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 2 || inner.height < 2 {
            return;
        }

        if data.slices.is_empty() {
            frame.render_widget(
                Paragraph::new("No data.")
                    .style(theme::muted_text().bg(theme::PANEL_BG))
                    .wrap(Wrap { trim: true }),
                inner,
            );
            return;
        }

        let slices = data
            .slices
            .iter()
            .enumerate()
            .map(|(idx, slice)| {
                PieSlice::new(
                    slice.label.as_str(),
                    slice.value,
                    Self::pie_slice_color(data, slice, idx),
                )
            })
            .collect::<Vec<_>>();
        let pie_chart = PieChart::new(slices)
            .style(theme::primary_text().bg(theme::PANEL_BG))
            .show_legend(true)
            .show_percentages(true);
        frame.render_widget(pie_chart, inner);
    }

    fn pie_slice_color(data: &PieChartData, slice: &PieSliceData, idx: usize) -> Color {
        match data.title.as_str() {
            "Education Track" => match slice.label.as_str() {
                "BasicOnly" => Color::Gray,
                "HighSchool" => Color::Cyan,
                "Bachelor" => Color::Green,
                "Doctor" => Color::LightRed,
                _ => COUNTRY_COLORS[idx % COUNTRY_COLORS.len()],
            },
            "Employment Status" => match slice.label.as_str() {
                "Unemployed" => Color::Red,
                "Employed@Company" => Color::Green,
                "Employed@Bank" => Color::Blue,
                _ => COUNTRY_COLORS[idx % COUNTRY_COLORS.len()],
            },
            "Age Brackets" => match slice.label.as_str() {
                "Children (<18)" => Color::Cyan,
                "Working Age (18-64)" => Color::Green,
                "Elderly (65+)" => Color::Gray,
                _ => COUNTRY_COLORS[idx % COUNTRY_COLORS.len()],
            },
            "Wealth Breakdown" => match slice.label.as_str() {
                "Household Cash" => Color::Green,
                "Company Cash" => Color::Yellow,
                "Treasury" => Color::Cyan,
                "Bank Reserve" => Color::Magenta,
                _ => COUNTRY_COLORS[idx % COUNTRY_COLORS.len()],
            },
            _ => COUNTRY_COLORS[idx % COUNTRY_COLORS.len()],
        }
    }
}
