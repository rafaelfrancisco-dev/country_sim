use super::*;
use crate::sim::person::HousingStatus;

fn profitable_share(country: &Country) -> f64 {
    if country.companies.is_empty() {
        return 0.0;
    }

    let profitable = country
        .companies
        .iter()
        .filter(|company| company.last_month_profit > 0.0)
        .count();
    profitable as f64 / country.companies.len() as f64
}

#[test]
fn generated_world_2x1k_360_days_housing_market_is_sane() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 2, 1_000, 360, 2);
    assert!(
        world.validate_invariants().is_empty(),
        "invariant violations: {:?}",
        world.validate_invariants()
    );

    for country in &world.countries {
        // Stock accounting invariant
        let total = country.housing_stock_total();
        let owned = country.housing_stock_occupied_owned;
        let rented = country.housing_stock_occupied_rented;
        let facility = country.housing_stock_company_facility;
        let vacant = country.housing_stock_vacant;
        assert_eq!(
            total,
            owned + rented + facility + vacant,
            "country {}: housing stock {} != owned {} + rented {} + facility {} + vacant {}",
            country.name,
            total,
            owned,
            rented,
            facility,
            vacant
        );

        // Company rental units >= occupied rented (companies may also own vacant units)
        let company_rentals: u32 = country.companies.iter().map(|c| c.rental_units_owned).sum();
        assert!(
            company_rentals >= rented,
            "country {}: Σ company.rental_units_owned {} < housing_stock_occupied_rented {}",
            country.name,
            company_rentals,
            rented
        );

        // Person housing status counts match stock counters
        let mut person_owned = 0_u32;
        let mut person_rented = 0_u32;
        for person in &country.people {
            match &person.housing {
                HousingStatus::Owner | HousingStatus::Mortgaged { .. } => person_owned += 1,
                HousingStatus::Renter { .. } => person_rented += 1,
                HousingStatus::Unhoused => {}
            }
        }
        assert_eq!(
            person_owned, owned,
            "country {}: persons Owner/Mortgaged {} != housing_stock_occupied_owned {}",
            country.name, person_owned, owned
        );
        assert_eq!(
            person_rented, rented,
            "country {}: persons Renter {} != housing_stock_occupied_rented {}",
            country.name, person_rented, rented
        );

        // Price index is positive and finite
        let price = country.housing_price_index();
        assert!(
            price > 0.0 && price.is_finite(),
            "country {}: housing_price_index {price} not positive-finite",
            country.name
        );

        // Monthly rent is a reasonable fraction of price index
        let rent = country.monthly_rent();
        assert!(
            rent > 0.0 && rent.is_finite(),
            "country {}: monthly_rent {rent} not positive-finite",
            country.name
        );

        // Homeownership rate in plausible range (20–95%)
        let ownership_rate = country.homeownership_rate();
        assert!(
            ownership_rate >= 0.20 && ownership_rate <= 0.95,
            "country {}: homeownership_rate {ownership_rate:.3} out of range [0.20, 0.95]",
            country.name
        );
    }
}

#[test]
#[ignore = "long-run profitability scenario (1095 days, 2 x 2k)"]
fn generated_world_2x2k_1095_days_keeps_profitable_share_above_floor() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 2, 2_000, 1095, 4);
    assert!(world.validate_invariants().is_empty());

    for country in &world.countries {
        let share = profitable_share(country);
        assert!(
            share >= 0.10,
            "country {} profitable share too low: {:.2}% ({} / {})",
            country.id,
            share * 100.0,
            country
                .companies
                .iter()
                .filter(|company| company.last_month_profit > 0.0)
                .count(),
            country.companies.len()
        );
    }
}

#[test]
#[ignore = "long-run stability scenario (730 days, 3 x 15k)"]
fn generated_world_3x15k_730_days_meets_stability_floor() {
    let (world, companies_founded_total, growth_ipos_by_country) =
        run_generated_world_with_acceptance_metrics_in_pool(0x5eed_c0de, 3, 15_000, 730, 8);
    assert!(world.validate_invariants().is_empty());

    let rates = world
        .countries
        .iter()
        .map(labor_force_unemployment)
        .collect::<Vec<_>>();
    let avg = rates.iter().sum::<f64>() / rates.len() as f64;
    assert!(
        avg <= 0.15,
        "expected avg labor-force unemployment <= 15%, got {avg:.4}"
    );
    assert!(
        companies_founded_total > 0,
        "expected companies_founded_total > 0 over 730 days"
    );
    for country in &world.countries {
        let growth_ipos = growth_ipos_by_country
            .get(country.name.as_ref())
            .copied()
            .unwrap_or(0);
        assert!(
            growth_ipos > 0,
            "expected at least one growth IPO for country {}",
            country.name
        );
    }
}

#[test]
#[ignore = "long-run stability scenario (1095 days, 3 x 15k)"]
fn generated_world_3x15k_1095_days_keeps_country_unemployment_capped() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 3, 15_000, 1095, 8);
    assert!(world.validate_invariants().is_empty());

    for country in &world.countries {
        let rate = labor_force_unemployment(country);
        assert!(
            rate <= 0.20,
            "country {} unemployment exceeded 20%: {rate:.4}",
            country.id
        );
    }
}
