use std::mem::size_of;

use super::person::Person;
use crate::seed::{format_generation_seed, parse_generation_seed};

pub const MIN_COUNTRIES: u16 = 1;
pub const MAX_COUNTRIES: u16 = 50;
pub const MIN_THOUSANDS_PER_COUNTRY: u32 = 10;
pub const MAX_THOUSANDS_PER_COUNTRY: u32 = 20_000;
pub const CITIZENS_PER_THOUSAND: u32 = 1_000;
pub const MEMORY_WARNING_THRESHOLD_BYTES: u64 = (2.5 * 1024.0 * 1024.0 * 1024.0) as u64;
pub const DEFAULT_CURRENCY_SYMBOL: &str = "€";
const DEFAULT_SETUP_GENERATION_SEED: u64 = 0x5eed_c0de;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SetupField {
    CountryCount,
    ThousandsPerCountry,
    GenerationSeed,
    SaveName,
    ExistingSaves,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupForm {
    pub country_count_input: String,
    pub thousands_per_country_input: String,
    pub generation_seed_input: String,
    pub active_field: SetupField,
    pub validation_error: Option<String>,
    pub memory_warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupValues {
    pub country_count: u16,
    pub thousands_per_country: u32,
    pub generation_seed: u64,
    pub estimated_person_bytes: u64,
}

impl Default for SetupForm {
    fn default() -> Self {
        Self {
            country_count_input: String::new(),
            thousands_per_country_input: String::new(),
            generation_seed_input: format_generation_seed(DEFAULT_SETUP_GENERATION_SEED),
            active_field: SetupField::CountryCount,
            validation_error: None,
            memory_warning: None,
        }
    }
}

impl SetupForm {
    pub fn with_generation_seed(seed: u64) -> Self {
        Self {
            generation_seed_input: format_generation_seed(seed),
            ..Self::default()
        }
    }

    pub fn edit_char(&mut self, input: char) {
        self.validation_error = None;
        match self.active_field {
            SetupField::CountryCount if input.is_ascii_digit() => {
                self.country_count_input.push(input);
            }
            SetupField::ThousandsPerCountry if input.is_ascii_digit() => {
                self.thousands_per_country_input.push(input);
            }
            SetupField::GenerationSeed
                if input.is_ascii_hexdigit() || matches!(input, 'x' | 'X' | '_') =>
            {
                self.generation_seed_input.push(input);
            }
            // SaveName and ExistingSaves: editing handled by PersistenceCommand, not here.
            SetupField::SaveName | SetupField::ExistingSaves => return,
            _ => return,
        }
        self.refresh_memory_warning();
    }

    pub fn backspace(&mut self) {
        self.validation_error = None;
        match self.active_field {
            SetupField::CountryCount => {
                self.country_count_input.pop();
            }
            SetupField::ThousandsPerCountry => {
                self.thousands_per_country_input.pop();
            }
            SetupField::GenerationSeed => {
                self.generation_seed_input.pop();
            }
            // SaveName and ExistingSaves: handled by PersistenceCommand.
            SetupField::SaveName | SetupField::ExistingSaves => {}
        }
        self.refresh_memory_warning();
    }

    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            SetupField::CountryCount => SetupField::ThousandsPerCountry,
            SetupField::ThousandsPerCountry => SetupField::GenerationSeed,
            SetupField::GenerationSeed => SetupField::SaveName,
            SetupField::SaveName => SetupField::ExistingSaves,
            SetupField::ExistingSaves => SetupField::CountryCount,
        };
    }

    pub fn prev_field(&mut self) {
        self.active_field = match self.active_field {
            SetupField::CountryCount => SetupField::ExistingSaves,
            SetupField::ThousandsPerCountry => SetupField::CountryCount,
            SetupField::GenerationSeed => SetupField::ThousandsPerCountry,
            SetupField::SaveName => SetupField::GenerationSeed,
            SetupField::ExistingSaves => SetupField::SaveName,
        };
    }

    pub fn validate(&mut self) -> Result<SetupValues, String> {
        let country_count = self
            .country_count_input
            .parse::<u16>()
            .map_err(|_| "Country count must be a whole number.".to_string())?;

        if !(MIN_COUNTRIES..=MAX_COUNTRIES).contains(&country_count) {
            return Err(format!(
                "Country count must be between {} and {}.",
                MIN_COUNTRIES, MAX_COUNTRIES
            ));
        }

        let thousands_per_country = self
            .thousands_per_country_input
            .parse::<u32>()
            .map_err(|_| "Thousands per country must be a whole number.".to_string())?;

        if !(MIN_THOUSANDS_PER_COUNTRY..=MAX_THOUSANDS_PER_COUNTRY).contains(&thousands_per_country)
        {
            return Err(format!(
                "Thousands per country must be between {} and {}.",
                MIN_THOUSANDS_PER_COUNTRY, MAX_THOUSANDS_PER_COUNTRY
            ));
        }

        let generation_seed = parse_generation_seed(&self.generation_seed_input)
            .ok_or_else(|| "Generation seed must be decimal or 0x-prefixed hex.".to_string())?;

        let estimated_person_bytes =
            estimate_person_storage_bytes(country_count, thousands_per_country);
        self.memory_warning = build_memory_warning(estimated_person_bytes);

        Ok(SetupValues {
            country_count,
            thousands_per_country,
            generation_seed,
            estimated_person_bytes,
        })
    }

    pub fn apply_validation_result(
        &mut self,
        result: Result<SetupValues, String>,
    ) -> Option<SetupValues> {
        match result {
            Ok(values) => {
                self.validation_error = None;
                Some(values)
            }
            Err(error) => {
                self.validation_error = Some(error);
                None
            }
        }
    }

    fn refresh_memory_warning(&mut self) {
        let parsed_country = self.country_count_input.parse::<u16>();
        let parsed_thousands = self.thousands_per_country_input.parse::<u32>();

        self.memory_warning = match (parsed_country, parsed_thousands) {
            (Ok(country_count), Ok(thousands_per_country))
                if (MIN_COUNTRIES..=MAX_COUNTRIES).contains(&country_count)
                    && (MIN_THOUSANDS_PER_COUNTRY..=MAX_THOUSANDS_PER_COUNTRY)
                        .contains(&thousands_per_country) =>
            {
                let bytes = estimate_person_storage_bytes(country_count, thousands_per_country);
                build_memory_warning(bytes)
            }
            _ => None,
        };
    }
}

pub fn estimate_population(country_count: u16, thousands_per_country: u32) -> u64 {
    u64::from(country_count)
        .saturating_mul(u64::from(thousands_per_country))
        .saturating_mul(u64::from(CITIZENS_PER_THOUSAND))
}

pub fn estimate_person_storage_bytes(country_count: u16, thousands_per_country: u32) -> u64 {
    estimate_population(country_count, thousands_per_country)
        .saturating_mul(size_of::<Person>() as u64)
}

pub fn build_memory_warning(estimated_bytes: u64) -> Option<String> {
    if estimated_bytes <= MEMORY_WARNING_THRESHOLD_BYTES {
        return None;
    }

    let gib = estimated_bytes as f64 / 1024.0_f64.powi(3);
    Some(format!(
        "Estimated person storage is {:.2} GiB. This may exceed available memory.",
        gib
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_valid_values() {
        let mut form = SetupForm {
            country_count_input: "4".to_string(),
            thousands_per_country_input: "2500".to_string(),
            ..SetupForm::default()
        };

        let values = form.validate().expect("valid values must pass");

        assert_eq!(values.country_count, 4);
        assert_eq!(values.thousands_per_country, 2500);
        assert_eq!(values.generation_seed, DEFAULT_SETUP_GENERATION_SEED);
    }

    #[test]
    fn validate_accepts_minimum_thousands() {
        let mut form = SetupForm {
            country_count_input: "4".to_string(),
            thousands_per_country_input: "10".to_string(),
            ..SetupForm::default()
        };

        let values = form.validate().expect("minimum value must pass");

        assert_eq!(values.thousands_per_country, 10);
    }

    #[test]
    fn validate_rejects_non_numeric_country_count() {
        let mut form = SetupForm {
            country_count_input: "abc".to_string(),
            thousands_per_country_input: "2".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();

        assert_eq!(error, "Country count must be a whole number.");
    }

    #[test]
    fn validate_rejects_out_of_range_country_count() {
        let mut form = SetupForm {
            country_count_input: "0".to_string(),
            thousands_per_country_input: "2".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();

        assert_eq!(error, "Country count must be between 1 and 50.");
    }

    #[test]
    fn validate_rejects_out_of_range_thousands() {
        let mut form = SetupForm {
            country_count_input: "2".to_string(),
            thousands_per_country_input: "9".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();

        assert_eq!(error, "Thousands per country must be between 10 and 20000.");
    }

    #[test]
    fn memory_warning_is_non_blocking() {
        let mut form = SetupForm {
            country_count_input: "50".to_string(),
            thousands_per_country_input: "20000".to_string(),
            ..SetupForm::default()
        };

        let values = form.validate().expect("warning should not block submit");

        assert!(values.estimated_person_bytes > MEMORY_WARNING_THRESHOLD_BYTES);
        assert!(form.memory_warning.is_some());
    }

    #[test]
    fn validate_rejects_invalid_generation_seed() {
        let mut form = SetupForm {
            country_count_input: "2".to_string(),
            thousands_per_country_input: "2000".to_string(),
            generation_seed_input: "invalid".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();
        assert_eq!(error, "Generation seed must be decimal or 0x-prefixed hex.");
    }

    #[test]
    fn with_generation_seed_prefills_setup_seed_input() {
        let form = SetupForm::with_generation_seed(0xabc);
        assert_eq!(form.generation_seed_input, "0x0000000000000abc");
    }

    #[test]
    fn estimate_population_uses_thousand_scale() {
        assert_eq!(estimate_population(2, 3), 6_000);
    }
}
