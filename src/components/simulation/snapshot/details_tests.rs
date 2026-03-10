use std::sync::Arc;

use super::*;
use crate::components::simulation::ui_state::DetailsView;
use crate::sim::{
    country::Country,
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
    stock_market::ListedEquity,
};

fn sample_world_with_stock() -> World {
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        123,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.superficial_stock_units = [2, 1, 0, 3, 1];

    let mut country = Country::new(1, "A".to_string(), vec![person]);
    country.stock_market.listed.insert(
        0,
        ListedEquity::new(0, 0, Arc::from("ACO"), 12.5, 10_000, 8_000),
    );
    country.stock_market.person_portfolio_mut(1).add_shares(
        crate::sim::stock_market::StockSecurityId {
            country_id: 1,
            company_id: 0,
        },
        25,
        10.0,
    );

    World::new(vec![country])
}

#[test]
fn person_details_include_superficial_goods_and_stock_holdings() {
    let world = sample_world_with_stock();
    let ui = UiState {
        details_view: DetailsView::Person,
        ..UiState::default()
    };
    let lines = person_details_lines(&world, &ui);
    let text = lines.join("\n");

    assert!(text.contains("Current Goods On Hand:"));
    assert!(text.contains("Household Goods: 2"));
    assert!(text.contains("Entertainment: 1"));
    assert!(text.contains("Stock Holdings:"));
    assert!(text.contains("ACO: 25 shares"));
}

#[test]
fn person_details_show_last_completed_month_income_and_dividends() {
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        123,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.last_month_gross_pay = 1_000.0;
    person.last_month_tax_paid = 200.0;
    person.last_month_net_pay = 800.0;
    person.reset_monthly_trackers();

    let mut country = Country::new(1, "A".to_string(), vec![person]);
    country.stock_market.listed.insert(
        0,
        ListedEquity::new(0, 0, Arc::from("ACO"), 12.5, 10_000, 8_000),
    );
    {
        let portfolio = country.stock_market.person_portfolio_mut(1);
        portfolio.add_shares(
            crate::sim::stock_market::StockSecurityId {
                country_id: 1,
                company_id: 0,
            },
            25,
            10.0,
        );
        portfolio.dividends_mtd = 15.0;
    }
    country.stock_market.reset_mtd_trackers();

    let world = World::new(vec![country]);
    let ui = UiState {
        details_view: DetailsView::Person,
        ..UiState::default()
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);
    let text = person_details_lines(&world, &ui).join("\n");

    assert!(text.contains("Last Month Gross Pay:"));
    assert!(text.contains(&money(1_000.0, currency_symbol)));
    assert!(text.contains("Last Month Tax Paid:"));
    assert!(text.contains(&money(200.0, currency_symbol)));
    assert!(text.contains("Last Month Net Pay:"));
    assert!(text.contains(&money(800.0, currency_symbol)));
    assert!(text.contains("Last Month Dividends:"));
    assert!(text.contains(&money(15.0, currency_symbol)));
}

#[test]
fn company_details_include_listing_and_financial_stress_metrics() {
    let world = sample_world_with_stock();
    let ui = UiState {
        active_tab: RunningTab::Goods,
        details_view: DetailsView::Company,
        ..UiState::default()
    };
    let lines = company_details_lines(&world, &ui);
    let text = lines.join("\n");

    assert!(text.contains("Status & Market:"));
    assert!(text.contains("Listed: Yes"));
    assert!(text.contains("Ticker: ACO"));
    assert!(text.contains("Consecutive Missed Loan Payments:"));
}

#[test]
fn company_details_show_last_completed_month_payroll_after_reset() {
    let mut world = sample_world_with_stock();
    world.countries[0].companies[0].last_month_payroll_paid = 1_200.0;
    world.countries[0].companies[0].open_month();

    let ui = UiState {
        active_tab: RunningTab::Goods,
        details_view: DetailsView::Company,
        ..UiState::default()
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);
    let text = company_details_lines(&world, &ui).join("\n");

    assert!(text.contains("Last Month Payroll Paid:"));
    assert!(text.contains(&money(1_200.0, currency_symbol)));
}

#[test]
fn bank_details_include_balance_sheet_and_stability_sections() {
    let world = sample_world_with_stock();
    let ui = UiState {
        active_tab: RunningTab::Banks,
        details_view: DetailsView::Bank,
        ..UiState::default()
    };
    let lines = bank_details_lines(&world, &ui);
    let text = lines.join("\n");

    assert!(text.contains("Balance Sheet:"));
    assert!(text.contains("Flow Metrics:"));
    assert!(text.contains("Stability:"));
    assert!(text.contains("Liquidity Ratio:"));
}

#[test]
fn country_details_include_structural_trade_market_and_social_sections() {
    let world = sample_world_with_stock();
    let ui = UiState {
        details_view: DetailsView::Country,
        ..UiState::default()
    };
    let lines = country_details_lines(&world, &ui);
    let text = lines.join("\n");

    assert!(text.contains("Structure:"));
    assert!(text.contains("Trade & Tariffs:"));
    assert!(text.contains("Market Risk:"));
    assert!(text.contains("Social Pressure:"));
}

#[test]
fn country_policy_details_show_central_bank_fields() {
    let world = sample_world_with_stock();
    let ui = UiState {
        details_view: DetailsView::CountryPolicy,
        ..UiState::default()
    };
    let lines = country_policy_lines(&world, &ui);
    let text = lines.join("\n");

    assert!(text.contains("Target Inflation Rate"));
    assert!(text.contains("Neutral Policy Rate"));
    assert!(text.contains("Policy Rate Step"));
}
