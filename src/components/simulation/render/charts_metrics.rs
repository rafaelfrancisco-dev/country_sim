use ratatui::style::Color;

use crate::{components::simulation::Simulation, sim::world::MetricPoint};

#[derive(Debug, Copy, Clone)]
pub(super) enum AxisValueFormat {
    Money,
    Percent,
    Index,
    Count,
}

#[derive(Debug, Copy, Clone)]
pub(super) struct ChartSpec {
    pub title: &'static str,
    pub color: Color,
    pub value_fn: fn(&MetricPoint) -> f64,
    pub axis_format: AxisValueFormat,
}

macro_rules! metric_field_fn {
    ($name:ident, $field:ident) => {
        fn $name(point: &MetricPoint) -> f64 {
            point.$field
        }
    };
}

impl Simulation {
    pub(super) fn chart_specs_for_page(page_idx: usize) -> [ChartSpec; 8] {
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
                    title: "Mint Fraction %",
                    color: Color::LightCyan,
                    value_fn: Self::metric_monthly_mint_fraction_pct,
                    axis_format: AxisValueFormat::Percent,
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
                    title: "Company Input Spending",
                    color: Color::LightCyan,
                    value_fn: Self::metric_company_input_spending,
                    axis_format: AxisValueFormat::Money,
                },
            ],
            4 => [
                ChartSpec {
                    title: "Housing Price Index",
                    color: Color::LightGreen,
                    value_fn: Self::metric_housing_price_index,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Homeownership Rate %",
                    color: Color::LightYellow,
                    value_fn: Self::metric_homeownership_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Monthly Rent",
                    color: Color::LightMagenta,
                    value_fn: Self::metric_monthly_rent,
                    axis_format: AxisValueFormat::Money,
                },
                ChartSpec {
                    title: "Vacancy Rate %",
                    color: Color::LightRed,
                    value_fn: Self::metric_housing_vacancy_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Company Facility Units",
                    color: Color::Cyan,
                    value_fn: Self::metric_company_facility_units,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Monthly Foreclosures",
                    color: Color::Red,
                    value_fn: Self::metric_monthly_foreclosures,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Mortgage Rate %",
                    color: Color::Yellow,
                    value_fn: Self::metric_loan_rate_pct,
                    axis_format: AxisValueFormat::Percent,
                },
                ChartSpec {
                    title: "Rent/Price Ratio %",
                    color: Color::LightBlue,
                    value_fn: Self::metric_rent_to_price_pct,
                    axis_format: AxisValueFormat::Percent,
                },
            ],
            5 => [
                ChartSpec {
                    title: "Inbound Migration",
                    color: Color::LightGreen,
                    value_fn: Self::metric_monthly_immigrants,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Outbound Migration",
                    color: Color::LightRed,
                    value_fn: Self::metric_monthly_emigrants,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Net Migration",
                    color: Color::LightBlue,
                    value_fn: Self::metric_net_migration,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Work Migration",
                    color: Color::Yellow,
                    value_fn: Self::metric_monthly_migrants_for_work,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Distress Migration",
                    color: Color::Magenta,
                    value_fn: Self::metric_monthly_migrants_for_distress,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Immigration per 1k",
                    color: Color::Cyan,
                    value_fn: Self::metric_monthly_immigrants_per_1k,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Emigration per 1k",
                    color: Color::LightYellow,
                    value_fn: Self::metric_monthly_emigrants_per_1k,
                    axis_format: AxisValueFormat::Count,
                },
                ChartSpec {
                    title: "Gross Migration",
                    color: Color::LightCyan,
                    value_fn: Self::metric_gross_migration,
                    axis_format: AxisValueFormat::Count,
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
    pub(in crate::components::simulation::render) fn chart_titles_for_page(
        page_idx: usize,
    ) -> [&'static str; 8] {
        Self::chart_specs_for_page(page_idx).map(|spec| spec.title)
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
    fn metric_monthly_mint_fraction_pct(point: &MetricPoint) -> f64 {
        point.monthly_mint_fraction * 100.0
    }
    metric_field_fn!(metric_bonds_outstanding, bonds_outstanding);
    metric_field_fn!(metric_cpi_index, cpi_index);
    fn metric_inflation_rate_pct(point: &MetricPoint) -> f64 {
        point.inflation_rate * 100.0
    }
    fn metric_policy_rate_pct(point: &MetricPoint) -> f64 {
        point.policy_rate * 100.0
    }
    fn metric_real_policy_rate_pct(point: &MetricPoint) -> f64 {
        (point.policy_rate - point.inflation_rate) * 100.0
    }
    metric_field_fn!(metric_monthly_unmet_need_events, monthly_unmet_need_events);
    metric_field_fn!(
        metric_monthly_essential_debt_spending,
        monthly_essential_debt_spending
    );
    metric_field_fn!(metric_stock_market_cap, stock_market_cap);
    metric_field_fn!(metric_company_input_spending, company_input_spending);
    fn metric_deposit_rate_pct(point: &MetricPoint) -> f64 {
        point.deposit_rate * 100.0
    }
    fn metric_loan_rate_pct(point: &MetricPoint) -> f64 {
        point.loan_rate * 100.0
    }
    fn metric_rate_spread_pct(point: &MetricPoint) -> f64 {
        (point.loan_rate - point.deposit_rate) * 100.0
    }
    fn metric_real_deposit_rate_pct(point: &MetricPoint) -> f64 {
        (point.deposit_rate - point.inflation_rate) * 100.0
    }
    metric_field_fn!(metric_savings_interest_paid, monthly_savings_interest_paid);
    fn metric_real_loan_rate_pct(point: &MetricPoint) -> f64 {
        (point.loan_rate - point.inflation_rate) * 100.0
    }
    fn metric_credit_to_money_pct(point: &MetricPoint) -> f64 {
        if point.total_money_supply <= 0.0 {
            0.0
        } else {
            (point.created_money_outstanding / point.total_money_supply) * 100.0
        }
    }
    fn metric_reserve_coverage_pct(point: &MetricPoint) -> f64 {
        if point.created_money_outstanding <= 0.0 {
            0.0
        } else {
            (point.bank_reserve / point.created_money_outstanding) * 100.0
        }
    }
    metric_field_fn!(metric_housing_price_index, housing_price_index);
    metric_field_fn!(metric_monthly_rent, monthly_rent);
    fn metric_homeownership_rate_pct(point: &MetricPoint) -> f64 {
        point.homeownership_rate * 100.0
    }
    fn metric_housing_vacancy_rate_pct(point: &MetricPoint) -> f64 {
        point.housing_vacancy_rate * 100.0
    }
    metric_field_fn!(metric_company_facility_units, company_facility_units);
    metric_field_fn!(metric_monthly_foreclosures, monthly_foreclosures);
    fn metric_rent_to_price_pct(point: &MetricPoint) -> f64 {
        if point.housing_price_index <= 0.0 {
            0.0
        } else {
            (point.monthly_rent / point.housing_price_index) * 100.0
        }
    }
    metric_field_fn!(metric_monthly_immigrants, monthly_immigrants);
    metric_field_fn!(metric_monthly_emigrants, monthly_emigrants);
    metric_field_fn!(metric_monthly_migrants_for_work, monthly_migrants_for_work);
    metric_field_fn!(
        metric_monthly_migrants_for_distress,
        monthly_migrants_for_distress
    );
    fn metric_net_migration(point: &MetricPoint) -> f64 {
        point.monthly_immigrants - point.monthly_emigrants
    }
    fn metric_monthly_immigrants_per_1k(point: &MetricPoint) -> f64 {
        if point.population <= 0.0 {
            0.0
        } else {
            point.monthly_immigrants / point.population * 1_000.0
        }
    }
    fn metric_monthly_emigrants_per_1k(point: &MetricPoint) -> f64 {
        if point.population <= 0.0 {
            0.0
        } else {
            point.monthly_emigrants / point.population * 1_000.0
        }
    }
    fn metric_gross_migration(point: &MetricPoint) -> f64 {
        point.monthly_immigrants + point.monthly_emigrants
    }
}
