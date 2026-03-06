use super::*;
use crate::sim::{
    essentials::{
        estimated_country_monthly_requirement, is_stockable_essential_good,
        monthly_requirement_for_good, production_capacity_units_for_workers,
    },
    good::good_by_id,
};

impl Country {
    pub(crate) fn essential_requirement_units_for_good(&self, good_id: u8) -> f64 {
        if !is_stockable_essential_good(good_id) {
            return 0.0;
        }

        let essentials = &self.sim_params.essentials;
        self.people
            .iter()
            .enumerate()
            .map(|(person_idx, _)| {
                let household_size = self.household_size_for_person(person_idx);
                f64::from(monthly_requirement_for_good(
                    essentials,
                    good_id,
                    household_size,
                ))
            })
            .sum()
    }

    pub(crate) fn estimated_requirement_units_for_good(&self, good_id: u8) -> f64 {
        estimated_country_monthly_requirement(
            &self.sim_params.essentials,
            good_id,
            self.people.len(),
        )
    }

    pub(crate) fn essential_inventory_units_for_good(&self, good_id: u8) -> f64 {
        if !is_stockable_essential_good(good_id) {
            return 0.0;
        }

        self.companies
            .iter()
            .filter(|company| company.produced_good_id == good_id)
            .map(|company| f64::from(company.inventory_units))
            .sum()
    }

    pub(crate) fn essential_manufacturing_capacity_units_for_good(&self, good_id: u8) -> f64 {
        let Some(good) = good_by_id(good_id) else {
            return 0.0;
        };

        self.companies
            .iter()
            .filter(|company| company.produced_good_id == good_id)
            .map(|company| {
                production_capacity_units_for_workers(
                    &self.sim_params.essentials,
                    self.sim_params.goods_market.worker_effort_per_month,
                    good.effort_per_unit,
                    company.manufacturing_employee_count,
                )
            })
            .sum()
    }

    pub(crate) fn essential_people_counts_for_good(&self, good_id: u8) -> (usize, usize) {
        let bit = 1_u16 << good_id;
        self.people.iter().fold((0_usize, 0_usize), |(unmet, severe), person| {
            (
                unmet + usize::from((person.last_month_unmet_needs_mask & bit) != 0),
                severe + usize::from((person.last_month_severe_unmet_needs_mask & bit) != 0),
            )
        })
    }

    pub(crate) fn essential_company_counts_for_good(&self, good_id: u8) -> (usize, usize) {
        self.companies.iter().fold((0_usize, 0_usize), |(all, public), company| {
            if company.produced_good_id != good_id {
                return (all, public);
            }

            (
                all + 1,
                public + usize::from(company.is_public_utility),
            )
        })
    }

    pub(crate) fn essential_inventory_cover_months_for_good(&self, good_id: u8) -> f64 {
        let required_units = self.essential_requirement_units_for_good(good_id);
        if required_units <= 0.0 {
            return 0.0;
        }

        self.essential_inventory_units_for_good(good_id) / required_units
    }
}
