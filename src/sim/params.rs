use super::person::EducationTrack;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DemographicParams {
    pub births_per_1000_per_year: f64,
}

impl Default for DemographicParams {
    fn default() -> Self {
        Self {
            births_per_1000_per_year: 17.0,
        }
    }
}

impl DemographicParams {
    pub fn births_per_person_per_day(&self) -> f64 {
        self.births_per_1000_per_year.max(0.0) / 1000.0 / 365.0
    }

    pub fn annual_death_probability_for_age_years(&self, age_years: u8) -> f64 {
        match age_years {
            0 => 0.005,
            1..=14 => 0.0002,
            15..=24 => 0.0007,
            25..=34 => 0.0013,
            35..=44 => 0.0020,
            45..=54 => 0.0045,
            55..=64 => 0.0095,
            65..=74 => 0.0220,
            75..=84 => 0.0570,
            85..=94 => 0.1400,
            _ => 0.3000,
        }
    }

    pub fn daily_death_probability_for_age_years(&self, age_years: u8) -> f64 {
        let annual = self
            .annual_death_probability_for_age_years(age_years)
            .clamp(0.0, 1.0);
        1.0 - (1.0 - annual).powf(1.0 / 365.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EducationParams {
    pub basic_only_share: f64,
    pub high_school_share: f64,
    pub bachelor_share: f64,
    pub medical_doctor_share: f64,
}

impl Default for EducationParams {
    fn default() -> Self {
        Self {
            basic_only_share: 0.38,
            high_school_share: 0.37,
            bachelor_share: 0.22,
            medical_doctor_share: 0.03,
        }
    }
}

impl EducationParams {
    pub fn pick_track(&self, roll: f64) -> EducationTrack {
        let roll = roll.clamp(0.0, 1.0);
        let threshold_basic = self.basic_only_share;
        let threshold_high = threshold_basic + self.high_school_share;
        let threshold_bachelor = threshold_high + self.bachelor_share;

        if roll < threshold_basic {
            EducationTrack::BasicOnly
        } else if roll < threshold_high {
            EducationTrack::HighSchool
        } else if roll < threshold_bachelor {
            EducationTrack::Bachelor
        } else {
            EducationTrack::MedicalDoctor
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LaborParams {
    pub baseline_unemployment_rate: f64,
    pub seek_base: f64,
    pub seek_age_slope: f64,
    pub seek_cash_pressure_scale: f64,
    pub seek_cash_reference: f64,
    pub seek_probability_cap: f64,
}

impl Default for LaborParams {
    fn default() -> Self {
        Self {
            baseline_unemployment_rate: 0.06,
            seek_base: 0.002,
            seek_age_slope: 0.0002,
            seek_cash_pressure_scale: 0.01,
            seek_cash_reference: 3_000.0,
            seek_probability_cap: 0.03,
        }
    }
}

impl LaborParams {
    pub fn daily_seek_probability(&self, age_years: u8, cash_balance: f64) -> f64 {
        let age_component = (i32::from(age_years) - 18).max(0) as f64 * self.seek_age_slope;
        let cash_pressure = ((self.seek_cash_reference - cash_balance) / self.seek_cash_reference)
            .clamp(0.0, 1.0)
            * self.seek_cash_pressure_scale;

        (self.seek_base + age_component + cash_pressure).min(self.seek_probability_cap)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CompanyGrowthParams {
    pub min_profitable_months: u8,
    pub min_cash_reserve: f64,
    pub payroll_reserve_multiplier: f64,
    pub profit_per_opening: f64,
    pub max_monthly_openings: u16,
}

impl Default for CompanyGrowthParams {
    fn default() -> Self {
        Self {
            min_profitable_months: 3,
            min_cash_reserve: 100_000.0,
            payroll_reserve_multiplier: 1.5,
            profit_per_opening: 50_000.0,
            max_monthly_openings: 8,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GoodsMarketParams {
    pub tariff_rate: f64,
    pub worker_effort_per_month: f64,
    pub min_margin_rate: f64,
    pub price_raise_rate: f64,
    pub price_cut_rate: f64,
    pub sell_through_raise_threshold: f64,
    pub sell_through_cut_threshold: f64,
    pub excess_inventory_months: f64,
    pub excess_inventory_cut_rate: f64,
}

impl Default for GoodsMarketParams {
    fn default() -> Self {
        Self {
            tariff_rate: 0.15,
            worker_effort_per_month: 24.0,
            min_margin_rate: 0.0,
            price_raise_rate: 0.05,
            price_cut_rate: 0.05,
            sell_through_raise_threshold: 0.90,
            sell_through_cut_threshold: 0.70,
            excess_inventory_months: 2.0,
            excess_inventory_cut_rate: 0.05,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FiscalParams {
    /// Monthly benefit paid to each unemployed working-age (18–65) adult,
    /// expressed as a fraction of `median_income_reference`.
    pub unemployment_benefit_rate: f64,
    /// Reference monthly income used to compute the unemployment benefit amount.
    pub median_income_reference: f64,
    /// If an unemployed person's cash is below this threshold AND they have
    /// unmet essential needs, the government tops up their cash to this amount.
    pub basic_needs_cash_threshold: f64,
}

impl Default for FiscalParams {
    fn default() -> Self {
        Self {
            unemployment_benefit_rate: 0.50,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
        }
    }
}

impl FiscalParams {
    pub fn monthly_unemployment_benefit(&self) -> f64 {
        self.unemployment_benefit_rate * self.median_income_reference
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimulationParams {
    pub demographic: DemographicParams,
    pub education: EducationParams,
    pub labor: LaborParams,
    pub company_growth: CompanyGrowthParams,
    pub goods_market: GoodsMarketParams,
    pub fiscal: FiscalParams,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiscal_benefit_is_rate_times_reference_income() {
        let params = FiscalParams {
            unemployment_benefit_rate: 0.5,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
        };
        assert!((params.monthly_unemployment_benefit() - 1_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fiscal_benefit_is_zero_when_rate_is_zero() {
        let params = FiscalParams {
            unemployment_benefit_rate: 0.0,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
        };
        assert!((params.monthly_unemployment_benefit()).abs() < f64::EPSILON);
    }

    #[test]
    fn fiscal_benefit_scales_with_reference_income() {
        let params = FiscalParams {
            unemployment_benefit_rate: 0.4,
            median_income_reference: 3_000.0,
            basic_needs_cash_threshold: 500.0,
        };
        assert!((params.monthly_unemployment_benefit() - 1_200.0).abs() < f64::EPSILON);
    }
}
