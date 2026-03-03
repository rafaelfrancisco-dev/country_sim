use super::*;

pub(in crate::components::simulation) fn chart_scope_label(world: &World, ui: &UiState) -> String {
    match ui.chart_scope {
        ChartScope::SelectedCountry => {
            let country_name = ui
                .selected_country(world)
                .map(|country| country.name.as_str())
                .unwrap_or("N/A");
            format!("Selected: {country_name}")
        }
        ChartScope::WorldAggregate => "World Aggregate".to_string(),
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
) -> (
    Vec<StockRowSnapshot>,
    Vec<String>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
) {
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
        format!("Market Cap: {}", money(market.total_market_cap())),
        format!("Listed Companies: {}", market.listed_count()),
        format!(
            "Bank Equity Exposure: {}",
            money(market.bank_equity_exposure())
        ),
        format!("MTD IPO Raised: {}", money(market.ipo_raised_mtd)),
        format!("MTD Dividends Paid: {}", money(market.dividends_paid_mtd)),
        format!(
            "MTD Forced Liquidations: {}",
            money(market.forced_liquidations_mtd)
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
                .get(equity.company_id as usize)
                .map(|c| c.name.clone())
                .unwrap_or_default();

            let bank_shares = market
                .bank_portfolio
                .position(equity.company_id)
                .map_or(0, |p| p.shares);
            let bank_pct = if equity.shares_outstanding > 0 {
                bank_shares as f64 / equity.shares_outstanding as f64 * 100.0
            } else {
                0.0
            };

            let abs_row = page_start + row_idx;
            StockRowSnapshot {
                ticker: equity.ticker.clone(),
                company_name,
                last_price: money(equity.last_price),
                day_pct: format!("{:+.2}%", equity.day_pct_change() * 100.0),
                market_cap: money(equity.market_cap),
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
    let price_history = selected_equity
        .map(|e| e.price_history.clone())
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
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return Vec::new();
    };
    let Some(person) = ui.selected_person_on_page(world) else {
        return Vec::new();
    };

    let market = &country.stock_market;
    let Some(portfolio) = market.person_portfolios.get(&person.id) else {
        return vec!["  No stock holdings.".to_string()];
    };

    if portfolio.positions.is_empty() {
        return vec!["  No stock holdings.".to_string()];
    }

    let total_value = portfolio.total_equity_value(market);
    let mut lines = vec![
        String::new(),
        "-- Stock Holdings -----------------------------".to_string(),
        format!("  Total Equity Value: {}", money(total_value)),
        format!("  MTD Realized PnL: {}", money(portfolio.realized_pnl_mtd)),
        format!("  MTD Dividends: {}", money(portfolio.dividends_mtd)),
    ];

    for pos in &portfolio.positions {
        let equity = market.listed.get(&pos.company_id);
        let ticker = equity.map(|e| e.ticker.as_str()).unwrap_or("???");
        let last_price = equity.map_or(0.0, |e| e.last_price);
        let value = pos.shares as f64 * last_price;
        let unrealized = pos.unrealized_pnl(last_price);
        let pledged_note = if pos.pledged_shares > 0 {
            format!(" ({}??)", pos.pledged_shares)
        } else {
            String::new()
        };
        lines.push(format!(
            "  {ticker}: {} shares{pledged_note} @ avg ${:.2} | last ${:.2} | val {} | PnL {}",
            pos.shares,
            pos.avg_cost,
            last_price,
            money(value),
            money(unrealized),
        ));
    }

    lines
}

pub(in crate::components::simulation) fn compact_num(n: f64) -> String {
    let abs = n.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{n:.2}");
    };
    let scaled = n / divisor;
    let r = (scaled * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}{suffix}")
    } else {
        format!("{r:.1}{suffix}")
    }
}

pub(in crate::components::simulation) fn compact_count(n: impl Into<f64>) -> String {
    let n: f64 = n.into();
    let abs = n.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{n:.0}");
    };
    let scaled = n / divisor;
    let r = (scaled * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}{suffix}")
    } else {
        format!("{r:.1}{suffix}")
    }
}

pub(in crate::components::simulation) fn money(amount: f64) -> String {
    compact_num(amount)
}
