use crossterm::event::{KeyCode, KeyEvent};

use super::Simulation;
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

    pub(super) fn map_running_key(&self, key: KeyEvent) -> Option<GameCommand> {
        let world = self.engine.world()?;

        match key.code {
            KeyCode::Tab => return Some(GameCommand::NextTab),
            KeyCode::BackTab => return Some(GameCommand::PrevTab),
            KeyCode::Char(' ') => return Some(GameCommand::TogglePause),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'n') => {
                return Some(GameCommand::StepOneDay);
            }
            KeyCode::Char(c)
                if c.eq_ignore_ascii_case(&'t') && world.active_tab == RunningTab::Charts =>
            {
                return Some(GameCommand::ToggleChartScope);
            }
            KeyCode::Char('<') if world.active_tab == RunningTab::Charts => {
                return Some(GameCommand::ChartPrevCountry);
            }
            KeyCode::Char('>') if world.active_tab == RunningTab::Charts => {
                return Some(GameCommand::ChartNextCountry);
            }
            _ => {}
        }

        if world.active_tab != RunningTab::Main {
            return None;
        }

        match key.code {
            KeyCode::Left => Some(GameCommand::FocusCountrySelection),
            KeyCode::Right => Some(GameCommand::FocusPeopleSelection),
            KeyCode::Up if world.details_view == DetailsView::List => match world.selection_focus {
                SelectionFocus::Country => Some(GameCommand::SelectPrevCountryRow),
                SelectionFocus::People => Some(GameCommand::SelectPrevPersonRow),
            },
            KeyCode::Down if world.details_view == DetailsView::List => match world.selection_focus
            {
                SelectionFocus::Country => Some(GameCommand::SelectNextCountryRow),
                SelectionFocus::People => Some(GameCommand::SelectNextPersonRow),
            },
            KeyCode::Enter
                if world.details_view == DetailsView::List
                    && world.selection_focus == SelectionFocus::People =>
            {
                Some(GameCommand::OpenSelectedPersonDetails)
            }
            KeyCode::Enter
                if world.details_view == DetailsView::List
                    && world.selection_focus == SelectionFocus::Country =>
            {
                Some(GameCommand::OpenSelectedCountryDetails)
            }
            KeyCode::Esc if world.details_view != DetailsView::List => {
                Some(GameCommand::CloseDetails)
            }
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'j' => match world.selection_focus {
                    SelectionFocus::Country => Some(GameCommand::CountryPagePrev),
                    SelectionFocus::People => Some(GameCommand::PeoplePagePrev),
                },
                'k' => match world.selection_focus {
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

    fn setup_running_world(simulation: &mut Simulation) {
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);
    }

    #[test]
    fn running_keymap_handles_row_navigation_and_details_commands() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);

        let world = simulation
            .engine
            .world()
            .expect("running world should exist");
        assert_eq!(world.details_view, DetailsView::List);
        assert_eq!(world.selection_focus, SelectionFocus::Country);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Up)),
            Some(GameCommand::SelectPrevCountryRow)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Down)),
            Some(GameCommand::SelectNextCountryRow)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('k'))),
            Some(GameCommand::CountryPageNext)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter)),
            Some(GameCommand::OpenSelectedCountryDetails)
        );

        simulation.engine.apply(GameCommand::FocusPeopleSelection);
        simulation.engine.apply(GameCommand::SelectNextPersonRow);
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('k'))),
            Some(GameCommand::PeoplePageNext)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter)),
            Some(GameCommand::OpenSelectedPersonDetails)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc)),
            None
        );
    }

    #[test]
    fn running_keymap_esc_is_only_active_in_details_view() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);
        simulation
            .engine
            .apply(GameCommand::OpenSelectedCountryDetails);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc)),
            Some(GameCommand::CloseDetails)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter)),
            None
        );
    }

    #[test]
    fn running_keymap_maps_tab_and_chart_controls() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Tab)),
            Some(GameCommand::NextTab)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::BackTab)),
            Some(GameCommand::PrevTab)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('t'))),
            None
        );

        simulation.engine.apply(GameCommand::NextTab);
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('t'))),
            Some(GameCommand::ToggleChartScope)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('<'))),
            Some(GameCommand::ChartPrevCountry)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('>'))),
            Some(GameCommand::ChartNextCountry)
        );
    }

    #[test]
    fn charts_tab_keymap_disables_list_navigation() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);
        simulation.engine.apply(GameCommand::NextTab);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Left)),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Up)),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter)),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc)),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('j'))),
            None
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('k'))),
            None
        );

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char(' '))),
            Some(GameCommand::TogglePause)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Char('n'))),
            Some(GameCommand::StepOneDay)
        );
    }
}
