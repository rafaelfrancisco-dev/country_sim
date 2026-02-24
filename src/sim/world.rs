use super::{country::Country, person::Person, tickable::Tickable};

pub const PEOPLE_PAGE_SIZE: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub day: u64,
    pub countries: Vec<Country>,
    pub selected_country: usize,
    pub paused: bool,
    pub people_page: usize,
}

impl World {
    pub fn new(countries: Vec<Country>) -> Self {
        Self {
            day: 0,
            countries,
            selected_country: 0,
            paused: false,
            people_page: 0,
        }
    }

    pub fn total_population(&self) -> usize {
        self.countries.iter().map(Country::population).sum()
    }

    pub fn selected_country(&self) -> Option<&Country> {
        self.countries.get(self.selected_country)
    }

    pub fn select_next_country(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        self.selected_country = (self.selected_country + 1) % self.countries.len();
        self.people_page = 0;
    }

    pub fn select_prev_country(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        self.selected_country = if self.selected_country == 0 {
            self.countries.len() - 1
        } else {
            self.selected_country - 1
        };
        self.people_page = 0;
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
        let max_page = self.total_people_pages().saturating_sub(1);
        if self.people_page < max_page {
            self.people_page += 1;
        }
    }

    pub fn prev_people_page(&mut self) {
        self.people_page = self.people_page.saturating_sub(1);
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
    fn country_selection_wraps_in_both_directions() {
        let countries = vec![
            Country::new(1, "A".to_string(), build_people(1)),
            Country::new(2, "B".to_string(), build_people(1)),
        ];

        let mut world = World::new(countries);

        world.select_prev_country();
        assert_eq!(world.selected_country, 1);

        world.select_next_country();
        assert_eq!(world.selected_country, 0);
    }

    #[test]
    fn people_pagination_clamps_at_bounds() {
        let country = Country::new(1, "Page".to_string(), build_people(PEOPLE_PAGE_SIZE + 5));
        let mut world = World::new(vec![country]);

        world.next_people_page();
        assert_eq!(world.people_page, 1);

        world.next_people_page();
        assert_eq!(world.people_page, 1);

        world.prev_people_page();
        world.prev_people_page();
        assert_eq!(world.people_page, 0);
    }
}
