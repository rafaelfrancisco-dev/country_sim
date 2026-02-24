use super::{
    company::{DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies},
    country::Country,
    job::{JobTypeId, common_job_types},
    money::{from_cents, round_cents, to_cents},
    names::country_name_from_seed,
    person::Person,
};

pub const DEFAULT_MONEY_PER_CITIZEN: f64 = 50_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct GenerationConfig {
    pub country_count: u16,
    pub population_per_country: u32,
    pub money_per_citizen: f64,
    pub companies_per_country: u16,
    pub seed: u64,
}

impl GenerationConfig {
    pub fn with_defaults(country_count: u16, population_per_country: u32, seed: u64) -> Self {
        Self {
            country_count,
            population_per_country,
            money_per_citizen: DEFAULT_MONEY_PER_CITIZEN,
            companies_per_country: DEFAULT_COMPANIES_PER_COUNTRY,
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
}

pub fn generate_countries(config: &GenerationConfig) -> Vec<Country> {
    let mut rng = DeterministicRng::new(config.seed);
    let mut countries = Vec::with_capacity(config.country_count as usize);

    for country_idx in 0..config.country_count {
        let country_seed = rng.next_u64() ^ ((country_idx as u64) << 32);
        let country_name = country_name_from_seed(country_seed);
        let mut companies = build_default_companies(&country_name, config.companies_per_country);

        let mut people = Vec::with_capacity(config.population_per_country as usize);
        for person_id in 0..config.population_per_country {
            let job_type_id = pick_weighted_job_id(&mut rng);
            let employer_company_id = pick_employer_for_job(job_type_id, &companies, &mut rng);
            let name_seed = rng.next_u32();
            people.push(Person::new(
                person_id,
                job_type_id,
                employer_company_id,
                name_seed,
            ));
        }

        let total_supply =
            round_cents(config.population_per_country as f64 * config.money_per_citizen);
        let household_supply = round_cents(total_supply * 0.5);
        let company_supply = round_cents(total_supply - household_supply);

        seed_balances_to_people(&mut people, household_supply);
        seed_balances_to_companies(&mut companies, company_supply);

        countries.push(Country::with_economy(
            country_idx,
            country_name,
            people,
            companies,
            total_supply,
        ));
    }

    countries
}

fn pick_weighted_job_id(rng: &mut DeterministicRng) -> JobTypeId {
    let jobs = common_job_types();
    let total_weight: u64 = jobs
        .iter()
        .map(|job| u64::from(job.assignment_weight))
        .sum();

    if total_weight == 0 {
        return 0;
    }

    let mut roll = rng.next_u64() % total_weight;
    for job in jobs {
        let weight = u64::from(job.assignment_weight);
        if roll < weight {
            return job.id;
        }
        roll -= weight;
    }

    jobs.last().map_or(0, |job| job.id)
}

fn pick_employer_for_job(
    job_type_id: JobTypeId,
    companies: &[super::company::Company],
    rng: &mut DeterministicRng,
) -> u16 {
    if companies.is_empty() {
        return 0;
    }

    let mut selected_company = None;
    let mut seen_matching = 0_u64;
    for (idx, company) in companies.iter().enumerate() {
        if !company.supports_job(job_type_id) {
            continue;
        }

        seen_matching = seen_matching.saturating_add(1);
        if rng.next_u64() % seen_matching == 0 {
            selected_company = Some(idx as u16);
        }
    }

    selected_company.unwrap_or_else(|| (rng.next_u64() as usize % companies.len()) as u16)
}

fn seed_balances_to_people(people: &mut [Person], total_supply: f64) {
    if people.is_empty() {
        return;
    }

    let total_cents = to_cents(total_supply);
    let base = total_cents / people.len() as i64;
    let remainder = total_cents % people.len() as i64;

    for (idx, person) in people.iter_mut().enumerate() {
        let extra = if (idx as i64) < remainder { 1 } else { 0 };
        person.cash_balance = from_cents(base + extra);
    }
}

fn seed_balances_to_companies(companies: &mut [super::company::Company], total_supply: f64) {
    if companies.is_empty() {
        return;
    }

    let total_cents = to_cents(total_supply);
    let base = total_cents / companies.len() as i64;
    let remainder = total_cents % companies.len() as i64;

    for (idx, company) in companies.iter_mut().enumerate() {
        let extra = if (idx as i64) < remainder { 1 } else { 0 };
        company.cash_balance = from_cents(base + extra);
    }
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
    fn every_company_offers_multiple_job_types() {
        let countries = generate_countries(&config(2));

        assert!(
            countries
                .iter()
                .flat_map(|country| country.companies.iter())
                .all(|company| company.offered_job_types.len() > 1)
        );
    }

    #[test]
    fn initial_money_supply_is_split_between_people_and_companies() {
        let countries = generate_countries(&config(3));

        for country in countries {
            assert!((country.household_cash_total - country.company_cash_total).abs() <= 0.01);
            let expected_total = round_cents(country.initial_money_supply);
            assert!((country.total_money_supply - expected_total).abs() <= 0.01);
        }
    }

    #[test]
    fn generation_is_deterministic_for_same_seed() {
        let cfg = config(4);
        let first = generate_countries(&cfg);
        let second = generate_countries(&cfg);

        assert_eq!(first, second);
    }
}
