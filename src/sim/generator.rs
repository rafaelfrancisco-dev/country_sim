use super::{
    company::{Company, DEFAULT_COMPANIES_PER_COUNTRY},
    country::Country,
    currency::generated_currency,
    essentials::{
        STOCKABLE_ESSENTIAL_GOOD_IDS, estimated_country_monthly_requirement,
        estimated_country_stock_target_units, production_capacity_units_for_workers,
        target_manufacturing_headcount_for_units,
    },
    good::{GOODS_COUNT, common_goods, good_by_id},
    job::{JobTypeId, common_job_types, job_is_eligible_for_habilitation, salary_for_job},
    money::{from_cents, round_cents, to_cents},
    names::country_name_from_seed,
    params::SimulationParams,
    person::{EmploymentStatus, HabilitationLevel, HousingStatus, Person},
};
use rayon::prelude::*;

#[path = "generator/companies.rs"]
mod companies;
#[path = "generator/population.rs"]
mod population;

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

fn person_rng(c_seed: u64, person_id: u32) -> DeterministicRng {
    DeterministicRng::new(mix_seed(
        c_seed ^ (person_id as u64).wrapping_add(0x1234_5678_9abc_def0),
    ))
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

    let hiring_weights = population::build_company_hiring_weights(companies, rng);
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
    let c_seed = country_seed(config.seed, country_idx);
    let mut rng = DeterministicRng::new(c_seed);
    let country_name = country_name_from_seed(country_seed(config.seed ^ 0xfeed_f00d, country_idx));
    let company_count = if config.companies_per_country == DEFAULT_COMPANIES_PER_COUNTRY {
        companies::derive_initial_company_count(config.population_per_country, &config.sim_params)
    } else {
        config.companies_per_country
    };
    let mut companies = companies::build_initial_companies(
        &country_name,
        config.population_per_country,
        company_count,
        &config.sim_params,
    );
    companies::randomize_company_prices(&mut companies, &config.sim_params, &mut rng);
    let job_cache = build_job_selection_cache(&companies, &mut rng);

    let mut people: Vec<Person> = (0..config.population_per_country)
        .into_par_iter()
        .map(|person_id| {
            let mut prng = person_rng(c_seed, person_id);
            let age_days = population::pick_initial_age_days(&mut prng);
            let education_track = config.sim_params.education.pick_track(prng.next_f64());
            let name_seed = prng.next_u32();

            let mut person = Person::new_with_profile(
                person_id,
                EmploymentStatus::Unemployed,
                name_seed,
                age_days,
                education_track,
            );

            if person.can_seek_job()
                && prng.next_f64() >= config.sim_params.labor.baseline_unemployment_rate
                && let Some(job_type_id) = population::pick_weighted_job_id_for_habilitation(
                    &mut prng,
                    person.habilitation,
                    &job_cache,
                )
            {
                let employer_company_id = population::pick_employer_for_job(
                    job_type_id,
                    companies.len(),
                    &job_cache.employer_pickers,
                    &mut prng,
                );
                person.set_employment(employer_company_id, job_type_id, 0);
            }

            person
        })
        .collect();

    companies::rebalance_essential_sector_workforce(&mut people, &companies, &config.sim_params);

    let total_supply = round_cents(config.population_per_country as f64 * config.money_per_citizen);
    let household_supply = round_cents(total_supply * 0.5);
    let company_supply = round_cents(total_supply - household_supply);

    population::seed_balances_to_people(&mut people, household_supply);
    population::seed_balances_to_companies(&mut companies, company_supply);

    let mut country = Country::with_economy_and_params(
        country_idx,
        country_name,
        people,
        companies,
        total_supply,
        config.sim_params.clone(),
    );
    country.currency = generated_currency(country_idx, config.seed);
    companies::seed_initial_essential_supply(&mut country);
    companies::fund_initial_bank_reserve(&mut country);
    companies::seed_initial_bank_openings(&mut country);
    super::generator_banks::seed_initial_private_banks(&mut country);
    population::seed_initial_housing(&mut country);
    country.rebuild_households();
    country.seed_initial_ipos();
    country
}

#[cfg(test)]
#[path = "generator/tests.rs"]
mod tests;
