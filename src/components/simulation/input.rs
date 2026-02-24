use crossterm::event::{KeyCode, KeyEvent};

use super::{RunningInputState, Simulation};
use crate::sim::{
    GameCommand,
    world::{DetailsView, RunningTab, SelectionFocus},
};

impl Simulation {
    pub(super) fn map_setup_key(&self, key: KeyEvent) -> Option<GameCommand> {
        match key.code {
            KeyCode::Tab => Some(GameCommand::SetupNextField),
            KeyCode::BackTab => Some(GameCommand::SetupPrevField),
            KeyCode::Backspace => Some(GameCommand::SetupBackspace),
            KeyCode::Enter => Some(GameCommand::SetupSubmit),
            KeyCode::Char(c) if c.is_ascii_digit() => {
                Some(GameCommand::SetupEditDigit(c as u8 - b'0'))
            }
            _ => None,
        }
    }

    pub(super) fn map_running_key(
        &self,
        key: KeyEvent,
        input_state: RunningInputState,
    ) -> Option<GameCommand> {
        match key.code {
            KeyCode::Tab => return Some(GameCommand::NextTab),
            KeyCode::BackTab => return Some(GameCommand::PrevTab),
            KeyCode::Char(' ') => return Some(GameCommand::TogglePause),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'n') => {
                return Some(GameCommand::StepOneDay);
            }
            KeyCode::Char(c)
                if c.eq_ignore_ascii_case(&'t') && input_state.active_tab == RunningTab::Charts =>
            {
                return Some(GameCommand::ToggleChartScope);
            }
            KeyCode::Char('<') if input_state.active_tab == RunningTab::Charts => {
                return Some(GameCommand::ChartPrevCountry);
            }
            KeyCode::Char('>') if input_state.active_tab == RunningTab::Charts => {
                return Some(GameCommand::ChartNextCountry);
            }
            _ => {}
        }

        if input_state.active_tab != RunningTab::Main {
            return None;
        }

        match key.code {
            KeyCode::Left => Some(GameCommand::FocusCountrySelection),
            KeyCode::Right => Some(GameCommand::FocusPeopleSelection),
            KeyCode::Up if input_state.details_view == DetailsView::List => {
                match input_state.selection_focus {
                    SelectionFocus::Country => Some(GameCommand::SelectPrevCountryRow),
                    SelectionFocus::People => Some(GameCommand::SelectPrevPersonRow),
                }
            }
            KeyCode::Down if input_state.details_view == DetailsView::List => {
                match input_state.selection_focus {
                    SelectionFocus::Country => Some(GameCommand::SelectNextCountryRow),
                    SelectionFocus::People => Some(GameCommand::SelectNextPersonRow),
                }
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.selection_focus == SelectionFocus::People =>
            {
                Some(GameCommand::OpenSelectedPersonDetails)
            }
            KeyCode::Enter
                if input_state.details_view == DetailsView::List
                    && input_state.selection_focus == SelectionFocus::Country =>
            {
                Some(GameCommand::OpenSelectedCountryDetails)
            }
            KeyCode::Esc if input_state.details_view != DetailsView::List => {
                Some(GameCommand::CloseDetails)
            }
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'j' => match input_state.selection_focus {
                    SelectionFocus::Country => Some(GameCommand::CountryPagePrev),
                    SelectionFocus::People => Some(GameCommand::PeoplePagePrev),
                },
                'k' => match input_state.selection_focus {
                    SelectionFocus::Country => Some(GameCommand::CountryPageNext),
                    SelectionFocus::People => Some(GameCommand::PeoplePageNext),
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
        simulation.apply_command_and_wait(GameCommand::SetupEditDigit(1));
        simulation.apply_command_and_wait(GameCommand::SetupNextField);
        simulation.apply_command_and_wait(GameCommand::SetupEditDigit(1));
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
    fn running_keymap_handles_row_navigation_and_details_commands() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);

        let state = running_state(&simulation);
        assert_eq!(state.details_view, DetailsView::List);
        assert_eq!(state.selection_focus, SelectionFocus::Country);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Up), state),
            Some(GameCommand::SelectPrevCountryRow)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Down), state),
            Some(GameCommand::SelectNextCountryRow)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('k')), state),
            Some(GameCommand::CountryPageNext)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter), state),
            Some(GameCommand::OpenSelectedCountryDetails)
        );

        simulation.apply_command_and_wait(GameCommand::FocusPeopleSelection);
        simulation.apply_command_and_wait(GameCommand::SelectNextPersonRow);
        let people_state = running_state(&simulation);
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('k')), people_state),
            Some(GameCommand::PeoplePageNext)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter), people_state),
            Some(GameCommand::OpenSelectedPersonDetails)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc), people_state),
            None
        );
    }

    #[test]
    fn running_keymap_esc_is_only_active_in_details_view() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);
        simulation.apply_command_and_wait(GameCommand::OpenSelectedCountryDetails);
        let details_state = running_state(&simulation);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc), details_state),
            Some(GameCommand::CloseDetails)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter), details_state),
            None
        );
    }

    #[test]
    fn running_keymap_maps_tab_and_chart_controls() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);
        let list_state = running_state(&simulation);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Tab), list_state),
            Some(GameCommand::NextTab)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::BackTab), list_state),
            Some(GameCommand::PrevTab)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('t')), list_state),
            None
        );

        simulation.apply_command_and_wait(GameCommand::NextTab);
        let charts_state = running_state(&simulation);
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('t')), charts_state),
            Some(GameCommand::ToggleChartScope)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('<')), charts_state),
            Some(GameCommand::ChartPrevCountry)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('>')), charts_state),
            Some(GameCommand::ChartNextCountry)
        );
    }

    #[test]
    fn charts_tab_keymap_disables_list_navigation() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);
        simulation.apply_command_and_wait(GameCommand::NextTab);
        let charts_state = running_state(&simulation);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Left), charts_state),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Up), charts_state),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter), charts_state),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc), charts_state),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('j')), charts_state),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('k')), charts_state),
            None
        );

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char(' ')), charts_state),
            Some(GameCommand::TogglePause)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('n')), charts_state),
            Some(GameCommand::StepOneDay)
        );
    }
}
