use super::*;
use crate::sim::person::{ESSENTIAL_STOCK_SLOT_COUNT, HousingStatus};

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
        10 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    child.cash_balance = 0.0;
    child.housing = HousingStatus::Owner;
    child.essential_stock_units = [4.0; ESSENTIAL_STOCK_SLOT_COUNT];
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
        70 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    senior.cash_balance = 0.0;
    senior.housing = HousingStatus::Owner;
    senior.essential_stock_units = [4.0; ESSENTIAL_STOCK_SLOT_COUNT];
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
        unemployment_benefit_rate: 0.0,
        median_income_reference: 2_000.0,
        basic_needs_cash_threshold: threshold,
        ..Default::default()
    };

    let mut person = employed_adult(1, 200.0);
    person.last_month_unmet_needs_mask = 1;

    let mut country = make_gov_country(vec![person], 10_000.0, fiscal);

    country.process_month_end();

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
fn unemployed_with_unmet_essential_needs_can_receive_benefit_and_top_up_in_same_month() {
    let fiscal = FiscalParams {
        unemployment_benefit_rate: 0.2,
        median_income_reference: 1_000.0,
        basic_needs_cash_threshold: 500.0,
        ..Default::default()
    };
    let mut person = unemployed_adult(1, 100.0);
    person.last_month_unmet_needs_mask = 1;
    let mut country = make_gov_country(vec![person], 10_000.0, fiscal);

    country.process_month_end();

    assert!(
        (country.people[0].cash_balance - 500.0).abs() < 0.01,
        "expected stacked benefit+top-up to reach threshold; got {}",
        country.people[0].cash_balance
    );
    assert!(
        (country.monthly_gov_spending - 400.0).abs() < 0.01,
        "expected 200 benefit + 200 top-up; got {}",
        country.monthly_gov_spending
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
