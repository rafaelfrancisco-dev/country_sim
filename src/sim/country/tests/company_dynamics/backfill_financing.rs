use super::*;
#[test]
fn startup_can_form_with_unemployed_founders_when_enabled() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 0;
    companies[0].last_month_sales = 500_000.0;
    companies[0].last_month_profit = 200_000.0;
    companies[0].sales_history = vec![450_000.0, 500_000.0, 520_000.0];

    let mut people = Vec::new();
    for idx in 0..3_u32 {
        let mut person = Person::new_with_profile(
            idx + 1,
            EmploymentStatus::Unemployed,
            idx + 200,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = 150_000.0;
        people.push(person);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.company_formation.max_startups_per_month = Some(1);
    params.company_formation.min_founders = 3;
    params.company_formation.max_founders = 3;
    params.company_formation.allow_unemployed_founders = true;
    params.company_formation.labor_slack_startup_trigger = 0.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 2);
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyFounded { .. }))
    );
}

#[test]
fn startup_viability_gate_blocks_non_viable_plans() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 0;
    companies[0].last_month_sales = 1_000.0;
    companies[0].last_month_profit = 400.0;
    companies[0].sales_history = vec![900.0, 1_000.0, 1_100.0];

    let mut people = Vec::new();
    for idx in 0..3_u32 {
        let mut person = Person::new_with_profile(
            idx + 1,
            EmploymentStatus::Unemployed,
            idx + 400,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = 200_000.0;
        people.push(person);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.government_backfill.enabled = false;
    params.company_formation.max_startups_per_month = Some(1);
    params.company_formation.min_founders = 3;
    params.company_formation.max_founders = 3;
    params.company_formation.allow_unemployed_founders = true;
    params.company_formation.startup_max_payroll_to_revenue = 0.05;
    params.company_formation.labor_slack_startup_trigger = 0.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert!(
        !country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyFounded { .. }))
    );
}

#[test]
fn government_backfill_creates_company_for_each_missing_essential_good() {
    let companies = build_default_companies("A", 1);
    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 4,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], companies, 200_000.0, params);
    country.treasury_balance = 200_000.0;
    country.process_month_end();

    let funded_goods = country
        .companies
        .iter()
        .filter(|company| company.is_essential_backfill)
        .map(|company| company.produced_good_id)
        .collect::<HashSet<_>>();
    assert_eq!(funded_goods.len(), 4);
    for good_id in 1..=4_u8 {
        assert!(funded_goods.contains(&good_id));
    }

    let gov_company_events = country
        .pending_events
        .iter()
        .filter(|event| matches!(event, SimEvent::GovCompanyFunded { .. }))
        .count();
    assert_eq!(gov_company_events, 4);
}

#[test]
fn government_backfill_financing_uses_treasury_then_deficit() {
    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 0,
        openings_total: 3,
        manufacturing_openings: 2,
        runway_months: 1.5,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![], vec![], 1_000.0, params);
    country.treasury_balance = 1_000.0;
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert!(country.companies[0].is_public_utility);
    assert!(country.gov_debt > 0.0);
    assert!(country.primary_bank_mut().created_money_outstanding > 0.0);
    assert!(country.treasury_balance.abs() < 0.01);
    assert!(country.validate_money_invariants().is_empty());
}

#[test]
fn government_backfill_adds_capacity_when_essential_shortage_is_persistent() {
    let companies = build_default_companies("A", 5);
    let people = (0..50_u32)
        .map(|idx| {
            Person::new_with_profile(
                idx + 1,
                EmploymentStatus::Unemployed,
                idx + 500,
                30 * DAYS_PER_YEAR,
                EducationTrack::HighSchool,
            )
        })
        .collect::<Vec<_>>();

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_formation.max_startups_per_month = Some(0);
    params.government_backfill = GovernmentBackfillParams {
        enabled: true,
        essential_good_max_id_inclusive: 4,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 500_000.0, params);
    country.treasury_balance = 500_000.0;
    country.monthly_unmet_need_events = 200;

    country.process_month_end();

    assert!(country.companies.len() > 5);
    assert!(
        country
            .companies
            .iter()
            .any(|company| company.is_essential_backfill),
        "high shortage pressure should trigger extra public essential capacity"
    );
}
