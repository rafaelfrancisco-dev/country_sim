use super::*;
use crate::sim::person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus};
use crate::sim::world::MetricPoint;

#[path = "stocks_and_charts_format.rs"]
mod format;
pub(in crate::components::simulation) use format::{
    compact_count, compact_num, money, normalized_currency_symbol,
};

type StockTabData = (
    Vec<StockRowSnapshot>,
    Vec<String>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
);

pub(in crate::components::simulation) fn chart_scope_label(world: &World, ui: &UiState) -> String {
    match ui.chart_scope {
        ChartScope::SelectedCountry => {
            let country_name_owned: String = ui
                .selected_country(world)
                .map(|country| country.name.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let country_name = country_name_owned.as_str();
            format!("Selected: {country_name}")
        }
        ChartScope::WorldAggregate => "World Aggregate".to_string(),
    }
}

pub(in crate::components::simulation) fn build_charts_tab_snapshot(
    world: &World,
    ui: &UiState,
    selected_country_disaster_days: &[u64],
) -> RunningTabSnapshot {
    let chart_scope_label = chart_scope_label(world, ui);
    let pie_page = build_pie_charts_page(world, ui);
    let chart_history: Vec<MetricPoint> = match ui.chart_scope {
        ChartScope::SelectedCountry => world
            .country_history
            .get(ui.selected_country)
            .map(|h| h.iter().copied().collect())
            .unwrap_or_default(),
        ChartScope::WorldAggregate => world.world_history.iter().copied().collect(),
    };
    let disaster_days = match ui.chart_scope {
        ChartScope::SelectedCountry => selected_country_disaster_days.to_vec(),
        ChartScope::WorldAggregate => disaster_days_world(world),
    };

    RunningTabSnapshot::Charts(Box::new(ChartsTabSnapshot {
        chart_scope_label,
        chart_page: ui.chart_page,
        total_chart_pages: crate::components::simulation::ui_state::CHART_PAGE_COUNT,
        chart_history: chart_history.into(),
        disaster_days: disaster_days.into(),
        pie_page,
    }))
}

fn build_pie_charts_page(world: &World, ui: &UiState) -> PieChartsPageSnapshot {
    let population_by_country = build_pie_chart(
        "Population by Country",
        world
            .countries
            .iter()
            .map(|country| (country.name.to_string(), country.population() as f64)),
    );
    let money_supply_by_country = build_pie_chart(
        "Total Money Supply by Country",
        world
            .countries
            .iter()
            .map(|country| (country.name.to_string(), country.total_money_supply)),
    );
    let stock_cap_by_country = build_pie_chart(
        "Stock Market Cap by Country",
        world.countries.iter().map(|country| {
            (
                country.name.to_string(),
                country.stock_market.total_market_cap(),
            )
        }),
    );
    let treasury_by_country = build_pie_chart(
        "Treasury Balance by Country",
        world
            .countries
            .iter()
            .map(|country| (country.name.to_string(), country.treasury_balance.max(0.0))),
    );

    let Some(country) = ui.selected_country(world) else {
        return PieChartsPageSnapshot {
            population_by_country,
            money_supply_by_country,
            stock_cap_by_country,
            treasury_by_country,
            education_tracks: empty_pie_chart("Education Track"),
            employment_status: empty_pie_chart("Employment Status"),
            age_brackets: empty_pie_chart("Age Brackets"),
            wealth_breakdown: empty_pie_chart("Wealth Breakdown"),
        };
    };

    let mut education_basic_only = 0.0;
    let mut education_high_school = 0.0;
    let mut education_bachelor = 0.0;
    let mut education_doctor = 0.0;
    let mut unemployed = 0.0;
    let mut employed_company = 0.0;
    let mut employed_bank = 0.0;
    let mut children = 0.0;
    let mut working_age = 0.0;
    let mut elderly = 0.0;

    for person in &country.people {
        match person.education_track {
            EducationTrack::BasicOnly => education_basic_only += 1.0,
            EducationTrack::HighSchool => education_high_school += 1.0,
            EducationTrack::Bachelor => education_bachelor += 1.0,
            EducationTrack::Doctor => education_doctor += 1.0,
        }

        match person.employment {
            EmploymentStatus::Unemployed => unemployed += 1.0,
            EmploymentStatus::Employed { .. } => employed_company += 1.0,
            EmploymentStatus::EmployedAtBank { .. } => employed_bank += 1.0,
        }

        let age_years = person.age_days / DAYS_PER_YEAR;
        if age_years < 18 {
            children += 1.0;
        } else if age_years >= 65 {
            elderly += 1.0;
        } else {
            working_age += 1.0;
        }
    }

    PieChartsPageSnapshot {
        population_by_country,
        money_supply_by_country,
        stock_cap_by_country,
        treasury_by_country,
        education_tracks: build_pie_chart(
            "Education Track",
            [
                ("BasicOnly".to_string(), education_basic_only),
                ("HighSchool".to_string(), education_high_school),
                ("Bachelor".to_string(), education_bachelor),
                ("Doctor".to_string(), education_doctor),
            ],
        ),
        employment_status: build_pie_chart(
            "Employment Status",
            [
                ("Unemployed".to_string(), unemployed),
                ("Employed@Company".to_string(), employed_company),
                ("Employed@Bank".to_string(), employed_bank),
            ],
        ),
        age_brackets: build_pie_chart(
            "Age Brackets",
            [
                ("Children (<18)".to_string(), children),
                ("Working Age (18-64)".to_string(), working_age),
                ("Elderly (65+)".to_string(), elderly),
            ],
        ),
        wealth_breakdown: build_pie_chart(
            "Wealth Breakdown",
            [
                ("Household Cash".to_string(), country.household_cash_total()),
                ("Company Cash".to_string(), country.company_cash_total()),
                ("Treasury".to_string(), country.treasury_balance().max(0.0)),
                (
                    "Bank Reserve".to_string(),
                    country
                        .public_bank()
                        .map(|bank| bank.reserve_balance.max(0.0))
                        .unwrap_or(0.0),
                ),
            ],
        ),
    }
}

fn build_pie_chart(
    title: impl Into<String>,
    slices: impl IntoIterator<Item = (String, f64)>,
) -> PieChartData {
    let mut slices = slices
        .into_iter()
        .filter(|(_, value)| value.is_finite() && *value > 0.0)
        .map(|(label, value)| PieSliceData { label, value })
        .collect::<Vec<_>>();
    slices.sort_by(|left, right| {
        right
            .value
            .partial_cmp(&left.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    PieChartData {
        title: title.into(),
        slices,
    }
}

fn empty_pie_chart(title: impl Into<String>) -> PieChartData {
    PieChartData {
        title: title.into(),
        slices: Vec::new(),
    }
}

pub(in crate::components::simulation) fn goods_mask_names(mask: u16) -> String {
    let names = common_goods()
        .iter()
        .filter(|good| (mask & (1_u16 << good.id)) != 0)
        .map(|good| good.name)
        .collect::<Vec<_>>();

    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

pub(in crate::components::simulation) fn build_stock_tab_data(
    world: &World,
    ui: &UiState,
    currency_symbol: &str,
) -> StockTabData {
    let Some(country) = ui.selected_country(world) else {
        return (
            Vec::new(),
            vec!["No country selected.".to_string()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
    };

    let market = &country.stock_market;

    // Summary panel
    let summary_lines = vec![
        format!("Country: {} (Month {})", country.name, country.month),
        format!(
            "Market Cap: {}",
            money(market.total_market_cap(), currency_symbol)
        ),
        format!("Listed Companies: {}", market.listed_count()),
        format!(
            "Bank Equity Exposure: {}",
            money(
                world.market_bank_portfolio_value(country.id),
                currency_symbol
            )
        ),
        format!(
            "MTD IPO Raised: {}",
            money(market.ipo_raised_mtd, currency_symbol)
        ),
        format!(
            "Last Month Dividends Paid: {}",
            money(market.last_completed_month_dividends_paid, currency_symbol)
        ),
        format!(
            "MTD Forced Liquidations: {}",
            money(market.forced_liquidations_mtd, currency_symbol)
        ),
    ];

    // Build stock rows sorted by market cap descending
    let mut equities: Vec<_> = market.listed.values().collect();
    equities.sort_by(|a, b| {
        b.market_cap
            .partial_cmp(&a.market_cap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    const STOCKS_PAGE_SIZE: usize = 15;
    let page_start = ui.stock_page.saturating_mul(STOCKS_PAGE_SIZE);
    let page_end = (page_start + STOCKS_PAGE_SIZE).min(equities.len());
    let page_equities = &equities[page_start.min(equities.len())..page_end];

    let stock_rows: Vec<StockRowSnapshot> = page_equities
        .iter()
        .enumerate()
        .map(|(row_idx, equity)| {
            let company_name = country
                .companies
                .get(equity.current_company_id as usize)
                .map(|c| c.name.to_string())
                .unwrap_or_default();

            let bank_shares = market
                .bank_portfolio
                .position(crate::sim::stock_market::StockSecurityId {
                    country_id: country.id,
                    company_id: equity.company_id,
                })
                .map_or(0, |p| p.shares);
            let bank_pct = if equity.shares_outstanding > 0 {
                bank_shares as f64 / equity.shares_outstanding as f64 * 100.0
            } else {
                0.0
            };

            let abs_row = page_start + row_idx;
            StockRowSnapshot {
                ticker: equity.ticker.to_string(),
                company_name,
                last_price: money(equity.last_price, currency_symbol),
                day_pct: format!("{:+.2}%", equity.closed_day_pct_change() * 100.0),
                market_cap: money(equity.market_cap, currency_symbol),
                volume: compact_count(equity.day_volume as f64),
                float_shares: compact_count(equity.free_float_shares as f64),
                bank_pct: format!("{:.1}%", bank_pct),
                is_selected: abs_row == ui.stock_selected_row,
                is_even: abs_row.is_multiple_of(2),
            }
        })
        .collect();

    // Charts for the selected stock
    let selected_equity = equities.get(ui.stock_selected_row);
    let price_history: Vec<f64> = selected_equity
        .map(|e| e.price_history.iter().copied().collect())
        .unwrap_or_default();
    let volume_history = selected_equity
        .map(|e| e.volume_history.iter().map(|&v| v as f64).collect())
        .unwrap_or_default();

    // Country stock index history
    let index_history: Vec<f64> = market
        .daily_index_history
        .iter()
        .map(|p| p.index_value)
        .collect();

    // Bank equity exposure history
    let bank_exposure_history: Vec<f64> = market
        .daily_index_history
        .iter()
        .map(|p| p.bank_equity_exposure)
        .collect();

    (
        stock_rows,
        summary_lines,
        price_history,
        volume_history,
        index_history,
        bank_exposure_history,
    )
}

pub(in crate::components::simulation) fn stock_person_details_lines(
    world: &World,
    ui: &UiState,
    currency_symbol: &str,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return Vec::new();
    };
    let Some(person) = ui.selected_person_on_page(world) else {
        return Vec::new();
    };

    person_stock_holdings_lines(world, country, person.id, currency_symbol)
        .into_iter()
        .map(|line| format!("  {line}"))
        .collect()
}

pub(in crate::components::simulation) fn person_stock_holdings_lines(
    world: &World,
    country: &crate::sim::country::Country,
    person_id: u32,
    currency_symbol: &str,
) -> Vec<String> {
    let market = &country.stock_market;
    let Some(portfolio) = market.person_portfolios.get(&person_id) else {
        return vec!["Stock Holdings: None".to_string()];
    };
    if portfolio.positions.is_empty() {
        return vec!["Stock Holdings: None".to_string()];
    }

    let total_value = portfolio
        .positions
        .iter()
        .map(|pos| {
            world
                .countries
                .iter()
                .find(|listing_country| listing_country.id == pos.security_id.country_id)
                .and_then(|listing_country| {
                    listing_country
                        .stock_market
                        .listed
                        .get(&pos.security_id.company_id)
                })
                .map_or(0.0, |equity| pos.market_value(equity.last_price))
        })
        .sum::<f64>();
    let mut lines = vec![
        "Stock Holdings:".to_string(),
        format!(
            "Total Equity Value: {}",
            money(total_value, currency_symbol)
        ),
        format!(
            "MTD Realized PnL: {}",
            money(portfolio.realized_pnl_mtd, currency_symbol)
        ),
        format!(
            "Last Month Dividends: {}",
            money(portfolio.last_completed_month_dividends, currency_symbol)
        ),
    ];

    for pos in &portfolio.positions {
        let equity = world
            .countries
            .iter()
            .find(|listing_country| listing_country.id == pos.security_id.country_id)
            .and_then(|listing_country| {
                listing_country
                    .stock_market
                    .listed
                    .get(&pos.security_id.company_id)
            });
        let ticker_owned: String = equity.map(|e| e.ticker.to_string()).unwrap_or_else(|| "???".to_string());
        let ticker: &str = &ticker_owned;
        let last_price = equity.map_or(0.0, |e| e.last_price);
        let value = pos.market_value(last_price);
        let unrealized = pos.unrealized_pnl(last_price);
        let country_prefix = if pos.security_id.country_id == country.id {
            String::new()
        } else {
            let listing_country = world
                .countries
                .iter()
                .find(|listing_country| listing_country.id == pos.security_id.country_id)
                .map(|listing_country| listing_country.name.as_ref())
                .unwrap_or("Foreign");
            format!("[{listing_country}] ")
        };
        let pledged_note = if pos.pledged_shares > 0 {
            format!(" | pledged {}", pos.pledged_shares)
        } else {
            String::new()
        };
        lines.push(format!(
            "{country_prefix}{ticker}: {} shares{pledged_note} @ avg {} | last {} | val {} | PnL {}",
            pos.shares,
            money(pos.avg_cost, currency_symbol),
            money(last_price, currency_symbol),
            money(value, currency_symbol),
            money(unrealized, currency_symbol),
        ));
    }

    lines
}

#[cfg(test)]
#[path = "stocks_and_charts_tests.rs"]
mod tests;
