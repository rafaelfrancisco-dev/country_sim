use super::*;
use crate::sim::good::common_goods;

impl UiState {
    pub fn ensure_valid(&mut self, world: &World) {
        if world.countries.is_empty() {
            self.selected_country = 0;
            self.chart_page = 0;
            self.details_page = 0;
            self.people_page = 0;
            self.selected_person_row = 0;
            self.company_page = 0;
            self.selected_company_row = 0;
            self.goods_main_page = GoodsMainPage::Companies;
            self.goods_production_page = 0;
            self.bank_page = 0;
            self.selected_bank_row = 0;
            self.disaster_selected_row = 0;
            self.company_workers_scroll = 0;
            self.bank_depositors_scroll = 0;
            self.disaster_last_apply_status = None;
            self.details_view = DetailsView::List;
            self.country_policy_form = CountryPolicyFormState::default();
            return;
        }

        self.selected_country = self.selected_country.min(world.countries.len() - 1);
        self.chart_page = self.chart_page.min(CHART_PAGE_COUNT.saturating_sub(1));
        self.goods_production_page = self
            .goods_production_page
            .min(self.total_goods_production_pages().saturating_sub(1));
        let people_pages = self.total_people_pages(world);
        self.people_page = self.people_page.min(people_pages.saturating_sub(1));
        self.clamp_selected_person_row(world);

        let company_pages = self.total_company_pages(world);
        self.company_page = self.company_page.min(company_pages.saturating_sub(1));
        self.clamp_selected_company_row(world);
        let bank_pages = self.total_bank_pages(world);
        self.bank_page = self.bank_page.min(bank_pages.saturating_sub(1));
        self.clamp_selected_bank_row(world);
        self.clamp_selected_disaster_row();
        for intensity_idx in &mut self.disaster_intensity_index {
            *intensity_idx = (*intensity_idx).min(DISASTER_INTENSITY_COUNT.saturating_sub(1));
        }

        if matches!(self.details_view, DetailsView::Person)
            && self.selected_person_on_page(world).is_none()
        {
            self.details_view = DetailsView::List;
            self.details_page = 0;
            self.company_workers_scroll = 0;
        }
        if matches!(self.details_view, DetailsView::Company)
            && self.selected_company_on_page(world).is_none()
        {
            self.details_view = DetailsView::List;
            self.details_page = 0;
            self.company_workers_scroll = 0;
        }
        if matches!(self.details_view, DetailsView::Bank)
            && self.selected_bank_on_page(world).is_none()
        {
            self.details_view = DetailsView::List;
            self.details_page = 0;
            self.bank_depositors_scroll = 0;
        }
        if matches!(self.details_view, DetailsView::CountryPolicy)
            && self.selected_country(world).is_none()
        {
            self.details_view = DetailsView::List;
            self.details_page = 0;
            self.company_workers_scroll = 0;
            self.country_policy_form = CountryPolicyFormState::default();
        }

        if let Some(max_page) = self.max_details_page_for_view() {
            self.details_page = self.details_page.min(max_page);
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
        let total = self
            .selected_country(world)
            .map_or(0, |c| c.companies.len());
        if total == 0 {
            return;
        }
        let abs = self.selected_company_absolute_index();
        let new_abs = (abs + COMPANY_PAGE_SIZE).min(total.saturating_sub(1));
        self.company_page = new_abs / COMPANY_PAGE_SIZE;
        self.selected_company_row = new_abs % COMPANY_PAGE_SIZE;
    }

    pub(super) fn prev_company_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Company;
        let total = self
            .selected_country(world)
            .map_or(0, |c| c.companies.len());
        if total == 0 {
            return;
        }
        let abs = self.selected_company_absolute_index();
        let new_abs = abs.saturating_sub(COMPANY_PAGE_SIZE);
        self.company_page = new_abs / COMPANY_PAGE_SIZE;
        self.selected_company_row = new_abs % COMPANY_PAGE_SIZE;
    }

    pub(super) fn select_next_company_row(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Company;
        let total = self
            .selected_country(world)
            .map_or(0, |c| c.companies.len());
        if total == 0 {
            self.selected_company_row = 0;
            return;
        }
        let abs = self.selected_company_absolute_index();
        if abs + 1 < total {
            let new_abs = abs + 1;
            self.company_page = new_abs / COMPANY_PAGE_SIZE;
            self.selected_company_row = new_abs % COMPANY_PAGE_SIZE;
        }
    }

    pub(super) fn select_prev_company_row(&mut self, _world: &World) {
        self.selection_focus = SelectionFocus::Company;
        let abs = self.selected_company_absolute_index();
        if abs > 0 {
            let new_abs = abs - 1;
            self.company_page = new_abs / COMPANY_PAGE_SIZE;
            self.selected_company_row = new_abs % COMPANY_PAGE_SIZE;
        }
    }

    pub(super) fn next_goods_production_page(&mut self) {
        let max_page = self.total_goods_production_pages().saturating_sub(1);
        if self.goods_production_page < max_page {
            self.goods_production_page += 1;
        }
    }

    pub(super) fn prev_goods_production_page(&mut self) {
        self.goods_production_page = self.goods_production_page.saturating_sub(1);
    }

    pub(super) fn next_bank_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Bank;
        let max_page = self.total_bank_pages(world).saturating_sub(1);
        if self.bank_page < max_page {
            self.bank_page += 1;
        }
        self.clamp_selected_bank_row(world);
    }

    pub(super) fn prev_bank_page(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Bank;
        self.bank_page = self.bank_page.saturating_sub(1);
        self.clamp_selected_bank_row(world);
    }

    pub fn total_goods_production_pages(&self) -> usize {
        common_goods()
            .len()
            .div_ceil(GOODS_PRODUCTION_PAGE_SIZE)
            .max(1)
    }

    pub(super) fn select_next_bank_row(&mut self, world: &World) {
        self.selection_focus = SelectionFocus::Bank;
        let len = self.selected_banks_page_len(world);
        if len == 0 {
            self.selected_bank_row = 0;
            return;
        }

        let max_row = len.saturating_sub(1);
        if self.selected_bank_row < max_row {
            self.selected_bank_row += 1;
        }
    }

    pub(super) fn select_prev_bank_row(&mut self, _world: &World) {
        self.selection_focus = SelectionFocus::Bank;
        self.selected_bank_row = self.selected_bank_row.saturating_sub(1);
    }

    pub(super) fn select_next_fx_country(&mut self, world: &World) {
        if let Some(next) = fx_sorted_next(world, self.selected_country) {
            self.selected_country = next;
            self.reset_country_dependent_navigation();
        }
    }

    pub(super) fn select_prev_fx_country(&mut self, world: &World) {
        if let Some(prev) = fx_sorted_prev(world, self.selected_country) {
            self.selected_country = prev;
            self.reset_country_dependent_navigation();
        }
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

/// Returns the country index that comes after `current` in the FX table's sorted order
/// (sorted by fx_rate_local_per_base descending, then by name ascending).
fn fx_sorted_next(world: &World, current: usize) -> Option<usize> {
    let sorted = fx_sorted_indices(world);
    let pos = sorted.iter().position(|&i| i == current)?;
    sorted.get(pos + 1).copied()
}

/// Returns the country index that comes before `current` in the FX table's sorted order.
fn fx_sorted_prev(world: &World, current: usize) -> Option<usize> {
    let sorted = fx_sorted_indices(world);
    let pos = sorted.iter().position(|&i| i == current)?;
    if pos == 0 { None } else { sorted.get(pos - 1).copied() }
}

/// Country indices sorted the same way as the FX leaderboard table.
fn fx_sorted_indices(world: &World) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..world.countries.len()).collect();
    indices.sort_by(|&a, &b| {
        let rate_a = world.countries[a].fx_rate_local_per_base();
        let rate_b = world.countries[b].fx_rate_local_per_base();
        rate_b
            .partial_cmp(&rate_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| world.countries[a].name().cmp(world.countries[b].name()))
    });
    indices
}
