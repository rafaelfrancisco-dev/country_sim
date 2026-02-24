use super::{person::Person, tickable::Tickable};

#[derive(Debug, Clone, PartialEq)]
pub struct Country {
    pub id: u16,
    pub name: String,
    pub people: Vec<Person>,
    pub total_balance: i64,
    pub avg_net_per_day: f64,
}

impl Country {
    pub fn new(id: u16, name: String, people: Vec<Person>) -> Self {
        let mut country = Self {
            id,
            name,
            people,
            total_balance: 0,
            avg_net_per_day: 0.0,
        };
        country.recompute_aggregates();
        country
    }

    pub fn population(&self) -> usize {
        self.people.len()
    }

    pub fn recompute_aggregates(&mut self) {
        self.total_balance = self
            .people
            .iter()
            .map(|person| i64::from(person.balance))
            .sum();

        if self.people.is_empty() {
            self.avg_net_per_day = 0.0;
            return;
        }

        let total_daily_net: i64 = self
            .people
            .iter()
            .map(|person| i64::from(person.daily_net()))
            .sum();

        self.avg_net_per_day = total_daily_net as f64 / self.people.len() as f64;
    }
}

impl Tickable for Country {
    fn tick_day(&mut self) {
        for person in &mut self.people {
            person.tick_day();
        }

        self.recompute_aggregates();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_person(id: u32, income: u16, expense: u16) -> Person {
        Person::new(id, income, expense, id + 10)
    }

    #[test]
    fn country_tick_updates_all_people_and_aggregates() {
        let people = vec![create_person(1, 100, 60), create_person(2, 120, 100)];
        let mut country = Country::new(1, "Solaria".to_string(), people);

        country.tick_day();

        assert_eq!(country.people[0].balance, 40);
        assert_eq!(country.people[1].balance, 20);
        assert_eq!(country.total_balance, 60);
    }

    #[test]
    fn country_avg_net_per_day_is_correct() {
        let people = vec![
            create_person(1, 200, 100),
            create_person(2, 120, 100),
            create_person(3, 80, 140),
        ];

        let country = Country::new(2, "Arion".to_string(), people);

        assert!((country.avg_net_per_day - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_country_has_zero_aggregates() {
        let country = Country::new(0, "Empty".to_string(), Vec::new());

        assert_eq!(country.total_balance, 0);
        assert!((country.avg_net_per_day - 0.0).abs() < f64::EPSILON);
    }
}
