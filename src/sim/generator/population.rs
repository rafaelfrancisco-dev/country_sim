use super::*;

pub(super) fn pick_initial_age_days(rng: &mut DeterministicRng) -> u16 {
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

pub(super) fn pick_weighted_job_id_for_habilitation(
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

pub(super) fn pick_employer_for_job(
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

pub(super) fn build_company_hiring_weights(
    companies: &[Company],
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

pub(super) fn seed_balances_to_people(people: &mut [Person], total_supply: f64) {
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

const HOUSING_GOOD_ID: u8 = 2;

pub(super) fn seed_initial_housing(country: &mut Country) {
    let params = country.sim_params().housing;
    let population = country.people.len();
    if population == 0 {
        return;
    }

    let initial_price = round_cents(params.initial_price_index.max(0.0));
    let monthly_rent = round_cents(initial_price * params.rent_as_fraction_of_price);
    country.housing_price_index = initial_price;
    country.monthly_rent = monthly_rent;

    let total_stock = ((population as f64) / (1.0 - params.target_vacancy_rate)).ceil() as u32;
    country.housing_stock_total = total_stock;

    let housing_companies: Vec<usize> = country
        .companies
        .iter()
        .enumerate()
        .filter(|(_, c)| c.produced_good_id == HOUSING_GOOD_ID)
        .map(|(idx, _)| idx)
        .collect();

    let owner_threshold = (params.initial_owner_rate * 10_000.0) as u64;
    let renter_threshold =
        ((params.initial_owner_rate + params.initial_renter_rate) * 10_000.0) as u64;

    let mut occupied_owned: u32 = 0;
    let mut occupied_rented: u32 = 0;

    for person in &mut country.people {
        let age = person.age_years();
        if age < 18 {
            continue;
        }

        let roll = mix_seed(u64::from(person.id).wrapping_add(0xc0ffee)) % 10_000;
        if roll < owner_threshold {
            person.housing = HousingStatus::Owner;
            occupied_owned = occupied_owned.saturating_add(1);
        } else if roll < renter_threshold && !housing_companies.is_empty() {
            let company_idx = housing_companies[mix_seed(u64::from(person.id).wrapping_add(0xbeef))
                as usize
                % housing_companies.len()];
            person.housing = HousingStatus::Renter {
                landlord_company_id: company_idx as u16,
                landlord_country_id: country.id,
            };
            occupied_rented = occupied_rented.saturating_add(1);
        }
    }

    country.housing_stock_occupied_owned = occupied_owned;
    country.housing_stock_occupied_rented = occupied_rented;
    let occupied_total = occupied_owned.saturating_add(occupied_rented);
    country.housing_stock_vacant = total_stock.saturating_sub(occupied_total);

    if !housing_companies.is_empty() && occupied_rented > 0 {
        let units_per_company = occupied_rented / housing_companies.len() as u32;
        let remainder = occupied_rented % housing_companies.len() as u32;
        for (i, &company_idx) in housing_companies.iter().enumerate() {
            let extra = if (i as u32) < remainder { 1 } else { 0 };
            country.companies[company_idx].rental_units_owned = units_per_company + extra;
        }
    }
}

pub(super) fn seed_balances_to_companies(companies: &mut [Company], total_supply: f64) {
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
