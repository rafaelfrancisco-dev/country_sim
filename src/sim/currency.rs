use std::hash::Hash;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencySpec {
    pub code: Arc<str>,
    pub name: Arc<str>,
    pub symbol: Arc<str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Currency {
    pub spec: CurrencySpec,
    pub issuer_country_idx: usize,
}

impl Eq for Currency {}

impl CurrencySpec {
    pub fn new(code: &'static str, name: &'static str, symbol: &'static str) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            symbol: symbol.into(),
        }
    }
}

const COMMON_CURRENCIES: [(&str, &str, &str); 15] = [
    ("USD", "US Dollar", "$"),
    ("EUR", "Euro", "€"),
    ("JPY", "Japanese Yen", "¥"),
    ("GBP", "Pound Sterling", "£"),
    ("CAD", "Canadian Dollar", "$"),
    ("AUD", "Australian Dollar", "$"),
    ("CNY", "Chinese Yuan", "¥"),
    ("INR", "Indian Rupee", "₹"),
    ("KRW", "South Korean Won", "₩"),
    ("RUB", "Russian Ruble", "₽"),
    ("TRY", "Turkish Lira", "₺"),
    ("UAH", "Ukrainian Hryvnia", "₴"),
    ("THB", "Thai Baht", "฿"),
    ("VND", "Vietnamese Dong", "₫"),
    ("PHP", "Philippine Peso", "₱"),
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
