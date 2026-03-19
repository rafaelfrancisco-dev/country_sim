use crate::sim::{
    country::{Country, DAYS_PER_MONTH},
    essentials::{
        STOCKABLE_ESSENTIAL_GOOD_IDS, monthly_requirement_for_good, stockable_essential_slot,
        target_buffer_units,
    },
    good::GoodId,
    person::{ESSENTIAL_STOCK_SLOT_COUNT, SUPERFICIAL_GOOD_SLOT_COUNT},
};
use super::{BuyerDemand, hash64};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CountryGoodsRuntime {
    pub(super) stockable_state_month: u64,
    pub(super) person_count: usize,
    pub(super) stockable_initial_deficits: Vec<[u32; ESSENTIAL_STOCK_SLOT_COUNT]>,
    pub(super) stockable_deficits: Vec<[u32; ESSENTIAL_STOCK_SLOT_COUNT]>,
    pub(super) stockable_targets: Vec<[u32; ESSENTIAL_STOCK_SLOT_COUNT]>,
    pub(super) stockable_active_buyers: [Vec<usize>; ESSENTIAL_STOCK_SLOT_COUNT],
    pub(super) stockable_active_positions: [Vec<usize>; ESSENTIAL_STOCK_SLOT_COUNT],
    pub(super) superficial_zero_stock_buyers: [Vec<usize>; SUPERFICIAL_GOOD_SLOT_COUNT],
    pub(super) superficial_zero_stock_positions: [Vec<usize>; SUPERFICIAL_GOOD_SLOT_COUNT],
}

impl Default for CountryGoodsRuntime {
    fn default() -> Self {
        Self {
            stockable_state_month: u64::MAX,
            person_count: 0,
            stockable_initial_deficits: Vec::new(),
            stockable_deficits: Vec::new(),
            stockable_targets: Vec::new(),
            stockable_active_buyers: std::array::from_fn(|_| Vec::new()),
            stockable_active_positions: std::array::from_fn(|_| Vec::new()),
            superficial_zero_stock_buyers: std::array::from_fn(|_| Vec::new()),
            superficial_zero_stock_positions: std::array::from_fn(|_| Vec::new()),
        }
    }
}

impl CountryGoodsRuntime {
    pub(super) fn rebuild_from_country(&mut self, country: &Country) {
        let person_count = country.people.len();
        self.person_count = person_count;
        self.stockable_state_month = country.month;
        self.stockable_initial_deficits
            .resize(person_count, [0; ESSENTIAL_STOCK_SLOT_COUNT]);
        self.stockable_deficits
            .resize(person_count, [0; ESSENTIAL_STOCK_SLOT_COUNT]);
        self.stockable_targets
            .resize(person_count, [0; ESSENTIAL_STOCK_SLOT_COUNT]);

        for buyers in &mut self.stockable_active_buyers {
            buyers.clear();
        }
        for positions in &mut self.stockable_active_positions {
            positions.clear();
            positions.resize(person_count, usize::MAX);
        }
        for buyers in &mut self.superficial_zero_stock_buyers {
            buyers.clear();
        }
        for positions in &mut self.superficial_zero_stock_positions {
            positions.clear();
            positions.resize(person_count, usize::MAX);
        }

        for (person_idx, person) in country.people.iter().enumerate() {
            let household_size = country.household_size_for_person(person_idx);
            for &good_id in &STOCKABLE_ESSENTIAL_GOOD_IDS {
                let Some(slot_idx) = stockable_essential_slot(good_id) else {
                    continue;
                };
                let target_units = stock_target_units(country, good_id, household_size);
                let current_units = person.essential_stock_units[slot_idx].max(0.0);
                let deficit = (target_units - current_units).ceil().max(0.0) as u32;
                self.stockable_targets[person_idx][slot_idx] =
                    target_units.ceil().max(0.0) as u32;
                self.stockable_initial_deficits[person_idx][slot_idx] = deficit;
                self.stockable_deficits[person_idx][slot_idx] = deficit;
                if deficit > 0 {
                    self.add_stockable_active_buyer(person_idx, slot_idx);
                }
            }

            for stock_slot in 0..SUPERFICIAL_GOOD_SLOT_COUNT {
                if person.superficial_stock_units[stock_slot] == 0 {
                    self.add_superficial_zero_stock(person_idx, stock_slot);
                }
            }
        }
    }

    pub(super) fn fill_stockable_demands(
        &self,
        good_id: GoodId,
        day_in_month: u64,
        out: &mut Vec<BuyerDemand>,
    ) {
        let Some(slot_idx) = stockable_essential_slot(good_id) else {
            return;
        };
        for &person_idx in &self.stockable_active_buyers[slot_idx] {
            let initial_units = self
                .stockable_initial_deficits
                .get(person_idx)
                .map(|deficits| deficits[slot_idx])
                .unwrap_or(0);
            if initial_units == 0 {
                continue;
            }
            let remaining_units = self
                .stockable_deficits
                .get(person_idx)
                .map(|deficits| deficits[slot_idx])
                .unwrap_or(0);
            let purchased_units = initial_units.saturating_sub(remaining_units);
            let release_phase = stockable_release_phase(person_idx, slot_idx);
            let released_units = ((u64::from(initial_units) * day_in_month + release_phase)
                / DAYS_PER_MONTH)
                .min(u64::from(initial_units)) as u32;
            let units = released_units.saturating_sub(purchased_units);
            if units == 0 {
                continue;
            }
            out.push(BuyerDemand { person_idx, units });
        }
        out.sort_unstable_by_key(|demand| demand.person_idx);
    }

    pub(super) fn add_stockable_active_buyer(&mut self, person_idx: usize, slot_idx: usize) {
        if person_idx >= self.person_count {
            return;
        }
        if self.stockable_active_positions[slot_idx][person_idx] != usize::MAX {
            return;
        }
        self.stockable_active_positions[slot_idx][person_idx] =
            self.stockable_active_buyers[slot_idx].len();
        self.stockable_active_buyers[slot_idx].push(person_idx);
    }

    pub(super) fn remove_stockable_active_buyer(&mut self, person_idx: usize, slot_idx: usize) {
        if person_idx >= self.person_count {
            return;
        }
        let remove_idx = self.stockable_active_positions[slot_idx][person_idx];
        if remove_idx == usize::MAX {
            return;
        }
        let Some(swapped_person_idx) = self.stockable_active_buyers[slot_idx].pop() else {
            return;
        };
        if remove_idx < self.stockable_active_buyers[slot_idx].len() {
            self.stockable_active_buyers[slot_idx][remove_idx] = swapped_person_idx;
            self.stockable_active_positions[slot_idx][swapped_person_idx] = remove_idx;
        }
        self.stockable_active_positions[slot_idx][person_idx] = usize::MAX;
    }

    pub(super) fn add_superficial_zero_stock(&mut self, person_idx: usize, stock_slot: usize) {
        if person_idx >= self.person_count || stock_slot >= SUPERFICIAL_GOOD_SLOT_COUNT {
            return;
        }
        if self.superficial_zero_stock_positions[stock_slot][person_idx] != usize::MAX {
            return;
        }
        self.superficial_zero_stock_positions[stock_slot][person_idx] =
            self.superficial_zero_stock_buyers[stock_slot].len();
        self.superficial_zero_stock_buyers[stock_slot].push(person_idx);
    }

    pub(super) fn remove_superficial_zero_stock(&mut self, person_idx: usize, stock_slot: usize) {
        if person_idx >= self.person_count || stock_slot >= SUPERFICIAL_GOOD_SLOT_COUNT {
            return;
        }
        let remove_idx = self.superficial_zero_stock_positions[stock_slot][person_idx];
        if remove_idx == usize::MAX {
            return;
        }
        let Some(swapped_person_idx) = self.superficial_zero_stock_buyers[stock_slot].pop() else {
            return;
        };
        if remove_idx < self.superficial_zero_stock_buyers[stock_slot].len() {
            self.superficial_zero_stock_buyers[stock_slot][remove_idx] = swapped_person_idx;
            self.superficial_zero_stock_positions[stock_slot][swapped_person_idx] = remove_idx;
        }
        self.superficial_zero_stock_positions[stock_slot][person_idx] = usize::MAX;
    }
}

pub(super) fn stock_target_units(
    country: &Country,
    good_id: GoodId,
    household_size: usize,
) -> f32 {
    let essentials = &country.sim_params.essentials;
    monthly_requirement_for_good(essentials, good_id, household_size)
        + target_buffer_units(essentials, good_id, household_size)
}

fn stockable_release_phase(person_idx: usize, slot_idx: usize) -> u64 {
    hash64(((person_idx as u64) << 8) ^ slot_idx as u64) % DAYS_PER_MONTH
}
