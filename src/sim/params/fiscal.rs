#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FiscalParams {
    pub unemployment_benefit_rate: f64,
    pub median_income_reference: f64,
    pub basic_needs_cash_threshold: f64,
    pub max_benefit_months: u32,
    pub austerity_debt_gdp_threshold: f64,
    pub austerity_benefit_reduction: f64,
    pub austerity_corporate_tax_surcharge: f64,
}

impl Default for FiscalParams {
    fn default() -> Self {
        Self {
            unemployment_benefit_rate: 0.50,
            median_income_reference: 2_000.0,
            basic_needs_cash_threshold: 500.0,
            max_benefit_months: 12,
            austerity_debt_gdp_threshold: 0.80,
            austerity_benefit_reduction: 0.30,
            austerity_corporate_tax_surcharge: 0.05,
        }
    }
}

impl FiscalParams {
    pub fn monthly_unemployment_benefit(&self) -> f64 {
        self.unemployment_benefit_rate * self.median_income_reference
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EssentialEconomyParams {
    pub water_monthly_units_per_person: f32,
    pub food_monthly_units_per_person: f32,
    pub electricity_household_base_units: f32,
    pub electricity_monthly_units_per_person: f32,
    pub medicine_monthly_units_per_person: f32,
    pub water_buffer_months: f32,
    pub food_buffer_months: f32,
    pub electricity_buffer_months: f32,
    pub medicine_buffer_months: f32,
    pub medicine_min_buffer_units: f32,
    pub severe_unmet_share_threshold: f32,
    pub chronic_unmet_months_threshold: u8,
    pub target_household_size: usize,
    pub target_manufacturing_workers_per_initial_company: u16,
    pub target_manufacturing_workers_per_backfill_company: u16,
    pub production_staffing_buffer: f64,
    pub initial_inventory_cover_months: f32,
    pub shortage_unmet_share_trigger: f64,
    pub shortage_severe_share_trigger: f64,
    pub shortage_inventory_cover_months_trigger: f64,
    pub shortage_target_months_of_cover: f64,
    pub max_backfill_companies_per_good_per_month: u16,
}

impl Default for EssentialEconomyParams {
    fn default() -> Self {
        Self {
            water_monthly_units_per_person: 1.8,
            food_monthly_units_per_person: 1.5,
            electricity_household_base_units: 0.12,
            electricity_monthly_units_per_person: 0.05,
            medicine_monthly_units_per_person: 0.08,
            water_buffer_months: 0.20,
            food_buffer_months: 0.20,
            electricity_buffer_months: 0.10,
            medicine_buffer_months: 0.20,
            medicine_min_buffer_units: 0.25,
            severe_unmet_share_threshold: 0.25,
            chronic_unmet_months_threshold: 3,
            target_household_size: 3,
            target_manufacturing_workers_per_initial_company: 80,
            target_manufacturing_workers_per_backfill_company: 120,
            production_staffing_buffer: 1.15,
            initial_inventory_cover_months: 0.75,
            shortage_unmet_share_trigger: 0.02,
            shortage_severe_share_trigger: 0.005,
            shortage_inventory_cover_months_trigger: 0.30,
            shortage_target_months_of_cover: 1.10,
            max_backfill_companies_per_good_per_month: 2,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GovernmentBackfillParams {
    pub enabled: bool,
    pub essential_good_max_id_inclusive: u8,
    pub openings_total: u16,
    pub manufacturing_openings: u16,
    pub runway_months: f64,
    pub base_wage_multiplier: f64,
    pub vacancy_escalation_rate: f64,
    pub max_wage_multiplier: f64,
    pub switch_incentive_buffer: f64,
    pub public_utility_price_cap_multiplier: f64,
}

impl Default for GovernmentBackfillParams {
    fn default() -> Self {
        Self {
            enabled: true,
            essential_good_max_id_inclusive: 4,
            openings_total: 3,
            manufacturing_openings: 2,
            runway_months: 1.5,
            base_wage_multiplier: 1.15,
            vacancy_escalation_rate: 0.04,
            max_wage_multiplier: 1.35,
            switch_incentive_buffer: 0.05,
            public_utility_price_cap_multiplier: 1.5,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HousingMarketParams {
    pub mortgage_term_months: u16,
    pub mortgage_down_payment_rate: f64,
    pub mortgage_max_dti: f64,
    pub mortgage_spread_over_policy: f64,
    pub foreclosure_missed_months: u8,
    pub rent_as_fraction_of_price: f64,
    pub target_vacancy_rate: f64,
    pub max_monthly_price_change: f64,
    pub renter_purchase_probability: f64,
    pub initial_owner_rate: f64,
    pub initial_renter_rate: f64,
    pub initial_price_index: f64,
}

impl Default for HousingMarketParams {
    fn default() -> Self {
        Self {
            mortgage_term_months: 240,
            mortgage_down_payment_rate: 0.20,
            mortgage_max_dti: 0.28,
            mortgage_spread_over_policy: 0.02,
            foreclosure_missed_months: 3,
            rent_as_fraction_of_price: 0.007,
            target_vacancy_rate: 0.05,
            max_monthly_price_change: 0.02,
            renter_purchase_probability: 0.01,
            initial_owner_rate: 0.60,
            initial_renter_rate: 0.35,
            initial_price_index: 80_000.0,
        }
    }
}
