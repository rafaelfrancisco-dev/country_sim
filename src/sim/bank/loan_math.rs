use crate::sim::money::round_cents;

pub fn payment_for(principal: f64, annual_rate: f64, term_months: u16) -> f64 {
    if principal <= 0.0 || term_months == 0 {
        return 0.0;
    }

    let monthly_rate = monthly_interest_rate(annual_rate);
    if monthly_rate <= f64::EPSILON {
        return round_cents(principal / f64::from(term_months));
    }

    let n = f64::from(term_months);
    let factor = (1.0 - (1.0 + monthly_rate).powf(-n)) / monthly_rate;
    round_cents(principal / factor)
}

pub fn max_principal_for_payment(payment: f64, annual_rate: f64, term_months: u16) -> f64 {
    if payment <= 0.0 || term_months == 0 {
        return 0.0;
    }

    let monthly_rate = monthly_interest_rate(annual_rate);
    if monthly_rate <= f64::EPSILON {
        return round_cents(payment * f64::from(term_months));
    }

    let n = f64::from(term_months);
    let factor = (1.0 - (1.0 + monthly_rate).powf(-n)) / monthly_rate;
    round_cents(payment * factor)
}

pub(super) fn monthly_interest_rate(annual_rate: f64) -> f64 {
    annual_rate.max(0.0) / 12.0
}
