use super::*;

#[test]
fn money_formats_with_currency_and_k_suffix() {
    assert_eq!(money(11_500.0, "€"), "€11.5k");
    assert_eq!(money(-11_500.0, "€"), "-€11.5k");
}

#[test]
fn money_falls_back_to_default_symbol_when_empty() {
    assert_eq!(money(99.0, ""), "€99.00");
}
