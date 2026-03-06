use super::*;
use crate::sim::essentials::{
    ESSENTIAL_NEEDS_MASK, HOUSING_GOOD_ID, STOCKABLE_ESSENTIAL_GOOD_IDS, housing_is_met,
    monthly_requirement_for_good, stockable_essential_slot,
};

impl Country {
    pub(super) fn evaluate_stockable_essential_needs(&mut self) {
        let essentials = self.sim_params.essentials;
        self.monthly_unmet_need_events = 0;
        self.monthly_severe_unmet_need_events = 0;
        self.monthly_chronic_unmet_need_count = 0;

        for person_idx in 0..self.people.len() {
            let household_size = self.household_size_for_person(person_idx);
            let person = &mut self.people[person_idx];
            person.last_month_needs_met_mask &= !ESSENTIAL_NEEDS_MASK;
            person.last_month_unmet_needs_mask &= !ESSENTIAL_NEEDS_MASK;
            person.last_month_severe_unmet_needs_mask = 0;
            person.last_month_chronic_unmet = false;

            for good_id in STOCKABLE_ESSENTIAL_GOOD_IDS {
                let Some(slot_idx) = stockable_essential_slot(good_id) else {
                    continue;
                };
                let required_units =
                    monthly_requirement_for_good(&essentials, good_id, household_size);
                if required_units <= 0.0 {
                    continue;
                }

                let available = person.essential_stock_units[slot_idx].max(0.0);
                let bit = 1_u16 << good_id;
                if available + 0.001 >= required_units {
                    person.essential_stock_units[slot_idx] = (available - required_units).max(0.0);
                    person.last_month_needs_met_mask |= bit;
                    continue;
                }

                let unmet_units = (required_units - available).max(0.0);
                let unmet_events = unmet_units.ceil() as u64;
                person.essential_stock_units[slot_idx] = 0.0;
                person.last_month_unmet_needs_mask |= bit;
                self.monthly_unmet_need_events =
                    self.monthly_unmet_need_events.saturating_add(unmet_events);

                let unmet_share = unmet_units / required_units.max(0.01);
                if unmet_share >= essentials.severe_unmet_share_threshold {
                    person.last_month_severe_unmet_needs_mask |= bit;
                    self.monthly_severe_unmet_need_events = self
                        .monthly_severe_unmet_need_events
                        .saturating_add(unmet_events);
                }
            }
        }
    }

    pub(super) fn finalize_housing_and_chronic_unmet(&mut self) {
        let essentials = self.sim_params.essentials;
        self.rebuild_households();

        let mut sheltered_by_household = vec![false; self.households.len()];
        for person in &self.people {
            let household_idx = person.household_id as usize;
            if household_idx >= sheltered_by_household.len() {
                continue;
            }
            if housing_is_met(
                &essentials,
                &person.housing,
                person.last_month_rent_paid,
                person.last_month_mortgage_payment,
                self.monthly_rent,
            ) {
                sheltered_by_household[household_idx] = true;
            }
        }

        let housing_bit = 1_u16 << HOUSING_GOOD_ID;
        let mut chronic_count = 0_u64;
        for household in &mut self.households {
            household.sheltered = sheltered_by_household
                .get(household.id as usize)
                .copied()
                .unwrap_or(false);
        }

        for person in &mut self.people {
            person.last_month_needs_met_mask &= !housing_bit;
            person.last_month_unmet_needs_mask &= !housing_bit;
            person.last_month_severe_unmet_needs_mask &= !housing_bit;

            let sheltered = sheltered_by_household
                .get(person.household_id as usize)
                .copied()
                .unwrap_or(false);
            if sheltered {
                person.last_month_needs_met_mask |= housing_bit;
            } else {
                person.last_month_unmet_needs_mask |= housing_bit;
                person.last_month_severe_unmet_needs_mask |= housing_bit;
                self.monthly_unmet_need_events = self.monthly_unmet_need_events.saturating_add(1);
                self.monthly_severe_unmet_need_events =
                    self.monthly_severe_unmet_need_events.saturating_add(1);
            }

            if (person.last_month_severe_unmet_needs_mask & ESSENTIAL_NEEDS_MASK) != 0 {
                person.consecutive_severe_unmet_months =
                    person.consecutive_severe_unmet_months.saturating_add(1);
            } else {
                person.consecutive_severe_unmet_months = 0;
            }
            person.last_month_chronic_unmet =
                person.consecutive_severe_unmet_months >= essentials.chronic_unmet_months_threshold;
            if person.last_month_chronic_unmet {
                chronic_count = chronic_count.saturating_add(1);
            }
        }

        self.monthly_chronic_unmet_need_count = chronic_count;
    }
}
