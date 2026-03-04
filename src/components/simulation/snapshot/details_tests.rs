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
        ListedEquity::new(0, "ACO".to_string(), 12.5, 10_000, 8_000),
    );
    country
        .stock_market
        .person_portfolio_mut(1)
        .add_shares(0, 25, 10.0);

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
