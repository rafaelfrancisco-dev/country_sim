use super::super::{
    BANK_PAGE_SIZE, COMPANY_PAGE_SIZE, COUNTRY_PAGE_SIZE, PEOPLE_PAGE_SIZE, UiState,
};
use crate::sim::{bank::Bank, company::Company, person::Person, world::World};

impl UiState {
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
