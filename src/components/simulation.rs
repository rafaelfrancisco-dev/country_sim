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
    sim::{GameCommand, GameEngine, GamePhase, names::person_name_from_seed},
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
        match key.code {
            KeyCode::Char(' ') => Some(GameCommand::TogglePause),
            KeyCode::Left => Some(GameCommand::SelectPrevCountry),
            KeyCode::Right => Some(GameCommand::SelectNextCountry),
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'n' => Some(GameCommand::StepOneDay),
                'j' => Some(GameCommand::PeoplePagePrev),
                'k' => Some(GameCommand::PeoplePageNext),
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

        let [countries_area, people_area] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(body);

        self.draw_countries_table(frame, countries_area);
        self.draw_people_table(frame, people_area);

        let footer_text = format!(
            "Controls: [Space] pause/resume  [n] step day (paused)  [←/→] select country  [j/k] people page  [q] quit | Page {}/{}",
            world.people_page + 1,
            world.total_people_pages()
        );
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

        let rows = world.countries.iter().enumerate().map(|(idx, country)| {
            let style = if idx == world.selected_country {
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
        let rows = page_people.iter().map(|person| {
            let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
            let name = person_name_from_seed(name_seed);

            Row::new([
                Cell::from(person.id.to_string()),
                Cell::from(name),
                Cell::from(person.income.to_string()),
                Cell::from(person.expense.to_string()),
                Cell::from(person.balance.to_string()),
            ])
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
