use crate::sim::{UiCommand, person::Person, world::World};

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UiState {
    pub details_view: DetailsView,
    pub selection_focus: SelectionFocus,
    pub active_tab: RunningTab,
    pub chart_scope: ChartScope,
    pub selected_country: usize,
    pub people_page: usize,
    pub selected_person_row: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            details_view: DetailsView::List,
            selection_focus: SelectionFocus::Country,
            active_tab: RunningTab::Main,
            chart_scope: ChartScope::SelectedCountry,
            selected_country: 0,
            people_page: 0,
            selected_person_row: 0,
        }
    }
}

impl UiState {
    pub fn apply(&mut self, command: UiCommand, world: &World) {
        self.ensure_valid(world);
        match command {
            UiCommand::FocusCountrySelection => self.selection_focus = SelectionFocus::Country,
            UiCommand::FocusPeopleSelection => self.selection_focus = SelectionFocus::People,
            UiCommand::SelectPrevCountryRow => self.select_prev_country_row(world),
            UiCommand::SelectNextCountryRow => self.select_next_country_row(world),
            UiCommand::SelectPrevPersonRow => self.select_prev_person_row(world),
            UiCommand::SelectNextPersonRow => self.select_next_person_row(world),
            UiCommand::CountryPageNext => self.next_country_page(world),
            UiCommand::CountryPagePrev => self.prev_country_page(world),
            UiCommand::PeoplePageNext => self.next_people_page(world),
            UiCommand::PeoplePagePrev => self.prev_people_page(world),
            UiCommand::OpenSelectedPersonDetails => self.open_selected_person_details(world),
            UiCommand::OpenSelectedCountryDetails => self.open_selected_country_details(world),
            UiCommand::CloseDetails => self.details_view = DetailsView::List,
            UiCommand::NextTab => self.next_tab(),
            UiCommand::PrevTab => self.prev_tab(),
            UiCommand::ToggleChartScope => self.toggle_chart_scope(),
            UiCommand::ChartPrevCountry => self.select_prev_country_row(world),
            UiCommand::ChartNextCountry => self.select_next_country_row(world),
        }
    }

    pub fn selected_country<'a>(
        &self,
        world: &'a World,
    ) -> Option<&'a crate::sim::country::Country> {
        world.countries.get(self.selected_country)
    }

    pub fn total_country_pages(&self, world: &World) -> usize {
        if world.countries.is_empty() {
            1
        } else {
            world.countries.len().div_ceil(COUNTRY_PAGE_SIZE)
        }
    }

    pub fn country_page(&self) -> usize {
        self.selected_country / COUNTRY_PAGE_SIZE
    }

    pub fn selected_countries_page<'a>(
        &self,
        world: &'a World,
    ) -> &'a [crate::sim::country::Country] {
        if world.countries.is_empty() {
            return &world.countries[0..0];
        }

        let start = self.country_page().saturating_mul(COUNTRY_PAGE_SIZE);
        let end = (start + COUNTRY_PAGE_SIZE).min(world.countries.len());
        &world.countries[start..end]
    }

    pub fn total_people_pages(&self, world: &World) -> usize {
        let Some(country) = self.selected_country(world) else {
            return 1;
        };
        if country.people.is_empty() {
            1
        } else {
            country.people.len().div_ceil(PEOPLE_PAGE_SIZE)
        }
    }

    pub fn selected_people_page<'a>(&self, world: &'a World) -> Option<&'a [Person]> {
        let country = self.selected_country(world)?;
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

    pub fn selected_people_page_len(&self, world: &World) -> usize {
        self.selected_people_page(world)
            .map_or(0, |page| page.len())
    }

    pub fn selected_person_on_page<'a>(&self, world: &'a World) -> Option<&'a Person> {
        self.selected_people_page(world)?
            .get(self.selected_person_row)
    }

    pub fn ensure_valid(&mut self, world: &World) {
        if world.countries.is_empty() {
            self.selected_country = 0;
            self.people_page = 0;
            self.selected_person_row = 0;
            self.details_view = DetailsView::List;
            return;
        }

        self.selected_country = self.selected_country.min(world.countries.len() - 1);
        let people_pages = self.total_people_pages(world);
        self.people_page = self.people_page.min(people_pages.saturating_sub(1));
        self.clamp_selected_person_row(world);

        if matches!(self.details_view, DetailsView::Person)
            && self.selected_person_on_page(world).is_none()
        {
            self.details_view = DetailsView::List;
        }
    }

    fn select_next_country_row(&mut self, world: &World) {
        if world.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;
        let max_index = world.countries.len().saturating_sub(1);
        if self.selected_country < max_index {
            self.selected_country += 1;
            self.reset_people_navigation();
        }
    }

    fn select_prev_country_row(&mut self, world: &World) {
        if world.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;
        let prev = self.selected_country.saturating_sub(1);
        if prev != self.selected_country {
            self.selected_country = prev;
            self.reset_people_navigation();
        }
    }

    fn next_country_page(&mut self, world: &World) {
        if world.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;

        let current_page = self.country_page();
        let next_start = (current_page + 1).saturating_mul(COUNTRY_PAGE_SIZE);
        if next_start >= world.countries.len() {
            return;
        }

        let row_in_page = self.selected_country % COUNTRY_PAGE_SIZE;
        let next_index = (next_start + row_in_page).min(world.countries.len() - 1);
        if next_index != self.selected_country {
            self.selected_country = next_index;
            self.reset_people_navigation();
        }
    }

    fn prev_country_page(&mut self, world: &World) {
        if world.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;

        let current_page = self.country_page();
        if current_page == 0 {
            return;
        }

        let prev_start = (current_page - 1).saturating_mul(COUNTRY_PAGE_SIZE);
        let row_in_page = self.selected_country % COUNTRY_PAGE_SIZE;
        let prev_index = (prev_start + row_in_page).min(world.countries.len() - 1);
        if prev_index != self.selected_country {
            self.selected_country = prev_index;
            self.reset_people_navigation();
        }
    }

    fn next_people_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::People;
        let max_page = self.total_people_pages(world).saturating_sub(1);
        if self.people_page < max_page {
            self.people_page += 1;
        }
        self.clamp_selected_person_row(world);
    }

    fn prev_people_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::People;
        self.people_page = self.people_page.saturating_sub(1);
        self.clamp_selected_person_row(world);
    }

    fn select_next_person_row(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::People;
        let len = self.selected_people_page_len(world);
        if len == 0 {
            self.selected_person_row = 0;
            return;
        }

        let max_row = len.saturating_sub(1);
        if self.selected_person_row < max_row {
            self.selected_person_row += 1;
        }
    }

    fn select_prev_person_row(&mut self, _world: &World) {
        self.selection_focus = SelectionFocus::People;
        self.selected_person_row = self.selected_person_row.saturating_sub(1);
    }

    fn open_selected_person_details(&mut self, world: &World) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_person_on_page(world).is_some() {
            self.details_view = DetailsView::Person;
        }
    }

    fn open_selected_country_details(&mut self, world: &World) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_country(world).is_some() {
            self.details_view = DetailsView::Country;
        }
    }

    fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            RunningTab::Main => RunningTab::Charts,
            RunningTab::Charts => RunningTab::Main,
        };
    }

    fn prev_tab(&mut self) {
        self.next_tab();
    }

    fn toggle_chart_scope(&mut self) {
        self.chart_scope = match self.chart_scope {
            ChartScope::SelectedCountry => ChartScope::WorldAggregate,
            ChartScope::WorldAggregate => ChartScope::SelectedCountry,
        };
    }

    fn reset_people_navigation(&mut self) {
        self.people_page = 0;
        self.selected_person_row = 0;
        self.details_view = DetailsView::List;
    }

    fn clamp_selected_person_row(&mut self, world: &World) {
        let len = self.selected_people_page_len(world);
        if len == 0 {
            self.selected_person_row = 0;
        } else {
            self.selected_person_row = self.selected_person_row.min(len - 1);
        }
    }
}
