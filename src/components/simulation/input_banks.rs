use crossterm::event::{KeyCode, KeyEvent};

use super::{RunningInputState, Simulation, SimulationCommand, ui_state::DetailsView};
use crate::{components::simulation::hotkeys, sim::UiCommand};

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
                    && c.eq_ignore_ascii_case(&'w')
                    && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::SelectPrevBankRow))
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty()
                    && c.eq_ignore_ascii_case(&'s')
                    && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::SelectNextBankRow))
            }
            KeyCode::Enter if input_state.details_view == DetailsView::List => {
                Some(SimulationCommand::Ui(UiCommand::OpenSelectedBankDetails))
            }
            KeyCode::Esc if input_state.details_view != DetailsView::List => {
                Some(SimulationCommand::Ui(UiCommand::CloseDetails))
            }
            _ if hotkeys::is_page_prev_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::BankPagePrev))
            }
            _ if hotkeys::is_page_next_key(key)
                && input_state.details_view == DetailsView::List =>
            {
                Some(SimulationCommand::Ui(UiCommand::BankPageNext))
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
