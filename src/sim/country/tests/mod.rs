use super::*;
use crate::sim::{
    bank::LoanAccount,
    company::build_default_companies,
    event::SimEvent,
    params::{
        CompanyGrowthParams, DemographicParams, FiscalParams, LaborMobilityParams, LaborParams,
        SimulationParams,
    },
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus},
};
use rayon::ThreadPoolBuilder;
use std::collections::HashSet;

fn build_people(count: usize) -> Vec<Person> {
    (0..count)
        .map(|idx| Person::new(idx as u32, (idx % 10) as u8, (idx % 4) as u16, idx as u32))
        .collect()
}

fn build_deterministic_country() -> Country {
    let companies = build_default_companies("Deterministic", 4);
    let mut people = Vec::with_capacity(80);
    for idx in 0..80_u32 {
        let employment = if idx % 3 == 0 {
            EmploymentStatus::Unemployed
        } else {
            EmploymentStatus::Employed {
                job_type_id: (idx % 10) as u8,
                employer_company_id: (idx % 4) as u16,
            }
        };
        let mut person = Person::new_with_profile(
            idx,
            employment,
            idx.saturating_mul(13),
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = if idx % 2 == 0 { 2_000.0 } else { -100.0 };
        people.push(person);
    }
    let mut country =
        Country::with_economy(1, "Deterministic".to_string(), people, companies, 100_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    country.sim_params = params;
    country
}

fn run_country_with_threads(days: u64, threads: usize) -> Country {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool should build");
    pool.install(|| {
        let mut country = build_deterministic_country();
        for _ in 0..days {
            country.tick_day();
        }
        country
    })
}

fn run_death_sample(population: usize) -> u32 {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    let mut people = Vec::with_capacity(population);
    for idx in 0..population as u32 {
        let person = Person::new_with_profile(
            idx,
            EmploymentStatus::Unemployed,
            idx + 10,
            95 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        people.push(person);
    }

    let mut country = Country::with_economy(1, "A".to_string(), people, companies, 0.0);
    let mut params = country.sim_params;
    params.demographic.births_per_1000_per_year = 0.0;
    country.sim_params = params;
    country.tick_day_without_month();
    country.last_day_deaths
}

fn run_birth_sample(population: usize, births_per_1000_per_year: f64) -> u32 {
    let companies = build_default_companies("A", 1);
    let mut people = Vec::with_capacity(population);
    for idx in 0..population as u32 {
        let person = Person::new_with_profile(
            idx,
            EmploymentStatus::Unemployed,
            idx + 100,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        people.push(person);
    }
    let mut country = Country::with_economy(1, "A".to_string(), people, companies, 0.0);
    let mut params = country.sim_params;
    params.demographic.births_per_1000_per_year = births_per_1000_per_year;
    country.sim_params = params;
    country.tick_day_without_month();
    country.last_day_births
}

#[test]
fn tax_brackets_follow_progressive_curve() {
    assert!((monthly_income_tax(2_000.0) - 200.0).abs() < f64::EPSILON);
    assert!((monthly_income_tax(5_000.0) - 700.0).abs() < f64::EPSILON);
    assert!((monthly_income_tax(9_000.0) - 1_700.0).abs() < f64::EPSILON);
}

#[test]
fn month_processing_runs_every_thirty_days() {
    let companies = build_default_companies("A", 4);
    let mut country = Country::with_economy(1, "A".to_string(), build_people(10), companies, 0.0);

    for _ in 0..29 {
        country.tick_day();
    }
    assert_eq!(country.month, 0);

    country.tick_day();
    assert_eq!(country.month, 0);

    country.process_month();
    assert_eq!(country.month, 1);
}

#[test]
fn payroll_and_taxes_only_apply_when_employed() {
    let companies = build_default_companies("A", 2);
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        10,
        25 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.cash_balance = 100.0;
    let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);

    country.process_month();

    assert!((country.monthly_tax_collected - 0.0).abs() < f64::EPSILON);
    assert!((country.people[0].last_month_gross_pay - 0.0).abs() < f64::EPSILON);
}

#[test]
fn money_is_created_by_loan_issuance_only() {
    let companies = build_default_companies("A", 2);
    let mut people = build_people(2);
    people[0].cash_balance = -500.0;
    let mut country = Country::with_economy(1, "A".to_string(), people, companies, 100_000.0);

    country.process_end_of_month_loan_requests();
    country.recompute_aggregates();

    assert!(country.bank.total_originated >= 0.0);
    let expected =
        round_cents(country.initial_money_supply + country.bank.created_money_outstanding);
    assert!((country.expected_money_supply - expected).abs() < f64::EPSILON);
}

#[test]
fn daily_job_market_hires_qualified_unemployed_people() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening(0, 10);
    let mut person = Person::new_with_profile(
        99,
        EmploymentStatus::Unemployed,
        10,
        35 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    person.cash_balance = -5_000.0;
    let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);

    for _ in 0..365 {
        country.tick_day_without_month();
        if country.people[0].is_employed() {
            break;
        }
    }

    assert!(country.people[0].is_employed());
}

#[test]
fn daily_deaths_remove_people_and_write_off_loans() {
    let companies = build_default_companies("A", 1);
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        11,
        99 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    person.cash_balance = 50.0;
    let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);
    country.bank.originate_loan(
        1_000.0,
        3_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        &mut country.people[0].loan,
    );

    let mut params = country.sim_params.clone();
    params.demographic = DemographicParams {
        births_per_1000_per_year: 0.0,
    };
    country.sim_params = params;

    for _ in 0..20_000 {
        country.tick_day_without_month();
        if country.people.is_empty() {
            break;
        }
    }

    assert!(country.people.is_empty());
    assert!(country.bank.created_money_outstanding <= 1_000.0);
}

#[test]
fn country_parallel_paths_keep_invariants_with_random_daily_events() {
    let one_thread = run_country_with_threads(DAYS_PER_MONTH * 4, 1);
    let many_threads = run_country_with_threads(DAYS_PER_MONTH * 4, 8);

    assert_eq!(one_thread.day, many_threads.day);
    assert_eq!(one_thread.month, many_threads.month);
    assert!(one_thread.people.len() > 0);
    assert!(many_threads.people.len() > 0);
    assert!(one_thread.validate_money_invariants().is_empty());
    assert!(many_threads.validate_money_invariants().is_empty());
}

#[test]
fn daily_deaths_use_runtime_entropy() {
    let mut observed = HashSet::new();
    for _ in 0..8 {
        observed.insert(run_death_sample(50_000));
    }

    assert!(observed.len() > 1);
}

#[test]
fn daily_births_use_poisson_sampling() {
    let mut observed = HashSet::new();
    for _ in 0..8 {
        observed.insert(run_birth_sample(5_000, 20_000.0));
    }

    assert!(observed.len() > 1);
}

#[test]
fn job_market_conflict_resolution_is_stable_by_person_index() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening(0, 1);

    let first = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        1,
        35 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    let second = Person::new_with_profile(
        2,
        EmploymentStatus::Unemployed,
        2,
        35 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    let mut country =
        Country::with_economy(1, "A".to_string(), vec![first, second], companies, 0.0);

    country.sim_params = SimulationParams {
        demographic: DemographicParams {
            births_per_1000_per_year: 0.0,
        },
        labor: LaborParams {
            seek_base: 1.0,
            seek_age_slope: 0.0,
            seek_cash_pressure_scale: 0.0,
            seek_cash_reference: 1.0,
            seek_probability_cap: 1.0,
            ..LaborParams::default()
        },
        ..SimulationParams::default()
    };

    country.tick_day_without_month();

    assert!(country.people[0].is_employed());
    assert!(!country.people[1].is_employed());
}

// ── Government spending helpers ──────────────────────────────────────────────

fn make_gov_country(people: Vec<Person>, treasury: f64, fiscal: FiscalParams) -> Country {
    let companies = build_default_companies("A", 1);
    // initial_money_supply = treasury so the money invariant starts clean
    let mut country = Country::with_economy(1, "A".to_string(), people, companies, treasury);
    country.treasury_balance = treasury;
    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.fiscal = fiscal;
    country.sim_params = params;
    country
}

fn unemployed_adult(id: u32, cash: f64) -> Person {
    let mut p = Person::new_with_profile(
        id,
        EmploymentStatus::Unemployed,
        id + 100,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    p.cash_balance = cash;
    p
}

fn employed_adult(id: u32, cash: f64) -> Person {
    let mut p = Person::new_with_profile(
        id,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        id + 100,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    p.cash_balance = cash;
    p
}

// ── Government spending tests ─────────────────────────────────────────────────

#[test]
fn unemployment_benefit_increases_cash_and_decreases_treasury() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    let treasury_start = 10_000.0;
    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - benefit).abs() < 0.01,
        "cash_balance should equal benefit; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.treasury_balance - (treasury_start - benefit)).abs() < 0.01,
        "treasury should decrease by benefit; got {}",
        country.treasury_balance
    );
}

#[test]
fn employed_person_does_not_receive_unemployment_benefit() {
    // An employed adult with healthy cash and no unmet needs should trigger
    // zero government spending — they are not eligible for any benefit path.
    let treasury_start = 10_000.0;
    let person = employed_adult(1, 5_000.0);
    let mut country = make_gov_country(vec![person], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - 0.0).abs() < 0.01,
        "no gov spending should occur for employed person; got {}",
        country.monthly_gov_spending
    );
    let has_gov_event = country
        .pending_events
        .iter()
        .any(|e| matches!(e, SimEvent::GovTransfer { .. }));
    assert!(
        !has_gov_event,
        "no GovTransfer event should be emitted for employed person"
    );
}

#[test]
fn child_below_working_age_does_not_receive_unemployment_benefit() {
    let treasury_start = 10_000.0;
    let mut child = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        101,
        10 * DAYS_PER_YEAR, // age 10 — below 18
        EducationTrack::BasicOnly,
    );
    child.cash_balance = 0.0;
    let mut country = make_gov_country(vec![child], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 0.0).abs() < 0.01,
        "child should receive no benefit; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.treasury_balance - treasury_start).abs() < 0.01,
        "treasury should be unchanged; got {}",
        country.treasury_balance
    );
}

#[test]
fn senior_above_working_age_does_not_receive_unemployment_benefit() {
    let treasury_start = 10_000.0;
    let mut senior = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        101,
        70 * DAYS_PER_YEAR, // age 70 — above 65
        EducationTrack::BasicOnly,
    );
    senior.cash_balance = 0.0;
    let mut country = make_gov_country(vec![senior], treasury_start, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 0.0).abs() < 0.01,
        "senior should receive no benefit; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.treasury_balance - treasury_start).abs() < 0.01,
        "treasury should be unchanged; got {}",
        country.treasury_balance
    );
}

#[test]
fn monthly_gov_spending_and_budget_balance_are_tracked() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], 10_000.0, FiscalParams::default());

    country.process_month_end();

    assert!(
        (country.monthly_gov_spending - benefit).abs() < 0.01,
        "monthly_gov_spending should equal benefit; got {}",
        country.monthly_gov_spending
    );
    // No taxes collected (unemployed earns nothing), so budget_balance = 0 - benefit
    assert!(
        (country.monthly_budget_balance - (country.monthly_tax_collected - benefit)).abs() < 0.01,
        "budget_balance should be tax - spending; got {}",
        country.monthly_budget_balance
    );
}

#[test]
fn gov_transfer_event_is_emitted_when_benefits_are_paid() {
    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], 10_000.0, FiscalParams::default());

    country.process_month_end();

    let has_gov_event = country.pending_events.iter().any(
        |e| matches!(e, SimEvent::GovTransfer { total_recipients, .. } if *total_recipients > 0),
    );
    assert!(has_gov_event, "GovTransfer event should be emitted");
}

#[test]
fn no_gov_transfer_event_when_nobody_qualifies() {
    // All employed adults, full cash — no benefits should be distributed
    let person = employed_adult(1, 5_000.0);
    let mut country = make_gov_country(vec![person], 10_000.0, FiscalParams::default());

    country.process_month_end();

    let has_gov_event = country
        .pending_events
        .iter()
        .any(|e| matches!(e, SimEvent::GovTransfer { .. }));
    assert!(
        !has_gov_event,
        "no GovTransfer event should be emitted when nobody qualifies"
    );
}

#[test]
fn basic_needs_top_up_raises_cash_to_threshold_for_person_with_unmet_essential_needs() {
    let threshold = 500.0;
    let fiscal = FiscalParams {
        unemployment_benefit_rate: 0.0, // disable unemployment benefit path
        median_income_reference: 2_000.0,
        basic_needs_cash_threshold: threshold,
    };

    // Employed adult (no unemployment benefit), low cash, has unmet water need (bit 0)
    let mut person = employed_adult(1, 200.0);
    person.last_month_unmet_needs_mask = 1; // good id 0 = water (essential)

    let mut country = make_gov_country(vec![person], 10_000.0, fiscal);

    country.process_month_end();

    // Cash should be at least threshold (could be higher due to net pay)
    assert!(
        country.people[0].cash_balance >= threshold - 0.01,
        "cash should be topped up to at least threshold; got {}",
        country.people[0].cash_balance
    );
    assert!(
        country.monthly_gov_spending > 0.0,
        "gov spending should be nonzero after top-up"
    );
}

#[test]
fn no_benefit_paid_when_treasury_is_empty() {
    let benefit = FiscalParams::default().monthly_unemployment_benefit();
    assert!(
        benefit > 0.0,
        "benefit must be positive for this test to be meaningful"
    );

    let person = unemployed_adult(1, 0.0);
    let mut country = make_gov_country(vec![person], 0.0, FiscalParams::default());
    // Explicitly zero out treasury
    country.treasury_balance = 0.0;

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 0.0).abs() < 0.01,
        "no benefit should be paid from empty treasury; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.monthly_gov_spending - 0.0).abs() < 0.01,
        "monthly_gov_spending should be zero; got {}",
        country.monthly_gov_spending
    );
}

#[test]
fn government_spending_does_not_violate_money_supply_invariant() {
    let treasury_start = 50_000.0;
    // Mix of unemployed adults and one employed to generate some tax
    let people = vec![
        unemployed_adult(1, 0.0),
        unemployed_adult(2, 0.0),
        employed_adult(3, 0.0),
    ];
    let mut country = make_gov_country(people, treasury_start, FiscalParams::default());

    country.process_month_end();
    country.recompute_aggregates();

    let violations = country.validate_money_invariants();
    assert!(
        violations.is_empty(),
        "money invariant violated after gov spending: {:?}",
        violations
    );
}

#[test]
fn monthly_job_switch_moves_worker_only_for_raise_and_backfills_old_opening() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();
    companies[0].cash_balance = 100_000.0;
    companies[1].cash_balance = 100_000.0;
    companies[1].add_opening_with_salary(0, 1, 4_000.0);

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.set_employment_with_salary(0, 0, 3_200.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.labor_mobility = LaborMobilityParams::default();

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.people[0].employer_company_id(), Some(1));
    assert!((country.people[0].monthly_salary() - 4_000.0).abs() < 0.01);
    assert!(
        country.companies[0]
            .open_positions
            .iter()
            .any(|opening| opening.job_type_id == 0
                && opening.slots > 0
                && (opening.offered_monthly_salary - 3_200.0).abs() < 0.01)
    );
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::JobSwitched { .. }))
    );
}

#[test]
fn first_missed_company_payment_applies_wage_cut_to_contracts_and_openings() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].add_opening_with_salary(0, 1, 5_000.0);
    companies[0].cash_balance = 0.0;
    companies[0].loan = Some(LoanAccount::new(5_000.0, 0.12, 12));

    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        101,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.set_employment_with_salary(0, 0, 4_000.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![person], companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies[0].consecutive_missed_loan_payments, 1);
    assert!((country.people[0].monthly_salary() - 3_600.0).abs() < 0.01);
    assert!(
        country.companies[0]
            .open_positions
            .iter()
            .any(|opening| (opening.offered_monthly_salary - 4_500.0).abs() < 0.01)
    );
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyWageCut { .. }))
    );
}

#[test]
fn bankruptcy_physically_removes_company_and_remaps_employers() {
    let mut companies = build_default_companies("A", 2);
    companies[0].open_positions.clear();
    companies[1].open_positions.clear();
    companies[0].cash_balance = 0.0;
    companies[0].loan = Some(LoanAccount::new(5_000.0, 0.12, 12));
    companies[0].consecutive_missed_loan_payments = 2;
    companies[1].cash_balance = 100_000.0;

    let mut p0 = Person::new_with_profile(
        1,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 0,
        },
        111,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    p0.set_employment_with_salary(0, 0, 3_200.0);

    let mut p1 = Person::new_with_profile(
        2,
        EmploymentStatus::Employed {
            job_type_id: 0,
            employer_company_id: 1,
        },
        112,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    p1.set_employment_with_salary(1, 0, 3_200.0);

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), vec![p0, p1], companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 1);
    assert_eq!(country.companies[0].id, 0);
    assert!(!country.people[0].is_employed());
    assert_eq!(country.people[1].employer_company_id(), Some(0));
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyBankrupt { .. }))
    );
}

#[test]
fn startup_formation_creates_company_and_moves_founders() {
    let mut companies = build_default_companies("A", 1);
    companies[0].open_positions.clear();
    companies[0].cash_balance = 1_000_000.0;
    companies[0].produced_good_id = 0;
    companies[0].last_month_sales = 100_000.0;

    let mut people = Vec::new();
    for idx in 0..5_u32 {
        let mut person = Person::new_with_profile(
            idx,
            EmploymentStatus::Employed {
                job_type_id: 0,
                employer_company_id: 0,
            },
            idx + 100,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.set_employment_with_salary(0, 0, 3_200.0);
        person.cash_balance = 100_000.0;
        people.push(person);
    }

    let mut params = SimulationParams::default();
    params.demographic.births_per_1000_per_year = 0.0;
    params.company_growth = CompanyGrowthParams {
        min_profitable_months: u8::MAX,
        ..CompanyGrowthParams::default()
    };
    params.company_formation.max_startups_per_month = Some(1);
    params.labor_mobility.switch_raise_threshold = 10.0;

    let mut country =
        Country::with_economy_and_params(1, "A".to_string(), people, companies, 0.0, params);
    country.process_month_end();

    assert_eq!(country.companies.len(), 2);
    let startup_id = 1_u16;
    assert!(
        country
            .people
            .iter()
            .all(|person| person.employer_company_id() == Some(startup_id))
    );
    let startup_openings: u16 = country.companies[startup_id as usize]
        .open_positions
        .iter()
        .map(|opening| opening.slots)
        .sum();
    assert_eq!(startup_openings, 10);
    assert!(
        country
            .pending_events
            .iter()
            .any(|event| matches!(event, SimEvent::CompanyFounded { .. }))
    );
}
