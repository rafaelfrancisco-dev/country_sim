use std::collections::VecDeque;

use super::*;
use crate::sim::{tickable::Tickable, world::World};
#[test]
fn startup_can_form_with_unemployed_founders_when_enabled() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 7;
    companies[0].unit_price = 220.0;
    companies[0].last_month_sales = 120_000.0;
    companies[0].last_month_profit = 55_000.0;
    companies[0].sales_history = VecDeque::from(vec![110_000.0, 120_000.0, 125_000.0]);

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
    companies[0].sales_history = VecDeque::from(vec![900.0, 1_000.0, 1_100.0]);

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
fn transport_startup_produces_output_in_first_operating_month() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 7;
    companies[0].unit_price = 220.0;
    companies[0].inventory_units = 5_000;
    companies[0].last_month_sales = 100_000.0;
    companies[0].last_month_profit = 45_000.0;
    companies[0].sales_history = VecDeque::from(vec![95_000.0, 100_000.0, 105_000.0]);

    let mut people = Vec::new();
    for idx in 0..3_u32 {
        let mut founder = Person::new_with_profile(
            idx + 1,
            EmploymentStatus::Unemployed,
            idx + 900,
            30 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        founder.cash_balance = 200_000.0;
        people.push(founder);
    }
    for idx in 0..250_u32 {
        let mut consumer = Person::new_with_profile(
            idx + 10,
            EmploymentStatus::Unemployed,
            idx + 1_200,
            30 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        consumer.cash_balance = 50_000.0;
        people.push(consumer);
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
    params.company_formation.startup_bank_loan_enabled = false;
    params.labor_mobility.switch_raise_threshold = 10.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month();

    assert_eq!(country.companies.len(), 2);
    assert_eq!(country.companies[1].produced_good_id, 7);
    assert!(country.people.iter().take(3).all(|person| {
        person.employer_company_id() == Some(1) && person.job_type_id() == Some(3)
    }));

    let mut world = World::new(vec![country]);
    world.countries[0]
        .sim_params
        .company_formation
        .max_startups_per_month = Some(0);
    for _ in 0..30 {
        world.tick_day();
    }

    let startup = &world.countries[0].companies[1];
    assert!(startup.last_completed_month_units_produced > 0);
}

#[test]
fn startup_initial_hiring_is_capped_to_a_single_opening() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 500_000.0;
    companies[0].produced_good_id = 7;
    companies[0].unit_price = 220.0;
    companies[0].last_month_sales = 120_000.0;
    companies[0].last_month_profit = 55_000.0;
    companies[0].sales_history = VecDeque::from(vec![110_000.0, 120_000.0, 125_000.0]);

    let mut people = Vec::new();
    for idx in 0..3_u32 {
        let mut person = Person::new_with_profile(
            idx + 1,
            EmploymentStatus::Unemployed,
            idx + 2_000,
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

    let startup = &country.companies[1];
    let startup_slots: u16 = startup.open_positions.iter().map(|opening| opening.slots).sum();
    assert_eq!(startup_slots, 1);
    assert_eq!(startup.open_positions[0].job_type_id, 3);
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

#[test]
fn public_essential_company_scales_manufacturing_openings_with_population_need() {
    let people = (0..10_000_u32)
        .map(|idx| {
            Person::new_with_profile(
                idx + 1,
                EmploymentStatus::Unemployed,
                idx + 700,
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
        essential_good_max_id_inclusive: 0,
        ..GovernmentBackfillParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, vec![], 500_000.0, params);
    country.treasury_balance = 500_000.0;
    country.create_public_essential_company(0, country.sim_params.government_backfill);

    let company = country
        .companies
        .iter()
        .find(|company| company.is_public_utility)
        .expect("public utility should exist");
    let manufacturing_opening = company
        .open_positions
        .iter()
        .find(|opening| opening.job_type_id == 5)
        .expect("manufacturing openings should exist");

    assert!(
        manufacturing_opening.slots
            > country
                .sim_params
                .government_backfill
                .manufacturing_openings,
        "manufacturing openings should scale above the fixed backfill floor"
    );
}
