use super::World;
use crate::sim::{
    good::{GOODS_COUNT, GoodId, common_goods},
    money::{from_cents, round_cents, to_cents},
    person::SUPERFICIAL_GOOD_SLOT_COUNT,
};
use rand::Rng;
use rayon::prelude::*;
use std::{cell::RefCell, sync::Arc};

const PEOPLE_CHUNK_SIZE: usize = 2_048;
const ESSENTIAL_GOODS_COUNT: u8 = 4;
const DYNAMIC_TARIFF_INCREASE_STEP: f64 = 0.05;
const MAX_DYNAMIC_TARIFF_RATE: f64 = 0.75;
const SUPERFICIAL_GOOD_START_ID: GoodId = 10;
const SUPERFICIAL_GOOD_IDS: [GoodId; SUPERFICIAL_GOOD_SLOT_COUNT] = [10, 11, 12, 13, 14];

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SellerQuote {
    seller_country_idx: usize,
    seller_company_idx: usize,
    base_price_cents: i64,
    tariff_cents: i64,
    landed_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SupplierRef {
    seller_country_idx: usize,
    seller_company_idx: usize,
    base_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct QuoteTemplate {
    seller_country_idx: usize,
    seller_company_idx: usize,
    base_price_cents: i64,
    tariff_cents: i64,
    landed_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct BuyerSlot {
    round: u64,
    person_idx: usize,
    buyer_country_idx: usize,
    country_rr_order: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct PlannedIntent {
    buyer_country_idx: usize,
    person_idx: usize,
    pending_units: u32,
    quote: SellerQuote,
    essential: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum IntentDecision {
    None,
    Unmet {
        buyer_country_idx: usize,
        person_idx: usize,
        pending_units: u32,
    },
    Candidate(PlannedIntent),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ApplyPlannedIntentOutcome {
    Bought,
    InventoryUnavailable,
    Unmet,
}

#[derive(Debug, Default)]
struct PlanningScratch {
    buyer_slots: Vec<BuyerSlot>,
    decisions: Vec<IntentDecision>,
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
struct TariffRateLookupCache {
    world_ptr: usize,
    month_stamp: u64,
    country_count: usize,
    rates_by_good: Arc<Vec<Vec<f64>>>,
}

#[derive(Debug, Default)]
struct RoundRobinScratch {
    pending_demands: Vec<Vec<u32>>,
    needs_met: Vec<Vec<u8>>,
    needs_unmet: Vec<Vec<u8>>,
    max_people: usize,
    planning: PlanningScratch,
}

thread_local! {
    static TARIFF_RATE_LOOKUP_CACHE: RefCell<Option<TariffRateLookupCache>> = const { RefCell::new(None) };
}

impl RoundRobinScratch {
    fn prepare_for_world(&mut self, countries: &[crate::sim::country::Country]) {
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

