use std::{collections::VecDeque, sync::Arc};

use super::*;

#[test]
fn company_related_events_preserve_matches_for_historical_company_ids() {
    let mut world = World::new(vec![Country::new(
        1,
        "A".to_string(),
        vec![Person::new(1, 30, 0, 11)],
    )]);
    world.countries[0].companies[0].remember_historical_id(1);
    world.event_log = VecDeque::from(vec![SimEvent::JobSwitched {
        country_name: Arc::from("A"),
        person_id: 1,
        from_company_id: 1,
        to_company_id: 2,
        monthly_salary: 100.0,
        day: 4,
    }]);

    let ui = UiState {
        active_tab: RunningTab::Goods,
        details_view: DetailsView::Company,
        events_category: EventsCategoryFilter::CompaniesLabor,
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Goods(goods_tab) = snapshot.tab else {
        panic!("goods tab payload expected");
    };
    assert_eq!(goods_tab.company_related_events.events_total_count, 1);
    assert!(
        goods_tab.company_related_events.event_log_lines[0].contains("Company #1 -> #2"),
        "expected historical company id to match related events"
    );
}

#[test]
fn company_workers_rows_include_tenure_and_sort_by_longest_tenure() {
    use crate::sim::person::{EducationTrack, EmploymentStatus, DAYS_PER_YEAR};

    let mut long_tenure = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        111,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    long_tenure.set_employment_with_salary(0, 0, 2_900.0, 10);

    let mut short_tenure = Person::new_with_profile(
        2,
        EmploymentStatus::Unemployed,
        222,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    short_tenure.set_employment_with_salary(0, 1, 4_200.0, 70);

    let country = Country::new(1, "A".to_string(), vec![short_tenure, long_tenure]);
    let mut world = World::new(vec![country]);
    world.day = 100;

    let ui = UiState {
        active_tab: RunningTab::Goods,
        details_view: DetailsView::Company,
        ..UiState::default()
    };
    let snapshot = snapshot_from_world_with_ui(world, ui);

    let RunningTabSnapshot::Goods(goods_tab) = snapshot.tab else {
        panic!("goods tab payload expected");
    };
    assert_eq!(goods_tab.company_worker_rows.len(), 2);
    assert_eq!(goods_tab.company_worker_rows[0].tenure_months, "3");
    assert_eq!(goods_tab.company_worker_rows[1].tenure_months, "1");
}
