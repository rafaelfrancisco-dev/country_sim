use super::{TAX_BRACKET_1_MAX, TAX_BRACKET_2_MAX, round_cents};

pub fn monthly_income_tax(gross_pay: f64) -> f64 {
    let mut remaining = gross_pay.max(0.0);
    let mut tax = 0.0;

    let first_bracket = remaining.min(TAX_BRACKET_1_MAX);
    tax += first_bracket * 0.10;
    remaining -= first_bracket;

    let second_span = TAX_BRACKET_2_MAX - TAX_BRACKET_1_MAX;
    let second_bracket = remaining.min(second_span);
    tax += second_bracket * 0.20;
    remaining -= second_bracket;

    if remaining > 0.0 {
        tax += remaining * 0.30;
    }

    round_cents(tax)
}
