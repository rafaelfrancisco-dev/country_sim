use super::*;

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
    assert!((person.last_month_needs_met_mask & 1_u16) != 0);
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
fn tariff_increases_for_exported_good_when_domestic_stock_is_depleted_and_need_unmet() {
    let mut country_a_companies = vec![Company::new(0, "A Water".to_string(), vec![0, 5], 0)];
    country_a_companies[0].inventory_units = 0;
    country_a_companies[0].unit_price = 50.0;
    country_a_companies[0].cash_balance = 1_000.0;

    let mut country_b_companies = vec![Company::new(0, "B Water".to_string(), vec![0, 5], 0)];
    country_b_companies[0].inventory_units = 4;
    country_b_companies[0].unit_price = 10.0;
    country_b_companies[0].cash_balance = 1_000.0;

    let country_a = Country::with_economy(
        0,
        "A".to_string(),
        vec![unemployed_person(1, 300.0)],
        country_a_companies,
        1_300.0,
    );
    let country_b = Country::with_economy(
        1,
        "B".to_string(),
        vec![unemployed_person(2, 300.0)],
        country_b_companies,
        1_300.0,
    );

    let mut world = World::new(vec![country_a, country_b]);
    let before = world.countries[1].export_tariffs_by_good[0];

    for _ in 0..20 {
        world.tick_day();
    }
    world.adjust_dynamic_country_tariffs();

    let after = world.countries[1].export_tariffs_by_good[0];

    assert!(after > before);
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
