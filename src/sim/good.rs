pub type GoodId = u8;
use super::job::JobTypeId;

pub const GOODS_COUNT: usize = 18;
pub const WATER_GOOD_ID: GoodId = 0;
pub const FOOD_GOOD_ID: GoodId = 1;
pub const HOUSING_GOOD_ID: GoodId = 2;
pub const ELECTRICITY_GOOD_ID: GoodId = 3;
pub const MEDICINE_GOOD_ID: GoodId = 4;
pub const CLOTHING_GOOD_ID: GoodId = 5;
pub const HYGIENE_PRODUCTS_GOOD_ID: GoodId = 6;
pub const TRANSPORT_GOOD_ID: GoodId = 7;
pub const INTERNET_GOOD_ID: GoodId = 8;
pub const EDUCATION_SERVICES_GOOD_ID: GoodId = 9;
pub const HOUSEHOLD_GOODS_GOOD_ID: GoodId = 10;
pub const ELECTRONICS_GOOD_ID: GoodId = 11;
pub const APPLIANCES_GOOD_ID: GoodId = 12;
pub const CARS_GOOD_ID: GoodId = 13;
pub const ENTERTAINMENT_GOOD_ID: GoodId = 14;
pub const RAW_METALS_GOOD_ID: GoodId = 15;
pub const TIMBER_GOOD_ID: GoodId = 16;
pub const FUEL_GOOD_ID: GoodId = 17;

const OFFICE_CLERK_JOB_ID: JobTypeId = 2;
const DRIVER_JOB_ID: JobTypeId = 3;
const CONSTRUCTION_WORKER_JOB_ID: JobTypeId = 4;
const MANUFACTURING_OPERATOR_JOB_ID: JobTypeId = 5;
const REGISTERED_NURSE_JOB_ID: JobTypeId = 7;
const TEACHER_JOB_ID: JobTypeId = 8;
const SOFTWARE_DEVELOPER_JOB_ID: JobTypeId = 9;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Good {
    pub id: GoodId,
    pub name: &'static str,
    pub necessity_rank: u8,
    pub effort_per_unit: f64,
    pub monthly_demand_factor: f64,
    pub base_unit_cost: f64,
    /// Durable goods (e.g. Housing) are managed outside the round-robin goods market.
    pub is_durable: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CompanyInputRecipe {
    pub input_units_per_output: [u16; GOODS_COUNT],
}

impl CompanyInputRecipe {
    pub fn units_required_for(self, good_id: GoodId) -> u16 {
        self.input_units_per_output
            .get(good_id as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn inputs(self) -> impl Iterator<Item = (GoodId, u16)> {
        self.input_units_per_output
            .into_iter()
            .enumerate()
            .filter_map(|(good_idx, units)| {
                if units == 0 {
                    None
                } else {
                    Some((good_idx as GoodId, units))
                }
            })
    }
}

const fn recipe(entries: &[(GoodId, u16)]) -> CompanyInputRecipe {
    let mut input_units_per_output = [0_u16; GOODS_COUNT];
    let mut idx = 0usize;
    while idx < entries.len() {
        let (good_id, units) = entries[idx];
        input_units_per_output[good_id as usize] = units;
        idx += 1;
    }
    CompanyInputRecipe {
        input_units_per_output,
    }
}

const COMMON_GOODS: [Good; GOODS_COUNT] = [
    Good {
        id: 0,
        name: "Water",
        necessity_rank: 1,
        effort_per_unit: 2.5,
        monthly_demand_factor: 2.2,
        base_unit_cost: 6.0,
        is_durable: false,
    },
    Good {
        id: 1,
        name: "Food",
        necessity_rank: 2,
        effort_per_unit: 3.0,
        monthly_demand_factor: 1.8,
        base_unit_cost: 14.0,
        is_durable: false,
    },
    Good {
        id: 2,
        name: "Housing",
        necessity_rank: 3,
        effort_per_unit: 8.0,
        monthly_demand_factor: 0.45,
        base_unit_cost: 520.0,
        is_durable: true,
    },
    Good {
        id: 3,
        name: "Electricity",
        necessity_rank: 4,
        effort_per_unit: 6.5,
        monthly_demand_factor: 0.35,
        base_unit_cost: 95.0,
        is_durable: false,
    },
    Good {
        id: 4,
        name: "Medicine",
        necessity_rank: 5,
        effort_per_unit: 5.0,
        monthly_demand_factor: 0.15,
        base_unit_cost: 60.0,
        is_durable: false,
    },
    Good {
        id: 5,
        name: "Clothing",
        necessity_rank: 6,
        effort_per_unit: 4.0,
        monthly_demand_factor: 0.8,
        base_unit_cost: 38.0,
        is_durable: false,
    },
    Good {
        id: 6,
        name: "Hygiene Products",
        necessity_rank: 7,
        effort_per_unit: 3.3,
        monthly_demand_factor: 1.0,
        base_unit_cost: 22.0,
        is_durable: false,
    },
    Good {
        id: 7,
        name: "Transport",
        necessity_rank: 8,
        effort_per_unit: 5.5,
        monthly_demand_factor: 1.0,
        base_unit_cost: 70.0,
        is_durable: false,
    },
    Good {
        id: 8,
        name: "Internet",
        necessity_rank: 9,
        effort_per_unit: 4.5,
        monthly_demand_factor: 1.0,
        base_unit_cost: 44.0,
        is_durable: false,
    },
    Good {
        id: 9,
        name: "Education Services",
        necessity_rank: 10,
        effort_per_unit: 8.0,
        monthly_demand_factor: 0.5,
        base_unit_cost: 130.0,
        is_durable: false,
    },
    Good {
        id: 10,
        name: "Household Goods",
        necessity_rank: 11,
        effort_per_unit: 4.8,
        monthly_demand_factor: 0.7,
        base_unit_cost: 58.0,
        is_durable: false,
    },
    Good {
        id: 11,
        name: "Electronics",
        necessity_rank: 12,
        effort_per_unit: 9.5,
        monthly_demand_factor: 0.4,
        base_unit_cost: 180.0,
        is_durable: false,
    },
    Good {
        id: 12,
        name: "Appliances",
        necessity_rank: 13,
        effort_per_unit: 11.0,
        monthly_demand_factor: 0.3,
        base_unit_cost: 240.0,
        is_durable: false,
    },
    Good {
        id: 13,
        name: "Cars",
        necessity_rank: 14,
        effort_per_unit: 24.0,
        monthly_demand_factor: 0.12,
        base_unit_cost: 2_000.0,
        is_durable: false,
    },
    Good {
        id: 14,
        name: "Entertainment",
        necessity_rank: 15,
        effort_per_unit: 4.2,
        monthly_demand_factor: 1.2,
        base_unit_cost: 32.0,
        is_durable: false,
    },
    Good {
        id: 15,
        name: "Raw Metals",
        necessity_rank: 16,
        effort_per_unit: 4.4,
        monthly_demand_factor: 0.05,
        base_unit_cost: 18.0,
        is_durable: false,
    },
    Good {
        id: 16,
        name: "Timber",
        necessity_rank: 17,
        effort_per_unit: 3.8,
        monthly_demand_factor: 0.06,
        base_unit_cost: 15.0,
        is_durable: false,
    },
    Good {
        id: 17,
        name: "Fuel",
        necessity_rank: 18,
        effort_per_unit: 4.1,
        monthly_demand_factor: 0.08,
        base_unit_cost: 20.0,
        is_durable: false,
    },
];

const MATERIAL_FREE_RECIPE: CompanyInputRecipe = recipe(&[]);
const COMPANY_INPUT_RECIPES: [CompanyInputRecipe; GOODS_COUNT] = [
    MATERIAL_FREE_RECIPE,                                    // Water
    MATERIAL_FREE_RECIPE,                                    // Food
    recipe(&[(TIMBER_GOOD_ID, 1), (RAW_METALS_GOOD_ID, 1)]), // Housing
    MATERIAL_FREE_RECIPE,                                    // Electricity
    MATERIAL_FREE_RECIPE,                                    // Medicine
    recipe(&[(TIMBER_GOOD_ID, 1)]),                          // Clothing
    recipe(&[(FUEL_GOOD_ID, 1)]),                            // Hygiene Products
    MATERIAL_FREE_RECIPE,                                    // Transport
    MATERIAL_FREE_RECIPE,                                    // Internet
    MATERIAL_FREE_RECIPE,                                    // Education Services
    recipe(&[(TIMBER_GOOD_ID, 1), (FUEL_GOOD_ID, 1)]),       // Household Goods
    recipe(&[(RAW_METALS_GOOD_ID, 1), (FUEL_GOOD_ID, 1)]),   // Electronics
    recipe(&[(RAW_METALS_GOOD_ID, 2), (FUEL_GOOD_ID, 1)]),   // Appliances
    recipe(&[(RAW_METALS_GOOD_ID, 3), (FUEL_GOOD_ID, 2)]),   // Cars
    MATERIAL_FREE_RECIPE,                                    // Entertainment
    MATERIAL_FREE_RECIPE,                                    // Raw Metals
    MATERIAL_FREE_RECIPE,                                    // Timber
    MATERIAL_FREE_RECIPE,                                    // Fuel
];

const MINE_JOBS: [JobTypeId; 3] = [
    CONSTRUCTION_WORKER_JOB_ID,
    DRIVER_JOB_ID,
    MANUFACTURING_OPERATOR_JOB_ID,
];
const BASIC_PRODUCTION_JOBS: [JobTypeId; 3] = [
    DRIVER_JOB_ID,
    MANUFACTURING_OPERATOR_JOB_ID,
    OFFICE_CLERK_JOB_ID,
];
const HOUSING_JOBS: [JobTypeId; 3] = [
    CONSTRUCTION_WORKER_JOB_ID,
    DRIVER_JOB_ID,
    OFFICE_CLERK_JOB_ID,
];
const MANUFACTURING_JOBS: [JobTypeId; 3] = [
    MANUFACTURING_OPERATOR_JOB_ID,
    DRIVER_JOB_ID,
    OFFICE_CLERK_JOB_ID,
];
const MEDICINE_JOBS: [JobTypeId; 3] = [REGISTERED_NURSE_JOB_ID, OFFICE_CLERK_JOB_ID, DRIVER_JOB_ID];
const EDUCATION_JOBS: [JobTypeId; 2] = [TEACHER_JOB_ID, OFFICE_CLERK_JOB_ID];
const INTERNET_JOBS: [JobTypeId; 2] = [SOFTWARE_DEVELOPER_JOB_ID, OFFICE_CLERK_JOB_ID];
const TRANSPORT_JOBS: [JobTypeId; 2] = [DRIVER_JOB_ID, OFFICE_CLERK_JOB_ID];
const ENTERTAINMENT_JOBS: [JobTypeId; 2] = [OFFICE_CLERK_JOB_ID, DRIVER_JOB_ID];

pub fn common_goods() -> &'static [Good; GOODS_COUNT] {
    &COMMON_GOODS
}

pub fn good_by_id(id: GoodId) -> Option<&'static Good> {
    COMMON_GOODS.iter().find(|good| good.id == id)
}

pub fn recipe_for_good(good_id: GoodId) -> &'static CompanyInputRecipe {
    COMPANY_INPUT_RECIPES
        .get(good_id as usize)
        .unwrap_or(&MATERIAL_FREE_RECIPE)
}

pub fn industry_job_types_for_good(good_id: GoodId) -> &'static [JobTypeId] {
    match good_id {
        RAW_METALS_GOOD_ID | FUEL_GOOD_ID => &MINE_JOBS,
        TIMBER_GOOD_ID | WATER_GOOD_ID | FOOD_GOOD_ID | ELECTRICITY_GOOD_ID => {
            &BASIC_PRODUCTION_JOBS
        }
        HOUSING_GOOD_ID => &HOUSING_JOBS,
        CLOTHING_GOOD_ID
        | HYGIENE_PRODUCTS_GOOD_ID
        | HOUSEHOLD_GOODS_GOOD_ID
        | ELECTRONICS_GOOD_ID
        | APPLIANCES_GOOD_ID
        | CARS_GOOD_ID => &MANUFACTURING_JOBS,
        MEDICINE_GOOD_ID => &MEDICINE_JOBS,
        EDUCATION_SERVICES_GOOD_ID => &EDUCATION_JOBS,
        INTERNET_GOOD_ID => &INTERNET_JOBS,
        TRANSPORT_GOOD_ID => &TRANSPORT_JOBS,
        ENTERTAINMENT_GOOD_ID => &ENTERTAINMENT_JOBS,
        _ => &[],
    }
}

pub fn production_job_for_good(good_id: GoodId) -> Option<JobTypeId> {
    match good_id {
        WATER_GOOD_ID
        | FOOD_GOOD_ID
        | ELECTRICITY_GOOD_ID
        | CLOTHING_GOOD_ID
        | HYGIENE_PRODUCTS_GOOD_ID
        | HOUSEHOLD_GOODS_GOOD_ID
        | ELECTRONICS_GOOD_ID
        | APPLIANCES_GOOD_ID
        | CARS_GOOD_ID
        | RAW_METALS_GOOD_ID
        | TIMBER_GOOD_ID
        | FUEL_GOOD_ID => Some(MANUFACTURING_OPERATOR_JOB_ID),
        HOUSING_GOOD_ID => Some(CONSTRUCTION_WORKER_JOB_ID),
        MEDICINE_GOOD_ID => Some(REGISTERED_NURSE_JOB_ID),
        EDUCATION_SERVICES_GOOD_ID => Some(TEACHER_JOB_ID),
        INTERNET_GOOD_ID => Some(SOFTWARE_DEVELOPER_JOB_ID),
        TRANSPORT_GOOD_ID => Some(DRIVER_JOB_ID),
        ENTERTAINMENT_GOOD_ID => Some(OFFICE_CLERK_JOB_ID),
        _ => None,
    }
}

pub fn is_production_job_for_good(good_id: GoodId, job_id: JobTypeId) -> bool {
    production_job_for_good(good_id) == Some(job_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_eighteen_common_goods_exist() {
        assert_eq!(common_goods().len(), 18);
    }

    #[test]
    fn goods_ids_are_unique() {
        let mut ids = common_goods()
            .iter()
            .map(|good| good.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), 18);
    }

    #[test]
    fn necessity_rank_is_strictly_increasing() {
        let ranks = common_goods()
            .iter()
            .map(|good| good.necessity_rank)
            .collect::<Vec<_>>();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        sorted.dedup();

        assert_eq!(ranks, sorted);
        assert_eq!(ranks.first().copied(), Some(1));
        assert_eq!(ranks.last().copied(), Some(18));
    }

    #[test]
    fn appended_upstream_goods_have_expected_ids() {
        assert_eq!(
            good_by_id(RAW_METALS_GOOD_ID).map(|good| good.name),
            Some("Raw Metals")
        );
        assert_eq!(
            good_by_id(TIMBER_GOOD_ID).map(|good| good.name),
            Some("Timber")
        );
        assert_eq!(good_by_id(FUEL_GOOD_ID).map(|good| good.name), Some("Fuel"));
    }

    #[test]
    fn downstream_goods_expose_expected_recipes() {
        let housing_recipe = recipe_for_good(HOUSING_GOOD_ID);
        assert_eq!(housing_recipe.units_required_for(TIMBER_GOOD_ID), 1);
        assert_eq!(housing_recipe.units_required_for(RAW_METALS_GOOD_ID), 1);

        let cars_recipe = recipe_for_good(CARS_GOOD_ID);
        assert_eq!(cars_recipe.units_required_for(RAW_METALS_GOOD_ID), 3);
        assert_eq!(cars_recipe.units_required_for(FUEL_GOOD_ID), 2);

        let electricity_recipe = recipe_for_good(ELECTRICITY_GOOD_ID);
        assert_eq!(electricity_recipe.inputs().count(), 0);
    }

    #[test]
    fn industry_job_catalogs_are_sector_specific() {
        assert_eq!(
            industry_job_types_for_good(RAW_METALS_GOOD_ID),
            &[
                CONSTRUCTION_WORKER_JOB_ID,
                DRIVER_JOB_ID,
                MANUFACTURING_OPERATOR_JOB_ID
            ]
        );
        assert_eq!(
            industry_job_types_for_good(EDUCATION_SERVICES_GOOD_ID),
            &[TEACHER_JOB_ID, OFFICE_CLERK_JOB_ID]
        );
        assert_eq!(
            industry_job_types_for_good(INTERNET_GOOD_ID),
            &[SOFTWARE_DEVELOPER_JOB_ID, OFFICE_CLERK_JOB_ID]
        );
    }

    #[test]
    fn production_job_mapping_matches_sector_output_roles() {
        assert_eq!(
            production_job_for_good(HOUSING_GOOD_ID),
            Some(CONSTRUCTION_WORKER_JOB_ID)
        );
        assert_eq!(
            production_job_for_good(MEDICINE_GOOD_ID),
            Some(REGISTERED_NURSE_JOB_ID)
        );
        assert_eq!(
            production_job_for_good(EDUCATION_SERVICES_GOOD_ID),
            Some(TEACHER_JOB_ID)
        );
        assert_eq!(
            production_job_for_good(INTERNET_GOOD_ID),
            Some(SOFTWARE_DEVELOPER_JOB_ID)
        );
        assert_eq!(
            production_job_for_good(TRANSPORT_GOOD_ID),
            Some(DRIVER_JOB_ID)
        );
        assert_eq!(
            production_job_for_good(ENTERTAINMENT_GOOD_ID),
            Some(OFFICE_CLERK_JOB_ID)
        );
    }
}
