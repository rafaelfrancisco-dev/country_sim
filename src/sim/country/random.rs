pub(super) fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(super) fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}

pub(super) fn roll_probability(seed: u64, probability: f64) -> bool {
    if probability <= 0.0 {
        return false;
    }
    if probability >= 1.0 {
        return true;
    }

    unit_interval(seed) < probability
}
