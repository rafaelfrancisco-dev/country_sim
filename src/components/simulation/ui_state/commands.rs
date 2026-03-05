use super::*;
use crate::sim::{CountryPolicyOverrides, GameCommand};

impl UiState {
    pub fn apply(&mut self, command: UiCommand, world: &World) -> Option<GameCommand> {
        self.ensure_valid(world);
        let mut pending_domain_command = None;
        match command {
            UiCommand::FocusCountrySelection => self.selection_focus = SelectionFocus::Country,
            UiCommand::FocusPeopleSelection => self.selection_focus = SelectionFocus::People,
            UiCommand::FocusCompanySelection => self.selection_focus = SelectionFocus::Company,
            UiCommand::FocusBankSelection => self.selection_focus = SelectionFocus::Bank,
            UiCommand::SelectPrevCountryRow => self.select_prev_country_row(world),
            UiCommand::SelectNextCountryRow => self.select_next_country_row(world),
            UiCommand::SelectPrevPersonRow => self.select_prev_person_row(world),
            UiCommand::SelectNextPersonRow => self.select_next_person_row(world),
            UiCommand::SelectPrevCompanyRow => self.select_prev_company_row(world),
            UiCommand::SelectNextCompanyRow => self.select_next_company_row(world),
            UiCommand::SelectPrevBankRow => self.select_prev_bank_row(world),
            UiCommand::SelectNextBankRow => self.select_next_bank_row(world),
            UiCommand::CountryPageNext => self.next_country_page(world),
            UiCommand::CountryPagePrev => self.prev_country_page(world),
            UiCommand::PeoplePageNext => self.next_people_page(world),
            UiCommand::PeoplePagePrev => self.prev_people_page(world),
            UiCommand::CompanyPageNext => self.next_company_page(world),
            UiCommand::CompanyPagePrev => self.prev_company_page(world),
            UiCommand::BankPageNext => self.next_bank_page(world),
            UiCommand::BankPagePrev => self.prev_bank_page(world),
            UiCommand::OpenSelectedPersonDetails => self.open_selected_person_details(world),
            UiCommand::OpenSelectedCountryDetails => self.open_selected_country_details(world),
            UiCommand::OpenSelectedCountryPolicyDetails => {
                self.open_selected_country_policy_details(world);
            }
            UiCommand::OpenSelectedCompanyDetails => self.open_selected_company_details(world),
            UiCommand::OpenSelectedBankDetails => self.open_selected_bank_details(world),
            UiCommand::JumpToEmployerCompany => self.jump_to_employer_company(world),
            UiCommand::CloseCountryPolicyDetails => {
                if self.details_view == DetailsView::CountryPolicy {
                    self.details_view = DetailsView::Country;
                    self.details_page = 0;
                    self.country_policy_form.is_editing = false;
                    self.country_policy_form.draft_input.clear();
                    self.country_policy_form.validation_error = None;
                }
            }
            UiCommand::CountryPolicyPrevField => {
                if self.details_view == DetailsView::CountryPolicy
                    && !self.country_policy_form.is_editing
                {
                    self.country_policy_form.active_field =
                        self.country_policy_form.active_field.prev();
                    self.country_policy_form.validation_error = None;
                }
            }
            UiCommand::CountryPolicyNextField => {
                if self.details_view == DetailsView::CountryPolicy
                    && !self.country_policy_form.is_editing
                {
                    self.country_policy_form.active_field =
                        self.country_policy_form.active_field.next();
                    self.country_policy_form.validation_error = None;
                }
            }
            UiCommand::CountryPolicyStartEdit => {
                if self.details_view == DetailsView::CountryPolicy
                    && !self.country_policy_form.is_editing
                    && let Some(country) = self.selected_country(world)
                {
                    let value = Self::country_policy_field_value(
                        country,
                        self.country_policy_form.active_field,
                    );
                    self.country_policy_form.draft_input = Self::format_country_policy_value(
                        self.country_policy_form.active_field,
                        value,
                    );
                    self.country_policy_form.is_editing = true;
                    self.country_policy_form.validation_error = None;
                }
            }
            UiCommand::CountryPolicyEditChar(c) => {
                if self.details_view == DetailsView::CountryPolicy
                    && self.country_policy_form.is_editing
                    && (c.is_ascii_digit() || c == '.')
                {
                    self.country_policy_form.draft_input.push(c);
                }
            }
            UiCommand::CountryPolicyBackspace => {
                if self.details_view == DetailsView::CountryPolicy
                    && self.country_policy_form.is_editing
                {
                    self.country_policy_form.draft_input.pop();
                }
            }
            UiCommand::CountryPolicySaveEdit => {
                pending_domain_command = self.commit_country_policy_edit(world);
            }
            UiCommand::CountryPolicyCancelEdit => {
                if self.details_view == DetailsView::CountryPolicy
                    && self.country_policy_form.is_editing
                {
                    self.country_policy_form.is_editing = false;
                    self.country_policy_form.draft_input.clear();
                    self.country_policy_form.validation_error = None;
                }
            }
            UiCommand::CountryPolicyResetManaged => {
                if self.details_view == DetailsView::CountryPolicy
                    && !self.country_policy_form.is_editing
                    && self.selected_country(world).is_some()
                {
                    self.country_policy_form.validation_error = None;
                    pending_domain_command = Some(GameCommand::UpdateCountryPolicy {
                        country_idx: self.selected_country,
                        policy: CountryPolicyOverrides::managed_defaults(),
                    });
                }
            }
            UiCommand::CloseDetails => {
                self.details_view = DetailsView::List;
                self.details_page = 0;
                self.company_workers_scroll = 0;
                self.bank_depositors_scroll = 0;
                self.country_policy_form = CountryPolicyFormState::default();
            }
            UiCommand::DetailsPagePrev => {
                self.details_page = self.details_page.saturating_sub(1);
            }
            UiCommand::DetailsPageNext => {
                if let Some(max_page) = self.max_details_page_for_view() {
                    self.details_page = self.details_page.saturating_add(1).min(max_page);
                } else {
                    self.details_page = self.details_page.saturating_add(1);
                }
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
            UiCommand::BanksPrevCountry => self.select_prev_country_row(world),
            UiCommand::BanksNextCountry => self.select_next_country_row(world),
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
            UiCommand::EventsNextCategory => {
                self.events_category = self.events_category.next();
                self.events_scroll = 0;
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
            UiCommand::CompanyWorkersScrollUp => {
                self.company_workers_scroll = self.company_workers_scroll.saturating_sub(1);
            }
            UiCommand::CompanyWorkersScrollDown => {
                self.company_workers_scroll = self.company_workers_scroll.saturating_add(1);
            }
            UiCommand::BankDepositorsScrollUp => {
                self.bank_depositors_scroll = self.bank_depositors_scroll.saturating_sub(1);
            }
            UiCommand::BankDepositorsScrollDown => {
                self.bank_depositors_scroll = self.bank_depositors_scroll.saturating_add(1);
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
            UiCommand::DisastersPrevCountry => self.select_prev_country_row(world),
            UiCommand::DisastersNextCountry => self.select_next_country_row(world),
            UiCommand::DisastersPrevRow => {
                self.disaster_selected_row = self.disaster_selected_row.saturating_sub(1);
            }
            UiCommand::DisastersNextRow => {
                self.disaster_selected_row = self.disaster_selected_row.saturating_add(1);
            }
            UiCommand::DisastersPrevIntensity => {
                self.adjust_disaster_intensity(-1);
            }
            UiCommand::DisastersNextIntensity => {
                self.adjust_disaster_intensity(1);
            }
            UiCommand::DisastersToggleEnabled => {
                if let Some(enabled) = self.disaster_enabled.get_mut(self.disaster_selected_row) {
                    *enabled = !*enabled;
                }
            }
            UiCommand::DisastersApply => {
                let requests = self.build_disaster_requests();
                if requests.is_empty() {
                    self.disaster_last_apply_status =
                        Some("Enable at least one disaster before applying.".to_string());
                } else {
                    self.disaster_last_apply_status =
                        Some(format!("Applied {} disaster(s).", requests.len()));
                    pending_domain_command = Some(GameCommand::ApplyDisasters {
                        country_idx: self.selected_country,
                        requests,
                    });
                }
            }
        }

        pending_domain_command
    }

    pub(super) fn max_details_page_for_view(&self) -> Option<usize> {
        match self.details_view {
            DetailsView::Person => Some(1),
            DetailsView::Company => Some(3),
            DetailsView::Bank => Some(3),
            DetailsView::CountryPolicy | DetailsView::List => Some(0),
            DetailsView::Country => None,
        }
    }

    fn commit_country_policy_edit(&mut self, world: &World) -> Option<GameCommand> {
        if self.details_view != DetailsView::CountryPolicy || !self.country_policy_form.is_editing {
            return None;
        }
        let country = self.selected_country(world)?;

        let field = self.country_policy_form.active_field;
        let raw_input = self.country_policy_form.draft_input.trim();
        let Ok(value) = raw_input.parse::<f64>() else {
            self.country_policy_form.validation_error = Some("Value must be numeric.".to_string());
            return None;
        };
        if !value.is_finite() {
            self.country_policy_form.validation_error = Some("Value must be finite.".to_string());
            return None;
        }

        let tax = country.sim_params.tax;
        let is_valid = match field {
            CountryPolicyField::TaxBracket1Max => value >= 0.0,
            CountryPolicyField::TaxBracket2Max => value >= tax.bracket_1_max.max(0.0),
            CountryPolicyField::TaxRate1
            | CountryPolicyField::TaxRate2
            | CountryPolicyField::TaxRate3
            | CountryPolicyField::UnemploymentBenefitRate => (0.0..=1.0).contains(&value),
            CountryPolicyField::MedianIncomeReference
            | CountryPolicyField::BasicNeedsCashThreshold => value >= 0.0,
        };
        if !is_valid {
            self.country_policy_form.validation_error =
                Some(Self::country_policy_validation_message(field).to_string());
            return None;
        }

        let mut policy = CountryPolicyOverrides::from_country(country);
        match field {
            CountryPolicyField::TaxBracket1Max => policy.bracket_1_max = value,
            CountryPolicyField::TaxBracket2Max => policy.bracket_2_max = value,
            CountryPolicyField::TaxRate1 => policy.rate_1 = value,
            CountryPolicyField::TaxRate2 => policy.rate_2 = value,
            CountryPolicyField::TaxRate3 => policy.rate_3 = value,
            CountryPolicyField::UnemploymentBenefitRate => policy.unemployment_benefit_rate = value,
            CountryPolicyField::MedianIncomeReference => policy.median_income_reference = value,
            CountryPolicyField::BasicNeedsCashThreshold => {
                policy.basic_needs_cash_threshold = value
            }
        }

        self.country_policy_form.is_editing = false;
        self.country_policy_form.draft_input.clear();
        self.country_policy_form.validation_error = None;

        Some(GameCommand::UpdateCountryPolicy {
            country_idx: self.selected_country,
            policy,
        })
    }

    fn country_policy_validation_message(field: CountryPolicyField) -> &'static str {
        match field {
            CountryPolicyField::TaxBracket1Max => "Bracket 1 max must be >= 0.",
            CountryPolicyField::TaxBracket2Max => "Bracket 2 max must be >= Bracket 1 max.",
            CountryPolicyField::TaxRate1
            | CountryPolicyField::TaxRate2
            | CountryPolicyField::TaxRate3 => "Tax rates must be between 0 and 1.",
            CountryPolicyField::UnemploymentBenefitRate => {
                "Unemployment benefit rate must be between 0 and 1."
            }
            CountryPolicyField::MedianIncomeReference => "Median income reference must be >= 0.",
            CountryPolicyField::BasicNeedsCashThreshold => {
                "Basic-needs cash threshold must be >= 0."
            }
        }
    }

    fn country_policy_field_value(
        country: &crate::sim::country::Country,
        field: CountryPolicyField,
    ) -> f64 {
        match field {
            CountryPolicyField::TaxBracket1Max => country.sim_params.tax.bracket_1_max,
            CountryPolicyField::TaxBracket2Max => country.sim_params.tax.bracket_2_max,
            CountryPolicyField::TaxRate1 => country.sim_params.tax.rate_1,
            CountryPolicyField::TaxRate2 => country.sim_params.tax.rate_2,
            CountryPolicyField::TaxRate3 => country.sim_params.tax.rate_3,
            CountryPolicyField::UnemploymentBenefitRate => {
                country.sim_params.fiscal.unemployment_benefit_rate
            }
            CountryPolicyField::MedianIncomeReference => {
                country.sim_params.fiscal.median_income_reference
            }
            CountryPolicyField::BasicNeedsCashThreshold => {
                country.sim_params.fiscal.basic_needs_cash_threshold
            }
        }
    }

    fn format_country_policy_value(field: CountryPolicyField, value: f64) -> String {
        match field {
            CountryPolicyField::TaxRate1
            | CountryPolicyField::TaxRate2
            | CountryPolicyField::TaxRate3
            | CountryPolicyField::UnemploymentBenefitRate => format!("{value:.4}"),
            CountryPolicyField::TaxBracket1Max
            | CountryPolicyField::TaxBracket2Max
            | CountryPolicyField::MedianIncomeReference
            | CountryPolicyField::BasicNeedsCashThreshold => format!("{value:.2}"),
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

    pub fn total_bank_pages(&self, world: &World) -> usize {
        let Some(country) = self.selected_country(world) else {
            return 1;
        };
        let total = country.banks.len().saturating_add(1);
        if total == 0 {
            1
        } else {
            total.div_ceil(BANK_PAGE_SIZE)
        }
    }

    pub fn selected_banks_page_len(&self, world: &World) -> usize {
        let Some(country) = self.selected_country(world) else {
            return 0;
        };
        let total = country.banks.len().saturating_add(1);
        if total == 0 {
            return 0;
        }
        let start = self.bank_page.saturating_mul(BANK_PAGE_SIZE);
        if start >= total {
            return 0;
        }
        let end = (start + BANK_PAGE_SIZE).min(total);
        end.saturating_sub(start)
    }

    pub fn selected_bank_on_page<'a>(&self, world: &'a World) -> Option<&'a Bank> {
        self.selected_bank_by_absolute_index(world, self.selected_bank_absolute_index())
    }

    pub fn selected_bank_absolute_index(&self) -> usize {
        self.bank_page
            .saturating_mul(BANK_PAGE_SIZE)
            .saturating_add(self.selected_bank_row)
    }

    pub fn selected_bank_by_absolute_index<'a>(
        &self,
        world: &'a World,
        absolute_index: usize,
    ) -> Option<&'a Bank> {
        let country = self.selected_country(world)?;
        if absolute_index == 0 {
            return Some(&country.bank);
        }
        country.banks.get(absolute_index.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests;
