use crossterm::event::{KeyCode, KeyEvent};

use super::{RunningInputState, Simulation, SimulationCommand, ui_state::DetailsView};
use crate::{
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
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
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
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'n') => {
                return Some(SimulationCommand::Domain(GameCommand::StepOneDay));
            }
            KeyCode::Char(c)
                if c.eq_ignore_ascii_case(&'t') && input_state.active_tab == RunningTab::Charts =>
            {
                return Some(SimulationCommand::Ui(UiCommand::ToggleChartScope));
            }
            KeyCode::Char('<') if input_state.active_tab == RunningTab::Charts => {
                return Some(SimulationCommand::Ui(UiCommand::ChartPrevCountry));
            }
            KeyCode::Char('>') if input_state.active_tab == RunningTab::Charts => {
                return Some(SimulationCommand::Ui(UiCommand::ChartNextCountry));
            }
            _ => {}
        }

        if input_state.active_tab != RunningTab::Main {
            return None;
        }

        match key.code {
            KeyCode::Left => Some(SimulationCommand::Ui(UiCommand::FocusCountrySelection)),
            KeyCode::Right => Some(SimulationCommand::Ui(UiCommand::FocusPeopleSelection)),
            KeyCode::Up if input_state.details_view == DetailsView::List => {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevCountryRow))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::SelectPrevPersonRow))
                    }
                }
            }
            KeyCode::Down if input_state.details_view == DetailsView::List => {
                match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextCountryRow))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::SelectNextPersonRow))
                    }
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
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'j' => match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPagePrev))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::PeoplePagePrev))
                    }
                },
                'k' => match input_state.selection_focus {
                    SelectionFocus::Country => {
                        Some(SimulationCommand::Ui(UiCommand::CountryPageNext))
                    }
                    SelectionFocus::People => {
                        Some(SimulationCommand::Ui(UiCommand::PeoplePageNext))
                    }
                },
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::simulation::snapshot::SnapshotPhase;

    fn setup_running_world(simulation: &mut Simulation) {
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupNextField);
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupSubmit);
    }

    fn running_state(simulation: &Simulation) -> RunningInputState {
        match simulation.snapshot_phase() {
            SnapshotPhase::Running(world) => RunningInputState {
                details_view: world.details_view,
                selection_focus: world.selection_focus,
                active_tab: world.active_tab,
            },
            SnapshotPhase::Setup(_) => panic!("running world should exist"),
        }
    }

    #[test]
    fn running_keymap_maps_tab_and_chart_controls() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);
        let list_state = running_state(&simulation);

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
    }
}
