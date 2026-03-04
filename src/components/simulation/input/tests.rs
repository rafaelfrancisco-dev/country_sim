use super::*;

#[test]
fn running_keymap_maps_tab_and_chart_controls() {
    let simulation = Simulation::new();
    let list_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Main,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    let charts_state = RunningInputState {
        active_tab: RunningTab::Charts,
        ..list_state
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Tab), list_state),
        Some(SimulationCommand::Ui(UiCommand::NextTab))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::BackTab), list_state),
        Some(SimulationCommand::Ui(UiCommand::PrevTab))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('t')), list_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('t')), charts_state),
        Some(SimulationCommand::Ui(UiCommand::ToggleChartScope))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('m')), list_state),
        Some(SimulationCommand::Ui(UiCommand::ToggleSimMode))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Left), list_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('a')), list_state),
        Some(SimulationCommand::Ui(UiCommand::FocusCountrySelection))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), list_state),
        Some(SimulationCommand::Ui(UiCommand::SelectPrevCountryRow))
    );
}

#[test]
fn running_keymap_maps_person_details_company_jump() {
    let simulation = Simulation::new();
    let state = RunningInputState {
        details_view: DetailsView::Person,
        details_page: 0,
        selection_focus: SelectionFocus::People,
        active_tab: RunningTab::Main,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('c')), state),
        Some(SimulationCommand::Ui(UiCommand::JumpToEmployerCompany))
    );
}

#[test]
fn chart_and_events_remap_applies_strict_keys() {
    let simulation = Simulation::new();
    let chart_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Charts,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char(',')), chart_state),
        Some(SimulationCommand::Ui(UiCommand::ChartPrevCountry))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('.')), chart_state),
        Some(SimulationCommand::Ui(UiCommand::ChartNextCountry))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('<')), chart_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageUp), chart_state),
        Some(SimulationCommand::Ui(UiCommand::ChartPagePrev))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), chart_state),
        Some(SimulationCommand::Ui(UiCommand::ChartPageNext))
    );

    let events_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Events,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('o')), events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsToggleSort))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('f')), events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsNextCategory))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsScrollDown))
    );
}

#[test]
fn events_search_mode_keeps_characters_for_query() {
    let simulation = Simulation::new();
    let events_search_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Events,
        events_search_active: true,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('f')), events_search_state),
        Some(SimulationCommand::Ui(UiCommand::EventsSearchChar('f')))
    );
}

#[test]
fn detail_page_keys_map_in_main_and_goods_details_views() {
    let simulation = Simulation::new();

    let main_list = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::People,
        active_tab: RunningTab::Main,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), main_list),
        Some(SimulationCommand::Ui(UiCommand::PeoplePageNext))
    );

    let main_person_details = RunningInputState {
        details_view: DetailsView::Person,
        ..main_list
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), main_person_details),
        Some(SimulationCommand::Ui(UiCommand::DetailsPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageUp), main_person_details),
        Some(SimulationCommand::Ui(UiCommand::DetailsPagePrev))
    );

    let goods_company_details = RunningInputState {
        details_view: DetailsView::Company,
        details_page: 0,
        selection_focus: SelectionFocus::Company,
        active_tab: RunningTab::Goods,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), goods_company_details),
        Some(SimulationCommand::Ui(UiCommand::DetailsPageNext))
    );
}

#[test]
fn country_policy_keymap_uses_form_mode_controls() {
    let simulation = Simulation::new();
    let country_details = RunningInputState {
        details_view: DetailsView::Country,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Main,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('o')), country_details),
        Some(SimulationCommand::Ui(
            UiCommand::OpenSelectedCountryPolicyDetails
        ))
    );

    let policy_view = RunningInputState {
        details_view: DetailsView::CountryPolicy,
        country_policy_is_editing: false,
        ..country_details
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), policy_view),
        Some(SimulationCommand::Ui(UiCommand::CountryPolicyPrevField))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Enter), policy_view),
        Some(SimulationCommand::Ui(UiCommand::CountryPolicyStartEdit))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('r')), policy_view),
        Some(SimulationCommand::Ui(UiCommand::CountryPolicyResetManaged))
    );

    let policy_editing = RunningInputState {
        country_policy_is_editing: true,
        ..policy_view
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('5')), policy_editing),
        Some(SimulationCommand::Ui(UiCommand::CountryPolicyEditChar('5')))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Enter), policy_editing),
        Some(SimulationCommand::Ui(UiCommand::CountryPolicySaveEdit))
    );
}

#[test]
fn person_related_events_page_uses_events_controls() {
    let simulation = Simulation::new();
    let person_events_state = RunningInputState {
        details_view: DetailsView::Person,
        details_page: 1,
        selection_focus: SelectionFocus::People,
        active_tab: RunningTab::Main,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), person_events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsScrollUp))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('/')), person_events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsToggleSearch))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Esc), person_events_state),
        Some(SimulationCommand::Ui(UiCommand::CloseDetails))
    );

    let searching_state = RunningInputState {
        events_search_active: true,
        ..person_events_state
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Esc), searching_state),
        Some(SimulationCommand::Ui(UiCommand::EventsToggleSearch))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('c')), searching_state),
        Some(SimulationCommand::Ui(UiCommand::EventsSearchChar('c')))
    );
}

#[test]
fn company_related_events_page_uses_events_controls() {
    let simulation = Simulation::new();
    let company_events_state = RunningInputState {
        details_view: DetailsView::Company,
        details_page: 1,
        selection_focus: SelectionFocus::Company,
        active_tab: RunningTab::Goods,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), company_events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsScrollDown))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('o')), company_events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsToggleSort))
    );
}
