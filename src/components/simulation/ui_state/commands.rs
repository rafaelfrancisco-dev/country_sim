use super::*;

impl UiState {
    pub fn apply(&mut self, command: UiCommand, world: &World) {
        self.ensure_valid(world);
        match command {
            UiCommand::FocusCountrySelection => self.selection_focus = SelectionFocus::Country,
            UiCommand::FocusPeopleSelection => self.selection_focus = SelectionFocus::People,
            UiCommand::FocusCompanySelection => self.selection_focus = SelectionFocus::Company,
            UiCommand::SelectPrevCountryRow => self.select_prev_country_row(world),
            UiCommand::SelectNextCountryRow => self.select_next_country_row(world),
            UiCommand::SelectPrevPersonRow => self.select_prev_person_row(world),
            UiCommand::SelectNextPersonRow => self.select_next_person_row(world),
            UiCommand::SelectPrevCompanyRow => self.select_prev_company_row(world),
            UiCommand::SelectNextCompanyRow => self.select_next_company_row(world),
            UiCommand::CountryPageNext => self.next_country_page(world),
            UiCommand::CountryPagePrev => self.prev_country_page(world),
            UiCommand::PeoplePageNext => self.next_people_page(world),
            UiCommand::PeoplePagePrev => self.prev_people_page(world),
            UiCommand::CompanyPageNext => self.next_company_page(world),
            UiCommand::CompanyPagePrev => self.prev_company_page(world),
            UiCommand::OpenSelectedPersonDetails => self.open_selected_person_details(world),
            UiCommand::OpenSelectedCountryDetails => self.open_selected_country_details(world),
            UiCommand::OpenSelectedCompanyDetails => self.open_selected_company_details(world),
            UiCommand::JumpToEmployerCompany => self.jump_to_employer_company(world),
            UiCommand::CloseDetails => {
                self.details_view = DetailsView::List;
                self.details_page = 0;
            }
            UiCommand::DetailsPagePrev => {
                self.details_page = self.details_page.saturating_sub(1);
            }
            UiCommand::DetailsPageNext => {
                self.details_page = self.details_page.saturating_add(1);
            }
            UiCommand::ToggleSimMode => {}
            UiCommand::NextTab => self.next_tab(),
            UiCommand::PrevTab => self.prev_tab(),
            UiCommand::ToggleChartScope => self.toggle_chart_scope(),
            UiCommand::ChartPagePrev => self.prev_chart_page(),
            UiCommand::ChartPageNext => self.next_chart_page(),
            UiCommand::ChartPrevCountry => self.select_prev_country_row(world),
            UiCommand::ChartNextCountry => self.select_next_country_row(world),
            UiCommand::GoodsPrevCountry => self.select_prev_country_row(world),
            UiCommand::GoodsNextCountry => self.select_next_country_row(world),
            UiCommand::EventsScrollUp => {
                self.events_scroll = self.events_scroll.saturating_sub(1);
            }
            UiCommand::EventsScrollDown => {
                self.events_scroll = self.events_scroll.saturating_add(1);
            }
            UiCommand::EventsToggleSort => {
                self.events_sort = match self.events_sort {
                    EventsSort::NewestFirst => EventsSort::OldestFirst,
                    EventsSort::OldestFirst => EventsSort::NewestFirst,
                };
            }
            UiCommand::EventsToggleSearch => {
                self.events_search_active = !self.events_search_active;
            }
            UiCommand::EventsSearchChar(c) => {
                self.events_search_query.push(c);
            }
            UiCommand::EventsSearchBackspace => {
                self.events_search_query.pop();
            }
            UiCommand::EventsSearchClear => {
                self.events_search_query.clear();
            }
            UiCommand::StocksPrevCountry => self.select_prev_country_row(world),
            UiCommand::StocksNextCountry => self.select_next_country_row(world),
            UiCommand::StocksFocusCountry => {
                self.selection_focus = SelectionFocus::Country;
            }
            UiCommand::StocksFocusTable => {
                self.selection_focus = SelectionFocus::Stock;
            }
            UiCommand::StocksPrevRow => {
                self.stock_selected_row = self.stock_selected_row.saturating_sub(1);
            }
            UiCommand::StocksNextRow => {
                self.stock_selected_row = self.stock_selected_row.saturating_add(1);
            }
            UiCommand::StocksPagePrev => {
                self.stock_page = self.stock_page.saturating_sub(1);
                self.stock_selected_row = 0;
            }
            UiCommand::StocksPageNext => {
                self.stock_page = self.stock_page.saturating_add(1);
                self.stock_selected_row = 0;
            }
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

    pub fn total_company_pages(&self, world: &World) -> usize {
        let Some(country) = self.selected_country(world) else {
            return 1;
        };
        if country.companies.is_empty() {
            1
        } else {
            country.companies.len().div_ceil(COMPANY_PAGE_SIZE)
        }
    }

    pub fn selected_companies_page<'a>(&self, world: &'a World) -> Option<&'a [Company]> {
        let country = self.selected_country(world)?;
        let len = country.companies.len();
        if len == 0 {
            return Some(&country.companies[0..0]);
        }

        let start = self.company_page.saturating_mul(COMPANY_PAGE_SIZE);
        if start >= len {
            return Some(&country.companies[0..0]);
        }

        let end = (start + COMPANY_PAGE_SIZE).min(len);
        Some(&country.companies[start..end])
    }

    pub fn selected_companies_page_len(&self, world: &World) -> usize {
        self.selected_companies_page(world)
            .map_or(0, |page| page.len())
    }

    pub fn selected_company_on_page<'a>(&self, world: &'a World) -> Option<&'a Company> {
        self.selected_companies_page(world)?
            .get(self.selected_company_row)
    }

    pub fn selected_company_absolute_index(&self) -> usize {
        self.company_page
            .saturating_mul(COMPANY_PAGE_SIZE)
            .saturating_add(self.selected_company_row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{country::Country, person::Person};

    fn build_people(count: usize) -> Vec<Person> {
        (0..count)
            .map(|idx| Person::new(idx as u32, (idx % 10) as u8, 0, idx as u32 + 1))
            .collect()
    }

    fn sample_world() -> World {
        World::new(vec![Country::new(1, "A".to_string(), build_people(4))])
    }

    fn sample_world_two_countries() -> World {
        World::new(vec![
            Country::new(1, "A".to_string(), build_people(4)),
            Country::new(2, "B".to_string(), build_people(3)),
        ])
    }

    #[test]
    fn chart_page_commands_clamp_to_valid_range() {
        let world = sample_world();
        let mut ui = UiState::default();

        for _ in 0..16 {
            ui.apply(UiCommand::ChartPageNext, &world);
        }
        assert_eq!(ui.chart_page, CHART_PAGE_COUNT - 1);

        for _ in 0..16 {
            ui.apply(UiCommand::ChartPagePrev, &world);
        }
        assert_eq!(ui.chart_page, 0);
    }

    #[test]
    fn ensure_valid_clamps_chart_page() {
        let world = sample_world();
        let mut ui = UiState {
            chart_page: CHART_PAGE_COUNT + 5,
            ..UiState::default()
        };

        ui.ensure_valid(&world);
        assert_eq!(ui.chart_page, CHART_PAGE_COUNT - 1);
    }

    #[test]
    fn ensure_valid_resets_chart_page_when_world_is_empty() {
        let world = World::new(vec![]);
        let mut ui = UiState {
            chart_page: 2,
            ..UiState::default()
        };

        ui.ensure_valid(&world);
        assert_eq!(ui.chart_page, 0);
    }

    #[test]
    fn details_page_commands_adjust_and_close_resets() {
        let world = sample_world();
        let mut ui = UiState {
            details_view: DetailsView::Person,
            details_page: 0,
            ..UiState::default()
        };

        ui.apply(UiCommand::DetailsPageNext, &world);
        ui.apply(UiCommand::DetailsPageNext, &world);
        assert_eq!(ui.details_page, 2);

        ui.apply(UiCommand::DetailsPagePrev, &world);
        assert_eq!(ui.details_page, 1);

        ui.apply(UiCommand::CloseDetails, &world);
        assert_eq!(ui.details_view, DetailsView::List);
        assert_eq!(ui.details_page, 0);
    }

    #[test]
    fn opening_details_resets_details_page() {
        let world = sample_world();
        let mut ui = UiState {
            selection_focus: SelectionFocus::People,
            details_page: 4,
            ..UiState::default()
        };

        ui.apply(UiCommand::OpenSelectedPersonDetails, &world);
        assert_eq!(ui.details_view, DetailsView::Person);
        assert_eq!(ui.details_page, 0);
    }

    #[test]
    fn changing_country_resets_details_page() {
        let world = sample_world_two_countries();
        let mut ui = UiState {
            details_view: DetailsView::Country,
            details_page: 6,
            people_page: 1,
            company_page: 1,
            selected_country: 0,
            ..UiState::default()
        };

        ui.apply(UiCommand::SelectNextCountryRow, &world);
        assert_eq!(ui.selected_country, 1);
        assert_eq!(ui.details_view, DetailsView::List);
        assert_eq!(ui.details_page, 0);
        assert_eq!(ui.people_page, 0);
        assert_eq!(ui.company_page, 0);
    }
}
