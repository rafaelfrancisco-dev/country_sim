use crossterm::event::{KeyCode, KeyEvent};

use super::{RunningInputState, Simulation, SimulationCommand, ui_state::DetailsView};
use crate::{
    components::simulation::hotkeys, components::simulation::ui_state::SelectionFocus,
    sim::UiCommand,
};

impl Simulation {
    pub(super) fn map_banks_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<SimulationCommand> {
        if hotkeys::is_country_prev_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::BanksPrevCountry));
        }
        if hotkeys::is_country_next_key(key) {
            return Some(SimulationCommand::Ui(UiCommand::BanksNextCountry));
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
                    && self.is_bank_depositors_page(input_state) =>
            {
                Some(SimulationCommand::Ui(UiCommand::BankDepositorsScrollUp))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'s')
                    && self.is_bank_depositors_page(input_state) =>
            {
                Some(SimulationCommand::Ui(UiCommand::BankDepositorsScrollDown))
            }
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
                Some(SimulationCommand::Ui(UiCommand::FocusBankSelection))
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
                    SelectionFocus::Bank => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevBankRow))
                    }
                    SelectionFocus::People | SelectionFocus::Company | SelectionFocus::Stock => {
                        None
                    }
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
                    SelectionFocus::Bank => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextBankRow))
                    }
                    SelectionFocus::People | SelectionFocus::Company | SelectionFocus::Stock => {
                        None
                    }
                }
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.selection_focus == SelectionFocus::Bank =>
            {
                Some(SimulationCommand::Ui(UiCommand::OpenSelectedBankDetails))
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
                    SelectionFocus::Bank => Some(SimulationCommand::Ui(UiCommand::BankPagePrev)),
                    SelectionFocus::People | SelectionFocus::Company | SelectionFocus::Stock => {
                        None
                    }
                }
            }
            _ if hotkeys::is_page_next_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPageNext))
                    }
                    SelectionFocus::Bank => Some(SimulationCommand::Ui(UiCommand::BankPageNext)),
                    SelectionFocus::People | SelectionFocus::Company | SelectionFocus::Stock => {
                        None
                    }
                }
            }
            _ => None,
        }
    }

    pub(super) fn is_bank_related_events_page(&self, input_state: RunningInputState) -> bool {
        matches!(input_state.details_view, DetailsView::Bank) && input_state.details_page == 2
    }

    pub(super) fn is_bank_depositors_page(&self, input_state: RunningInputState) -> bool {
        matches!(input_state.details_view, DetailsView::Bank) && input_state.details_page == 3
    }
}
