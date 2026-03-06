use super::*;
pub(in crate::components::simulation) fn country_details_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

    let company_count = country.companies().len();
    let public_utility_count = country
        .companies()
        .iter()
        .filter(|company| company.is_public_utility)
        .count();
    let essential_backfill_count = country
        .companies()
        .iter()
        .filter(|company| company.is_essential_backfill)
        .count();
    let total_open_slots: u64 = country
        .companies()
        .iter()
        .flat_map(|company| company.open_positions.iter())
        .map(|opening| u64::from(opening.slots))
        .sum();
    let listed_count = country.stock_market().listed_count();
    let total_banks = country.banks().len();
    let public_banks = country
        .all_banks()
        .filter(|bank| bank.kind == crate::sim::bank::BankKind::Public)
        .count();
    let banks_in_resolution = country
        .all_banks()
        .filter(|bank| bank.is_in_resolution)
        .count();

    let trade_balance =
        country.monthly_goods_export_revenue() - country.monthly_goods_import_spending();
    let avg_export_tariff = if country.export_tariffs_by_good().is_empty() {
        0.0
    } else {
        country.export_tariffs_by_good().iter().sum::<f64>()
            / country.export_tariffs_by_good().len() as f64
    };
    let (max_tariff_good_id, max_export_tariff) = country
        .export_tariffs_by_good()
        .iter()
        .enumerate()
        .max_by(|(_, lhs), (_, rhs)| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(good_idx, tariff)| (good_idx as u8, *tariff))
        .unwrap_or((0, 0.0));
    let max_tariff_good_label = good_by_id(max_tariff_good_id).map_or("Unknown", |good| good.name);

    let market_cap = country.stock_market().total_market_cap();
    let index_value = country.stock_market().index_value();
    let bank_equity_exposure = world.market_bank_portfolio_value(country.id());
    let daily_turnover = country.stock_market().total_daily_turnover();

    vec![
        format!("Name: {}", country.name()),
        format!("Country ID: {}", country.id()),
        format!("Day: {}", world.day()),
        format!("Month: {}", country.month()),
        String::new(),
        "Structure:".to_string(),
        format!("Population: {}", compact_count(country.population() as f64)),
        format!("Companies: {}", compact_count(company_count as f64)),
        format!(
            "Public Utility / Essential Backfill: {}/{}",
            compact_count(public_utility_count as f64),
            compact_count(essential_backfill_count as f64)
        ),
        format!(
            "Open Positions (slots): {}",
            compact_count(total_open_slots as f64)
        ),
        format!("Listed Companies: {}", compact_count(listed_count as f64)),
        format!(
            "Banks (Public / In Resolution): {} ({} / {})",
            compact_count(total_banks as f64),
            compact_count(public_banks as f64),
            compact_count(banks_in_resolution as f64)
        ),
        String::new(),
        "Money & Fiscal:".to_string(),
        format!(
            "Household Cash: {}",
            money(country.household_cash_total(), currency_symbol)
        ),
        format!(
            "Company Cash: {}",
            money(country.company_cash_total(), currency_symbol)
        ),
        format!(
            "Treasury: {}",
            money(country.treasury_balance(), currency_symbol)
        ),
        format!(
            "Bank Reserve: {}",
            money(country.total_bank_reserves(), currency_symbol)
        ),
        format!(
            "Loans Outstanding: {}",
            money(country.total_bank_created_money(), currency_symbol)
        ),
        format!(
            "Money Supply: {}",
            money(country.total_money_supply(), currency_symbol)
        ),
        format!(
            "Expected Supply: {}",
            money(country.expected_money_supply(), currency_symbol)
        ),
        format!(
            "Supply Gap: {}",
            money(country.money_supply_gap(), currency_symbol)
        ),
        format!(
            "MTD Tax Collected: {}",
            money(country.monthly_tax_collected(), currency_symbol)
        ),
        format!(
            "MTD Gov Spending: {}",
            money(country.monthly_gov_spending(), currency_symbol)
        ),
        format!(
            "MTD Budget Balance: {}",
            money(country.monthly_budget_balance(), currency_symbol)
        ),
        format!(
            "Monetized Debt: {}",
            money(country.gov_debt(), currency_symbol)
        ),
        format!(
            "Bonds Outstanding: {}",
            money(country.bonds_outstanding(), currency_symbol)
        ),
        format!(
            "Bond Coupon Rate: {:.2}%",
            country.bond_coupon_rate() * 100.0
        ),
        format!(
            "Bond Months to Maturity: {}",
            country.bond_months_to_maturity()
        ),
        format!(
            "MTD Coupon Paid: {}",
            money(country.monthly_coupon_paid(), currency_symbol)
        ),
        format!(
            "MTD Bonds Issued: {}",
            money(country.monthly_bonds_issued(), currency_symbol)
        ),
        format!(
            "MTD Bonds Matured: {}",
            money(country.monthly_bonds_matured(), currency_symbol)
        ),
        "Monetary Operations:".to_string(),
        if country.last_mint_inflation_shortfall() == 0.0
            && country.last_mint_unemployment_gap() == 0.0
        {
            "  Mint Active: No".to_string()
        } else {
            format!(
                "  Inflation Gap: {:.2}pp / Unemp Gap: {:.2}pp",
                country.last_mint_inflation_shortfall() * 100.0,
                country.last_mint_unemployment_gap() * 100.0
            )
        },
        if country.last_mint_inflation_shortfall() != 0.0
            || country.last_mint_unemployment_gap() != 0.0
        {
            format!(
                "  MTD Minted:     {} (+{:.3}% of supply)",
                money(country.monthly_money_minted(), currency_symbol),
                country.last_mint_fraction() * 100.0
            )
        } else {
            String::new()
        },
        format!(
            "  Cumul Minted:   {}",
            money(country.cumulative_money_minted(), currency_symbol)
        ),
        String::new(),
        "Prices & Rates:".to_string(),
        format!("CPI Index: {:.4}", country.cpi_index()),
        format!(
            "Inflation Rate (ann.): {:.2}%",
            country.monthly_inflation_rate() * 100.0
        ),
        format!("Policy Rate: {:.2}%", country.policy_rate() * 100.0),
        format!("Bank Loan Rate: {:.2}%", country.bank_loan_rate() * 100.0),
        format!("Deposit Rate:  {:.2}%", country.deposit_rate() * 100.0),
        format!(
            "MTD Svgs Int:  {}",
            money(country.monthly_savings_interest_paid(), currency_symbol)
        ),
        String::new(),
        "Trade & Tariffs:".to_string(),
        format!(
            "MTD Household Spending: {}",
            money(country.monthly_household_spending(), currency_symbol)
        ),
        format!(
            "MTD Imports / Exports: {} / {}",
            money(country.monthly_goods_import_spending(), currency_symbol),
            money(country.monthly_goods_export_revenue(), currency_symbol)
        ),
        format!(
            "MTD Trade Balance: {}",
            money(trade_balance, currency_symbol)
        ),
        format!(
            "MTD Tariffs Collected: {}",
            money(country.monthly_tariff_collected(), currency_symbol)
        ),
        format!("Average Export Tariff: {:.2}%", avg_export_tariff * 100.0),
        format!(
            "Max Export Tariff: {:.2}% ({max_tariff_good_label})",
            max_export_tariff * 100.0
        ),
        String::new(),
        "Market Risk:".to_string(),
        format!("Stock Market Cap: {}", money(market_cap, currency_symbol)),
        format!("Stock Index: {:.2}", index_value),
        format!(
            "Bank Equity Exposure: {}",
            money(bank_equity_exposure, currency_symbol)
        ),
        format!("Daily Turnover: {}", money(daily_turnover, currency_symbol)),
        format!(
            "Active Traders / Buy / Sell Orders: {} / {} / {}",
            compact_count(country.stock_market.day_active_traders_count as f64),
            compact_count(country.stock_market.day_buy_orders_count as f64),
            compact_count(country.stock_market.day_sell_orders_count as f64)
        ),
        String::new(),
        "Social Pressure:".to_string(),
        format!(
            "MTD Unmet Need Events: {}",
            compact_count(country.monthly_unmet_need_events as f64)
        ),
        format!(
            "MTD Essential Debt Spending: {}",
            money(country.monthly_essential_debt_spending, currency_symbol)
        ),
        format!(
            "MTD Hires: {}",
            compact_count(country.monthly_hires_total as f64)
        ),
        format!(
            "Last Day Births / Deaths: {} / {}",
            compact_count(country.last_day_births as f64),
            compact_count(country.last_day_deaths as f64)
        ),
        String::new(),
        format!(
            "People Page: {}/{}",
            ui.people_page + 1,
            ui.total_people_pages(world)
        ),
    ]
}

pub(in crate::components::simulation) fn country_policy_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

    let form = &ui.country_policy_form;
    let fields = [
        (
            CountryPolicyField::TaxBracket1Max,
            "Tax Bracket 1 Max",
            country.sim_params.tax.bracket_1_max,
            false,
            true,
        ),
        (
            CountryPolicyField::TaxBracket2Max,
            "Tax Bracket 2 Max",
            country.sim_params.tax.bracket_2_max,
            false,
            true,
        ),
        (
            CountryPolicyField::TaxRate1,
            "Tax Rate 1",
            country.sim_params.tax.rate_1,
            true,
            false,
        ),
        (
            CountryPolicyField::TaxRate2,
            "Tax Rate 2",
            country.sim_params.tax.rate_2,
            true,
            false,
        ),
        (
            CountryPolicyField::TaxRate3,
            "Tax Rate 3",
            country.sim_params.tax.rate_3,
            true,
            false,
        ),
        (
            CountryPolicyField::UnemploymentBenefitRate,
            "Unemployment Benefit Rate",
            country.sim_params.fiscal.unemployment_benefit_rate,
            true,
            false,
        ),
        (
            CountryPolicyField::MedianIncomeReference,
            "Median Income Reference",
            country.sim_params.fiscal.median_income_reference,
            false,
            true,
        ),
        (
            CountryPolicyField::BasicNeedsCashThreshold,
            "Basic Needs Cash Threshold",
            country.sim_params.fiscal.basic_needs_cash_threshold,
            false,
            true,
        ),
        (
            CountryPolicyField::MintInflationGapWeight,
            "Mint Inflation Gap Weight",
            country.sim_params.monetary.mint_inflation_gap_weight,
            true,
            false,
        ),
        (
            CountryPolicyField::MintUnemploymentGapWeight,
            "Mint Unemployment Gap Weight",
            country.sim_params.monetary.mint_unemployment_gap_weight,
            true,
            false,
        ),
        (
            CountryPolicyField::MintMaxFractionExpectedSupply,
            "Mint Max Fraction Expected Supply",
            country
                .sim_params
                .monetary
                .mint_max_fraction_expected_supply,
            true,
            false,
        ),
    ];

    let mut lines = vec![
        format!("Country: {}", country.name),
        format!("Country ID: {}", country.id),
        String::new(),
        "Editable Policy Overrides".to_string(),
        "Changes apply immediately and affect the next monthly cycle.".to_string(),
        String::new(),
    ];

    for (field, label, value, is_rate, is_money) in fields {
        let focused = form.active_field == field;
        let pointer = if focused { ">" } else { " " };
        let formatted_value = if is_rate {
            format!("{:.4}", value)
        } else if is_money {
            money(value, currency_symbol)
        } else {
            format!("{:.2}", value)
        };
        let mut line = format!("{pointer} {label}: {formatted_value}");

        if focused && form.is_editing {
            line.push_str(&format!("  | draft: {}", form.draft_input));
        }

        lines.push(line);
    }

    lines.push(String::new());
    if form.is_editing {
        lines.push("Editing: type value, [Enter] save, [Esc] cancel.".to_string());
    } else {
        lines.push(
            "Navigate: [w/s] field, [Enter] edit, [r] reset managed defaults, [Esc] back."
                .to_string(),
        );
    }
    if let Some(error) = &form.validation_error {
        lines.push(format!("Validation: {error}"));
    }

    lines
}
