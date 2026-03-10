use crate::{
    components::simulation::{
        RunningInputState, Simulation, SimulationCommand,
        ui_state::{DetailsView, GoodsMainPage, RunningTab, SelectionFocus},
    },
    sim::UiCommand,
};
use crossterm::event::{KeyCode, KeyEvent};

#[test]
fn running_keymap_maps_tab_and_chart_controls() {
    let simulation = Simulation::new();
    let list_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Main,
        goods_main_page: GoodsMainPage::Companies,
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
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('c')), state),
        Some(SimulationCommand::Ui(UiCommand::JumpToEmployerCompany))
    );
}

#[test]
fn running_keymap_maps_stocks_enter_to_open_selected_company() {
    let simulation = Simulation::new();
    let state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Stocks,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Enter), state),
        Some(SimulationCommand::Ui(
            UiCommand::StocksOpenSelectedCompanyDetails
        ))
    );
}

#[test]
fn fx_tab_maps_ws_to_scroll_list() {
    let simulation = Simulation::new();
    let state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Fx,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), state),
        Some(SimulationCommand::Ui(UiCommand::FxPrevCountry))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), state),
        Some(SimulationCommand::Ui(UiCommand::FxNextCountry))
    );
    // comma/period no longer bound on FX tab
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char(',')), state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('.')), state),
        None
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
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: true,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('f')), events_search_state),
        Some(SimulationCommand::Ui(UiCommand::EventsSearchChar('f')))
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
        goods_main_page: GoodsMainPage::Companies,
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
        goods_main_page: GoodsMainPage::Companies,
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
        details_page: 2,
        selection_focus: SelectionFocus::Company,
        active_tab: RunningTab::Goods,
        goods_main_page: GoodsMainPage::Companies,
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

#[test]
fn company_workers_page_uses_worker_scroll_controls() {
    let simulation = Simulation::new();
    let workers_state = RunningInputState {
        details_view: DetailsView::Company,
        details_page: 3,
        selection_focus: SelectionFocus::Company,
        active_tab: RunningTab::Goods,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), workers_state),
        Some(SimulationCommand::Ui(UiCommand::CompanyWorkersScrollUp))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), workers_state),
        Some(SimulationCommand::Ui(UiCommand::CompanyWorkersScrollDown))
    );
}

#[test]
fn banks_list_keymap_navigates_and_opens_details() {
    let simulation = Simulation::new();
    let banks_list_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Bank,
        active_tab: RunningTab::Banks,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('a')), banks_list_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('d')), banks_list_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), banks_list_state),
        Some(SimulationCommand::Ui(UiCommand::SelectPrevBankRow))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), banks_list_state),
        Some(SimulationCommand::Ui(UiCommand::BankPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), banks_list_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Enter), banks_list_state),
        Some(SimulationCommand::Ui(UiCommand::OpenSelectedBankDetails))
    );
}

#[test]
fn bank_detail_pages_map_events_and_depositor_scroll() {
    let simulation = Simulation::new();
    let bank_events_state = RunningInputState {
        details_view: DetailsView::Bank,
        details_page: 2,
        selection_focus: SelectionFocus::Bank,
        active_tab: RunningTab::Banks,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), bank_events_state),
        Some(SimulationCommand::Ui(UiCommand::EventsScrollDown))
    );

    let bank_depositors_state = RunningInputState {
        details_page: 3,
        ..bank_events_state
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), bank_depositors_state),
        Some(SimulationCommand::Ui(UiCommand::BankDepositorsScrollUp))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), bank_depositors_state),
        Some(SimulationCommand::Ui(UiCommand::BankDepositorsScrollDown))
    );
}

#[test]
fn disasters_tab_maps_navigation_toggle_and_apply_keys() {
    let simulation = Simulation::new();
    let disasters_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Disasters,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char(',')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersPrevCountry))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('.')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersNextCountry))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('w')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersPrevRow))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersNextRow))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('a')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersPrevIntensity))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('d')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersNextIntensity))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('x')), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersToggleEnabled))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Enter), disasters_state),
        Some(SimulationCommand::Ui(UiCommand::DisastersApply))
    );
}
