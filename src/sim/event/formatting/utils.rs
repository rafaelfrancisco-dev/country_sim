use crate::sim::event::EntityKind;

pub(super) fn entity_kind_label(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Person => "Person",
        EntityKind::Company => "Company",
        EntityKind::Country => "Country",
    }
}

pub(super) fn format_money(amount: f64, currency_symbol: &str) -> String {
    let symbol = normalized_currency_symbol(currency_symbol);
    let sign = if amount < 0.0 { "-" } else { "" };
    format!("{sign}{symbol}{}", compact_money_amount(amount.abs()))
}

pub(super) fn format_money_fixed(amount: f64, currency_symbol: &str, precision: usize) -> String {
    let symbol = normalized_currency_symbol(currency_symbol);
    let sign = if amount < 0.0 { "-" } else { "" };
    format!(
        "{sign}{symbol}{value:.precision$}",
        value = amount.abs(),
        precision = precision
    )
}

fn normalized_currency_symbol(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        crate::sim::setup::DEFAULT_CURRENCY_SYMBOL
    } else {
        trimmed
    }
}

fn compact_money_amount(amount: f64) -> String {
    let abs = amount.abs();
    let (divisor, suffix) = if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if abs >= 10_000.0 {
        (1_000.0, "k")
    } else {
        return format!("{amount:.2}");
    };
    let scaled = amount / divisor;
    let rounded = (scaled * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}{suffix}")
    } else {
        format!("{rounded:.1}{suffix}")
    }
}
