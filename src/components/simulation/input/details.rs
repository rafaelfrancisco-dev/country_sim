use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    components::simulation::{
        RunningInputState, Simulation, SimulationCommand, hotkeys,
        ui_state::{DetailsView, SelectionFocus},
    },
    sim::UiCommand,
};

impl Simulation {
    pub(in crate::components::simulation) fn map_main_key(
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
                    SelectionFocus::Company | SelectionFocus::Bank | SelectionFocus::Stock => None,
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
                    SelectionFocus::Company | SelectionFocus::Bank | SelectionFocus::Stock => None,
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
                    SelectionFocus::Company | SelectionFocus::Bank | SelectionFocus::Stock => None,
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
                    SelectionFocus::Company | SelectionFocus::Bank | SelectionFocus::Stock => None,
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

    pub(in crate::components::simulation) fn is_entity_related_events_page(
        &self,
        input_state: RunningInputState,
    ) -> bool {
        self.is_person_related_events_page(input_state)
            || self.is_company_related_events_page(input_state)
            || self.is_bank_related_events_page(input_state)
    }

    fn is_person_related_events_page(&self, input_state: RunningInputState) -> bool {
        matches!(input_state.details_view, DetailsView::Person) && input_state.details_page == 1
    }

    fn is_company_related_events_page(&self, input_state: RunningInputState) -> bool {
        matches!(input_state.details_view, DetailsView::Company) && input_state.details_page == 2
    }

    pub(in crate::components::simulation) fn is_company_workers_page(
        &self,
        input_state: RunningInputState,
    ) -> bool {
        matches!(input_state.details_view, DetailsView::Company) && input_state.details_page == 3
    }
}
