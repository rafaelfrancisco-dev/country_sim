use super::super::Simulation;
use std::collections::HashSet;

#[test]
fn chart_specs_are_unique_across_all_pages() {
    let mut seen = HashSet::new();

    for page_idx in 0..6 {
        for title in Simulation::chart_titles_for_page(page_idx) {
            assert!(seen.insert(title), "duplicate chart title found: {title}");
        }
    }
}

#[test]
fn page_four_uses_interest_and_credit_specific_metrics() {
    let titles = Simulation::chart_titles_for_page(3);

    assert!(titles.contains(&"Deposit Rate %"));
    assert!(titles.contains(&"Loan Rate %"));
    assert!(titles.contains(&"Rate Spread %"));
    assert!(titles.contains(&"Real Deposit Rate %"));
    assert!(titles.contains(&"Monthly Savings Interest Paid"));
    assert!(titles.contains(&"Real Loan Rate %"));
    assert!(titles.contains(&"Credit-to-Money %"));
    assert!(titles.contains(&"Reserve Coverage %"));

    assert!(!titles.contains(&"Policy Rate %"));
    assert!(!titles.contains(&"Inflation Rate %"));
    assert!(!titles.contains(&"Loan Book"));
}

#[test]
fn migration_page_exposes_country_flow_titles() {
    let titles = Simulation::chart_titles_for_page(5);

    assert!(titles.contains(&"Inbound Migration"));
    assert!(titles.contains(&"Outbound Migration"));
    assert!(titles.contains(&"Net Migration"));
}
