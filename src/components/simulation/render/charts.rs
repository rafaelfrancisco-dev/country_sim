use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use super::disaster_markers;
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

macro_rules! metric_field_fn {
    ($name:ident, $field:ident) => {
        fn $name(p: &MetricPoint) -> f64 {
            p.$field
        }
    };
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
        let disaster_days = tab.disaster_days.as_ref();

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
                    disaster_days,
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
                    title: "Monthly Money Minted",
                    color: Color::LightYellow,
                    value_fn: Self::metric_monthly_money_minted,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Cumulative Money Minted",
                    color: Color::LightCyan,
                    value_fn: Self::metric_cumulative_money_minted,
                    axis_format: AxisValueFormat::Money,
                },
            ],
            2 => [
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
            _ => [
                ChartSpec {
                    title: "Deposit Rate %",
                    color: Color::Cyan,
                    value_fn: Self::metric_deposit_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Loan Rate %",
                    color: Color::Yellow,
                    value_fn: Self::metric_loan_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Rate Spread %",
                    color: Color::LightRed,
                    value_fn: Self::metric_rate_spread_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Real Deposit Rate %",
                    color: Color::Magenta,
                    value_fn: Self::metric_real_deposit_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Monthly Savings Interest Paid",
                    color: Color::LightGreen,
                    value_fn: Self::metric_savings_interest_paid,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Real Loan Rate %",
                    color: Color::LightYellow,
                    value_fn: Self::metric_real_loan_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Credit-to-Money %",
                    color: Color::LightBlue,
                    value_fn: Self::metric_credit_to_money_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Reserve Coverage %",
                    color: Color::LightCyan,
                    value_fn: Self::metric_reserve_coverage_pct,
                    axis_format: AxisValueFormat::Percent,
                },
            ],
        }
    }

    #[cfg(test)]
    pub(super) fn chart_titles_for_page(page_idx: usize) -> [&'static str; 8] {
        Self::chart_specs_for_page(page_idx).map(|spec| spec.title)
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
        disaster_days: &[u64],
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
        let marker_positions = disaster_markers::disaster_month_positions(disaster_days);
        let marker_segments =
            disaster_markers::marker_segments(&marker_positions, x_min, x_max, y_min, y_max);
        let mut datasets = Vec::with_capacity(1 + marker_segments.len());
        datasets.push(dataset);
        datasets.extend(disaster_markers::marker_datasets(&marker_segments));

        let chart = Chart::new(datasets)
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

    metric_field_fn!(metric_population, population);
    metric_field_fn!(metric_total_money_supply, total_money_supply);
    metric_field_fn!(metric_household_cash_total, household_cash_total);
    metric_field_fn!(metric_company_cash_total, company_cash_total);
    metric_field_fn!(metric_created_money_outstanding, created_money_outstanding);
    metric_field_fn!(metric_bank_reserve, bank_reserve);
    metric_field_fn!(metric_expected_money_supply, expected_money_supply);
    metric_field_fn!(metric_money_supply_gap, money_supply_gap);
    metric_field_fn!(metric_treasury_balance, treasury_balance);
    metric_field_fn!(metric_budget_balance, budget_balance);
    metric_field_fn!(metric_monthly_tax_collected, monthly_tax_collected);
    metric_field_fn!(metric_monthly_gov_spending, monthly_gov_spending);
    metric_field_fn!(metric_gov_debt, gov_debt);
    metric_field_fn!(metric_monthly_money_minted, monthly_money_minted);
    metric_field_fn!(metric_cumulative_money_minted, cumulative_money_minted);
    metric_field_fn!(metric_bonds_outstanding, bonds_outstanding);
    metric_field_fn!(metric_cpi_index, cpi_index);
    fn metric_inflation_rate_pct(p: &MetricPoint) -> f64 {
        p.inflation_rate * 100.0
    }
    fn metric_policy_rate_pct(p: &MetricPoint) -> f64 {
        p.policy_rate * 100.0
    }
    fn metric_real_policy_rate_pct(p: &MetricPoint) -> f64 {
        (p.policy_rate - p.inflation_rate) * 100.0
    }
    metric_field_fn!(metric_monthly_unmet_need_events, monthly_unmet_need_events);
    metric_field_fn!(
        metric_monthly_essential_debt_spending,
        monthly_essential_debt_spending
    );
    metric_field_fn!(metric_stock_market_cap, stock_market_cap);
    metric_field_fn!(
        metric_stock_bank_equity_exposure,
        stock_bank_equity_exposure
    );
    fn metric_deposit_rate_pct(p: &MetricPoint) -> f64 {
        p.deposit_rate * 100.0
    }
    fn metric_loan_rate_pct(p: &MetricPoint) -> f64 {
        p.loan_rate * 100.0
    }
    fn metric_rate_spread_pct(p: &MetricPoint) -> f64 {
        (p.loan_rate - p.deposit_rate) * 100.0
    }
    fn metric_real_deposit_rate_pct(p: &MetricPoint) -> f64 {
        (p.deposit_rate - p.inflation_rate) * 100.0
    }
    metric_field_fn!(metric_savings_interest_paid, monthly_savings_interest_paid);
    fn metric_real_loan_rate_pct(p: &MetricPoint) -> f64 {
        (p.loan_rate - p.inflation_rate) * 100.0
    }
    fn metric_credit_to_money_pct(p: &MetricPoint) -> f64 {
        if p.total_money_supply <= 0.0 {
            0.0
        } else {
            (p.created_money_outstanding / p.total_money_supply) * 100.0
        }
    }
    fn metric_reserve_coverage_pct(p: &MetricPoint) -> f64 {
        if p.created_money_outstanding <= 0.0 {
            0.0
        } else {
            (p.bank_reserve / p.created_money_outstanding) * 100.0
        }
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
