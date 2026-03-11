use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    components::simulation::{
        RunningInputState, Simulation, SimulationCommand, hotkeys,
        ui_state::{DetailsView, GoodsMainPage, RunningTab, SelectionFocus},
    },
    sim::{GameCommand, PersistenceCommand, UiCommand},
};

impl Simulation {
    pub(in crate::components::simulation) fn map_setup_key(
        &self,
        key: KeyEvent,
    ) -> Option<SimulationCommand> {
        use crate::components::simulation::snapshot::SnapshotPhase;
        use crate::sim::setup::SetupField;

        // Read the active field from the last snapshot to route input correctly.
        let active_field = match &self.last_snapshot.phase {
            SnapshotPhase::Setup(s) => s.active_field,
            _ => SetupField::CountryCount,
        };

        match key.code {
            KeyCode::Tab => return Some(SimulationCommand::Domain(GameCommand::SetupNextField)),
            KeyCode::BackTab => return Some(SimulationCommand::Domain(GameCommand::SetupPrevField)),
            _ => {}
        }

        // When ExistingSaves is focused, w/s navigate the list and Enter loads.
        if active_field == SetupField::ExistingSaves {
            return match key.code {
                KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'w') => {
                    Some(SimulationCommand::Domain(GameCommand::Persist(PersistenceCommand::SaveListSelectPrev)))
                }
                KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'s') => {
                    Some(SimulationCommand::Domain(GameCommand::Persist(PersistenceCommand::SaveListSelectNext)))
                }
                KeyCode::Enter => {
                    Some(SimulationCommand::Domain(GameCommand::Persist(PersistenceCommand::LoadSelectedSave)))
                }
                _ => None,
            };
        }

        // When SaveName is focused, characters edit the save name.
        if active_field == SetupField::SaveName {
            return match key.code {
                KeyCode::Backspace => Some(SimulationCommand::Domain(GameCommand::Persist(PersistenceCommand::SaveNameBackspace))),
                KeyCode::Enter => Some(SimulationCommand::Domain(GameCommand::SetupSubmit)),
                KeyCode::Char(c) if !c.is_control() => {
                    Some(SimulationCommand::Domain(GameCommand::Persist(PersistenceCommand::SaveNameEditChar(c))))
                }
                _ => None,
            };
        }

        // Default field input routing.
        match key.code {
            KeyCode::Backspace => Some(SimulationCommand::Domain(GameCommand::SetupBackspace)),
            KeyCode::Enter => Some(SimulationCommand::Domain(GameCommand::SetupSubmit)),
            KeyCode::Char(c) if !c.is_control() => {
                Some(SimulationCommand::Domain(GameCommand::SetupEditChar(c)))
            }
            _ => None,
        }
    }

    pub(in crate::components::simulation) fn map_running_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        match key.code {
            KeyCode::Tab => return Some(SimulationCommand::Ui(UiCommand::NextTab)),
            KeyCode::BackTab => return Some(SimulationCommand::Ui(UiCommand::PrevTab)),
            KeyCode::Char(' ') => return Some(SimulationCommand::Domain(GameCommand::TogglePause)),
            // Shift+S → save current run
            KeyCode::Char('S') if key.modifiers == KeyModifiers::SHIFT => {
                return Some(SimulationCommand::Domain(GameCommand::Persist(PersistenceCommand::SaveCurrentRun)));
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'n') => {
                return Some(SimulationCommand::Domain(GameCommand::StepOneDay));
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'m') => {
                return Some(SimulationCommand::Ui(UiCommand::ToggleSimMode));
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'c')
                    && input_state.details_view == DetailsView::Person
                    && input_state.details_page == 0 =>
            {
                return Some(SimulationCommand::Ui(UiCommand::JumpToEmployerCompany));
            }
            _ => {}
        }

        match input_state.active_tab {
            RunningTab::Charts => self.map_chart_key(key),
            RunningTab::Goods => self.map_goods_key(key, input_state),
            RunningTab::Banks => self.map_banks_key(key, input_state),
            RunningTab::Main => self.map_main_key(key, input_state),
            RunningTab::Stocks => self.map_stocks_key(key, input_state),
            RunningTab::Fx => self.map_fx_key(key),
            RunningTab::Disasters => self.map_disasters_key(key),
            RunningTab::Events => self.map_events_key(key, input_state),
        }
    }

    fn map_chart_key(&self, key: KeyEvent) -> Option<SimulationCommand> {
        if hotkeys::is_char(key, 't') {
            return Some(SimulationCommand::Ui(UiCommand::ToggleChartScope));
        }
        if hotkeys::is_country_prev_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::ChartPrevCountry));
        }
        if hotkeys::is_country_next_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::ChartNextCountry));
        }
        if hotkeys::is_page_prev_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::ChartPagePrev));
        }
        if hotkeys::is_page_next_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::ChartPageNext));
        }

        None
    }

    fn map_goods_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        if hotkeys::is_char(key, 'g') && input_state.details_view == DetailsView::List {
            return Some(SimulationCommand::Ui(UiCommand::ToggleGoodsMainPage));
        }
        if hotkeys::is_country_prev_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::GoodsPrevCountry));
        }
        if hotkeys::is_country_next_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::GoodsNextCountry));
        }
        if input_state.details_view != DetailsView::List {
            if hotkeys::is_page_prev_key(key) {
                return Some(SimulationCommand::Ui(UiCommand::DetailsPagePrev));
            }
            if hotkeys::is_page_next_key(key) {
                return Some(SimulationCommand::Ui(UiCommand::DetailsPageNext));
            }
        }

        if self.is_entity_related_events_page(input_state) {
            if matches!(key.code, KeyCode::Esc) && !input_state.events_search_active {
                return Some(SimulationCommand::Ui(UiCommand::CloseDetails));
            }
            return self.map_events_key(key, input_state);
        }

        match key.code {
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'w')
                    && self.is_company_workers_page(input_state) =>
            {
                Some(SimulationCommand::Ui(UiCommand::CompanyWorkersScrollUp))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'s')
                    && self.is_company_workers_page(input_state) =>
            {
                Some(SimulationCommand::Ui(UiCommand::CompanyWorkersScrollDown))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'w')
                    && input_state.details_view == DetailsView::List
                    && input_state.goods_main_page == GoodsMainPage::Companies =>
            {
                Some(SimulationCommand::Ui(UiCommand::SelectPrevCompanyRow))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'s')
                    && input_state.details_view == DetailsView::List
                    && input_state.goods_main_page == GoodsMainPage::Companies =>
            {
                Some(SimulationCommand::Ui(UiCommand::SelectNextCompanyRow))
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.goods_main_page == GoodsMainPage::Companies =>
            {
                Some(SimulationCommand::Ui(UiCommand::OpenSelectedCompanyDetails))
            }
            KeyCode::Esc if input_state.details_view != DetailsView::List => {
                Some(SimulationCommand::Ui(UiCommand::CloseDetails))
            }
            _ if hotkeys::is_page_prev_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                if input_state.goods_main_page == GoodsMainPage::ProductionDemand {
                    return Some(SimulationCommand::Ui(UiCommand::GoodsProductionPagePrev));
                }
                Some(SimulationCommand::Ui(UiCommand::CompanyPagePrev))
            }
            _ if hotkeys::is_page_next_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                if input_state.goods_main_page == GoodsMainPage::ProductionDemand {
                    return Some(SimulationCommand::Ui(UiCommand::GoodsProductionPageNext));
                }
                Some(SimulationCommand::Ui(UiCommand::CompanyPageNext))
            }
            _ => None,
        }
    }

    fn map_stocks_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        if hotkeys::is_country_prev_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::StocksPrevCountry));
        }
        if hotkeys::is_country_next_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::StocksNextCountry));
        }

        match key.code {
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'a') => {
                Some(SimulationCommand::Ui(UiCommand::StocksFocusCountry))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'d') => {
                Some(SimulationCommand::Ui(UiCommand::StocksFocusTable))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'w') => {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevCountryRow))
                    }
                    _ => Some(SimulationCommand::Ui(UiCommand::StocksPrevRow)),
                }
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'s') => {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextCountryRow))
                    }
                    _ => Some(SimulationCommand::Ui(UiCommand::StocksNextRow)),
                }
            }
            KeyCode::Enter if input_state.details_view == DetailsView::List => Some(
                SimulationCommand::Ui(UiCommand::StocksOpenSelectedCompanyDetails),
            ),
            _ if hotkeys::is_page_prev_key(key) => {
                Some(SimulationCommand::Ui(UiCommand::StocksPagePrev))
            }
            _ if hotkeys::is_page_next_key(key) => {
                Some(SimulationCommand::Ui(UiCommand::StocksPageNext))
            }
            _ => None,
        }
    }

    fn map_fx_key(&self, key: KeyEvent) -> Option<SimulationCommand> {
        match key.code {
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'w') => {
                Some(SimulationCommand::Ui(UiCommand::FxPrevCountry))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'s') => {
                Some(SimulationCommand::Ui(UiCommand::FxNextCountry))
            }
            _ => None,
        }
    }

    fn map_disasters_key(&self, key: KeyEvent) -> Option<SimulationCommand> {
        if hotkeys::is_country_prev_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::DisastersPrevCountry));
        }
        if hotkeys::is_country_next_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::DisastersNextCountry));
        }

        match key.code {
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'w') => {
                Some(SimulationCommand::Ui(UiCommand::DisastersPrevRow))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'s') => {
                Some(SimulationCommand::Ui(UiCommand::DisastersNextRow))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'a') => {
                Some(SimulationCommand::Ui(UiCommand::DisastersPrevIntensity))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'d') => {
                Some(SimulationCommand::Ui(UiCommand::DisastersNextIntensity))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'x') => {
                Some(SimulationCommand::Ui(UiCommand::DisastersToggleEnabled))
            }
            KeyCode::Enter => Some(SimulationCommand::Ui(UiCommand::DisastersApply)),
            _ => None,
        }
    }

    pub(in crate::components::simulation) fn map_events_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        if input_state.events_search_active {
            return match key.code {
                _ if hotkeys::is_ctrl_u(key) => {
                    Some(SimulationCommand::Ui(UiCommand::EventsSearchClear))
                }
                KeyCode::Backspace => Some(SimulationCommand::Ui(UiCommand::EventsSearchBackspace)),
                KeyCode::Esc => Some(SimulationCommand::Ui(UiCommand::EventsToggleSearch)),
                KeyCode::Char(c) => Some(SimulationCommand::Ui(UiCommand::EventsSearchChar(c))),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'w') => {
                Some(SimulationCommand::Ui(UiCommand::EventsScrollUp))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'s') => {
                Some(SimulationCommand::Ui(UiCommand::EventsScrollDown))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'f') => {
                Some(SimulationCommand::Ui(UiCommand::EventsNextCategory))
            }
            KeyCode::Char('/') => Some(SimulationCommand::Ui(UiCommand::EventsToggleSearch)),
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'o') => {
                Some(SimulationCommand::Ui(UiCommand::EventsToggleSort))
            }
            _ => None,
        }
    }
}
