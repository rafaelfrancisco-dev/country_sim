use super::*;

#[test]
fn benefit_not_paid_after_duration_cap_exceeded() {
    let fiscal = FiscalParams {
        max_benefit_months: 2,
        ..FiscalParams::default()
    };
    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 2;
    let treasury_start = 50_000.0;

    let mut country = make_gov_country(vec![person], treasury_start, fiscal);
    country.sim_params.banking.public_bank_recapitalize_amount = f64::MAX;
    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - 0.0).abs() < 0.01,
        "no gov spending should occur after duration cap; got {}",
        country.monthly_gov_spending
    );
    assert!(
        country.people[0].months_unemployed > fiscal.max_benefit_months,
        "months_unemployed should exceed cap; got {}",
        country.people[0].months_unemployed
    );
}

#[test]
fn benefit_paid_while_within_duration_cap() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    let fiscal = FiscalParams {
        max_benefit_months: 12,
        ..FiscalParams::default()
    };
    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 5;
    let treasury_start = 50_000.0;

    let mut country = make_gov_country(vec![person], treasury_start, fiscal);
    country.sim_params.banking.public_bank_recapitalize_amount = f64::MAX;
    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - benefit).abs() < 0.01,
        "benefit should be paid within duration cap; got {}",
        country.monthly_gov_spending
    );
    assert!(
        country.people[0].months_unemployed <= fiscal.max_benefit_months,
        "months_unemployed should be within cap; got {}",
        country.people[0].months_unemployed
    );
}

#[test]
fn months_unemployed_resets_to_zero_on_employment() {
    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 10;

    let employed = employed_adult(2, 0.0);

    let fiscal = FiscalParams::default();
    let treasury_start = 50_000.0;
    let mut country = make_gov_country(vec![person, employed], treasury_start, fiscal);

    country.process_month_end();

    assert_eq!(
        country.people[1].months_unemployed, 0,
        "employed person months_unemployed should reset to 0"
    );
    assert!(
        country.people[0].months_unemployed > 0,
        "unemployed person months_unemployed should be incremented"
    );
}

#[test]
fn austerity_activates_when_debt_exceeds_gdp_threshold() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].last_month_sales = 1_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.fiscal.austerity_debt_gdp_threshold = 0.80;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    country.gov_debt = 10_000.0;

    country.process_month_end();

    assert!(
        country.austerity_active,
        "austerity should be active when debt/GDP > threshold"
    );
}

#[test]
fn austerity_does_not_activate_below_threshold() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].last_month_sales = 1_000_000.0;

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.government_backfill.enabled = false;
    params.fiscal.austerity_debt_gdp_threshold = 0.80;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 0.0, params);
    country.gov_debt = 1_000.0;

    country.process_month_end();

    assert!(
        !country.austerity_active,
        "austerity should not be active below threshold"
    );
}

#[test]
fn austerity_reduces_benefit_payout() {
    let fiscal = FiscalParams {
        austerity_benefit_reduction: 0.30,
        austerity_debt_gdp_threshold: 0.80,
        ..FiscalParams::default()
    };
    let base_benefit = fiscal.monthly_unemployment_benefit();
    let expected_reduced = round_cents(base_benefit * 0.70);

    let mut person = unemployed_adult(1, 0.0);
    person.months_unemployed = 0;
    let treasury_start = 50_000.0;

    let mut country = make_gov_country(vec![person], treasury_start, fiscal);
    country.sim_params.banking.public_bank_recapitalize_amount = f64::MAX;
    country.gov_debt = 1_000_000.0;
    country.companies[0].last_month_sales = 100.0;

    country.process_month_end();

    assert!(country.austerity_active, "austerity should be active");
    assert!(
        (country.monthly_gov_spending - expected_reduced).abs() < 0.01,
        "benefit should be reduced 30% under austerity; expected {expected_reduced}, got {}",
        country.monthly_gov_spending
    );
}
