use super::*;

impl UiState {
    pub(super) fn open_selected_person_details(&mut self, world: &World) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_person_on_page(world).is_some() {
            self.details_view = DetailsView::Person;
        }
    }

    pub(super) fn open_selected_country_details(&mut self, world: &World) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_country(world).is_some() {
            self.details_view = DetailsView::Country;
        }
    }

    pub(super) fn open_selected_company_details(&mut self, world: &World) {
        if self.details_view != DetailsView::List {
            return;
        }

        if self.selected_company_on_page(world).is_some() {
            self.details_view = DetailsView::Company;
            self.active_tab = RunningTab::Goods;
            self.selection_focus = SelectionFocus::Company;
        }
    }

    pub(super) fn jump_to_employer_company(&mut self, world: &World) {
        let Some(person) = self.selected_person_on_page(world) else {
            return;
        };
        let Some(employer_company_id) = person.employer_company_id() else {
            return;
        };

        self.active_tab = RunningTab::Goods;
        self.selection_focus = SelectionFocus::Company;

        let absolute_index = employer_company_id as usize;
        self.company_page = absolute_index / COMPANY_PAGE_SIZE;
        self.selected_company_row = absolute_index % COMPANY_PAGE_SIZE;
        self.clamp_selected_company_row(world);

        if self.selected_company_on_page(world).is_some() {
            self.details_view = DetailsView::Company;
        }
    }

    pub(super) fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            RunningTab::Main => RunningTab::Goods,
            RunningTab::Goods => RunningTab::Stocks,
            RunningTab::Stocks => RunningTab::Charts,
            RunningTab::Charts => RunningTab::Events,
            RunningTab::Events => RunningTab::Main,
        };
    }

    pub(super) fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            RunningTab::Main => RunningTab::Events,
            RunningTab::Goods => RunningTab::Main,
            RunningTab::Stocks => RunningTab::Goods,
            RunningTab::Charts => RunningTab::Stocks,
            RunningTab::Events => RunningTab::Charts,
        };
    }

    pub(super) fn toggle_chart_scope(&mut self) {
        self.chart_scope = match self.chart_scope {
            ChartScope::SelectedCountry => ChartScope::WorldAggregate,
            ChartScope::WorldAggregate => ChartScope::SelectedCountry,
        };
    }

    pub(super) fn reset_country_dependent_navigation(&mut self) {
        self.people_page = 0;
        self.selected_person_row = 0;
        self.company_page = 0;
        self.selected_company_row = 0;
        self.details_view = DetailsView::List;
    }

    pub(super) fn clamp_selected_person_row(&mut self, world: &World) {
        let len = self.selected_people_page_len(world);
        if len == 0 {
            self.selected_person_row = 0;
        } else {
            self.selected_person_row = self.selected_person_row.min(len - 1);
        }
    }

    pub(super) fn clamp_selected_company_row(&mut self, world: &World) {
        let len = self.selected_companies_page_len(world);
        if len == 0 {
            self.selected_company_row = 0;
        } else {
            self.selected_company_row = self.selected_company_row.min(len - 1);
        }
    }
}
