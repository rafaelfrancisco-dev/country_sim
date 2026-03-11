use std::collections::VecDeque;

use super::*;
use crate::sim::params::CompanyGrowthParams;

#[test]
fn average_monthly_revenue_uses_available_history() {
    let mut company = Company::new(1, "Test".to_string(), vec![0, 1, 2], 1);
    company.sales_history = VecDeque::from(vec![10.0, 20.0, 40.0]);

    assert!((company.average_monthly_revenue() - 23.33).abs() < 0.01);
}

#[test]
fn average_profit_over_months_requires_enough_history() {
    let mut company = Company::new(1, "Test".to_string(), vec![0, 1, 2], 1);
    company.profit_history = VecDeque::from(vec![10.0, 20.0, 40.0]);

    assert!((company.average_profit_over_months(4) - 0.0).abs() < 0.01);
    assert!((company.average_profit_over_months(3) - 23.33).abs() < 0.01);
}

#[test]
fn default_companies_offer_multiple_job_types() {
    let companies = build_default_companies("A", 2);

    assert_eq!(companies.len(), 2);
    assert!(
        companies
            .iter()
            .all(|company| company.offered_job_types.len() > 1)
    );
}

#[test]
fn default_companies_cover_goods_catalog_with_default_count() {
    let companies = build_default_companies("A", DEFAULT_COMPANIES_PER_COUNTRY);
    let mut goods = companies
        .iter()
        .map(|company| company.produced_good_id)
        .collect::<Vec<_>>();
    goods.sort_unstable();
    goods.dedup();

    assert_eq!(companies.len(), DEFAULT_COMPANIES_PER_COUNTRY as usize);
    assert_eq!(goods.len(), 15);
}

#[test]
fn growth_openings_require_profitability_and_reserve() {
    let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 2);
    company.consecutive_profitable_months = 3;
    company.cash_balance = 200_000.0;
    company.monthly_payroll_budget = 50_000.0;
    company.last_month_profit = 180_000.0;

    let created = company.open_positions_from_growth(&CompanyGrowthParams::default(), 12);

    assert!(created > 0);
    assert!(!company.open_positions.is_empty());
}

#[test]
fn price_cut_with_excess_inventory_clamps_to_at_cost_floor() {
    let market = GoodsMarketParams::default();
    let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 0);
    company.unit_price = 6.3;
    company.inventory_units = 10;
    company.last_month_units_sold = 0;

    company.close_month(&market);

    assert!((company.unit_price - 6.0).abs() < 0.01);
}

#[test]
fn price_raise_has_no_upper_clamp() {
    let market = GoodsMarketParams::default();
    let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 0);
    company.unit_price = 23.9;
    company.inventory_units = 0;
    company.last_month_units_sold = 100;

    company.close_month(&market);

    assert!((company.unit_price - 24.38).abs() < 0.01);
}

#[test]
fn recompute_profitability_includes_rent_revenue() {
    let mut company = Company::new(1, "HousingCo".to_string(), vec![0, 1, 2], 2);
    company.last_month_sales = 0.0;
    company.last_month_rent_revenue = 50_000.0;
    company.last_month_payroll_paid = 20_000.0;
    company.last_month_loan_payment = 5_000.0;

    company.recompute_profitability();

    assert!((company.last_month_profit - 25_000.0).abs() < f64::EPSILON);
    assert_eq!(company.consecutive_profitable_months, 1);
    assert_eq!(company.consecutive_loss_months, 0);
}

#[test]
fn recompute_profitability_sales_and_rent_combined() {
    let mut company = Company::new(1, "MixedCo".to_string(), vec![0, 1, 2], 2);
    company.last_month_sales = 100_000.0;
    company.last_month_rent_revenue = 30_000.0;
    company.last_month_payroll_paid = 80_000.0;
    company.last_month_loan_payment = 10_000.0;

    company.recompute_profitability();

    assert!((company.last_month_profit - 40_000.0).abs() < f64::EPSILON);
}

#[test]
fn open_month_preserves_last_completed_finance_metrics() {
    let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 0);
    company.last_month_payroll_paid = 12_000.0;
    company.last_month_profit = 2_500.0;
    company.last_month_loan_payment = 900.0;
    company.last_month_new_loan = 1_800.0;
    company.last_month_savings_interest = 75.0;

    company.open_month();

    assert!((company.last_completed_month_payroll_paid - 12_000.0).abs() < f64::EPSILON);
    assert!((company.last_completed_month_profit - 2_500.0).abs() < f64::EPSILON);
    assert!((company.last_completed_month_loan_payment - 900.0).abs() < f64::EPSILON);
    assert!((company.last_completed_month_new_loan - 1_800.0).abs() < f64::EPSILON);
    assert!((company.last_completed_month_savings_interest - 75.0).abs() < f64::EPSILON);
    assert!((company.last_month_payroll_paid - 0.0).abs() < f64::EPSILON);
    assert!((company.last_month_loan_payment - 0.0).abs() < f64::EPSILON);
    assert!((company.last_month_new_loan - 0.0).abs() < f64::EPSILON);
    assert!((company.last_month_savings_interest - 0.0).abs() < f64::EPSILON);
}
