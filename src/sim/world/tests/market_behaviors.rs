use super::*;
use crate::sim::params::SimulationParams;
use crate::sim::world::goods_market::SellerQuote;

#[test]
fn global_goods_market_chooses_cheapest_landed_supplier_and_collects_tariff() {
    let mut country_a_companies = vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)];
    country_a_companies[0].inventory_units = 20;
    country_a_companies[0].unit_price = 20.0;
    country_a_companies[0].cash_balance = 1_000.0;

    let mut country_b_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
    country_b_companies[0].inventory_units = 20;
    country_b_companies[0].unit_price = 10.0;
    country_b_companies[0].cash_balance = 1_000.0;

    let country_a = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 200.0)],
        country_a_companies,
        1_200.0,
    );
    let country_b = Country::with_economy(
        1,
        "B".to_string(),
        vec![unemployed_person(2, 200.0)],
        country_b_companies,
        1_200.0,
    );

    let mut world = World::new(vec![country_a, country_b]);
    for _ in 0..20 {
        world.tick_day();
    }

    assert_eq!(world.countries[0].companies[0].last_month_units_sold, 0);
    assert!(world.countries[1].companies[0].last_month_export_units_sold > 0);
    assert!((world.countries[0].monthly_tariff_collected - 0.0).abs() < 0.01);
    assert!(world.countries[1].monthly_tariff_collected > 0.0);
    assert!(world.countries[0].monthly_goods_import_spending > 0.0);
    assert!(world.countries[1].monthly_goods_export_revenue > 0.0);
}

#[test]
fn essentials_can_push_cash_negative_and_are_tracked_as_essential_debt_spending() {
    let mut companies = vec![Company::new(0, "Water".to_string(), vec![0, 5], 0)];
    companies[0].inventory_units = 20;
    companies[0].unit_price = 10.0;
    companies[0].cash_balance = 1_000.0;

    let country = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 5.0)],
        companies,
        1_005.0,
    );

    let mut world = World::new(vec![country]);
    for _ in 0..15 {
        world.tick_day();
    }

    let person = &world.countries[0].people[0];
    assert!(world.countries[0].monthly_essential_debt_spending > 0.0);
    assert!(person.last_month_goods_spending > 0.0);
    assert!(person.cash_balance < 0.0);
    assert!(person.essential_stock_units[0] > 0.0);
}

#[test]
fn non_essentials_respect_discretionary_budget_cap() {
    let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
    companies[0].inventory_units = 20;
    companies[0].unit_price = 50.0;
    companies[0].cash_balance = 1_000.0;

    let country = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 100.0)],
        companies,
        1_100.0,
    );

    let mut world = World::new(vec![country]);
    for _ in 0..20 {
        world.tick_day();
    }

    assert_eq!(world.countries[0].companies[0].last_month_units_sold, 0);
    assert!((world.countries[0].people[0].cash_balance - 100.0).abs() < 0.01);
}

#[test]
fn discretionary_budget_is_lower_when_policy_rate_is_higher() {
    let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
    companies[0].inventory_units = 20;
    companies[0].unit_price = 50.0;
    companies[0].cash_balance = 1_000.0;

    let country = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 10_000.0)],
        companies,
        11_000.0,
    );

    let mut world = World::new(vec![country]);
    world.countries[0].policy_rate = 0.01;
    let low_rate_budget = world.initial_discretionary_budgets()[0][0];
    world.countries[0].policy_rate = 0.15;
    let high_rate_budget = world.initial_discretionary_budgets()[0][0];

    assert!(low_rate_budget > high_rate_budget);
}

#[test]
fn discretionary_superficial_purchase_requires_empty_stock() {
    let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
    companies[0].inventory_units = 500;
    companies[0].unit_price = 5.0;
    companies[0].cash_balance = 1_000.0;

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        111,
        5 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    person.cash_balance = 10_000.0;
    person.superficial_stock_units[4] = 2;

    let country = Country::with_economy(0, "A".to_string(), vec![person], companies, 11_000.0);
    let mut world = World::new(vec![country]);
    for _ in 0..20 {
        world.tick_day();
    }

    assert_eq!(world.countries[0].people[0].superficial_stock_units[4], 2);
}

#[test]
fn discretionary_superficial_purchase_obeys_affordability_gate_and_depletes_stock() {
    let mut companies = vec![Company::new(0, "Fun".to_string(), vec![0, 5], 14)];
    companies[0].inventory_units = 2_000;
    companies[0].unit_price = 5.0;
    companies[0].cash_balance = 1_000.0;

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        222,
        5 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    person.cash_balance = 0.0;

    let country = Country::with_economy(0, "A".to_string(), vec![person], companies, 1_000.0);
    let mut world = World::new(vec![country]);
    for _ in 0..180 {
        world.tick_day();
    }
    assert_eq!(world.countries[0].people[0].superficial_stock_units[4], 0);

    world.countries[0].people[0].cash_balance = 100_000.0;
    let mut purchased_stock = 0_u16;
    for _ in 0..500 {
        world.tick_day();
        let stock = world.countries[0].people[0].superficial_stock_units[4];
        if stock > 0 {
            purchased_stock = stock;
            break;
        }
    }
    assert!(purchased_stock > 0);

    world.countries[0].people[0].cash_balance = 0.0;
    for _ in 0..40 {
        world.tick_day();
    }
    assert!(world.countries[0].people[0].superficial_stock_units[4] < purchased_stock);
}

#[test]
fn production_uses_only_manufacturing_workers() {
    let mut companies = vec![Company::new(0, "Water".to_string(), vec![1, 5], 0)];
    companies[0].inventory_units = 0;
    companies[0].cash_balance = 1_000.0;

    let country = Country::with_economy(
        0,
        "A".to_string(),
        vec![
            employed_person(1, 5, 0, 100.0),
            employed_person(2, 1, 0, 100.0),
        ],
        companies,
        1_200.0,
    );

    let mut world = World::new(vec![country]);
    for _ in 0..29 {
        world.tick_day();
    }

    assert_eq!(
        world.countries[0].companies[0].manufacturing_employee_count,
        1
    );
    assert_eq!(
        world.countries[0].companies[0].last_month_units_produced,
        116
    );
}

#[test]
fn tariff_increases_for_exported_good_when_low_stock_or_need_unmet() {
    let mut params = SimulationParams::default();
    params.goods_market.tariff_rate = 0.0;

    let mut country_a_companies = vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)];
    country_a_companies[0].inventory_units = 0;
    country_a_companies[0].unit_price = 50.0;
    country_a_companies[0].cash_balance = 1_000.0;

    let mut country_b_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
    country_b_companies[0].inventory_units = 0;
    country_b_companies[0].unit_price = 10.0;
    country_b_companies[0].cash_balance = 1_000.0;
    country_b_companies[0].last_month_export_units_sold = 1;

    let country_a = Country::with_economy_and_params(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 300.0)],
        country_a_companies,
        1_300.0,
        params.clone(),
    );
    let mut country_b = Country::with_economy_and_params(
        1,
        "B".to_string(),
        vec![unemployed_person(2, 300.0), unemployed_person(3, 300.0)],
        country_b_companies,
        1_600.0,
        params,
    );
    country_b.people[0].last_month_unmet_needs_mask = 1;

    let mut world = World::new(vec![country_a, country_b]);
    let before = world.countries[1].export_tariffs_by_good[0];
    world.adjust_dynamic_country_tariffs();

    let after = world.countries[1].export_tariffs_by_good[0];

    assert!(after > before);
}

#[test]
fn tariff_increases_when_exported_good_has_low_stock_even_without_unmet_needs() {
    let mut params = SimulationParams::default();
    params.goods_market.tariff_rate = 0.0;

    // Country A: expensive supplier (no unmet need pressure)
    let mut country_a_companies = vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)];
    country_a_companies[0].inventory_units = 50;
    country_a_companies[0].unit_price = 50.0;
    country_a_companies[0].cash_balance = 1_000.0;

    // Country B: exports with low stock (inventory < half of last month domestic sales)
    let mut country_b_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
    country_b_companies[0].last_month_export_units_sold = 1;
    country_b_companies[0].last_month_domestic_units_sold = 100;
    country_b_companies[0].inventory_units = 40; // < 100/2 = 50 → low stock
    country_b_companies[0].unit_price = 10.0;
    country_b_companies[0].cash_balance = 1_000.0;

    let country_a = Country::with_economy_and_params(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 300.0)],
        country_a_companies,
        1_300.0,
        params.clone(),
    );
    let country_b = Country::with_economy_and_params(
        1,
        "B".to_string(),
        vec![unemployed_person(2, 300.0)],
        country_b_companies,
        1_300.0,
        params,
    );
    // No unmet needs set — tariff should still rise due to low_stock

    let mut world = World::new(vec![country_a, country_b]);
    let before = world.countries[1].export_tariffs_by_good[0];
    world.adjust_dynamic_country_tariffs();
    let after = world.countries[1].export_tariffs_by_good[0];

    assert!(
        after > before,
        "tariff should rise when stock < half of domestic sales: before={before}, after={after}"
    );
}

#[test]
fn goods_purchase_keeps_incremental_aggregates_aligned_with_full_recompute() {
    let mut buyer_country = Country::with_economy(
        0,
        "Buyer".to_string(),
        vec![unemployed_person(1, 200.0)],
        vec![Company::new(0, "BuyerCo".to_string(), vec![0, 5], 0)],
        200.0,
    );
    let mut seller_company = Company::new(0, "SellerCo".to_string(), vec![0, 5], 0);
    seller_company.inventory_units = 10;
    seller_company.unit_price = 10.0;
    seller_company.cash_balance = 1_000.0;
    let seller_country = Country::with_economy(
        1,
        "Seller".to_string(),
        vec![unemployed_person(2, 0.0)],
        vec![seller_company],
        1_000.0,
    );

    buyer_country.export_tariffs_by_good[0] = 0.0;
    let mut world = World::new(vec![buyer_country, seller_country]);
    let quote = SellerQuote {
        seller_country_idx: 1,
        seller_company_idx: 0,
        base_price_cents: 1_000,
        tariff_cents: 200,
        import_tariff_cents: 0,
        landed_price_cents: 1_200,
    };

    world.execute_goods_purchase(0, 0, quote);

    let buyer_household_before = world.countries[0].household_cash_total;
    let buyer_total_before = world.countries[0].total_money_supply;
    let seller_company_before = world.countries[1].company_cash_total;
    let seller_total_before = world.countries[1].total_money_supply;

    world.countries[0].recompute_aggregates();
    world.countries[1].recompute_aggregates();

    assert!((world.countries[0].household_cash_total - buyer_household_before).abs() < 0.01);
    assert!((world.countries[0].total_money_supply - buyer_total_before).abs() < 0.01);
    assert!((world.countries[1].company_cash_total - seller_company_before).abs() < 0.01);
    assert!((world.countries[1].total_money_supply - seller_total_before).abs() < 0.01);
}

#[test]
fn tariff_decreases_toward_baseline_when_shortage_signal_is_absent() {
    let mut companies = vec![Company::new(0, "Water".to_string(), vec![0, 5], 0)];
    companies[0].inventory_units = 100;
    companies[0].unit_price = 10.0;
    companies[0].cash_balance = 1_000.0;

    let country = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 300.0)],
        companies,
        1_300.0,
    );
    let mut world = World::new(vec![country]);
    let baseline = world.countries[0].sim_params.goods_market.tariff_rate;
    world.countries[0].export_tariffs_by_good[0] = 0.60;
    let before = world.countries[0].export_tariffs_by_good[0];

    world.adjust_dynamic_country_tariffs();

    let after = world.countries[0].export_tariffs_by_good[0];
    assert!(after < before);
    assert!(after >= baseline - 1e-9);
}

#[test]
fn scarce_supply_is_shared_across_countries_with_round_robin_allocation() {
    let mut exporter_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
    exporter_companies[0].inventory_units = 2;
    exporter_companies[0].unit_price = 10.0;
    exporter_companies[0].cash_balance = 1_000.0;

    let country_a = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 300.0)],
        vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)],
        1_300.0,
    );
    let country_b = Country::with_economy(
        1,
        "B".to_string(),
        vec![unemployed_person(2, 300.0)],
        exporter_companies,
        1_300.0,
    );

    let mut world = World::new(vec![country_a, country_b]);
    for _ in 0..20 {
        world.tick_day();
    }

    assert!(world.countries[0].monthly_household_spending > 0.0);
    assert!(world.countries[1].monthly_household_spending > 0.0);
}

#[test]
fn households_group_adults_for_shared_utilities() {
    let people = vec![unemployed_person(1, 200.0), unemployed_person(2, 200.0)];
    let country = Country::with_economy(0, "A".to_string(), people, vec![], 400.0);

    assert_eq!(country.households.len(), 1);
    assert_eq!(country.household_size_for_person(0), 2);
    assert_eq!(
        country.people[0].household_id,
        country.people[1].household_id
    );
}

#[test]
fn repeated_severe_shortage_becomes_chronic() {
    let mut country = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 0.0)],
        vec![],
        0.0,
    );

    for _ in 0..2 {
        country.process_month_end();
        country.reset_month_trackers();
    }
    country.process_month_end();

    let person = &country.people[0];
    assert!(person.last_month_severe_unmet_needs_mask != 0);
    assert!(person.last_month_chronic_unmet);
    assert!(country.monthly_chronic_unmet_need_count >= 1);
}
