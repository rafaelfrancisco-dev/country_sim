use super::*;

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
            .get(&country.name)
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
