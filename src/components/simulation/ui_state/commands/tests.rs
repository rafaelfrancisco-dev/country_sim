use super::*;

use crate::sim::{country::Country, person::Person};

fn build_people(count: usize) -> Vec<Person> {
    (0..count)
        .map(|idx| Person::new(idx as u32, (idx % 10) as u8, 0, idx as u32 + 1))
        .collect()
}

fn sample_world() -> World {
    World::new(vec![Country::new(1, "A".to_string(), build_people(4))])
}

fn sample_world_two_countries() -> World {
    World::new(vec![
        Country::new(1, "A".to_string(), build_people(4)),
        Country::new(2, "B".to_string(), build_people(3)),
    ])
}

#[test]
fn chart_page_commands_clamp_to_valid_range() {
    let world = sample_world();
    let mut ui = UiState::default();

    for _ in 0..16 {
        ui.apply(UiCommand::ChartPageNext, &world);
    }
    assert_eq!(ui.chart_page, CHART_PAGE_COUNT - 1);

    for _ in 0..16 {
        ui.apply(UiCommand::ChartPagePrev, &world);
    }
    assert_eq!(ui.chart_page, 0);
}

#[test]
fn ensure_valid_clamps_chart_page() {
    let world = sample_world();
    let mut ui = UiState {
        chart_page: CHART_PAGE_COUNT + 5,
        ..UiState::default()
    };

    ui.ensure_valid(&world);
    assert_eq!(ui.chart_page, CHART_PAGE_COUNT - 1);
}

#[test]
fn ensure_valid_resets_chart_page_when_world_is_empty() {
    let world = World::new(vec![]);
    let mut ui = UiState {
        chart_page: 2,
        ..UiState::default()
    };

    ui.ensure_valid(&world);
    assert_eq!(ui.chart_page, 0);
}

#[test]
fn details_page_commands_adjust_and_close_resets() {
    let world = sample_world();
    let mut ui = UiState {
        details_view: DetailsView::Person,
        details_page: 0,
        ..UiState::default()
    };

    ui.apply(UiCommand::DetailsPageNext, &world);
    ui.apply(UiCommand::DetailsPageNext, &world);
    assert_eq!(ui.details_page, 1);

    ui.apply(UiCommand::DetailsPagePrev, &world);
    assert_eq!(ui.details_page, 0);

    ui.apply(UiCommand::CloseDetails, &world);
    assert_eq!(ui.details_view, DetailsView::List);
    assert_eq!(ui.details_page, 0);
}

#[test]
fn company_details_page_commands_clamp_to_page_three() {
    let world = sample_world();
    let mut ui = UiState {
        details_view: DetailsView::Company,
        details_page: 0,
        ..UiState::default()
    };

    for _ in 0..8 {
        ui.apply(UiCommand::DetailsPageNext, &world);
    }
    assert_eq!(ui.details_page, 3);

    ui.apply(UiCommand::DetailsPagePrev, &world);
    assert_eq!(ui.details_page, 2);
}

#[test]
fn bank_details_page_commands_clamp_to_page_three() {
    let world = sample_world();
    let mut ui = UiState {
        details_view: DetailsView::Bank,
        details_page: 0,
        ..UiState::default()
    };

    for _ in 0..8 {
        ui.apply(UiCommand::DetailsPageNext, &world);
    }
    assert_eq!(ui.details_page, 3);

    ui.apply(UiCommand::DetailsPagePrev, &world);
    assert_eq!(ui.details_page, 2);
}

#[test]
fn country_details_page_commands_remain_unbounded() {
    let world = sample_world();
    let mut ui = UiState {
        details_view: DetailsView::Country,
        details_page: 0,
        ..UiState::default()
    };

    ui.apply(UiCommand::DetailsPageNext, &world);
    ui.apply(UiCommand::DetailsPageNext, &world);
    assert_eq!(ui.details_page, 2);

    ui.apply(UiCommand::DetailsPagePrev, &world);
    assert_eq!(ui.details_page, 1);
}

#[test]
fn opening_details_resets_details_page() {
    let world = sample_world();
    let mut ui = UiState {
        selection_focus: SelectionFocus::People,
        details_page: 4,
        ..UiState::default()
    };

    ui.apply(UiCommand::OpenSelectedPersonDetails, &world);
    assert_eq!(ui.details_view, DetailsView::Person);
    assert_eq!(ui.details_page, 0);
}

#[test]
fn opening_selected_bank_details_switches_to_banks_tab() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.apply(UiCommand::OpenSelectedBankDetails, &world);
    assert_eq!(ui.active_tab, RunningTab::Banks);
    assert_eq!(ui.details_view, DetailsView::Bank);
    assert_eq!(ui.selection_focus, SelectionFocus::Bank);
    assert_eq!(ui.details_page, 0);
}

#[test]
fn changing_country_resets_details_page() {
    let world = sample_world_two_countries();
    let mut ui = UiState {
        details_view: DetailsView::Country,
        details_page: 6,
        people_page: 1,
        company_page: 1,
        bank_page: 1,
        selected_country: 0,
        ..UiState::default()
    };

    ui.apply(UiCommand::SelectNextCountryRow, &world);
    assert_eq!(ui.selected_country, 1);
    assert_eq!(ui.details_view, DetailsView::List);
    assert_eq!(ui.details_page, 0);
    assert_eq!(ui.people_page, 0);
    assert_eq!(ui.company_page, 0);
    assert_eq!(ui.bank_page, 0);
}

#[test]
fn events_category_cycles_and_resets_scroll() {
    let world = sample_world();
    let mut ui = UiState {
        events_scroll: 9,
        ..UiState::default()
    };

    ui.apply(UiCommand::EventsNextCategory, &world);
    assert_eq!(ui.events_category, EventsCategoryFilter::StockMarket);
    assert_eq!(ui.events_scroll, 0);

    for _ in 0..6 {
        ui.apply(UiCommand::EventsNextCategory, &world);
    }
    assert_eq!(ui.events_category, EventsCategoryFilter::All);
}

#[test]
fn country_policy_save_edit_emits_domain_command() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.apply(UiCommand::OpenSelectedCountryDetails, &world);
    ui.apply(UiCommand::OpenSelectedCountryPolicyDetails, &world);
    ui.apply(UiCommand::CountryPolicyStartEdit, &world);
    ui.country_policy_form.draft_input = "2500".to_string();

    let command = ui.apply(UiCommand::CountryPolicySaveEdit, &world);
    match command {
        Some(GameCommand::UpdateCountryPolicy {
            country_idx,
            policy,
        }) => {
            assert_eq!(country_idx, 0);
            assert!((policy.bracket_1_max - 2_500.0).abs() < f64::EPSILON);
        }
        other => panic!("expected UpdateCountryPolicy command, got {other:?}"),
    }
    assert!(!ui.country_policy_form.is_editing);
    assert!(ui.country_policy_form.validation_error.is_none());
}

#[test]
fn country_policy_invalid_value_keeps_editing_and_sets_error() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.apply(UiCommand::OpenSelectedCountryDetails, &world);
    ui.apply(UiCommand::OpenSelectedCountryPolicyDetails, &world);
    ui.apply(UiCommand::CountryPolicyStartEdit, &world);
    ui.country_policy_form.draft_input = "-1".to_string();

    let command = ui.apply(UiCommand::CountryPolicySaveEdit, &world);
    assert!(command.is_none());
    assert!(ui.country_policy_form.is_editing);
    assert!(ui.country_policy_form.validation_error.is_some());
}

#[test]
fn country_policy_allows_editing_mint_fields() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.apply(UiCommand::OpenSelectedCountryDetails, &world);
    ui.apply(UiCommand::OpenSelectedCountryPolicyDetails, &world);
    ui.country_policy_form.active_field = CountryPolicyField::MintMaxFractionExpectedSupply;
    ui.apply(UiCommand::CountryPolicyStartEdit, &world);
    ui.country_policy_form.draft_input = "0.1250".to_string();

    let command = ui.apply(UiCommand::CountryPolicySaveEdit, &world);
    let Some(GameCommand::UpdateCountryPolicy { policy, .. }) = command else {
        panic!("expected UpdateCountryPolicy command");
    };

    assert!((policy.mint_max_fraction_expected_supply - 0.125).abs() < f64::EPSILON);
}

#[test]
fn country_policy_rejects_mint_fraction_above_one() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.apply(UiCommand::OpenSelectedCountryDetails, &world);
    ui.apply(UiCommand::OpenSelectedCountryPolicyDetails, &world);
    ui.country_policy_form.active_field = CountryPolicyField::MintMaxFractionExpectedSupply;
    ui.apply(UiCommand::CountryPolicyStartEdit, &world);
    ui.country_policy_form.draft_input = "1.25".to_string();

    let command = ui.apply(UiCommand::CountryPolicySaveEdit, &world);
    assert!(command.is_none());
    assert_eq!(
        ui.country_policy_form.validation_error.as_deref(),
        Some("Mint max fraction must be between 0 and 1.")
    );
}

#[test]
fn country_policy_reset_managed_emits_defaults_command() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.apply(UiCommand::OpenSelectedCountryDetails, &world);
    ui.apply(UiCommand::OpenSelectedCountryPolicyDetails, &world);
    let command = ui.apply(UiCommand::CountryPolicyResetManaged, &world);

    let Some(GameCommand::UpdateCountryPolicy {
        country_idx,
        policy,
    }) = command
    else {
        panic!("expected UpdateCountryPolicy command");
    };
    let defaults = CountryPolicyOverrides::managed_defaults();

    assert_eq!(country_idx, 0);
    assert_eq!(policy, defaults);
}

#[test]
fn disaster_apply_emits_domain_command_with_selected_intensities() {
    let world = sample_world();
    let mut ui = UiState::default();

    ui.disaster_enabled[0] = true;
    ui.disaster_intensity_index[0] = 2;
    ui.disaster_enabled[3] = true;
    ui.disaster_intensity_index[3] = 1;

    let command = ui.apply(UiCommand::DisastersApply, &world);
    let Some(GameCommand::ApplyDisasters {
        country_idx,
        requests,
    }) = command
    else {
        panic!("expected ApplyDisasters command");
    };

    assert_eq!(country_idx, 0);
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].kind,
        crate::sim::DisasterKind::CompanyBankruptcyWave
    );
    assert_eq!(requests[0].intensity_pct, 25);
    assert_eq!(requests[1].kind, crate::sim::DisasterKind::BankReserveDrain);
    assert_eq!(requests[1].intensity_pct, 10);
}

#[test]
fn disaster_apply_without_selection_sets_status_and_emits_no_command() {
    let world = sample_world();
    let mut ui = UiState::default();

    let command = ui.apply(UiCommand::DisastersApply, &world);
    assert!(command.is_none());
    assert_eq!(
        ui.disaster_last_apply_status.as_deref(),
        Some("Enable at least one disaster before applying.")
    );
}
