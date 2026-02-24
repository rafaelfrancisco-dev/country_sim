use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Line,
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table, Tabs, Wrap,
    },
};
use tokio::sync::oneshot;

use super::Component;
use crate::{
    action::Action,
    sim::{
        GameCommand, GameEngine, GamePhase,
        country::Country,
        job::job_by_id,
        names::person_name_from_seed,
        world::{ChartScope, DetailsView, MetricPoint, RunningTab, SelectionFocus, World},
    },
};

#[derive(Debug, Default)]
pub struct Simulation {
    engine: GameEngine,
    monthly_tick_inflight: Option<oneshot::Receiver<Vec<Country>>>,
    pending_monthly_ticks: u64,
    observed_month_boundaries: u64,
}

impl Simulation {
    pub fn new() -> Self {
        Self::default()
    }

    fn money(amount: f64) -> String {
        format!("{amount:.2}")
    }

    fn reconcile_monthly_processing(&mut self) {
        self.poll_monthly_result();
        self.refresh_monthly_queue();
        self.start_monthly_tick_if_idle();
    }

    fn should_pause_daily_ticks_for_monthly_catchup(&self) -> bool {
        let Some(world) = self.engine.world() else {
            return false;
        };

        let required_months = world.completed_month_boundaries();
        let processed_months = world
            .countries
            .iter()
            .map(|country| country.month)
            .min()
            .unwrap_or(0);

        required_months.saturating_sub(processed_months) > 1
    }

    fn poll_monthly_result(&mut self) {
        let mut completed: Option<Vec<Country>> = None;
        let mut clear_inflight = false;

        if let Some(receiver) = &mut self.monthly_tick_inflight {
            match receiver.try_recv() {
                Ok(countries) => {
                    completed = Some(countries);
                    clear_inflight = true;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(oneshot::error::TryRecvError::Closed) => {
                    clear_inflight = true;
                }
            }
        }

        if clear_inflight {
            self.monthly_tick_inflight = None;
        }

        if let Some(countries) = completed {
            if let Some(world) = self.engine.world_mut() {
                world.apply_processed_month(countries);
            }
        }
    }

    fn refresh_monthly_queue(&mut self) {
        let Some(world) = self.engine.world() else {
            self.monthly_tick_inflight = None;
            self.pending_monthly_ticks = 0;
            self.observed_month_boundaries = 0;
            return;
        };

        let completed_boundaries = world.completed_month_boundaries();
        if completed_boundaries < self.observed_month_boundaries {
            self.pending_monthly_ticks = 0;
        } else if completed_boundaries > self.observed_month_boundaries {
            self.pending_monthly_ticks += completed_boundaries - self.observed_month_boundaries;
        }
        self.observed_month_boundaries = completed_boundaries;
    }

    fn start_monthly_tick_if_idle(&mut self) {
        if self.monthly_tick_inflight.is_some() || self.pending_monthly_ticks == 0 {
            return;
        }

        let Some(world) = self.engine.world() else {
            return;
        };
        let countries = world.countries.clone();

        let (tx, rx) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let mut processed = countries;
            for country in &mut processed {
                country.process_month();
            }

            let _ = tx.send(processed);
        });

        self.pending_monthly_ticks = self.pending_monthly_ticks.saturating_sub(1);
        self.monthly_tick_inflight = Some(rx);
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
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .areas(inner);
        let [stats, tabs_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Length(2)]).areas(header);

        let selected_country_name = world
            .selected_country()
            .map(|country| country.name.as_str())
            .unwrap_or("N/A");

        let run_state = if world.paused { "Paused" } else { "Running" };
        let selected_country_month = world.selected_country().map_or(0, |country| country.month);
        let header_text = format!(
            "Day: {} | Month: {} | State: {} | Selected Country: {} | Total Citizens: {}",
            world.day,
            selected_country_month,
            run_state,
            selected_country_name,
            world.total_population()
        );
        frame.render_widget(Paragraph::new(header_text), stats);
        self.draw_running_tabs(frame, tabs_area, world.active_tab);

        let footer_text = match world.active_tab {
            RunningTab::Main => self.draw_main_tab(frame, body),
            RunningTab::Charts => self.draw_charts_tab(frame, body),
        };

        frame.render_widget(
            Paragraph::new(footer_text).wrap(Wrap { trim: true }),
            footer,
        );
    }

    fn draw_running_tabs(&self, frame: &mut Frame, area: Rect, active_tab: RunningTab) {
        let selected = match active_tab {
            RunningTab::Main => 0,
            RunningTab::Charts => 1,
        };

        let tabs = Tabs::new(vec!["Main", "Charts"])
            .select(selected)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .title(" Running Tabs ")
                    .borders(Borders::ALL),
            );
        frame.render_widget(tabs, area);
    }

    fn draw_main_tab(&self, frame: &mut Frame, body: Rect) -> String {
        let Some(world) = self.engine.world() else {
            return String::new();
        };

        match world.details_view {
            DetailsView::List => {
                let [countries_area, people_area] =
                    Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .areas(body);

                self.draw_countries_table(frame, countries_area);
                self.draw_people_table(frame, people_area);

                format!(
                    "Controls: [Tab/Shift+Tab] switch tab  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [↑/↓] move row (focused list)  [j/k] page (focused list)  [Enter] details (focused row)  [q] quit | Country Page {}/{} | People Page {}/{}",
                    world.country_page() + 1,
                    world.total_country_pages(),
                    world.people_page + 1,
                    world.total_people_pages(),
                )
            }
            DetailsView::Person => {
                self.draw_person_details(frame, body);
                "Controls: [Tab/Shift+Tab] switch tab  [Esc] back to people list  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [j/k] page (focused list)  [q] quit".to_string()
            }
            DetailsView::Country => {
                self.draw_country_details(frame, body);
                "Controls: [Tab/Shift+Tab] switch tab  [Esc] back to lists  [Space] pause/resume  [n] step day (paused)  [←/→] focus country/people  [j/k] page (focused list)  [q] quit".to_string()
            }
        }
    }

    fn draw_charts_tab(&self, frame: &mut Frame, body: Rect) -> String {
        let Some(world) = self.engine.world() else {
            return String::new();
        };

        let scope_label = self.chart_scope_label(world);
        let history: &[MetricPoint] = match world.chart_scope {
            ChartScope::SelectedCountry => world
                .country_history
                .get(world.selected_country)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            ChartScope::WorldAggregate => world.world_history.as_slice(),
        };

        if history.is_empty() {
            let placeholder = Paragraph::new(
                "No history data yet. Advance the simulation by month to collect snapshots.",
            )
            .wrap(Wrap { trim: true })
            .block(Block::default().title(" Charts ").borders(Borders::ALL));
            frame.render_widget(placeholder, body);
        } else {
            let [top, bottom] =
                Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(body);
            let [population_area, supply_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(top);
            let [created_money_area, reserve_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(bottom);

            self.draw_metric_chart(
                frame,
                population_area,
                history,
                &format!("Population ({scope_label})"),
                Color::Cyan,
                |point| point.population,
            );
            self.draw_metric_chart(
                frame,
                supply_area,
                history,
                &format!("Total Money Supply ({scope_label})"),
                Color::Green,
                |point| point.total_money_supply,
            );
            self.draw_metric_chart(
                frame,
                created_money_area,
                history,
                &format!("Created Money Outstanding ({scope_label})"),
                Color::Yellow,
                |point| point.created_money_outstanding,
            );
            self.draw_metric_chart(
                frame,
                reserve_area,
                history,
                &format!("Bank Reserve ({scope_label})"),
                Color::Magenta,
                |point| point.bank_reserve,
            );
        }

        "Controls: [Tab/Shift+Tab] switch tab  [t] toggle scope  [</>] cycle country  [Space] pause/resume  [n] step day (paused)  [q] quit".to_string()
    }

    fn chart_scope_label(&self, world: &World) -> String {
        match world.chart_scope {
            ChartScope::SelectedCountry => {
                let country_name = world
                    .selected_country()
                    .map(|country| country.name.as_str())
                    .unwrap_or("N/A");
                format!("Selected: {country_name}")
            }
            ChartScope::WorldAggregate => "World Aggregate".to_string(),
        }
    }

    fn draw_metric_chart<F>(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &[MetricPoint],
        title: &str,
        color: Color,
        value_fn: F,
    ) where
        F: Fn(&MetricPoint) -> f64,
    {
        if history.is_empty() {
            frame.render_widget(
                Paragraph::new("No data.").block(
                    Block::default()
                        .title(format!(" {title} "))
                        .borders(Borders::ALL),
                ),
                area,
            );
            return;
        }

        let points = history
            .iter()
            .map(|point| (point.month as f64, value_fn(point)))
            .collect::<Vec<_>>();

        let first_month = history.first().map_or(0, |point| point.month);
        let last_month = history.last().map_or(first_month, |point| point.month);
        let x_min = first_month as f64;
        let mut x_max = last_month as f64;
        if (x_max - x_min).abs() < f64::EPSILON {
            x_max = x_min + 1.0;
        }

        let (value_min, value_max) = points
            .iter()
            .map(|(_, value)| *value)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        let (y_min, y_max) = Self::expanded_axis_bounds(value_min, value_max);

        let dataset = Dataset::default()
            .graph_type(GraphType::Line)
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(color))
            .data(&points);

        let chart = Chart::new(vec![dataset])
            .x_axis(
                Axis::default()
                    .title("Month")
                    .bounds([x_min, x_max])
                    .labels(vec![
                        first_month.to_string(),
                        last_month.max(first_month + 1).to_string(),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .bounds([y_min, y_max])
                    .labels(vec![Self::money(y_min), Self::money(y_max)]),
            )
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL),
            );

        frame.render_widget(chart, area);
    }

    fn expanded_axis_bounds(min: f64, max: f64) -> (f64, f64) {
        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        if (max - min).abs() < f64::EPSILON {
            let padding = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.10
            };
            let lower = (min - padding).min(min);
            let upper = (max + padding).max(max);
            return (lower, upper);
        }

        let span = max - min;
        let padding = span * 0.10;
        let lower = if min >= 0.0 {
            (min - padding).max(0.0)
        } else {
            min - padding
        };
        let upper = max + padding;
        (lower, upper)
    }

    fn draw_countries_table(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let header = Row::new([
            Cell::from("Country"),
            Cell::from("Population"),
            Cell::from("Households"),
            Cell::from("Companies"),
            Cell::from("Treasury"),
            Cell::from("Loans"),
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
                    Cell::from(Self::money(country.household_cash_total)),
                    Cell::from(Self::money(country.company_cash_total)),
                    Cell::from(Self::money(country.treasury_balance)),
                    Cell::from(Self::money(country.bank.created_money_outstanding)),
                ])
                .style(style)
            });

        let widths = [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(12),
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
            Cell::from("Job"),
            Cell::from("Employer"),
            Cell::from("Salary"),
            Cell::from("Cash"),
            Cell::from("Loan"),
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
                Cell::from(
                    job_by_id(person.job_type_id)
                        .map_or("Unknown".to_string(), |job| job.name.to_string()),
                ),
                Cell::from(
                    country
                        .company_name(person.employer_company_id)
                        .unwrap_or("N/A")
                        .to_string(),
                ),
                Cell::from(Self::money(person.monthly_salary())),
                Cell::from(Self::money(person.cash_balance)),
                Cell::from(Self::money(
                    person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance),
                )),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(22),
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(10),
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
                Paragraph::new("No country is currently selected.").block(
                    Block::default()
                        .title(" Person Details ")
                        .borders(Borders::ALL),
                ),
                area,
            );
            return;
        };

        let Some(person) = world.selected_person_on_page() else {
            frame.render_widget(
                Paragraph::new(
                    "No person available for the current selection. Press [Esc] to return.",
                )
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Person Details ")
                        .borders(Borders::ALL),
                ),
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
            Line::from(format!(
                "Job: {}",
                job_by_id(person.job_type_id).map_or("Unknown", |job| job.name)
            )),
            Line::from(format!(
                "Employer: {}",
                country
                    .company_name(person.employer_company_id)
                    .unwrap_or("N/A")
            )),
            Line::from(format!(
                "Monthly Salary: {}",
                Self::money(person.monthly_salary())
            )),
            Line::from(format!(
                "Cash Balance: {}",
                Self::money(person.cash_balance)
            )),
            Line::from(format!(
                "Loan Debt: {}",
                Self::money(person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance))
            )),
            Line::from(""),
            Line::from(format!(
                "Last Gross Pay: {}",
                Self::money(person.last_month_gross_pay)
            )),
            Line::from(format!(
                "Last Tax Paid: {}",
                Self::money(person.last_month_tax_paid)
            )),
            Line::from(format!(
                "Last Net Pay: {}",
                Self::money(person.last_month_net_pay)
            )),
            Line::from(format!(
                "Last Spending: {}",
                Self::money(person.last_month_spending)
            )),
            Line::from(format!(
                "Last Loan Payment: {}",
                Self::money(person.last_month_loan_payment)
            )),
            Line::from(format!(
                "Last New Loan: {}",
                Self::money(person.last_month_new_loan)
            )),
        ];

        frame.render_widget(
            Paragraph::new(details).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" Person Details ")
                    .borders(Borders::ALL),
            ),
            area,
        );
    }

    fn draw_country_details(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let Some(country) = world.selected_country() else {
            frame.render_widget(
                Paragraph::new("No country is currently selected.").block(
                    Block::default()
                        .title(" Country Details ")
                        .borders(Borders::ALL),
                ),
                area,
            );
            return;
        };

        let details = vec![
            Line::from(format!("Name: {}", country.name)),
            Line::from(format!("Country ID: {}", country.id)),
            Line::from(format!("Day: {}", world.day)),
            Line::from(format!("Month: {}", country.month)),
            Line::from(""),
            Line::from(format!("Population: {}", country.population())),
            Line::from(format!(
                "Household Cash: {}",
                Self::money(country.household_cash_total)
            )),
            Line::from(format!(
                "Company Cash: {}",
                Self::money(country.company_cash_total)
            )),
            Line::from(format!(
                "Treasury: {}",
                Self::money(country.treasury_balance)
            )),
            Line::from(format!(
                "Bank Reserve: {}",
                Self::money(country.bank.reserve_balance)
            )),
            Line::from(format!(
                "Loans Outstanding: {}",
                Self::money(country.bank.created_money_outstanding)
            )),
            Line::from(format!(
                "Money Supply: {}",
                Self::money(country.total_money_supply)
            )),
            Line::from(format!(
                "Expected Supply: {}",
                Self::money(country.expected_money_supply)
            )),
            Line::from(format!(
                "Supply Gap: {}",
                Self::money(country.money_supply_gap)
            )),
            Line::from(format!(
                "Last Monthly Tax: {}",
                Self::money(country.monthly_tax_collected)
            )),
            Line::from(format!(
                "Last Monthly Spending: {}",
                Self::money(country.monthly_household_spending)
            )),
            Line::from(format!(
                "People Page: {}/{}",
                world.people_page + 1,
                world.total_people_pages()
            )),
        ];

        frame.render_widget(
            Paragraph::new(details).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" Country Details ")
                    .borders(Borders::ALL),
            ),
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
            let should_block_daily = matches!(command, GameCommand::StepOneDay)
                && self.should_pause_daily_ticks_for_monthly_catchup();
            if !should_block_daily {
                self.engine.apply(command);
            }
            self.reconcile_monthly_processing();
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if matches!(action, Action::Tick) {
            if !self.should_pause_daily_ticks_for_monthly_catchup() {
                self.engine.apply(GameCommand::Tick);
            }
            self.reconcile_monthly_processing();
        } else if matches!(action, Action::Render) {
            self.reconcile_monthly_processing();
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
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);
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
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);

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
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);
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

    #[test]
    fn monthly_catchup_gate_blocks_only_when_more_than_one_month_behind() {
        let mut simulation = Simulation::new();
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);

        {
            let world = simulation
                .engine
                .world_mut()
                .expect("running world should exist");
            world.day = 30;
            for country in &mut world.countries {
                country.day = 30;
                country.month = 0;
            }
        }
        assert!(!simulation.should_pause_daily_ticks_for_monthly_catchup());

        {
            let world = simulation
                .engine
                .world_mut()
                .expect("running world should exist");
            world.day = 60;
            for country in &mut world.countries {
                country.day = 60;
                country.month = 0;
            }
        }
        assert!(simulation.should_pause_daily_ticks_for_monthly_catchup());

        {
            let world = simulation
                .engine
                .world_mut()
                .expect("running world should exist");
            for country in &mut world.countries {
                country.month = 1;
            }
        }
        assert!(!simulation.should_pause_daily_ticks_for_monthly_catchup());
    }
}
