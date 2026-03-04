use super::*;

#[test]
fn average_monthly_revenue_uses_available_history() {
    let mut company = Company::new(1, "Test".to_string(), vec![0, 1, 2], 1);
    company.sales_history = vec![10.0, 20.0, 40.0];

    assert!((company.average_monthly_revenue() - 23.33).abs() < 0.01);
}

#[test]
fn average_profit_over_months_requires_enough_history() {
    let mut company = Company::new(1, "Test".to_string(), vec![0, 1, 2], 1);
    company.profit_history = vec![10.0, 20.0, 40.0];

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

    let created = company.open_positions_from_growth(&SimulationParams::default(), 12);

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
