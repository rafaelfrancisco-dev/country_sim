use crate::sim::{good::GoodId, person::HousingStatus};

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
pub const ELECTRICITY_HOUSEHOLD_BASE_UNITS: f32 = 0.18;
pub const ELECTRICITY_PER_PERSON_UNITS: f32 = 0.07;
pub const DEFAULT_ESSENTIAL_BUFFER_MONTHS: f32 = 0.50;
pub const SEVERE_UNMET_SHARE_THRESHOLD: f32 = 0.25;
pub const CHRONIC_UNMET_MONTHS_THRESHOLD: u8 = 3;
pub const TARGET_HOUSEHOLD_SIZE: usize = 3;

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

pub fn monthly_requirement_for_good(good_id: GoodId, household_size: usize) -> f32 {
    let size = household_size.max(1) as f32;
    match good_id {
        WATER_GOOD_ID => 2.2,
        FOOD_GOOD_ID => 1.8,
        ELECTRICITY_GOOD_ID => {
            (ELECTRICITY_HOUSEHOLD_BASE_UNITS / size) + ELECTRICITY_PER_PERSON_UNITS
        }
        MEDICINE_GOOD_ID => 0.15,
        _ => 0.0,
    }
}

pub fn target_buffer_units(good_id: GoodId, household_size: usize) -> f32 {
    let requirement = monthly_requirement_for_good(good_id, household_size);
    match good_id {
        MEDICINE_GOOD_ID => requirement.max(1.0),
        _ if is_stockable_essential_good(good_id) => requirement * DEFAULT_ESSENTIAL_BUFFER_MONTHS,
        _ => 0.0,
    }
}

pub fn housing_is_met(
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
                    >= scheduled * f64::from(1.0_f32 - SEVERE_UNMET_SHARE_THRESHOLD)
        }
        HousingStatus::Renter { .. } => {
            let due = monthly_rent.max(0.0);
            due <= 0.0
                || last_month_rent_paid + 0.01
                    >= due * f64::from(1.0_f32 - SEVERE_UNMET_SHARE_THRESHOLD)
        }
    }
}
