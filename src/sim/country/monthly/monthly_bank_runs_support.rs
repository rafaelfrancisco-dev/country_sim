use super::round_cents;

pub(super) fn resolve_run_claim(
    requested: f64,
    available_ratio: f64,
    insurance_cap: f64,
    uninsured_haircut_rate: f64,
) -> (f64, f64, f64) {
    let requested = round_cents(requested.max(0.0));
    let paid_from_bank = round_cents(requested * available_ratio.clamp(0.0, 1.0));
    let shortfall = round_cents((requested - paid_from_bank).max(0.0));
    let insured = round_cents(shortfall.min(insurance_cap.max(0.0)));
    let uninsured = round_cents((shortfall - insured).max(0.0));
    let haircut_loss = round_cents(uninsured * uninsured_haircut_rate.clamp(0.0, 1.0));
    let resolution_support = round_cents((uninsured - haircut_loss).max(0.0));
    let treasury_support = round_cents(insured + resolution_support);
    (paid_from_bank, treasury_support, haircut_loss)
}

pub(super) fn run_switch_probability(
    base_fraction: f64,
    stress: f64,
    source_health: f64,
    target_health: f64,
    source_rate: f64,
    target_rate: f64,
    friction: f64,
) -> f64 {
    let health_edge = (target_health - source_health).clamp(-1.0, 1.0);
    let rate_edge = ((target_rate - source_rate) * 12.0).clamp(-1.0, 1.0);
    let impulse = (0.55 + 0.30 * stress + 0.25 * health_edge + 0.10 * rate_edge).clamp(0.0, 1.0);
    (base_fraction * impulse * friction.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

pub(super) fn stable_switch_roll(
    month: u64,
    day: u64,
    entity_id: u64,
    from_bank_id: u16,
    to_bank_id: u16,
    salt: u64,
) -> f64 {
    let mut x = month
        .wrapping_mul(1_000_003)
        .wrapping_add(day.wrapping_mul(10_007))
        .wrapping_add(entity_id.wrapping_mul(2_654_435_761))
        .wrapping_add(u64::from(from_bank_id).wrapping_mul(9_973))
        .wrapping_add(u64::from(to_bank_id).wrapping_mul(4_861))
        .wrapping_add(salt);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    ((x % 1_000_000) as f64) / 1_000_000.0
}
