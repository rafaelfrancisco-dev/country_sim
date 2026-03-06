use super::companies::{
    build_initial_companies, derive_initial_company_count, target_initial_company_counts,
};
use super::*;

fn config(seed: u64) -> GenerationConfig {
    GenerationConfig::with_defaults(2, 8, seed)
}

#[test]
fn generated_counts_are_exact() {
    let countries = generate_countries(&config(1));

    assert_eq!(countries.len(), 2);
    assert_eq!(countries[0].people.len(), 8);
    assert_eq!(countries[1].people.len(), 8);
}

#[test]
fn every_company_offers_multiple_job_types() {
    let countries = generate_countries(&config(2));

    assert!(
        countries
            .iter()
            .flat_map(|country| country.companies.iter())
            .all(|company| company.offered_job_types.len() > 1)
    );
}

#[test]
fn initial_money_supply_is_split_between_people_and_companies() {
    let countries = generate_countries(&config(3));

    for country in countries {
        let non_household_cash =
            round_cents(country.company_cash_total + country.total_bank_reserves());
        assert!((country.household_cash_total - non_household_cash).abs() <= 0.01);
        let expected_total = round_cents(country.initial_money_supply);
        assert!((country.total_money_supply - expected_total).abs() <= 0.01);
    }
}

#[test]
fn generation_creates_unemployed_population_slice() {
    let countries = generate_countries(&GenerationConfig::with_defaults(1, 500, 11));
    let unemployed = countries[0]
        .people
        .iter()
        .filter(|person| !person.is_employed())
        .count();

    assert!(unemployed > 0);
}

#[test]
fn setup_randomizes_initial_company_prices() {
    let countries = generate_countries(&GenerationConfig::with_defaults(2, 500, 9));
    let prices = countries
        .iter()
        .flat_map(|country| country.companies.iter().map(|company| company.unit_price))
        .collect::<Vec<_>>();
    let unique_prices = prices
        .iter()
        .map(|price| (price * 100.0).round() as i64)
        .collect::<std::collections::BTreeSet<_>>();

    assert!(unique_prices.len() > 10);
}

#[test]
fn target_initial_company_counts_scale_essentials_for_large_populations() {
    let counts = target_initial_company_counts(100_000);

    assert!(counts[0] >= 10);
    assert!(counts[1] >= 10);
    assert!(counts[3] >= 2);
    assert!(counts[4] >= 1);
    assert!(counts.iter().copied().map(u32::from).sum::<u32>() > 53);
}

#[test]
fn build_initial_companies_matches_default_good_targets() {
    let sim_params = SimulationParams::default();
    let company_count = derive_initial_company_count(100_000, &sim_params);
    let companies = build_initial_companies("A", 100_000, company_count, &sim_params);
    let mut counts = [0_u16; GOODS_COUNT];
    for company in companies {
        counts[company.produced_good_id as usize] =
            counts[company.produced_good_id as usize].saturating_add(1);
    }

    assert_eq!(counts, target_initial_company_counts(100_000));
}

#[test]
fn setup_creates_uneven_initial_company_headcounts() {
    let countries = generate_countries(&GenerationConfig::with_defaults(1, 10_000, 13));
    let headcounts = countries[0]
        .companies
        .iter()
        .map(|company| company.employee_count)
        .collect::<Vec<_>>();
    let unique_headcounts = headcounts
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();

    assert!(unique_headcounts.len() > 1);
}

#[test]
fn generation_seeds_inventory_and_manufacturing_capacity_for_stockable_essentials() {
    let country = generate_countries(&GenerationConfig::with_defaults(1, 10_000, 17))
        .into_iter()
        .next()
        .expect("country should be generated");

    for good_id in STOCKABLE_ESSENTIAL_GOOD_IDS {
        assert!(
            country.essential_inventory_units_for_good(good_id) > 0.0,
            "expected seeded inventory for essential good {good_id}"
        );
        assert!(
            country.essential_manufacturing_capacity_units_for_good(good_id) > 0.0,
            "expected manufacturing capacity for essential good {good_id}"
        );
    }
}
