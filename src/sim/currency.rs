use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencySpec {
    pub code: Arc<str>,
    pub name: Arc<str>,
    pub symbol: Arc<str>,
}

impl CurrencySpec {
    pub fn new(code: &'static str, name: &'static str, symbol: &'static str) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            symbol: symbol.into(),
        }
    }
}

const COMMON_CURRENCIES: [(&str, &str, &str); 16] = [
    ("USD", "US Dollar", "$"),
    ("EUR", "Euro", "EUR"),
    ("JPY", "Japanese Yen", "JPY"),
    ("GBP", "Pound Sterling", "GBP"),
    ("CHF", "Swiss Franc", "CHF"),
    ("CAD", "Canadian Dollar", "CAD"),
    ("AUD", "Australian Dollar", "AUD"),
    ("CNY", "Chinese Yuan", "CNY"),
    ("INR", "Indian Rupee", "INR"),
    ("BRL", "Brazilian Real", "BRL"),
    ("MXN", "Mexican Peso", "MXN"),
    ("KRW", "South Korean Won", "KRW"),
    ("SGD", "Singapore Dollar", "SGD"),
    ("SEK", "Swedish Krona", "SEK"),
    ("NOK", "Norwegian Krone", "NOK"),
    ("ZAR", "South African Rand", "ZAR"),
];

fn mix_seed(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub fn generated_currency(country_id: u16, generation_seed: u64) -> CurrencySpec {
    let seed = mix_seed(
        generation_seed
            ^ (u64::from(country_id)
                .wrapping_add(1)
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)),
    );
    let idx = (seed as usize) % COMMON_CURRENCIES.len();
    let (code, name, symbol) = COMMON_CURRENCIES[idx];
    CurrencySpec::new(code, name, symbol)
}
