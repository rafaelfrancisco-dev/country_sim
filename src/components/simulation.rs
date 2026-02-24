use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use super::Component;
use crate::{
    action::Action,
    sim::{
        GameCommand, GameEngine, GamePhase, names::person_name_from_seed,
        world::{DetailsView, SelectionFocus},
    },
};

#[derive(Debug, Default)]
pub struct Simulation {
    engine: GameEngine,
}

impl Simulation {
    pub fn new() -> Self {
        Self::default()
    }

    fn map_setup_key(&self, key: KeyEvent) -> Option<GameCommand> {
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

    fn map_running_key(&self, key: KeyEvent) -> Option<GameCommand> {
        let world = self.engine.world()?;

        match key.code {
            KeyCode::Char(' ') => Some(GameCommand::TogglePause),
            KeyCode::Left => Some(GameCommand::FocusCountrySelection),
            KeyCode::Right => Some(GameCommand::FocusPeopleSelection),
            KeyCode::Up if world.details_view == DetailsView::List => match world.selection_focus {
                SelectionFocus::Country => Some(GameCommand::SelectPrevCountryRow),
                SelectionFocus::People => Some(GameCommand::SelectPrevPersonRow),
            },
            KeyCode::Down if world.details_view == DetailsView::List => match world.selection_focus {
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
                'n' => Some(GameCommand::StepOneDay),
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

    fn draw_setup(&self, frame: &mut Frame, area: Rect) {
        let setup = match self.engine.setup() {
            Some(setup) => setup,
            None => return,
        };

        let block = Block::default()
            .title(" Country Simulation Setup ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [intro, countries, millions, status, help] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .areas(inner);

        frame.render_widget(
            Paragraph::new("Configure world size. Inputs are constrained to 1-50 countries and 1-20 million people per country.")
                .wrap(Wrap { trim: true }),
            intro,
        );

        let active_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let inactive_style = Style::default();

        let country_style = if setup.active_field == crate::sim::setup::SetupField::CountryCount {
            active_style
        } else {
            inactive_style
        };

        let million_style =
            if setup.active_field == crate::sim::setup::SetupField::MillionsPerCountry {
                active_style
            } else {
                inactive_style
            };

        let country_text = if setup.country_count_input.is_empty() {
            "<enter number>".to_string()
        } else {
            setup.country_count_input.clone()
        };

        let million_text = if setup.millions_per_country_input.is_empty() {
            "<enter number>".to_string()
        } else {
            setup.millions_per_country_input.clone()
        };

        frame.render_widget(
            Paragraph::new(country_text)
                .block(Block::default().title(" Countries ").borders(Borders::ALL))
                .style(country_style),
            countries,
        );

        frame.render_widget(
            Paragraph::new(million_text)
                .block(
                    Block::default()
                        .title(" Millions per Country ")
                        .borders(Borders::ALL),
                )
                .style(million_style),
            millions,
        );

        let mut status_lines = Vec::new();
        if let Some(error) = &setup.validation_error {
            status_lines.push(Line::styled(
                error.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(warning) = &setup.memory_warning {
            status_lines.push(Line::styled(
                warning.clone(),
                Style::default().fg(Color::Yellow),
            ));
        }
        if status_lines.is_empty() {
            status_lines.push(Line::from("Ready. Press Enter to generate world."));
        }

        frame.render_widget(
            Paragraph::new(status_lines).wrap(Wrap { trim: true }),
            status,
        );

        frame.render_widget(
            Paragraph::new("Keys: [Tab/Shift+Tab] change field, [0-9] edit, [Backspace] erase, [Enter] start simulation"),
            help,
        );
    }

    fn draw_running(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let block = Block::default()
            .title(" Country Simulation ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .areas(inner);

        let selected_country_name = world
            .selected_country()
            .map(|country| country.name.as_str())
            .unwrap_or("N/A");

        let run_state = if world.paused { "Paused" } else { "Running" };
        let header_text = format!(
            "Day: {} | State: {} | Selected Country: {} | Total Citizens: {}",
            world.day,
            run_state,
            selected_country_name,
            world.total_population()
        );
        frame.render_widget(Paragraph::new(header_text), header);

        let footer_text = match world.details_view {
            DetailsView::List => {
                let [countries_area, people_area] =
                    Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .areas(body);

                self.draw_countries_table(frame, countries_area);
                self.draw_people_table(frame, people_area);

                format!(
                    "Controls: [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [↑/↓] move row (focused list)  [j/k] page (focused list)  [Enter] details (focused row)  [q] quit | Country Page {}/{} | People Page {}/{}",
                    world.country_page() + 1,
                    world.total_country_pages(),
                    world.people_page + 1,
                    world.total_people_pages(),
                )
            }
            DetailsView::Person => {
                self.draw_person_details(frame, body);
                "Controls: [Esc] back to people list  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [j/k] page (focused list)  [q] quit".to_string()
            }
            DetailsView::Country => {
                self.draw_country_details(frame, body);
                "Controls: [Esc] back to lists  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [j/k] page (focused list)  [q] quit".to_string()
            }
        };

        frame.render_widget(Paragraph::new(footer_text), footer);
    }

    fn draw_countries_table(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let header = Row::new([
            Cell::from("Country"),
            Cell::from("Population"),
            Cell::from("Total Balance"),
            Cell::from("Avg Net/Day"),
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let country_page_start = world.country_page() * crate::sim::world::COUNTRY_PAGE_SIZE;
        let rows = world
            .selected_countries_page()
            .iter()
            .enumerate()
            .map(|(page_idx, country)| {
                let idx = country_page_start + page_idx;
                let style = if idx == world.selected_country
                    && world.selection_focus == SelectionFocus::Country
                {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                Row::new([
                    Cell::from(country.name.clone()),
                    Cell::from(country.population().to_string()),
                    Cell::from(country.total_balance.to_string()),
                    Cell::from(format!("{:.2}", country.avg_net_per_day)),
                ])
                .style(style)
            });

        let widths = [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(12),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(Block::default().title(" Countries ").borders(Borders::ALL));

        frame.render_widget(table, area);
    }

    fn draw_people_table(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let header = Row::new([
            Cell::from("Citizen ID"),
            Cell::from("Name"),
            Cell::from("Income"),
            Cell::from("Expense"),
            Cell::from("Balance"),
        ])
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

        let Some(country) = world.selected_country() else {
            let placeholder = Paragraph::new("No countries generated yet.")
                .block(Block::default().title(" People ").borders(Borders::ALL));
            frame.render_widget(placeholder, area);
            return;
        };

        let page_people = world.selected_people_page().unwrap_or(&[]);
        let rows = page_people.iter().enumerate().map(|(row_idx, person)| {
            let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
            let name = person_name_from_seed(name_seed);
            let style = if row_idx == world.selected_person_row
                && world.selection_focus == SelectionFocus::People
            {
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new([
                Cell::from(person.id.to_string()),
                Cell::from(name),
                Cell::from(person.income.to_string()),
                Cell::from(person.expense.to_string()),
                Cell::from(person.balance.to_string()),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(24),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(
                Block::default()
                    .title(" People (sample rows) ")
                    .borders(Borders::ALL),
            );

        frame.render_widget(table, area);
    }

    fn draw_person_details(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let Some(country) = world.selected_country() else {
            frame.render_widget(
                Paragraph::new("No country is currently selected.")
                    .block(Block::default().title(" Person Details ").borders(Borders::ALL)),
                area,
            );
            return;
        };

        let Some(person) = world.selected_person_on_page() else {
            frame.render_widget(
                Paragraph::new("No person available for the current selection. Press [Esc] to return.")
                    .wrap(Wrap { trim: true })
                    .block(Block::default().title(" Person Details ").borders(Borders::ALL)),
                area,
            );
            return;
        };

        let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
        let name = person_name_from_seed(name_seed);
        let details = vec![
            Line::from(format!("Name: {}", name)),
            Line::from(format!("Citizen ID: {}", person.id)),
            Line::from(format!("Country: {}", country.name)),
            Line::from(format!("Day: {}", world.day)),
            Line::from(""),
            Line::from(format!("Income: {}", person.income)),
            Line::from(format!("Expense: {}", person.expense)),
            Line::from(format!("Daily Net: {}", person.daily_net())),
            Line::from(format!("Balance: {}", person.balance)),
        ];

        frame.render_widget(
            Paragraph::new(details)
                .wrap(Wrap { trim: true })
                .block(Block::default().title(" Person Details ").borders(Borders::ALL)),
            area,
        );
    }

    fn draw_country_details(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let Some(country) = world.selected_country() else {
            frame.render_widget(
                Paragraph::new("No country is currently selected.")
                    .block(Block::default().title(" Country Details ").borders(Borders::ALL)),
                area,
            );
            return;
        };

        let details = vec![
            Line::from(format!("Name: {}", country.name)),
            Line::from(format!("Country ID: {}", country.id)),
            Line::from(format!("Day: {}", world.day)),
            Line::from(""),
            Line::from(format!("Population: {}", country.population())),
            Line::from(format!("Total Balance: {}", country.total_balance)),
            Line::from(format!("Avg Net/Day: {:.2}", country.avg_net_per_day)),
            Line::from(format!(
                "People Page: {}/{}",
                world.people_page + 1,
                world.total_people_pages()
            )),
        ];

        frame.render_widget(
            Paragraph::new(details)
                .wrap(Wrap { trim: true })
                .block(Block::default().title(" Country Details ").borders(Borders::ALL)),
            area,
        );
    }
}

impl Component for Simulation {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        let command = match self.engine.phase() {
            GamePhase::Setup(_) => self.map_setup_key(key),
            GamePhase::Running(_) => self.map_running_key(key),
        };

        if let Some(command) = command {
            self.engine.apply(command);
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if matches!(action, Action::Tick) {
            self.engine.apply(GameCommand::Tick);
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        match self.engine.phase() {
            GamePhase::Setup(_) => self.draw_setup(frame, area),
            GamePhase::Running(_) => self.draw_running(frame, area),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_keymap_handles_row_navigation_and_details_commands() {
        let mut simulation = Simulation::new();
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);

        let world = simulation.engine.world().expect("running world should exist");
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
        assert_eq!(simulation.map_running_key(KeyEvent::from(KeyCode::Esc)), None);
    }

    #[test]
    fn running_keymap_esc_is_only_active_in_details_view() {
        let mut simulation = Simulation::new();
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);
        simulation.engine.apply(GameCommand::OpenSelectedCountryDetails);

        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Esc)),
            Some(GameCommand::CloseDetails)
        );
        assert_eq!(
            simulation.map_running_key(KeyEvent::from(KeyCode::Enter)),
            None
        );
    }
}
