use super::{country::Country, person::Person, tickable::Tickable};

pub const PEOPLE_PAGE_SIZE: usize = 50;
pub const COUNTRY_PAGE_SIZE: usize = 10;

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
}

impl World {
    pub fn new(countries: Vec<Country>) -> Self {
        Self {
            day: 0,
            countries,
            selected_country: 0,
            paused: false,
            people_page: 0,
            selected_person_row: 0,
            details_view: DetailsView::List,
            selection_focus: SelectionFocus::Country,
        }
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
            country.tick_day();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_people(count: usize) -> Vec<Person> {
        (0..count)
            .map(|idx| Person::new(idx as u32, 100, 60, idx as u32 + 10))
            .collect()
    }

    #[test]
    fn world_tick_increments_day_and_ticks_countries() {
        let country = Country::new(1, "Axion".to_string(), build_people(2));
        let mut world = World::new(vec![country]);

        world.tick_day();

        assert_eq!(world.day, 1);
        assert_eq!(world.countries[0].people[0].balance, 40);
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
}
