use super::tickable::Tickable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: u32,
    pub income: u16,
    pub expense: u16,
    pub balance: i32,
    pub name_seed: u32,
}

impl Person {
    pub fn new(id: u32, income: u16, expense: u16, name_seed: u32) -> Self {
        Self {
            id,
            income,
            expense,
            balance: 0,
            name_seed,
        }
    }

    pub fn daily_net(&self) -> i32 {
        i32::from(self.income) - i32::from(self.expense)
    }
}

impl Tickable for Person {
    fn tick_day(&mut self) {
        self.balance = self.balance.saturating_add(self.daily_net());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_day_updates_balance() {
        let mut person = Person::new(1, 100, 60, 42);

        person.tick_day();

        assert_eq!(person.balance, 40);
    }

    #[test]
    fn tick_day_supports_negative_daily_net() {
        let mut person = Person::new(1, 80, 120, 13);

        person.tick_day();

        assert_eq!(person.balance, -40);
    }

    #[test]
    fn repeated_ticks_stay_within_expected_bounds() {
        let mut person = Person::new(7, 200, 40, 21);

        for _ in 0..100_000 {
            person.tick_day();
        }

        assert_eq!(person.balance, 16_000_000);
        assert!(person.balance > 0);
    }
}
