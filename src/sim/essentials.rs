use crate::sim::{
    good::GoodId,
    params::EssentialEconomyParams,
    person::HousingStatus,
};

pub const WATER_GOOD_ID: GoodId = 0;
pub const FOOD_GOOD_ID: GoodId = 1;
pub const HOUSING_GOOD_ID: GoodId = 2;
pub const ELECTRICITY_GOOD_ID: GoodId = 3;
pub const MEDICINE_GOOD_ID: GoodId = 4;

pub const ESSENTIAL_GOOD_IDS: [GoodId; 5] = [
    WATER_GOOD_ID,
    FOOD_GOOD_ID,
    HOUSING_GOOD_ID,
    ELECTRICITY_GOOD_ID,
    MEDICINE_GOOD_ID,
];
pub const STOCKABLE_ESSENTIAL_GOOD_IDS: [GoodId; 4] = [
    WATER_GOOD_ID,
    FOOD_GOOD_ID,
    ELECTRICITY_GOOD_ID,
    MEDICINE_GOOD_ID,
];
pub const ESSENTIAL_NEEDS_MASK: u16 = 0b0001_1111;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Household {
    pub id: u32,
    pub member_count: u16,
    pub sheltered: bool,
}

pub fn is_essential_good(good_id: GoodId) -> bool {
    good_id <= MEDICINE_GOOD_ID
}

pub fn is_stockable_essential_good(good_id: GoodId) -> bool {
    matches!(
        good_id,
        WATER_GOOD_ID | FOOD_GOOD_ID | ELECTRICITY_GOOD_ID | MEDICINE_GOOD_ID
    )
}

pub fn stockable_essential_slot(good_id: GoodId) -> Option<usize> {
    STOCKABLE_ESSENTIAL_GOOD_IDS
        .iter()
        .position(|candidate| *candidate == good_id)
}

pub fn estimated_household_count(
    params: &EssentialEconomyParams,
    population: usize,
) -> usize {
    population.div_ceil(params.target_household_size.max(1)).max(1)
}

pub fn estimated_country_monthly_requirement(
    params: &EssentialEconomyParams,
    good_id: GoodId,
    population: usize,
) -> f64 {
    if population == 0 {
        return 0.0;
    }

    match good_id {
        WATER_GOOD_ID => f64::from(params.water_monthly_units_per_person) * population as f64,
        FOOD_GOOD_ID => f64::from(params.food_monthly_units_per_person) * population as f64,
        ELECTRICITY_GOOD_ID => {
            let households = estimated_household_count(params, population);
            f64::from(params.electricity_household_base_units) * households as f64
                + f64::from(params.electricity_monthly_units_per_person) * population as f64
        }
        MEDICINE_GOOD_ID => f64::from(params.medicine_monthly_units_per_person) * population as f64,
        _ => 0.0,
    }
}

pub fn estimated_country_stock_target_units(
    params: &EssentialEconomyParams,
    good_id: GoodId,
    population: usize,
) -> f64 {
    if !is_stockable_essential_good(good_id) || population == 0 {
        return 0.0;
    }

    let household_size = params.target_household_size.max(1);
    let requirement = monthly_requirement_for_good(params, good_id, household_size);
    let buffer = target_buffer_units(params, good_id, household_size);
    f64::from(requirement + buffer) * population as f64
}

pub fn target_manufacturing_headcount_for_units(
    params: &EssentialEconomyParams,
    worker_effort_per_month: f64,
    effort_per_unit: f64,
    monthly_units: f64,
) -> u32 {
    if worker_effort_per_month <= 0.0 || effort_per_unit <= 0.0 || monthly_units <= 0.0 {
        return 0;
    }

    ((monthly_units * effort_per_unit * params.production_staffing_buffer) / worker_effort_per_month)
        .ceil()
        .max(0.0) as u32
}

pub fn production_capacity_units_for_workers(
    params: &EssentialEconomyParams,
    worker_effort_per_month: f64,
    effort_per_unit: f64,
    manufacturing_workers: u32,
) -> f64 {
    if worker_effort_per_month <= 0.0 || effort_per_unit <= 0.0 || manufacturing_workers == 0 {
        return 0.0;
    }

    (manufacturing_workers as f64 * worker_effort_per_month)
        / (effort_per_unit * params.production_staffing_buffer.max(1.0))
}

pub fn monthly_requirement_for_good(
    params: &EssentialEconomyParams,
    good_id: GoodId,
    household_size: usize,
) -> f32 {
    let size = household_size.max(1) as f32;
    match good_id {
        WATER_GOOD_ID => params.water_monthly_units_per_person,
        FOOD_GOOD_ID => params.food_monthly_units_per_person,
        ELECTRICITY_GOOD_ID => {
            (params.electricity_household_base_units / size)
                + params.electricity_monthly_units_per_person
        }
        MEDICINE_GOOD_ID => params.medicine_monthly_units_per_person,
        _ => 0.0,
    }
}

pub fn target_buffer_units(
    params: &EssentialEconomyParams,
    good_id: GoodId,
    household_size: usize,
) -> f32 {
    let requirement = monthly_requirement_for_good(params, good_id, household_size);
    match good_id {
        WATER_GOOD_ID => requirement * params.water_buffer_months,
        FOOD_GOOD_ID => requirement * params.food_buffer_months,
        ELECTRICITY_GOOD_ID => requirement * params.electricity_buffer_months,
        MEDICINE_GOOD_ID => {
            (requirement * params.medicine_buffer_months).max(params.medicine_min_buffer_units)
        }
        _ if is_stockable_essential_good(good_id) => 0.0,
        _ => 0.0,
    }
}

pub fn housing_is_met(
    params: &EssentialEconomyParams,
    housing: &HousingStatus,
    last_month_rent_paid: f64,
    last_month_mortgage_payment: f64,
    monthly_rent: f64,
) -> bool {
    match housing {
        HousingStatus::Unhoused => false,
        HousingStatus::Owner => true,
        HousingStatus::Mortgaged { mortgage } => {
            let scheduled = mortgage.monthly_payment.max(0.0);
            scheduled <= 0.0
                || last_month_mortgage_payment + 0.01
                    >= scheduled
                        * f64::from(1.0_f32 - params.severe_unmet_share_threshold)
        }
        HousingStatus::Renter { .. } => {
            let due = monthly_rent.max(0.0);
            due <= 0.0
                || last_month_rent_paid + 0.01
                    >= due * f64::from(1.0_f32 - params.severe_unmet_share_threshold)
        }
    }
}
