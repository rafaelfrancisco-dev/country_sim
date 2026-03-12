use super::*;
use crate::sim::good::{
    CLOTHING_GOOD_ID, ELECTRICITY_GOOD_ID, ENTERTAINMENT_GOOD_ID, FOOD_GOOD_ID,
    HYGIENE_PRODUCTS_GOOD_ID, INTERNET_GOOD_ID, MEDICINE_GOOD_ID, TRANSPORT_GOOD_ID, WATER_GOOD_ID,
    common_goods, industry_job_types_for_good,
};

pub(super) fn pick_weighted_job(
    seed: u64,
    weighted_jobs: &[(JobTypeId, u16)],
) -> Option<JobTypeId> {
    let total_weight: u64 = weighted_jobs
        .iter()
        .map(|(_, weight)| u64::from(*weight))
        .sum();
    if total_weight == 0 {
        return weighted_jobs.first().map(|(job_id, _)| *job_id);
    }

    let mut roll = hash64(seed) % total_weight;
    for (job_id, weight) in weighted_jobs {
        let weight = u64::from(*weight);
        if roll < weight {
            return Some(*job_id);
        }
        roll -= weight;
    }

    weighted_jobs.last().map(|(job_id, _)| *job_id)
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn company_good_for_index(company_index: usize) -> GoodId {
    let goods = common_goods();
    if company_index < goods.len() {
        return goods[company_index].id;
    }

    const EXTRA_GOOD_ROTATION: [GoodId; 9] = [
        FOOD_GOOD_ID,
        WATER_GOOD_ID,
        ELECTRICITY_GOOD_ID,
        TRANSPORT_GOOD_ID,
        INTERNET_GOOD_ID,
        ENTERTAINMENT_GOOD_ID,
        CLOTHING_GOOD_ID,
        MEDICINE_GOOD_ID,
        HYGIENE_PRODUCTS_GOOD_ID,
    ];
    let offset = company_index - goods.len();
    EXTRA_GOOD_ROTATION[offset % EXTRA_GOOD_ROTATION.len()]
}

pub fn build_default_companies(country_name: &str, count: u16) -> Vec<Company> {
    (0..count)
        .map(|idx| {
            let good_id = company_good_for_index(idx as usize);
            Company::new(
                idx,
                format!("{country_name} Co {}", idx + 1),
                industry_job_types_for_good(good_id).to_vec(),
                good_id,
            )
        })
        .collect()
}
