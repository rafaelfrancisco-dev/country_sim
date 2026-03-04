use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::components::simulation::{
    hotkeys,
    snapshot::{ChartsTabSnapshot, RunningSnapshot},
};
use crate::sim::world::MetricPoint;

#[derive(Debug, Copy, Clone)]
enum AxisValueFormat {
    Money,
    Percent,
    Index,
    Count,
}

#[derive(Debug, Copy, Clone)]
struct ChartSpec {
    title: &'static str,
    color: Color,
    value_fn: fn(&MetricPoint) -> f64,
    axis_format: AxisValueFormat,
}

impl Simulation {
    pub(super) fn draw_charts_tab(
        &self,
        frame: &mut Frame,
        body: Rect,
        world: &RunningSnapshot,
        tab: &ChartsTabSnapshot,
    ) -> String {
        let scope_label = tab.chart_scope_label.as_str();
        let history: &[MetricPoint] = tab.chart_history.as_ref();
        let total_pages = tab.total_chart_pages.max(1);
        let page_idx = tab.chart_page.min(total_pages.saturating_sub(1));
        let currency_symbol = world.currency_symbol.as_str();

        if history.is_empty() {
            let placeholder = Paragraph::new(
                "No history data yet. Advance the simulation by month to collect snapshots.",
            )
            .style(theme::primary_text().bg(theme::PANEL_BG))
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Charts ")
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text())
                    .style(Style::default().bg(theme::PANEL_BG)),
            );
            frame.render_widget(placeholder, body);
        } else {
            let [row1, row2, row3, row4] = Layout::vertical([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .areas(body);
            let [a1, a2] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row1);
            let [a3, a4] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row2);
            let [a5, a6] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row3);
            let [a7, a8] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(row4);
            let chart_areas = [a1, a2, a3, a4, a5, a6, a7, a8];
            let chart_specs = Self::chart_specs_for_page(page_idx);

            for (area, spec) in chart_areas.into_iter().zip(chart_specs.into_iter()) {
                self.draw_metric_chart(
                    frame,
                    area,
                    history,
                    &format!("{} ({scope_label})", spec.title),
                    spec.color,
                    spec.value_fn,
                    spec.axis_format,
                    currency_symbol,
                );
            }
        }

        format!(
            "Controls: {}  [t] toggle scope  [,/.] cycle country  [PgUp/PgDn] chart page | Chart Page {}/{}",
            hotkeys::GLOBAL_RUNNING_HINT,
            page_idx + 1,
            total_pages
        )
    }

    fn chart_specs_for_page(page_idx: usize) -> [ChartSpec; 8] {
        match page_idx {
            0 => [
                ChartSpec {
                    title: "Population",
                    color: Color::Cyan,
                    value_fn: Self::metric_population,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Total Money Supply",
                    color: Color::Green,
                    value_fn: Self::metric_total_money_supply,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Household Cash Total",
                    color: Color::Yellow,
                    value_fn: Self::metric_household_cash_total,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Company Cash Total",
                    color: Color::Magenta,
                    value_fn: Self::metric_company_cash_total,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Created Money Outstanding",
                    color: Color::LightBlue,
                    value_fn: Self::metric_created_money_outstanding,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Bank Reserve",
                    color: Color::LightRed,
                    value_fn: Self::metric_bank_reserve,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Expected Money Supply",
                    color: Color::LightYellow,
                    value_fn: Self::metric_expected_money_supply,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Money Supply Gap",
                    color: Color::LightCyan,
                    value_fn: Self::metric_money_supply_gap,
                    axis_format: AxisValueFormat::Money,
                },
            ],
            1 => [
                ChartSpec {
                    title: "Treasury Balance",
                    color: Color::Cyan,
                    value_fn: Self::metric_treasury_balance,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Budget Balance",
                    color: Color::Green,
                    value_fn: Self::metric_budget_balance,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Monthly Tax Collected",
                    color: Color::Yellow,
                    value_fn: Self::metric_monthly_tax_collected,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Monthly Gov Spending",
                    color: Color::Magenta,
                    value_fn: Self::metric_monthly_gov_spending,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Government Debt",
                    color: Color::LightBlue,
                    value_fn: Self::metric_gov_debt,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Bonds Outstanding",
                    color: Color::LightRed,
                    value_fn: Self::metric_bonds_outstanding,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Monthly Goods Import Spending",
                    color: Color::LightYellow,
                    value_fn: Self::metric_monthly_goods_import_spending,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Monthly Goods Export Revenue",
                    color: Color::LightCyan,
                    value_fn: Self::metric_monthly_goods_export_revenue,
                    axis_format: AxisValueFormat::Money,
                },
            ],
            _ => [
                ChartSpec {
                    title: "CPI Index",
                    color: Color::Cyan,
                    value_fn: Self::metric_cpi_index,
                    axis_format: AxisValueFormat::Index,
                },
                ChartSpec {
                    title: "Inflation Rate %",
                    color: Color::Green,
                    value_fn: Self::metric_inflation_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Policy Rate %",
                    color: Color::Yellow,
                    value_fn: Self::metric_policy_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Real Policy Rate %",
                    color: Color::Magenta,
                    value_fn: Self::metric_real_policy_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Monthly Unmet Need Events",
                    color: Color::LightBlue,
                    value_fn: Self::metric_monthly_unmet_need_events,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Monthly Essential Debt Spending",
                    color: Color::LightRed,
                    value_fn: Self::metric_monthly_essential_debt_spending,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Stock Market Cap",
                    color: Color::LightYellow,
                    value_fn: Self::metric_stock_market_cap,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Bank Equity Exposure",
                    color: Color::LightCyan,
                    value_fn: Self::metric_stock_bank_equity_exposure,
                    axis_format: AxisValueFormat::Money,
                },
            ],
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "chart rendering requires explicit visual + accessor parameters"
    )]
    fn draw_metric_chart(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &[MetricPoint],
        title: &str,
        color: Color,
        value_fn: fn(&MetricPoint) -> f64,
        axis_format: AxisValueFormat,
        currency_symbol: &str,
    ) {
        if history.is_empty() {
            frame.render_widget(
                Paragraph::new("No data.")
                    .style(theme::muted_text().bg(theme::PANEL_BG))
                    .block(
                        Block::default()
                            .title(format!(" {title} "))
                            .borders(Borders::ALL)
                            .border_style(theme::neutral_border())
                            .title_style(theme::primary_text())
                            .style(Style::default().bg(theme::PANEL_BG)),
                    ),
                area,
            );
            return;
        }

        let points = history
            .iter()
            .map(|point| (point.month as f64, value_fn(point)))
            .collect::<Vec<_>>();

        let first_month = history.first().map_or(0, |point| point.month);
        let last_month = history.last().map_or(first_month, |point| point.month);
        let x_min = first_month as f64;
        let mut x_max = last_month as f64;
        if (x_max - x_min).abs() < f64::EPSILON {
            x_max = x_min + 1.0;
        }

        let (value_min, value_max) = points
            .iter()
            .map(|(_, value)| *value)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        let (y_min, y_max) = Self::expanded_axis_bounds(value_min, value_max);

        let dataset = Dataset::default()
            .graph_type(GraphType::Line)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(color).bg(theme::PANEL_BG))
            .data(&points);

        let chart = Chart::new(vec![dataset])
            .style(Style::default().bg(theme::PANEL_BG))
            .x_axis(
                Axis::default()
                    .title("Month")
                    .style(theme::primary_text().bg(theme::PANEL_BG))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        first_month.to_string(),
                        last_month.max(first_month + 1).to_string(),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(theme::primary_text().bg(theme::PANEL_BG))
                    .bounds([y_min, y_max])
                    .labels(vec![
                        Self::format_axis_label(axis_format, y_min, currency_symbol),
                        Self::format_axis_label(axis_format, y_max, currency_symbol),
                    ]),
            )
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL)
                    .border_style(theme::neutral_border())
                    .title_style(theme::primary_text())
                    .style(Style::default().bg(theme::PANEL_BG)),
            );

        frame.render_widget(chart, area);
    }

    fn format_axis_label(
        axis_format: AxisValueFormat,
        value: f64,
        currency_symbol: &str,
    ) -> String {
        match axis_format {
            AxisValueFormat::Money => Self::money(value, currency_symbol),
            AxisValueFormat::Percent => format!("{value:.2}%"),
            AxisValueFormat::Index => format!("{value:.3}"),
            AxisValueFormat::Count => {
                if value.abs() >= 10_000.0 {
                    Self::compact_num(value)
                } else {
                    format!("{value:.0}")
                }
            }
        }
    }

    fn metric_population(point: &MetricPoint) -> f64 {
        point.population
    }

    fn metric_total_money_supply(point: &MetricPoint) -> f64 {
        point.total_money_supply
    }

    fn metric_household_cash_total(point: &MetricPoint) -> f64 {
        point.household_cash_total
    }

    fn metric_company_cash_total(point: &MetricPoint) -> f64 {
        point.company_cash_total
    }

    fn metric_created_money_outstanding(point: &MetricPoint) -> f64 {
        point.created_money_outstanding
    }

    fn metric_bank_reserve(point: &MetricPoint) -> f64 {
        point.bank_reserve
    }

    fn metric_expected_money_supply(point: &MetricPoint) -> f64 {
        point.expected_money_supply
    }

    fn metric_money_supply_gap(point: &MetricPoint) -> f64 {
        point.money_supply_gap
    }

    fn metric_treasury_balance(point: &MetricPoint) -> f64 {
        point.treasury_balance
    }

    fn metric_budget_balance(point: &MetricPoint) -> f64 {
        point.budget_balance
    }

    fn metric_monthly_tax_collected(point: &MetricPoint) -> f64 {
        point.monthly_tax_collected
    }

    fn metric_monthly_gov_spending(point: &MetricPoint) -> f64 {
        point.monthly_gov_spending
    }

    fn metric_gov_debt(point: &MetricPoint) -> f64 {
        point.gov_debt
    }

    fn metric_bonds_outstanding(point: &MetricPoint) -> f64 {
        point.bonds_outstanding
    }

    fn metric_monthly_goods_import_spending(point: &MetricPoint) -> f64 {
        point.monthly_goods_import_spending
    }

    fn metric_monthly_goods_export_revenue(point: &MetricPoint) -> f64 {
        point.monthly_goods_export_revenue
    }

    fn metric_cpi_index(point: &MetricPoint) -> f64 {
        point.cpi_index
    }

    fn metric_inflation_rate_pct(point: &MetricPoint) -> f64 {
        point.inflation_rate * 100.0
    }

    fn metric_policy_rate_pct(point: &MetricPoint) -> f64 {
        point.policy_rate * 100.0
    }

    fn metric_real_policy_rate_pct(point: &MetricPoint) -> f64 {
        (point.policy_rate - point.inflation_rate) * 100.0
    }

    fn metric_monthly_unmet_need_events(point: &MetricPoint) -> f64 {
        point.monthly_unmet_need_events
    }

    fn metric_monthly_essential_debt_spending(point: &MetricPoint) -> f64 {
        point.monthly_essential_debt_spending
    }

    fn metric_stock_market_cap(point: &MetricPoint) -> f64 {
        point.stock_market_cap
    }

    fn metric_stock_bank_equity_exposure(point: &MetricPoint) -> f64 {
        point.stock_bank_equity_exposure
    }

    fn expanded_axis_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        if (max - min).abs() < f64::EPSILON {
            let padding = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.10
            };
            let lower = (min - padding).min(min);
            let upper = (max + padding).max(max);
            return (lower, upper);
        }

        let span = max - min;
        let padding = span * 0.10;
        let lower = if min >= 0.0 {
            (min - padding).max(0.0)
        } else {
            min - padding
        };
        let upper = max + padding;
        (lower, upper)
    }
}
