use crate::{
    components::simulation::{
        RunningInputState, Simulation, SimulationCommand,
        ui_state::{DetailsView, GoodsMainPage, RunningTab, SelectionFocus},
    },
    sim::UiCommand,
};
use crossterm::event::{KeyCode, KeyEvent};

#[test]
fn chart_and_events_remap_applies_strict_keys() {
    let simulation = Simulation::new();
    let chart_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Charts,
        goods_main_page: GoodsMainPage::Companies,
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
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('u')), chart_state),
        Some(SimulationCommand::Ui(UiCommand::ChartPagePrev))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), chart_state),
        Some(SimulationCommand::Ui(UiCommand::ChartPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageUp), chart_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), chart_state),
        None
    );

    let events_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Country,
        active_tab: RunningTab::Events,
        goods_main_page: GoodsMainPage::Companies,
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
fn detail_page_keys_map_in_main_and_goods_details_views() {
    let simulation = Simulation::new();

    let main_list = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::People,
        active_tab: RunningTab::Main,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), main_list),
        Some(SimulationCommand::Ui(UiCommand::PeoplePageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), main_list),
        None
    );

    let main_person_details = RunningInputState {
        details_view: DetailsView::Person,
        ..main_list
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), main_person_details),
        Some(SimulationCommand::Ui(UiCommand::DetailsPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('u')), main_person_details),
        Some(SimulationCommand::Ui(UiCommand::DetailsPagePrev))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), main_person_details),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageUp), main_person_details),
        None
    );

    let goods_company_details = RunningInputState {
        details_view: DetailsView::Company,
        details_page: 0,
        selection_focus: SelectionFocus::Company,
        active_tab: RunningTab::Goods,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), goods_company_details),
        Some(SimulationCommand::Ui(UiCommand::DetailsPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), goods_company_details),
        None
    );
}

#[test]
fn goods_main_page_toggle_and_pagination_use_dedicated_commands() {
    let simulation = Simulation::new();
    let goods_list_state = RunningInputState {
        details_view: DetailsView::List,
        details_page: 0,
        selection_focus: SelectionFocus::Company,
        active_tab: RunningTab::Goods,
        goods_main_page: GoodsMainPage::Companies,
        events_search_active: false,
        country_policy_is_editing: false,
    };

    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('g')), goods_list_state),
        Some(SimulationCommand::Ui(UiCommand::ToggleGoodsMainPage))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), goods_list_state),
        Some(SimulationCommand::Ui(UiCommand::CompanyPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), goods_list_state),
        None
    );

    let production_state = RunningInputState {
        goods_main_page: GoodsMainPage::ProductionDemand,
        ..goods_list_state
    };
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('u')), production_state),
        Some(SimulationCommand::Ui(UiCommand::GoodsProductionPagePrev))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('i')), production_state),
        Some(SimulationCommand::Ui(UiCommand::GoodsProductionPageNext))
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageUp), production_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::PageDown), production_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Enter), production_state),
        None
    );
    assert_eq!(
        simulation.map_running_key(KeyEvent::from(KeyCode::Char('s')), production_state),
        None
    );
}
