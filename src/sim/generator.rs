use super::{
    company::{Company, DEFAULT_COMPANIES_PER_COUNTRY},
    country::Country,
    good::{GOODS_COUNT, common_goods, good_by_id},
    job::{JobTypeId, common_job_types},
    money::{from_cents, round_cents, to_cents},
    names::country_name_from_seed,
    params::SimulationParams,
    person::{EmploymentStatus, HabilitationLevel, Person},
};
use rayon::prelude::*;

pub const DEFAULT_MONEY_PER_CITIZEN: f64 = 50_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct GenerationConfig {
    pub country_count: u16,
    pub population_per_country: u32,
    pub money_per_citizen: f64,
    pub companies_per_country: u16,
    pub seed: u64,
    pub sim_params: SimulationParams,
}

impl GenerationConfig {
    pub fn with_defaults(country_count: u16, population_per_country: u32, seed: u64) -> Self {
        Self {
            country_count,
            population_per_country,
            money_per_citizen: DEFAULT_MONEY_PER_CITIZEN,
            companies_per_country: DEFAULT_COMPANIES_PER_COUNTRY,
            seed,
            sim_params: SimulationParams::default(),
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

    fn next_f64(&mut self) -> f64 {
        self.next_u64() as f64 / u64::MAX as f64
    }

    fn next_u16_inclusive(&mut self, min: u16, max: u16) -> u16 {
        if max <= min {
            return min;
        }

        let span = u32::from(max - min) + 1;
        min + (self.next_u32() % span) as u16
    }
}

const HABILITATION_BUCKETS: usize = 4;

#[derive(Debug, Clone, Default)]
struct EmployerPicker {
    company_ids: Vec<u16>,
    cumulative_weights: Vec<u64>,
    total_weight: u64,
}

#[derive(Debug, Clone)]
struct JobSelectionCache {
    job_ids_by_habilitation: [Vec<JobTypeId>; HABILITATION_BUCKETS],
    cumulative_weights_by_habilitation: [Vec<u64>; HABILITATION_BUCKETS],
    total_weights_by_habilitation: [u64; HABILITATION_BUCKETS],
    employer_pickers: Vec<EmployerPicker>,
}

fn mix_seed(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn country_seed(base_seed: u64, country_idx: u16) -> u64 {
    let country_salt = u64::from(country_idx)
        .wrapping_add(1)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mix_seed(base_seed ^ country_salt)
}

fn habilitation_bucket(level: HabilitationLevel) -> usize {
    match level {
        HabilitationLevel::NoSchoolYet => 0,
        HabilitationLevel::BasicSchool => 1,
        HabilitationLevel::HighSchool => 2,
        HabilitationLevel::HigherLearning => 3,
    }
}

fn build_job_selection_cache(
    companies: &[Company],
    rng: &mut DeterministicRng,
) -> JobSelectionCache {
    let jobs = common_job_types();
    let mut job_ids_by_habilitation: [Vec<JobTypeId>; HABILITATION_BUCKETS] =
        std::array::from_fn(|_| Vec::new());
    let mut cumulative_weights_by_habilitation: [Vec<u64>; HABILITATION_BUCKETS] =
        std::array::from_fn(|_| Vec::new());
    let mut total_weights_by_habilitation = [0_u64; HABILITATION_BUCKETS];

    for job in jobs {
        let required_bucket = habilitation_bucket(job.required_habilitation);
        for bucket in required_bucket..HABILITATION_BUCKETS {
            total_weights_by_habilitation[bucket] = total_weights_by_habilitation[bucket]
                .saturating_add(u64::from(job.assignment_weight));
            job_ids_by_habilitation[bucket].push(job.id);
            cumulative_weights_by_habilitation[bucket].push(total_weights_by_habilitation[bucket]);
        }
    }

    let hiring_weights = build_company_hiring_weights(companies, rng);
    let max_job_id = jobs.iter().map(|job| job.id as usize).max().unwrap_or(0);
    let mut employer_pickers = vec![EmployerPicker::default(); max_job_id.saturating_add(1)];
    for job in jobs {
        let mut picker = EmployerPicker::default();
        let mut cumulative = 0_u64;
        for (company_idx, company) in companies.iter().enumerate() {
            if !company.supports_job(job.id) {
                continue;
            }

            let weight = hiring_weights
                .get(company_idx)
                .copied()
                .map(u64::from)
                .unwrap_or(1);
            cumulative = cumulative.saturating_add(weight);
            picker.company_ids.push(company_idx as u16);
            picker.cumulative_weights.push(cumulative);
        }
        picker.total_weight = cumulative;
        employer_pickers[job.id as usize] = picker;
    }

    JobSelectionCache {
        job_ids_by_habilitation,
        cumulative_weights_by_habilitation,
        total_weights_by_habilitation,
        employer_pickers,
    }
}

pub fn generate_countries(config: &GenerationConfig) -> Vec<Country> {
    if config.country_count <= 1 {
        return (0..config.country_count)
            .map(|country_idx| generate_country(config, country_idx))
            .collect();
    }

    (0..config.country_count as usize)
        .into_par_iter()
        .map(|country_idx| generate_country(config, country_idx as u16))
        .collect()
}

fn generate_country(config: &GenerationConfig, country_idx: u16) -> Country {
    let mut rng = DeterministicRng::new(country_seed(config.seed, country_idx));
    let country_name = country_name_from_seed(country_seed(config.seed ^ 0xfeed_f00d, country_idx));
    let company_count = if config.companies_per_country == DEFAULT_COMPANIES_PER_COUNTRY {
        derive_initial_company_count(config.population_per_country)
    } else {
        config.companies_per_country
    };
    let mut companies =
        build_initial_companies(&country_name, config.population_per_country, company_count);
    randomize_company_prices(&mut companies, &config.sim_params, &mut rng);
    let job_cache = build_job_selection_cache(&companies, &mut rng);

    let mut people = Vec::with_capacity(config.population_per_country as usize);
    for person_id in 0..config.population_per_country {
        let age_days = pick_initial_age_days(&mut rng);
        let education_track = config.sim_params.education.pick_track(rng.next_f64());
        let name_seed = rng.next_u32();

        let mut person = Person::new_with_profile(
            person_id,
            EmploymentStatus::Unemployed,
            name_seed,
            age_days,
            education_track,
        );

        if person.can_seek_job()
            && rng.next_f64() >= config.sim_params.labor.baseline_unemployment_rate
            && let Some(job_type_id) =
                pick_weighted_job_id_for_habilitation(&mut rng, person.habilitation, &job_cache)
        {
            let employer_company_id = pick_employer_for_job(
                job_type_id,
                companies.len(),
                &job_cache.employer_pickers,
                &mut rng,
            );
            person.set_employment(employer_company_id, job_type_id, 0);
        }

        people.push(person);
    }

    let total_supply = round_cents(config.population_per_country as f64 * config.money_per_citizen);
    let household_supply = round_cents(total_supply * 0.5);
    let company_supply = round_cents(total_supply - household_supply);

    seed_balances_to_people(&mut people, household_supply);
    seed_balances_to_companies(&mut companies, company_supply);

    let mut country = Country::with_economy_and_params(
        country_idx,
        country_name,
        people,
        companies,
        total_supply,
        config.sim_params.clone(),
    );
    fund_initial_bank_reserve(&mut country);
    seed_initial_bank_openings(&mut country);
    super::generator_banks::seed_initial_private_banks(&mut country);
    country.seed_initial_ipos();
    country
}

fn derive_initial_company_count(population_per_country: u32) -> u16 {
    let total = target_initial_company_counts(population_per_country)
        .iter()
        .copied()
        .map(u32::from)
        .sum::<u32>();
    total
        .clamp(15, u32::from(u16::MAX))
        .try_into()
        .unwrap_or(u16::MAX)
}

fn target_initial_company_counts(population_per_country: u32) -> [u16; GOODS_COUNT] {
    let pop_scale = (population_per_country as f64 / 1_000.0).max(1.0);
    let market_depth = pop_scale.sqrt();
    std::array::from_fn(|good_idx| {
        let good = common_goods()[good_idx];
        let labor_load = good.monthly_demand_factor * good.effort_per_unit;
        let firms = ((market_depth * labor_load) / 12.0).ceil() as u16;
        firms.clamp(1, 8)
    })
}

fn build_initial_companies(
    country_name: &str,
    population_per_country: u32,
    company_count: u16,
) -> Vec<Company> {
    let company_count = usize::from(company_count);
    if company_count == 0 {
        return Vec::new();
    }

    let target_counts = target_initial_company_counts(population_per_country);
    let desired_total = target_counts
        .iter()
        .copied()
        .map(usize::from)
        .sum::<usize>();
    let company_goods = if company_count == desired_total {
        let mut goods = Vec::with_capacity(company_count);
        for (good_idx, count) in target_counts.iter().copied().enumerate() {
            for _ in 0..count {
                goods.push(common_goods()[good_idx].id);
            }
        }
        goods
    } else {
        allocate_company_goods_dhondt(company_count, &target_counts)
    };

    let available_job_ids = common_job_types()
        .iter()
        .map(|job| job.id)
        .collect::<Vec<_>>();
    company_goods
        .into_iter()
        .enumerate()
        .map(|(idx, good_id)| {
            Company::new(
                idx as u16,
                format!("{country_name} Co {}", idx + 1),
                available_job_ids.clone(),
                good_id,
            )
        })
        .collect()
}

fn allocate_company_goods_dhondt(company_count: usize, target_counts: &[u16]) -> Vec<u8> {
    let mut allocations = vec![0_u16; target_counts.len()];
    for _ in 0..company_count {
        let mut best_idx = 0usize;
        let mut best_score = f64::NEG_INFINITY;
        for (good_idx, &target) in target_counts.iter().enumerate() {
            let score = f64::from(target) / f64::from(allocations[good_idx].saturating_add(1));
            if score > best_score {
                best_score = score;
                best_idx = good_idx;
            }
        }
        allocations[best_idx] = allocations[best_idx].saturating_add(1);
    }

    let mut goods = Vec::with_capacity(company_count);
    for (good_idx, count) in allocations.into_iter().enumerate() {
        for _ in 0..count {
            goods.push(common_goods()[good_idx].id);
        }
    }
    goods
}

fn pick_initial_age_days(rng: &mut DeterministicRng) -> u16 {
    let roll = rng.next_f64();
    let age_years = if roll < 0.25 {
        rng.next_u16_inclusive(0, 14)
    } else if roll < 0.41 {
        rng.next_u16_inclusive(15, 24)
    } else if roll < 0.82 {
        rng.next_u16_inclusive(25, 54)
    } else if roll < 0.92 {
        rng.next_u16_inclusive(55, 64)
    } else if roll < 0.97 {
        rng.next_u16_inclusive(65, 74)
    } else {
        rng.next_u16_inclusive(75, 100)
    };

    let day_of_year = rng.next_u16_inclusive(0, 364);
    age_years.saturating_mul(365).saturating_add(day_of_year)
}

fn pick_weighted_job_id_for_habilitation(
    rng: &mut DeterministicRng,
    habilitation: HabilitationLevel,
    job_cache: &JobSelectionCache,
) -> Option<JobTypeId> {
    let bucket = habilitation_bucket(habilitation);
    let total_weight = job_cache.total_weights_by_habilitation[bucket];
    if total_weight == 0 {
        return None;
    }

    let roll = rng.next_u64() % total_weight;
    let index = job_cache.cumulative_weights_by_habilitation[bucket]
        .partition_point(|&weight| weight <= roll);
    job_cache.job_ids_by_habilitation[bucket]
        .get(index)
        .copied()
        .or_else(|| job_cache.job_ids_by_habilitation[bucket].last().copied())
}

fn pick_employer_for_job(
    job_type_id: JobTypeId,
    companies_len: usize,
    employer_pickers: &[EmployerPicker],
    rng: &mut DeterministicRng,
) -> u16 {
    if companies_len == 0 {
        return 0;
    }

    if let Some(picker) = employer_pickers.get(job_type_id as usize)
        && picker.total_weight > 0
    {
        let roll = rng.next_u64() % picker.total_weight;
        let index = picker
            .cumulative_weights
            .partition_point(|&weight| weight <= roll);
        if let Some(&company_id) = picker.company_ids.get(index) {
            return company_id;
        }
        if let Some(&company_id) = picker.company_ids.last() {
            return company_id;
        }
    }

    (rng.next_u64() as usize % companies_len) as u16
}

fn build_company_hiring_weights(
    companies: &[super::company::Company],
    rng: &mut DeterministicRng,
) -> Vec<u16> {
    companies
        .iter()
        .map(|company| {
            let Some(good) = good_by_id(company.produced_good_id) else {
                return rng.next_u16_inclusive(1, 100);
            };
            let labor_load = good.monthly_demand_factor * good.effort_per_unit;
            let jitter = 0.85 + 0.30 * rng.next_f64();
            let weight = (labor_load * 10.0 * jitter).round() as u16;
            weight.max(1)
        })
        .collect()
}

fn randomize_company_prices(
    companies: &mut [super::company::Company],
    sim_params: &SimulationParams,
    rng: &mut DeterministicRng,
) {
    for company in companies {
        let Some(good) = good_by_id(company.produced_good_id) else {
            continue;
        };

        let min_price = good.base_unit_cost * (1.0 + sim_params.goods_market.min_margin_rate);
        let price_factor = 0.85 + 0.30 * rng.next_f64();
        let candidate = company.unit_price * price_factor;
        company.unit_price = round_cents(candidate.max(min_price));
    }
}

fn fund_initial_bank_reserve(country: &mut Country) {
    let target = round_cents(
        country
            .sim_params()
            .stock_market
            .maker_bootstrap_reserve
            .max(0.0),
    );
    if target <= 0.0 || country.companies().is_empty() {
        return;
    }

    let total_company_cash: f64 = country.companies().iter().map(|c| c.cash_balance).sum();
    let transfer_total = round_cents(target.min(total_company_cash.max(0.0)));
    if transfer_total <= 0.0 {
        return;
    }

    let mut remaining = transfer_total;
    let company_count = country.companies().len() as f64;
    let companies_len = country.companies().len();
    for (idx, company) in country.companies_mut().iter_mut().enumerate() {
        if remaining <= 0.0 {
            break;
        }
        let planned = if idx + 1 == companies_len {
            remaining
        } else {
            round_cents(transfer_total / company_count)
        };
        let pulled = round_cents(planned.min(company.cash_balance.max(0.0)));
        company.cash_balance = round_cents(company.cash_balance - pulled);
        remaining = round_cents((remaining - pulled).max(0.0));
    }
    let actual_transfer = round_cents(transfer_total - remaining);
    let primary_bank = country.primary_bank_mut();
    primary_bank.reserve_balance = round_cents(primary_bank.reserve_balance + actual_transfer);
    country.recompute_aggregates();
}

fn seed_initial_bank_openings(country: &mut Country) {
    let params = country.sim_params().banking;
    let job_types = params.bank_employee_job_types;
    let multiplier = params.bank_wage_multiplier.max(1.0);

    let primary_bank = country.primary_bank_mut();
    let target = primary_bank.target_employee_count(
        primary_bank.deposit_liabilities,
        params.bank_min_employees,
        params.bank_max_employees,
        params.bank_employees_per_deposit_unit,
    );
    for i in 0..target {
        use super::job::salary_for_job;
        use super::money::round_cents;
        let job_id = job_types[i as usize % job_types.len()];
        let salary = round_cents(salary_for_job(job_id) * multiplier);
        primary_bank.add_opening_with_salary(job_id, 1, salary);
    }
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
            let non_household_cash =
                round_cents(country.company_cash_total + country.total_bank_reserves());
            assert!((country.household_cash_total - non_household_cash).abs() <= 0.01);
            let expected_total = round_cents(country.initial_money_supply);
            assert!((country.total_money_supply - expected_total).abs() <= 0.01);
        }
    }

    #[test]
    fn generation_creates_unemployed_population_slice() {
        let countries = generate_countries(&GenerationConfig::with_defaults(1, 500, 11));
        let unemployed = countries[0]
            .people
            .iter()
            .filter(|person| !person.is_employed())
            .count();

        assert!(unemployed > 0);
    }

    #[test]
    fn setup_randomizes_initial_company_prices() {
        let countries = generate_countries(&GenerationConfig::with_defaults(2, 500, 9));
        let prices = countries
            .iter()
            .flat_map(|country| country.companies.iter().map(|company| company.unit_price))
            .collect::<Vec<_>>();
        let unique_prices = prices
            .iter()
            .map(|price| (price * 100.0).round() as i64)
            .collect::<std::collections::BTreeSet<_>>();

        assert!(unique_prices.len() > 10);
    }

    #[test]
    fn target_initial_company_counts_scale_essentials_for_large_populations() {
        let counts = target_initial_company_counts(100_000);

        assert_eq!(counts[0], 5);
        assert_eq!(counts[1], 5);
        assert_eq!(counts[2], 3);
        assert_eq!(counts[3], 2);
        assert_eq!(counts[4], 1);
        assert_eq!(counts.iter().copied().map(u32::from).sum::<u32>(), 53);
    }

    #[test]
    fn build_initial_companies_matches_default_good_targets() {
        let company_count = derive_initial_company_count(100_000);
        let companies = build_initial_companies("A", 100_000, company_count);
        let mut counts = [0_u16; GOODS_COUNT];
        for company in companies {
            counts[company.produced_good_id as usize] =
                counts[company.produced_good_id as usize].saturating_add(1);
        }

        assert_eq!(counts, target_initial_company_counts(100_000));
    }

    #[test]
    fn setup_creates_uneven_initial_company_headcounts() {
        let countries = generate_countries(&GenerationConfig::with_defaults(1, 10_000, 13));
        let headcounts = countries[0]
            .companies
            .iter()
            .map(|company| company.employee_count)
            .collect::<Vec<_>>();
        let unique_headcounts = headcounts
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();

        assert!(unique_headcounts.len() > 1);
    }
}
