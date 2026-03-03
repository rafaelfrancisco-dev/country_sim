use super::*;

impl UiState {
    pub fn ensure_valid(&mut self, world: &World) {
        if world.countries.is_empty() {
            self.selected_country = 0;
            self.chart_page = 0;
            self.people_page = 0;
            self.selected_person_row = 0;
            self.company_page = 0;
            self.selected_company_row = 0;
            self.details_view = DetailsView::List;
            return;
        }

        self.selected_country = self.selected_country.min(world.countries.len() - 1);
        self.chart_page = self.chart_page.min(CHART_PAGE_COUNT.saturating_sub(1));
        let people_pages = self.total_people_pages(world);
        self.people_page = self.people_page.min(people_pages.saturating_sub(1));
        self.clamp_selected_person_row(world);

        let company_pages = self.total_company_pages(world);
        self.company_page = self.company_page.min(company_pages.saturating_sub(1));
        self.clamp_selected_company_row(world);

        if matches!(self.details_view, DetailsView::Person)
            && self.selected_person_on_page(world).is_none()
        {
            self.details_view = DetailsView::List;
        }
        if matches!(self.details_view, DetailsView::Company)
            && self.selected_company_on_page(world).is_none()
        {
            self.details_view = DetailsView::List;
        }
    }

    pub(super) fn select_next_country_row(&mut self, world: &World) {
        if world.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;
        let max_index = world.countries.len().saturating_sub(1);
        if self.selected_country < max_index {
            self.selected_country += 1;
            self.reset_country_dependent_navigation();
        }
    }

    pub(super) fn select_prev_country_row(&mut self, world: &World) {
        if world.countries.is_empty() {
            return;
        }

        self.selection_focus = SelectionFocus::Country;
        let prev = self.selected_country.saturating_sub(1);
        if prev != self.selected_country {
            self.selected_country = prev;
            self.reset_country_dependent_navigation();
        }
    }

    pub(super) fn next_country_page(&mut self, world: &World) {
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
            self.reset_country_dependent_navigation();
        }
    }

    pub(super) fn prev_country_page(&mut self, world: &World) {
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
            self.reset_country_dependent_navigation();
        }
    }

    pub(super) fn next_people_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::People;
        let max_page = self.total_people_pages(world).saturating_sub(1);
        if self.people_page < max_page {
            self.people_page += 1;
        }
        self.clamp_selected_person_row(world);
    }

    pub(super) fn prev_people_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::People;
        self.people_page = self.people_page.saturating_sub(1);
        self.clamp_selected_person_row(world);
    }

    pub(super) fn select_next_person_row(&mut self, world: &World) {
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

    pub(super) fn select_prev_person_row(&mut self, _world: &World) {
        self.selection_focus = SelectionFocus::People;
        self.selected_person_row = self.selected_person_row.saturating_sub(1);
    }

    pub(super) fn next_company_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Company;
        let max_page = self.total_company_pages(world).saturating_sub(1);
        if self.company_page < max_page {
            self.company_page += 1;
        }
        self.clamp_selected_company_row(world);
    }

    pub(super) fn prev_company_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Company;
        self.company_page = self.company_page.saturating_sub(1);
        self.clamp_selected_company_row(world);
    }

    pub(super) fn select_next_company_row(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Company;
        let len = self.selected_companies_page_len(world);
        if len == 0 {
            self.selected_company_row = 0;
            return;
        }

        let max_row = len.saturating_sub(1);
        if self.selected_company_row < max_row {
            self.selected_company_row += 1;
        }
    }

    pub(super) fn select_prev_company_row(&mut self, _world: &World) {
        self.selection_focus = SelectionFocus::Company;
        self.selected_company_row = self.selected_company_row.saturating_sub(1);
    }

    pub(super) fn next_chart_page(&mut self) {
        let max_page = CHART_PAGE_COUNT.saturating_sub(1);
        if self.chart_page < max_page {
            self.chart_page += 1;
        }
    }

    pub(super) fn prev_chart_page(&mut self) {
        self.chart_page = self.chart_page.saturating_sub(1);
    }
}
