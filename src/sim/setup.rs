use std::mem::size_of;

use super::person::Person;

pub const MIN_COUNTRIES: u16 = 1;
pub const MAX_COUNTRIES: u16 = 50;
pub const MIN_MILLIONS_PER_COUNTRY: f64 = 0.1;
pub const MAX_MILLIONS_PER_COUNTRY: f64 = 20.0;
pub const CITIZENS_PER_MILLION: u32 = 1_000_000;
pub const MEMORY_WARNING_THRESHOLD_BYTES: u64 = (2.5 * 1024.0 * 1024.0 * 1024.0) as u64;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SetupField {
    CountryCount,
    MillionsPerCountry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupForm {
    pub country_count_input: String,
    pub millions_per_country_input: String,
    pub active_field: SetupField,
    pub validation_error: Option<String>,
    pub memory_warning: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SetupValues {
    pub country_count: u16,
    pub millions_per_country: f64,
    pub estimated_person_bytes: u64,
}

impl Default for SetupForm {
    fn default() -> Self {
        Self {
            country_count_input: String::new(),
            millions_per_country_input: String::new(),
            active_field: SetupField::CountryCount,
            validation_error: None,
            memory_warning: None,
        }
    }
}

impl SetupForm {
    pub fn edit_char(&mut self, input: char) {
        self.validation_error = None;
        match self.active_field {
            SetupField::CountryCount if input.is_ascii_digit() => {
                self.country_count_input.push(input);
            }
            SetupField::MillionsPerCountry if input.is_ascii_digit() => {
                self.millions_per_country_input.push(input);
            }
            SetupField::MillionsPerCountry
                if input == '.' && !self.millions_per_country_input.contains('.') =>
            {
                self.millions_per_country_input.push(input);
            }
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
            SetupField::MillionsPerCountry => {
                self.millions_per_country_input.pop();
            }
        }
        self.refresh_memory_warning();
    }

    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            SetupField::CountryCount => SetupField::MillionsPerCountry,
            SetupField::MillionsPerCountry => SetupField::CountryCount,
        };
    }

    pub fn prev_field(&mut self) {
        self.next_field();
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

        let millions_per_country = self
            .millions_per_country_input
            .parse::<f64>()
            .map_err(|_| "Millions per country must be a number.".to_string())?;

        if !(MIN_MILLIONS_PER_COUNTRY..=MAX_MILLIONS_PER_COUNTRY).contains(&millions_per_country) {
            return Err(format!(
                "Millions per country must be between {:.1} and {:.1}.",
                MIN_MILLIONS_PER_COUNTRY, MAX_MILLIONS_PER_COUNTRY
            ));
        }

        let estimated_person_bytes =
            estimate_person_storage_bytes(country_count, millions_per_country);
        self.memory_warning = build_memory_warning(estimated_person_bytes);

        Ok(SetupValues {
            country_count,
            millions_per_country,
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
        let parsed_millions = self.millions_per_country_input.parse::<f64>();

        self.memory_warning = match (parsed_country, parsed_millions) {
            (Ok(country_count), Ok(millions_per_country))
                if (MIN_COUNTRIES..=MAX_COUNTRIES).contains(&country_count)
                    && (MIN_MILLIONS_PER_COUNTRY..=MAX_MILLIONS_PER_COUNTRY)
                        .contains(&millions_per_country) =>
            {
                let bytes = estimate_person_storage_bytes(country_count, millions_per_country);
                build_memory_warning(bytes)
            }
            _ => None,
        };
    }
}

pub fn estimate_population(country_count: u16, millions_per_country: f64) -> u64 {
    let estimate =
        f64::from(country_count) * millions_per_country * f64::from(CITIZENS_PER_MILLION);

    if !estimate.is_finite() || estimate <= 0.0 {
        return 0;
    }

    estimate.round().min(u64::MAX as f64) as u64
}

pub fn estimate_person_storage_bytes(country_count: u16, millions_per_country: f64) -> u64 {
    estimate_population(country_count, millions_per_country)
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
            millions_per_country_input: "2.5".to_string(),
            ..SetupForm::default()
        };

        let values = form.validate().expect("valid values must pass");

        assert_eq!(values.country_count, 4);
        assert_eq!(values.millions_per_country, 2.5);
    }

    #[test]
    fn validate_accepts_minimum_decimal_millions() {
        let mut form = SetupForm {
            country_count_input: "4".to_string(),
            millions_per_country_input: "0.1".to_string(),
            ..SetupForm::default()
        };

        let values = form.validate().expect("minimum value must pass");

        assert_eq!(values.millions_per_country, 0.1);
    }

    #[test]
    fn validate_rejects_non_numeric_country_count() {
        let mut form = SetupForm {
            country_count_input: "abc".to_string(),
            millions_per_country_input: "2".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();

        assert_eq!(error, "Country count must be a whole number.");
    }

    #[test]
    fn validate_rejects_out_of_range_country_count() {
        let mut form = SetupForm {
            country_count_input: "0".to_string(),
            millions_per_country_input: "2".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();

        assert_eq!(error, "Country count must be between 1 and 50.");
    }

    #[test]
    fn validate_rejects_out_of_range_millions() {
        let mut form = SetupForm {
            country_count_input: "2".to_string(),
            millions_per_country_input: "21".to_string(),
            ..SetupForm::default()
        };

        let error = form.validate().unwrap_err();

        assert_eq!(error, "Millions per country must be between 0.1 and 20.0.");
    }

    #[test]
    fn memory_warning_is_non_blocking() {
        let mut form = SetupForm {
            country_count_input: "50".to_string(),
            millions_per_country_input: "20".to_string(),
            ..SetupForm::default()
        };

        let values = form.validate().expect("warning should not block submit");

        assert!(values.estimated_person_bytes > MEMORY_WARNING_THRESHOLD_BYTES);
        assert!(form.memory_warning.is_some());
    }

    #[test]
    fn estimate_population_uses_million_scale() {
        assert_eq!(estimate_population(2, 3.5), 7_000_000);
    }
}
