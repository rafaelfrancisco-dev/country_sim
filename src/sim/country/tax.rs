use super::round_cents;
use crate::sim::params::TaxPolicyParams;

pub fn monthly_income_tax_with_params(gross_pay: f64, tax_policy: TaxPolicyParams) -> f64 {
    let bracket_1_max = tax_policy.bracket_1_max.max(0.0);
    let bracket_2_max = tax_policy.bracket_2_max.max(bracket_1_max);
    let rate_1 = tax_policy.rate_1.clamp(0.0, 1.0);
    let rate_2 = tax_policy.rate_2.clamp(0.0, 1.0);
    let rate_3 = tax_policy.rate_3.clamp(0.0, 1.0);

    let mut remaining = gross_pay.max(0.0);
    let mut tax = 0.0;

    let first_bracket = remaining.min(bracket_1_max);
    tax += first_bracket * rate_1;
    remaining -= first_bracket;

    let second_span = bracket_2_max - bracket_1_max;
    let second_bracket = remaining.min(second_span);
    tax += second_bracket * rate_2;
    remaining -= second_bracket;

    if remaining > 0.0 {
        tax += remaining * rate_3;
    }

    round_cents(tax)
}

pub fn monthly_income_tax(gross_pay: f64) -> f64 {
    monthly_income_tax_with_params(gross_pay, TaxPolicyParams::default())
}
