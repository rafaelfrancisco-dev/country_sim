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

fn profitable_private_share(country: &Country) -> f64 {
    let private_total = country
        .companies
        .iter()
        .filter(|company| !company.is_public_utility)
        .count();
    if private_total == 0 {
        return 0.0;
    }

    let profitable_private = country
        .companies
        .iter()
        .filter(|company| !company.is_public_utility && company.last_month_profit > 0.0)
        .count();
    profitable_private as f64 / private_total as f64
}

fn dormant_zero_activity_count(country: &Country) -> usize {
    country
        .companies
        .iter()
        .filter(|company| {
            company.employee_count == 0
                && company.open_positions.is_empty()
                && company.last_month_sales <= 0.0
                && company.last_month_units_produced == 0
                && company.last_completed_month_units_produced == 0
        })
        .count()
}

fn public_utility_count(country: &Country) -> usize {
    country
        .companies
        .iter()
        .filter(|company| company.is_public_utility)
        .count()
}

fn staffed_company_count(country: &Country) -> usize {
    country
        .companies
        .iter()
        .filter(|company| company.employee_count > 0)
        .count()
}

fn selling_company_count(country: &Country) -> usize {
    country
        .companies
        .iter()
        .filter(|company| company.last_month_sales > 0.0)
        .count()
}

fn selling_but_unprofitable_count(country: &Country) -> usize {
    country
        .companies
        .iter()
        .filter(|company| company.last_month_sales > 0.0 && company.last_month_profit <= 0.0)
        .count()
}

fn mature_private_profitable_share(country: &Country) -> f64 {
    let mature_private_total = country
        .companies
        .iter()
        .filter(|company| !company.is_public_utility && company.age_months >= 12)
        .count();
    if mature_private_total == 0 {
        return 0.0;
    }

    let profitable_mature_private = country
        .companies
        .iter()
        .filter(|company| {
            !company.is_public_utility
                && company.age_months >= 12
                && company.last_month_profit > 0.0
        })
        .count();
    profitable_mature_private as f64 / mature_private_total as f64
}

#[derive(Debug, Copy, Clone)]
struct ProfitabilityDiagnostics {
    profitable_share: f64,
    profitable_private_share: f64,
    profitable_public_share: f64,
    public_share: f64,
    dormant_private_share: f64,
    zero_revenue_share: f64,
    staffed_zero_revenue_share: f64,
    unstaffed_share: f64,
    no_manufacturing_zero_revenue_share: f64,
}

fn profitability_diagnostics(country: &Country) -> ProfitabilityDiagnostics {
    let total = country.companies.len();
    if total == 0 {
        return ProfitabilityDiagnostics {
            profitable_share: 0.0,
            profitable_private_share: 0.0,
            profitable_public_share: 0.0,
            public_share: 0.0,
            dormant_private_share: 0.0,
            zero_revenue_share: 0.0,
            staffed_zero_revenue_share: 0.0,
            unstaffed_share: 0.0,
            no_manufacturing_zero_revenue_share: 0.0,
        };
    }

    let profitable = country
        .companies
        .iter()
        .filter(|company| company.last_month_profit > 0.0)
        .count();
    let private_total = country
        .companies
        .iter()
        .filter(|company| !company.is_public_utility)
        .count();
    let private_profitable = country
        .companies
        .iter()
        .filter(|company| !company.is_public_utility && company.last_month_profit > 0.0)
        .count();
    let public_total = total.saturating_sub(private_total);
    let public_profitable = profitable.saturating_sub(private_profitable);
    let dormant_private = country
        .companies
        .iter()
        .filter(|company| {
            !company.is_public_utility
                && company.employee_count == 0
                && company.open_positions.is_empty()
                && company.last_month_sales <= 0.0
                && company.last_month_units_produced == 0
        })
        .count();
    let zero_revenue = country
        .companies
        .iter()
        .filter(|company| company.last_month_sales <= 0.0)
        .count();
    let staffed_zero_revenue = country
        .companies
        .iter()
        .filter(|company| company.employee_count > 0 && company.last_month_sales <= 0.0)
        .count();
    let unstaffed = country
        .companies
        .iter()
        .filter(|company| company.employee_count == 0)
        .count();
    let no_manufacturing_zero_revenue = country
        .companies
        .iter()
        .filter(|company| {
            company.employee_count > 0
                && company.manufacturing_employee_count == 0
                && company.last_month_sales <= 0.0
        })
        .count();

    ProfitabilityDiagnostics {
        profitable_share: profitable as f64 / total as f64,
        profitable_private_share: if private_total == 0 {
            0.0
        } else {
            private_profitable as f64 / private_total as f64
        },
        profitable_public_share: if public_total == 0 {
            0.0
        } else {
            public_profitable as f64 / public_total as f64
        },
        public_share: public_total as f64 / total as f64,
        dormant_private_share: dormant_private as f64 / total as f64,
        zero_revenue_share: zero_revenue as f64 / total as f64,
        staffed_zero_revenue_share: staffed_zero_revenue as f64 / total as f64,
        unstaffed_share: unstaffed as f64 / total as f64,
        no_manufacturing_zero_revenue_share: no_manufacturing_zero_revenue as f64 / total as f64,
    }
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
#[ignore = "long-run profitability diagnostics (730 days, 2 x 2k)"]
fn generated_world_2x2k_730_days_reports_profitability_mix() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 2, 2_000, 730, 4);
    let invariants = world.validate_invariants();
    if !invariants.is_empty() {
        println!("invariant_violations={invariants:?}");
    }

    for country in &world.countries {
        println!(
            "country={} companies={} public={} staffed={} selling={} selling_unprofitable={} profitable_share={:.2}% private_profitable_share={:.2}% mature_private_profitable_share={:.2}% dormant_zero_activity={} unemployment={:.2}%",
            country.id,
            country.companies.len(),
            public_utility_count(country),
            staffed_company_count(country),
            selling_company_count(country),
            selling_but_unprofitable_count(country),
            profitable_share(country) * 100.0,
            profitable_private_share(country) * 100.0,
            mature_private_profitable_share(country) * 100.0,
            dormant_zero_activity_count(country),
            labor_force_unemployment(country) * 100.0,
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

#[test]
fn generated_world_2x1k_360_days_stock_market_is_two_sided() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 2, 1_000, 360, 2);
    assert!(
        world.validate_invariants().is_empty(),
        "invariant violations: {:?}",
        world.validate_invariants()
    );

    let mut ups = 0_u64;
    let mut downs = 0_u64;
    for country in &world.countries {
        let market = country.stock_market();
        assert!(
            market.index_value().is_finite() && market.index_value() >= 0.0,
            "country {}: index_value {} not finite/non-negative",
            country.id,
            market.index_value()
        );
        for equity in market.listed.values() {
            assert!(
                equity.last_price.is_finite() && equity.last_price > 0.0,
                "country {} {}: last_price {} invalid",
                country.id,
                equity.ticker,
                equity.last_price
            );
            let closes: Vec<f64> = equity.price_history.iter().copied().collect();
            for pair in closes.windows(2) {
                if pair[0] <= 0.0 {
                    continue;
                }
                let change = (pair[1] - pair[0]) / pair[0];
                if change > 1e-9 {
                    ups += 1;
                } else if change < -1e-9 {
                    downs += 1;
                }
            }
        }
    }

    assert!(downs > 0, "expected some down closes, got {downs}");
    // Regression guard for the one-way price ratchet: buy limits used to be
    // capped below the maker ask, making up-closes ~0.6% of all moves. Healthy
    // two-sided pricing keeps up-moves at least a meaningful share of downs.
    assert!(
        ups * 20 >= downs && ups > 0,
        "up-closes {ups} vs down-closes {downs}: market is a one-way ratchet again"
    );
}

#[test]
#[ignore = "manual diagnostic: prints profitability mix after 730 days (2 x 1k)"]
fn generated_world_2x1k_730_days_prints_profitability_mix() {
    let world = run_generated_world_in_pool(0x5eed_c0de, 2, 1_000, 730, 4);
    assert!(world.validate_invariants().is_empty());

    for country in &world.countries {
        let diagnostics = profitability_diagnostics(country);
        println!(
            "country={} companies={} unemployment={:.2}% profitable={:.2}% private_profitable={:.2}% public_profitable={:.2}% public_share={:.2}% dormant_private_share={:.2}% zero_revenue_share={:.2}% staffed_zero_revenue_share={:.2}% unstaffed_share={:.2}% no_manufacturing_zero_revenue_share={:.2}%",
            country.id,
            country.companies.len(),
            labor_force_unemployment(country) * 100.0,
            diagnostics.profitable_share * 100.0,
            diagnostics.profitable_private_share * 100.0,
            diagnostics.profitable_public_share * 100.0,
            diagnostics.public_share * 100.0,
            diagnostics.dormant_private_share * 100.0,
            diagnostics.zero_revenue_share * 100.0,
            diagnostics.staffed_zero_revenue_share * 100.0,
            diagnostics.unstaffed_share * 100.0,
            diagnostics.no_manufacturing_zero_revenue_share * 100.0,
        );
    }
}
