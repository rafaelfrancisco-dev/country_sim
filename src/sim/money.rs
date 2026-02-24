pub fn round_cents(amount: f64) -> f64 {
    if !amount.is_finite() {
        return 0.0;
    }

    (amount * 100.0).round() / 100.0
}

pub fn to_cents(amount: f64) -> i64 {
    (round_cents(amount) * 100.0).round() as i64
}

pub fn from_cents(cents: i64) -> f64 {
    cents as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_cents_supports_half_up_behavior() {
        assert!((round_cents(10.005) - 10.01).abs() < f64::EPSILON);
        assert!((round_cents(10.004) - 10.00).abs() < f64::EPSILON);
    }

    #[test]
    fn cent_conversion_round_trips() {
        let original = 1234.56;
        let cents = to_cents(original);
        let round_trip = from_cents(cents);

        assert!((round_trip - original).abs() < f64::EPSILON);
    }
}
