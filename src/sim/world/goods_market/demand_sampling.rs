use super::*;

pub(in crate::sim::world) fn demand_units_for_person_good(
    day: u64,
    country_id: u16,
    person_id: u32,
    name_seed: u32,
    good_id: GoodId,
    monthly_demand_factor: f64,
) -> u32 {
    let daily_demand_factor = (monthly_demand_factor / 30.0).max(0.0);
    let integer_part = daily_demand_factor.floor().max(0.0) as u32;
    let fractional_part = (daily_demand_factor - f64::from(integer_part)).clamp(0.0, 1.0);

    if fractional_part <= 0.0 {
        return integer_part;
    }

    let seed = hash64(
        (day << 40)
            ^ (u64::from(country_id) << 24)
            ^ (u64::from(person_id) << 8)
            ^ u64::from(good_id)
            ^ u64::from(name_seed),
    );

    integer_part + u32::from(unit_interval(seed) < fractional_part)
}

pub(in crate::sim::world) fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(in crate::sim::world) fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}



