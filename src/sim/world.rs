use super::{country::Country, country::DAYS_PER_MONTH, person::Person, tickable::Tickable};

pub const PEOPLE_PAGE_SIZE: usize = 50;
pub const COUNTRY_PAGE_SIZE: usize = 10;
pub const HISTORY_LIMIT: usize = 120;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DetailsView {
    List,
    Person,
    Country,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectionFocus {
    Country,
    People,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RunningTab {
    Main,
    Charts,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChartScope {
    SelectedCountry,
    WorldAggregate,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MetricPoint {
    pub month: u64,
    pub population: f64,
    pub total_money_supply: f64,
    pub created_money_outstanding: f64,
    pub bank_reserve: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub day: u64,
    pub countries: Vec<Country>,
    pub selected_country: usize,
    pub paused: bool,
    pub people_page: usize,
    pub selected_person_row: usize,
    pub details_view: DetailsView,
    pub selection_focus: SelectionFocus,
    pub active_tab: RunningTab,
    pub chart_scope: ChartScope,
    pub world_history: Vec<MetricPoint>,
    pub country_history: Vec<Vec<MetricPoint>>,
}

impl World {
    pub fn new(countries: Vec<Country>) -> Self {
        let country_count = countries.len();
        let mut world = Self {
            day: 0,
            countries,
            selected_country: 0,
            paused: false,
            people_page: 0,
            selected_person_row: 0,
            details_view: DetailsView::List,
            selection_focus: SelectionFocus::Country,
            active_tab: RunningTab::Main,
            chart_scope: ChartScope::SelectedCountry,
            world_history: Vec::with_capacity(HISTORY_LIMIT),
            country_history: vec![Vec::with_capacity(HISTORY_LIMIT); country_count],
        };
        world.capture_snapshot();
        world
    }

    pub fn total_population(&self) -> usize {
        self.countries.iter().map(Country::population).sum()
    }

    pub fn selected_country(&self) -> Option<&Country> {
        self.countries.get(self.selected_country)
    }

    pub fn focus_country_selection(&mut self) {
        self.selection_focus = SelectionFocus::Country;
    }

    pub fn focus_people_selection(&mut self) {
        self.selection_focus = SelectionFocus::People;
    }

    pub fn select_next_country_row(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;
        let max_index = self.countries.len().saturating_sub(1);
        if self.selected_country < max_index {
            self.selected_country += 1;
            self.reset_people_navigation();
        }
    }

    pub fn select_prev_country_row(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;
        let prev = self.selected_country.saturating_sub(1);
        if prev != self.selected_country {
            self.selected_country = prev;
            self.reset_people_navigation();
        }
    }

    pub fn total_country_pages(&self) -> usize {
        if self.countries.is_empty() {
            1
        } else {
            self.countries.len().div_ceil(COUNTRY_PAGE_SIZE)
        }
    }

    pub fn country_page(&self) -> usize {
        self.selected_country / COUNTRY_PAGE_SIZE
    }

    pub fn selected_countries_page(&self) -> &[Country] {
        if self.countries.is_empty() {
            return &self.countries[0..0];
        }

        let start = self.country_page().saturating_mul(COUNTRY_PAGE_SIZE);
        let end = (start + COUNTRY_PAGE_SIZE).min(self.countries.len());
        &self.countries[start..end]
    }

    pub fn next_country_page(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;

        let current_page = self.country_page();
        let next_start = (current_page + 1).saturating_mul(COUNTRY_PAGE_SIZE);
        if next_start >= self.countries.len() {
            return;
        }

        let row_in_page = self.selected_country % COUNTRY_PAGE_SIZE;
        let next_index = (next_start + row_in_page).min(self.countries.len() - 1);
        if next_index != self.selected_country {
            self.selected_country = next_index;
            self.reset_people_navigation();
        }
    }

    pub fn prev_country_page(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;

        let current_page = self.country_page();
        if current_page == 0 {
            return;
        }

        let prev_start = (current_page - 1).saturating_mul(COUNTRY_PAGE_SIZE);
        let row_in_page = self.selected_country % COUNTRY_PAGE_SIZE;
        let prev_index = (prev_start + row_in_page).min(self.countries.len() - 1);
        if prev_index != self.selected_country {
            self.selected_country = prev_index;
            self.reset_people_navigation();
        }
    }

    pub fn total_people_pages(&self) -> usize {
        let Some(country) = self.selected_country() else {
            return 1;
        };

        if country.people.is_empty() {
            1
        } else {
            country.people.len().div_ceil(PEOPLE_PAGE_SIZE)
        }
    }

    pub fn selected_people_page(&self) -> Option<&[Person]> {
        let country = self.selected_country()?;
        let len = country.people.len();
        if len == 0 {
            return Some(&country.people[0..0]);
        }

        let start = self.people_page.saturating_mul(PEOPLE_PAGE_SIZE);
        if start >= len {
            return Some(&country.people[0..0]);
        }

        let end = (start + PEOPLE_PAGE_SIZE).min(len);
        Some(&country.people[start..end])
    }

    pub fn next_people_page(&mut self) {
        self.selection_focus = SelectionFocus::People;
        let max_page = self.total_people_pages().saturating_sub(1);
        if self.people_page < max_page {
            self.people_page += 1;
        }
        self.clamp_selected_person_row();
    }

    pub fn prev_people_page(&mut self) {
        self.selection_focus = SelectionFocus::People;
        self.people_page = self.people_page.saturating_sub(1);
        self.clamp_selected_person_row();
    }

    pub fn selected_people_page_len(&self) -> usize {
        self.selected_people_page().map_or(0, |page| page.len())
    }

    pub fn selected_person_on_page(&self) -> Option<&Person> {
        self.selected_people_page()?.get(self.selected_person_row)
    }

    pub fn select_next_person_row(&mut self) {
        self.selection_focus = SelectionFocus::People;
        let len = self.selected_people_page_len();
        if len == 0 {
            self.selected_person_row = 0;
            return;
        }

        let max_row = len.saturating_sub(1);
        if self.selected_person_row < max_row {
            self.selected_person_row += 1;
        }
    }

    pub fn select_prev_person_row(&mut self) {
        self.selection_focus = SelectionFocus::People;
        self.selected_person_row = self.selected_person_row.saturating_sub(1);
    }

    pub fn open_selected_person_details(&mut self) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_person_on_page().is_some() {
            self.details_view = DetailsView::Person;
        }
    }

    pub fn open_selected_country_details(&mut self) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_country().is_some() {
            self.details_view = DetailsView::Country;
        }
    }

    pub fn close_details(&mut self) {
        self.details_view = DetailsView::List;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            RunningTab::Main => RunningTab::Charts,
            RunningTab::Charts => RunningTab::Main,
        };
    }

    pub fn prev_tab(&mut self) {
        self.next_tab();
    }

    pub fn toggle_chart_scope(&mut self) {
        self.chart_scope = match self.chart_scope {
            ChartScope::SelectedCountry => ChartScope::WorldAggregate,
            ChartScope::WorldAggregate => ChartScope::SelectedCountry,
        };
    }

    pub fn chart_prev_country(&mut self) {
        self.select_prev_country_row();
    }

    pub fn chart_next_country(&mut self) {
        self.select_next_country_row();
    }

    pub fn completed_month_boundaries(&self) -> u64 {
        self.day / DAYS_PER_MONTH
    }

    pub fn apply_processed_month(&mut self, mut processed_countries: Vec<Country>) {
        if processed_countries.len() != self.countries.len() {
            return;
        }

        for (processed, current) in processed_countries.iter_mut().zip(&self.countries) {
            processed.day = current.day;
        }

        self.countries = processed_countries;
        self.capture_snapshot();
    }

    fn capture_snapshot(&mut self) {
        for (idx, country) in self.countries.iter().enumerate() {
            let point = Self::country_metric_point(country);
            if let Some(history) = self.country_history.get_mut(idx) {
                Self::push_metric_point(history, point);
            }
        }

        let aggregate = self.aggregate_metric_point();
        Self::push_metric_point(&mut self.world_history, aggregate);
    }

    fn aggregate_metric_point(&self) -> MetricPoint {
        let month = self
            .countries
            .iter()
            .map(|country| country.month)
            .max()
            .unwrap_or(0);
        let population = self
            .countries
            .iter()
            .map(|country| country.population() as f64)
            .sum();
        let total_money_supply = self
            .countries
            .iter()
            .map(|country| country.total_money_supply)
            .sum();
        let created_money_outstanding = self
            .countries
            .iter()
            .map(|country| country.bank.created_money_outstanding)
            .sum();
        let bank_reserve = self
            .countries
            .iter()
            .map(|country| country.bank.reserve_balance)
            .sum();

        MetricPoint {
            month,
            population,
            total_money_supply,
            created_money_outstanding,
            bank_reserve,
        }
    }

    fn country_metric_point(country: &Country) -> MetricPoint {
        MetricPoint {
            month: country.month,
            population: country.population() as f64,
            total_money_supply: country.total_money_supply,
            created_money_outstanding: country.bank.created_money_outstanding,
            bank_reserve: country.bank.reserve_balance,
        }
    }

    fn push_metric_point(history: &mut Vec<MetricPoint>, point: MetricPoint) {
        history.push(point);
        if history.len() > HISTORY_LIMIT {
            history.remove(0);
        }
    }

    fn reset_people_navigation(&mut self) {
        self.people_page = 0;
        self.selected_person_row = 0;
        self.details_view = DetailsView::List;
    }

    fn clamp_selected_person_row(&mut self) {
        let len = self.selected_people_page_len();
        if len == 0 {
            self.selected_person_row = 0;
        } else {
            self.selected_person_row = self.selected_person_row.min(len - 1);
        }
    }
}

impl Tickable for World {
    fn tick_day(&mut self) {
        self.day = self.day.saturating_add(1);

        for country in &mut self.countries {
            country.tick_day_without_month();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_people(count: usize) -> Vec<Person> {
        (0..count)
            .map(|idx| {
                Person::new(
                    idx as u32,
                    (idx % 10) as u8,
                    (idx % 4) as u16,
                    idx as u32 + 10,
                )
            })
            .collect()
    }

    #[test]
    fn world_tick_increments_day_and_ticks_countries() {
        let country = Country::new(1, "Axion".to_string(), build_people(2));
        let mut world = World::new(vec![country]);

        world.tick_day();

        assert_eq!(world.day, 1);
        assert_eq!(world.countries[0].day, 1);
        assert!((world.countries[0].people[0].cash_balance - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn country_row_selection_clamps_instead_of_wrapping() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(1)),
            Country::new(2, "B".to_string(), build_people(1)),
        ];
        let mut world = World::new(countries);

        world.select_prev_country_row();
        assert_eq!(world.selected_country, 0);

        world.select_next_country_row();
        world.select_next_country_row();
        assert_eq!(world.selected_country, 1);
    }

    #[test]
    fn people_pagination_clamps_at_bounds() {
        let country = Country::new(1, "Page".to_string(), build_people(PEOPLE_PAGE_SIZE + 5));
        let mut world = World::new(vec![country]);

        world.select_next_person_row();
        world.select_next_person_row();
        world.next_people_page();
        assert_eq!(world.people_page, 1);
        assert_eq!(world.selected_person_row, 2);

        world.next_people_page();
        assert_eq!(world.people_page, 1);

        world.prev_people_page();
        world.prev_people_page();
        assert_eq!(world.people_page, 0);
        assert_eq!(world.selected_person_row, 2);
    }

    #[test]
    fn selected_person_row_clamps_at_page_bounds() {
        let country = Country::new(1, "Rows".to_string(), build_people(2));
        let mut world = World::new(vec![country]);

        world.select_prev_person_row();
        assert_eq!(world.selected_person_row, 0);

        world.select_next_person_row();
        world.select_next_person_row();
        world.select_next_person_row();
        assert_eq!(world.selected_person_row, 1);
    }

    #[test]
    fn page_change_clamps_selected_row_for_short_last_page() {
        let country = Country::new(1, "Clamp".to_string(), build_people(PEOPLE_PAGE_SIZE + 1));
        let mut world = World::new(vec![country]);
        world.selected_person_row = PEOPLE_PAGE_SIZE - 1;

        world.next_people_page();

        assert_eq!(world.people_page, 1);
        assert_eq!(world.selected_person_row, 0);
    }

    #[test]
    fn country_selection_resets_people_navigation_and_details_state() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(3)),
            Country::new(2, "B".to_string(), build_people(3)),
        ];
        let mut world = World::new(countries);
        world.selected_person_row = 2;
        world.details_view = DetailsView::Person;

        world.select_next_country_row();

        assert_eq!(world.people_page, 0);
        assert_eq!(world.selected_person_row, 0);
        assert_eq!(world.details_view, DetailsView::List);
        assert_eq!(world.selection_focus, SelectionFocus::Country);
    }

    #[test]
    fn open_and_close_person_details_requires_selected_person() {
        let mut empty_world = World::new(vec![Country::new(1, "Empty".to_string(), vec![])]);

        empty_world.open_selected_person_details();
        assert_eq!(empty_world.details_view, DetailsView::List);

        let mut world = World::new(vec![Country::new(1, "Filled".to_string(), build_people(1))]);
        world.open_selected_person_details();
        assert_eq!(world.details_view, DetailsView::Person);

        world.close_details();
        assert_eq!(world.details_view, DetailsView::List);
    }

    #[test]
    fn open_and_close_country_details_requires_selected_country() {
        let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(1))]);

        world.open_selected_country_details();
        assert_eq!(world.details_view, DetailsView::Country);

        world.close_details();
        assert_eq!(world.details_view, DetailsView::List);
    }

    #[test]
    fn selection_focus_switches_between_country_and_people() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(3)),
            Country::new(2, "B".to_string(), build_people(3)),
        ];
        let mut world = World::new(countries);
        assert_eq!(world.selection_focus, SelectionFocus::Country);

        world.select_next_person_row();
        assert_eq!(world.selection_focus, SelectionFocus::People);

        world.select_next_country_row();
        assert_eq!(world.selection_focus, SelectionFocus::Country);
    }

    #[test]
    fn country_page_navigation_moves_selection_by_page() {
        let countries = (0..(COUNTRY_PAGE_SIZE * 2 + 1))
            .map(|idx| Country::new(idx as u16, format!("C{idx}"), build_people(1)))
            .collect();
        let mut world = World::new(countries);
        world.selected_country = 2;

        world.next_country_page();
        assert_eq!(world.selected_country, COUNTRY_PAGE_SIZE + 2);

        world.prev_country_page();
        assert_eq!(world.selected_country, 2);
    }

    #[test]
    fn world_starts_with_tab_scope_and_initial_history() {
        let world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

        assert_eq!(world.active_tab, RunningTab::Main);
        assert_eq!(world.chart_scope, ChartScope::SelectedCountry);
        assert_eq!(world.world_history.len(), 1);
        assert_eq!(world.country_history.len(), 1);
        assert_eq!(world.country_history[0].len(), 1);
        assert_eq!(world.world_history[0].month, 0);
    }

    #[test]
    fn world_tick_advances_days_without_running_monthly_processing() {
        let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(2))]);

        for _ in 0..DAYS_PER_MONTH {
            world.tick_day();
        }
        assert_eq!(world.world_history.len(), 1);
        assert_eq!(world.countries[0].month, 0);
        assert_eq!(world.completed_month_boundaries(), 1);
    }

    #[test]
    fn world_history_retains_only_latest_limit() {
        let mut world = World::new(vec![Country::new(1, "A".to_string(), build_people(1))]);

        for _ in 0..(HISTORY_LIMIT as u64 + 10) {
            for _ in 0..DAYS_PER_MONTH {
                world.tick_day();
            }
            let mut processed = world.countries.clone();
            for country in &mut processed {
                country.process_month();
            }
            world.apply_processed_month(processed);
        }

        assert_eq!(world.world_history.len(), HISTORY_LIMIT);
        assert_eq!(world.country_history[0].len(), HISTORY_LIMIT);
    }

    #[test]
    fn aggregate_history_matches_country_sums() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(2)),
            Country::new(2, "B".to_string(), build_people(3)),
        ];
        let world = World::new(countries);
        let aggregate = world.world_history[0];
        let country_sum = world.country_history.iter().map(|series| series[0]).fold(
            MetricPoint {
                month: 0,
                population: 0.0,
                total_money_supply: 0.0,
                created_money_outstanding: 0.0,
                bank_reserve: 0.0,
            },
            |acc, point| MetricPoint {
                month: acc.month.max(point.month),
                population: acc.population + point.population,
                total_money_supply: acc.total_money_supply + point.total_money_supply,
                created_money_outstanding: acc.created_money_outstanding
                    + point.created_money_outstanding,
                bank_reserve: acc.bank_reserve + point.bank_reserve,
            },
        );

        assert_eq!(aggregate.month, country_sum.month);
        assert!((aggregate.population - country_sum.population).abs() < f64::EPSILON);
        assert!(
            (aggregate.total_money_supply - country_sum.total_money_supply).abs() < f64::EPSILON
        );
        assert!(
            (aggregate.created_money_outstanding - country_sum.created_money_outstanding).abs()
                < f64::EPSILON
        );
        assert!((aggregate.bank_reserve - country_sum.bank_reserve).abs() < f64::EPSILON);
    }

    #[test]
    fn tab_scope_and_chart_country_commands_update_state() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(1)),
            Country::new(2, "B".to_string(), build_people(1)),
        ];
        let mut world = World::new(countries);

        world.next_tab();
        assert_eq!(world.active_tab, RunningTab::Charts);
        world.prev_tab();
        assert_eq!(world.active_tab, RunningTab::Main);

        world.toggle_chart_scope();
        assert_eq!(world.chart_scope, ChartScope::WorldAggregate);

        world.chart_next_country();
        assert_eq!(world.selected_country, 1);
        world.chart_prev_country();
        assert_eq!(world.selected_country, 0);
    }
}
