use super::*;

#[test]
fn fiscal_benefit_is_rate_times_reference_income() {
    let params = FiscalParams {
        unemployment_benefit_rate: 0.5,
        median_income_reference: 2_000.0,
        basic_needs_cash_threshold: 500.0,
        ..Default::default()
    };
    assert!((params.monthly_unemployment_benefit() - 1_000.0).abs() < f64::EPSILON);
}

#[test]
fn fiscal_benefit_is_zero_when_rate_is_zero() {
    let params = FiscalParams {
        unemployment_benefit_rate: 0.0,
        median_income_reference: 2_000.0,
        basic_needs_cash_threshold: 500.0,
        ..Default::default()
    };
    assert!((params.monthly_unemployment_benefit()).abs() < f64::EPSILON);
}

#[test]
fn fiscal_benefit_scales_with_reference_income() {
    let params = FiscalParams {
        unemployment_benefit_rate: 0.4,
        median_income_reference: 3_000.0,
        basic_needs_cash_threshold: 500.0,
        ..Default::default()
    };
    assert!((params.monthly_unemployment_benefit() - 1_200.0).abs() < f64::EPSILON);
}
