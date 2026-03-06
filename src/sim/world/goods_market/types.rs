use crate::sim::{good::GoodId, person::SUPERFICIAL_GOOD_SLOT_COUNT};
use std::{cell::RefCell, sync::Arc};

pub(in crate::sim::world) const PEOPLE_CHUNK_SIZE: usize = 2_048;
pub(in crate::sim::world) const DYNAMIC_TARIFF_INCREASE_STEP: f64 = 0.05;
pub(in crate::sim::world) const DYNAMIC_TARIFF_DECREASE_STEP: f64 = 0.03;
pub(in crate::sim::world) const MAX_DYNAMIC_TARIFF_RATE: f64 = 0.75;
pub(in crate::sim::world) const SUPERFICIAL_GOOD_START_ID: GoodId = 10;
pub(in crate::sim::world) const SUPERFICIAL_GOOD_IDS: [GoodId; SUPERFICIAL_GOOD_SLOT_COUNT] =
    [10, 11, 12, 13, 14];

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) struct SellerQuote {
    pub(in crate::sim::world) seller_country_idx: usize,
    pub(in crate::sim::world) seller_company_idx: usize,
    pub(in crate::sim::world) base_price_cents: i64,
    pub(in crate::sim::world) tariff_cents: i64,
    pub(in crate::sim::world) landed_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) struct SupplierRef {
    pub(in crate::sim::world) seller_country_idx: usize,
    pub(in crate::sim::world) seller_company_idx: usize,
    pub(in crate::sim::world) base_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) struct QuoteTemplate {
    pub(in crate::sim::world) seller_country_idx: usize,
    pub(in crate::sim::world) seller_company_idx: usize,
    pub(in crate::sim::world) base_price_cents: i64,
    pub(in crate::sim::world) tariff_cents: i64,
    pub(in crate::sim::world) landed_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) struct BuyerSlot {
    pub(in crate::sim::world) round: u64,
    pub(in crate::sim::world) person_idx: usize,
    pub(in crate::sim::world) buyer_country_idx: usize,
    pub(in crate::sim::world) country_rr_order: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) struct PlannedIntent {
    pub(in crate::sim::world) buyer_country_idx: usize,
    pub(in crate::sim::world) person_idx: usize,
    pub(in crate::sim::world) pending_units: u32,
    pub(in crate::sim::world) quote: SellerQuote,
    pub(in crate::sim::world) essential: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) enum IntentDecision {
    None,
    Unmet {
        buyer_country_idx: usize,
        person_idx: usize,
        pending_units: u32,
    },
    Candidate(PlannedIntent),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) enum ApplyPlannedIntentOutcome {
    Bought,
    InventoryUnavailable,
    Unmet,
}

#[derive(Debug, Default)]
pub(in crate::sim::world) struct PlanningScratch {
    pub(in crate::sim::world) buyer_slots: Vec<BuyerSlot>,
    pub(in crate::sim::world) decisions: Vec<IntentDecision>,
}

impl PlanningScratch {
    fn prepare_capacity(&mut self, max_people: usize, country_count: usize) {
        let max_slots = max_people.saturating_mul(country_count);
        if self.buyer_slots.capacity() < max_slots {
            self.buyer_slots
                .reserve(max_slots.saturating_sub(self.buyer_slots.capacity()));
        }
        if self.decisions.capacity() < max_slots {
            self.decisions
                .reserve(max_slots.saturating_sub(self.decisions.capacity()));
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::sim::world) struct TariffRateLookupCache {
    pub(in crate::sim::world) world_ptr: usize,
    pub(in crate::sim::world) month_stamp: u64,
    pub(in crate::sim::world) country_count: usize,
    pub(in crate::sim::world) rates_by_good: Arc<Vec<Vec<f64>>>,
}

#[derive(Debug, Default)]
pub(in crate::sim::world) struct RoundRobinScratch {
    pub(in crate::sim::world) pending_demands: Vec<Vec<u32>>,
    pub(in crate::sim::world) needs_met: Vec<Vec<u8>>,
    pub(in crate::sim::world) needs_unmet: Vec<Vec<u8>>,
    pub(in crate::sim::world) max_people: usize,
    pub(in crate::sim::world) planning: PlanningScratch,
}

thread_local! {
    pub(in crate::sim::world) static TARIFF_RATE_LOOKUP_CACHE: RefCell<Option<TariffRateLookupCache>> = const { RefCell::new(None) };
}

impl RoundRobinScratch {
    pub(in crate::sim::world) fn prepare_for_world(
        &mut self,
        countries: &[crate::sim::country::Country],
    ) {
        let country_count = countries.len();
        self.pending_demands.resize_with(country_count, Vec::new);
        self.needs_met.resize_with(country_count, Vec::new);
        self.needs_unmet.resize_with(country_count, Vec::new);

        self.max_people = 0;
        for (country_idx, country) in countries.iter().enumerate() {
            let person_count = country.people.len();
            self.max_people = self.max_people.max(person_count);

            let pending = &mut self.pending_demands[country_idx];
            if pending.capacity() < person_count {
                pending.reserve(person_count - pending.capacity());
            }
            pending.resize(person_count, 0);

            let needs_met = &mut self.needs_met[country_idx];
            if needs_met.capacity() < person_count {
                needs_met.reserve(person_count - needs_met.capacity());
            }
            needs_met.resize(person_count, 0);
            needs_met.fill(0);

            let needs_unmet = &mut self.needs_unmet[country_idx];
            if needs_unmet.capacity() < person_count {
                needs_unmet.reserve(person_count - needs_unmet.capacity());
            }
            needs_unmet.resize(person_count, 0);
            needs_unmet.fill(0);
        }
        self.planning
            .prepare_capacity(self.max_people, country_count);
    }
}
