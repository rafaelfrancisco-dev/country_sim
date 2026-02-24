use super::{country::Country, names::country_name_from_seed, person::Person};

pub const DEFAULT_INCOME_MIN: u16 = 80;
pub const DEFAULT_INCOME_MAX: u16 = 200;
pub const DEFAULT_EXPENSE_MIN: u16 = 40;
pub const DEFAULT_EXPENSE_MAX: u16 = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationConfig {
    pub country_count: u16,
    pub population_per_country: u32,
    pub income_min: u16,
    pub income_max: u16,
    pub expense_min: u16,
    pub expense_max: u16,
    pub seed: u64,
}

impl GenerationConfig {
    pub fn with_defaults(country_count: u16, population_per_country: u32, seed: u64) -> Self {
        Self {
            country_count,
            population_per_country,
            income_min: DEFAULT_INCOME_MIN,
            income_max: DEFAULT_INCOME_MAX,
            expense_min: DEFAULT_EXPENSE_MIN,
            expense_max: DEFAULT_EXPENSE_MAX,
            seed,
        }
    }
}

#[derive(Debug, Clone)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn range_u16(&mut self, min: u16, max: u16) -> u16 {
        if min >= max {
            return min;
        }

        let span = u32::from(max - min) + 1;
        min + (self.next_u64() % u64::from(span)) as u16
    }
}

pub fn generate_countries(config: &GenerationConfig) -> Vec<Country> {
    let mut rng = DeterministicRng::new(config.seed);
    let mut countries = Vec::with_capacity(config.country_count as usize);

    for country_idx in 0..config.country_count {
        let country_seed = rng.next_u64() ^ ((country_idx as u64) << 32);
        let country_name = country_name_from_seed(country_seed);

        let mut people = Vec::with_capacity(config.population_per_country as usize);
        for person_id in 0..config.population_per_country {
            let income = rng.range_u16(config.income_min, config.income_max);
            let expense = rng.range_u16(config.expense_min, config.expense_max);
            let name_seed = rng.next_u32();
            people.push(Person::new(person_id, income, expense, name_seed));
        }

        countries.push(Country::new(country_idx, country_name, people));
    }

    countries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(seed: u64) -> GenerationConfig {
        GenerationConfig::with_defaults(2, 8, seed)
    }

    #[test]
    fn generated_counts_are_exact() {
        let countries = generate_countries(&config(1));

        assert_eq!(countries.len(), 2);
        assert_eq!(countries[0].people.len(), 8);
        assert_eq!(countries[1].people.len(), 8);
    }

    #[test]
    fn generated_economy_values_stay_in_range() {
        let cfg = config(2);
        let countries = generate_countries(&cfg);

        for country in countries {
            for person in country.people {
                assert!((cfg.income_min..=cfg.income_max).contains(&person.income));
                assert!((cfg.expense_min..=cfg.expense_max).contains(&person.expense));
            }
        }
    }

    #[test]
    fn generation_is_deterministic_for_same_seed() {
        let cfg = config(3);
        let first = generate_countries(&cfg);
        let second = generate_countries(&cfg);

        assert_eq!(first, second);
    }
}
