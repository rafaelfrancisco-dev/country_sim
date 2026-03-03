use crossterm::event::{KeyCode, KeyEvent};

use super::{RunningInputState, Simulation, SimulationCommand, ui_state::DetailsView};
use crate::{
    components::simulation::hotkeys,
    components::simulation::ui_state::{RunningTab, SelectionFocus},
    sim::{GameCommand, UiCommand},
};

impl Simulation {
    pub(super) fn map_setup_key(&self, key: KeyEvent) -> Option<SimulationCommand> {
        match key.code {
            KeyCode::Tab => Some(SimulationCommand::Domain(GameCommand::SetupNextField)),
            KeyCode::BackTab => Some(SimulationCommand::Domain(GameCommand::SetupPrevField)),
            KeyCode::Backspace => Some(SimulationCommand::Domain(GameCommand::SetupBackspace)),
            KeyCode::Enter => Some(SimulationCommand::Domain(GameCommand::SetupSubmit)),
            KeyCode::Char(c) if c.is_ascii_alphanumeric() || matches!(c, '.' | '_') => {
                Some(SimulationCommand::Domain(GameCommand::SetupEditChar(c)))
            }
            _ => None,
        }
    }

    pub(super) fn map_running_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        match key.code {
            KeyCode::Tab => return Some(SimulationCommand::Ui(UiCommand::NextTab)),
            KeyCode::BackTab => return Some(SimulationCommand::Ui(UiCommand::PrevTab)),
            KeyCode::Char(' ') => return Some(SimulationCommand::Domain(GameCommand::TogglePause)),
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'n') => {
                return Some(SimulationCommand::Domain(GameCommand::StepOneDay));
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'m') => {
                return Some(SimulationCommand::Ui(UiCommand::ToggleSimMode));
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'c')
                    && input_state.details_view == DetailsView::Person =>
            {
                return Some(SimulationCommand::Ui(UiCommand::JumpToEmployerCompany));
            }
            _ => {}
        }

        match input_state.active_tab {
            RunningTab::Charts => self.map_chart_key(key),
            RunningTab::Goods => self.map_goods_key(key, input_state),
            RunningTab::Main => self.map_main_key(key, input_state),
            RunningTab::Stocks => self.map_stocks_key(key, input_state),
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

        match key.code {
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'a')
                    && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::FocusCountrySelection))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'d')
                    && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::FocusCompanySelection))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'w')
                    && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevCountryRow))
                    }
                    SelectionFocus::Company => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevCompanyRow))
                    }
                    SelectionFocus::People | SelectionFocus::Stock => None,
                }
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'s')
                    && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextCountryRow))
                    }
                    SelectionFocus::Company => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextCompanyRow))
                    }
                    SelectionFocus::People | SelectionFocus::Stock => None,
                }
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.selection_focus == SelectionFocus::Company =>
            {
                Some(SimulationCommand::Ui(UiCommand::OpenSelectedCompanyDetails))
            }
            KeyCode::Esc if input_state.details_view != DetailsView::List => {
                Some(SimulationCommand::Ui(UiCommand::CloseDetails))
            }
            _ if hotkeys::is_page_prev_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPagePrev))
                    }
                    SelectionFocus::Company => {
                        Some(SimulationCommand::Ui(UiCommand::CompanyPagePrev))
                    }
                    SelectionFocus::People | SelectionFocus::Stock => None,
                }
            }
            _ if hotkeys::is_page_next_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPageNext))
                    }
                    SelectionFocus::Company => {
                        Some(SimulationCommand::Ui(UiCommand::CompanyPageNext))
                    }
                    SelectionFocus::People | SelectionFocus::Stock => None,
                }
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
            _ if hotkeys::is_page_prev_key(key) => {
                Some(SimulationCommand::Ui(UiCommand::StocksPagePrev))
            }
            _ if hotkeys::is_page_next_key(key) => {
                Some(SimulationCommand::Ui(UiCommand::StocksPageNext))
            }
            _ => None,
        }
    }

    fn map_events_key(
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

    fn map_main_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        if input_state.details_view == DetailsView::CountryPolicy {
            return self.map_country_policy_key(key, input_state);
        }

        if input_state.details_view == DetailsView::Country
            && matches!(key.code, KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'o'))
        {
            return Some(SimulationCommand::Ui(
                UiCommand::OpenSelectedCountryPolicyDetails,
            ));
        }

        if input_state.details_view != DetailsView::List {
            if hotkeys::is_page_prev_key(key) {
                return Some(SimulationCommand::Ui(UiCommand::DetailsPagePrev));
            }
            if hotkeys::is_page_next_key(key) {
                return Some(SimulationCommand::Ui(UiCommand::DetailsPageNext));
            }
        }

        match key.code {
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'a')
                    && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::FocusCountrySelection))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'d')
                    && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::FocusPeopleSelection))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'w')
                    && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevCountryRow))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevPersonRow))
                    }
                    SelectionFocus::Company | SelectionFocus::Stock => None,
                }
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'s')
                    && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextCountryRow))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextPersonRow))
                    }
                    SelectionFocus::Company | SelectionFocus::Stock => None,
                }
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.selection_focus == SelectionFocus::People =>
            {
                Some(SimulationCommand::Ui(UiCommand::OpenSelectedPersonDetails))
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.selection_focus == SelectionFocus::Country =>
            {
                Some(SimulationCommand::Ui(UiCommand::OpenSelectedCountryDetails))
            }
            KeyCode::Esc if input_state.details_view != DetailsView::List => {
                Some(SimulationCommand::Ui(UiCommand::CloseDetails))
            }
            _ if hotkeys::is_page_prev_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPagePrev))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::PeoplePagePrev))
                    }
                    SelectionFocus::Company | SelectionFocus::Stock => None,
                }
            }
            _ if hotkeys::is_page_next_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPageNext))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::PeoplePageNext))
                    }
                    SelectionFocus::Company | SelectionFocus::Stock => None,
                }
            }
            _ => None,
        }
    }

    fn map_country_policy_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        if input_state.country_policy_is_editing {
            return match key.code {
                KeyCode::Esc => Some(SimulationCommand::Ui(UiCommand::CountryPolicyCancelEdit)),
                KeyCode::Enter => Some(SimulationCommand::Ui(UiCommand::CountryPolicySaveEdit)),
                KeyCode::Backspace => {
                    Some(SimulationCommand::Ui(UiCommand::CountryPolicyBackspace))
                }
                KeyCode::Char(c) if key.modifiers.is_empty() => {
                    Some(SimulationCommand::Ui(UiCommand::CountryPolicyEditChar(c)))
                }
                _ => None,
            };
        }

        match key.code {
            KeyCode::Esc => Some(SimulationCommand::Ui(UiCommand::CloseCountryPolicyDetails)),
            KeyCode::Enter => Some(SimulationCommand::Ui(UiCommand::CountryPolicyStartEdit)),
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'r') => {
                Some(SimulationCommand::Ui(UiCommand::CountryPolicyResetManaged))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'w') => {
                Some(SimulationCommand::Ui(UiCommand::CountryPolicyPrevField))
            }
            KeyCode::Char(c) if key.modifiers.is_empty() && c.eq_ignore_ascii_case(&'s') => {
                Some(SimulationCommand::Ui(UiCommand::CountryPolicyNextField))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
