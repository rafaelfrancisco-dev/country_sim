pub(in crate::components::simulation) fn compact_num(n: f64) -> String {
    let abs = n.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{n:.2}");
    };
    let rounded = ((n / divisor) * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}{suffix}")
    } else {
        format!("{rounded:.1}{suffix}")
    }
}

pub(in crate::components::simulation) fn compact_count(n: impl Into<f64>) -> String {
    let n: f64 = n.into();
    let abs = n.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{n:.0}");
    };
    let rounded = ((n / divisor) * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}{suffix}")
    } else {
        format!("{rounded:.1}{suffix}")
    }
}

pub(in crate::components::simulation) fn normalized_currency_symbol(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        crate::sim::setup::DEFAULT_CURRENCY_SYMBOL
    } else {
        trimmed
    }
}

pub(in crate::components::simulation) fn money(amount: f64, currency_symbol: &str) -> String {
    let symbol = normalized_currency_symbol(currency_symbol);
    let sign = if amount < 0.0 { "-" } else { "" };
    format!("{sign}{symbol}{}", compact_num(amount.abs()))
}
