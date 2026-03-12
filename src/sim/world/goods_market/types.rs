use crate::sim::{good::GoodId, person::SUPERFICIAL_GOOD_SLOT_COUNT};

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
    /// Export tariff — revenue goes to the seller's treasury.
    pub(in crate::sim::world) tariff_cents: i64,
    /// Import tariff — revenue goes to the buyer's treasury.
    pub(in crate::sim::world) import_tariff_cents: i64,
    pub(in crate::sim::world) landed_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) enum GoodsBuyer {
    Person {
        buyer_country_idx: usize,
        person_idx: usize,
    },
    Company {
        buyer_country_idx: usize,
        company_idx: usize,
        input_good_id: GoodId,
    },
}

impl GoodsBuyer {
    pub(in crate::sim::world) fn buyer_country_idx(self) -> usize {
        match self {
            Self::Person {
                buyer_country_idx, ..
            }
            | Self::Company {
                buyer_country_idx, ..
            } => buyer_country_idx,
        }
    }
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
    /// Export tariff — revenue goes to the seller's treasury.
    pub(in crate::sim::world) tariff_cents: i64,
    /// Import tariff — revenue goes to the buyer's treasury.
    pub(in crate::sim::world) import_tariff_cents: i64,
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
pub(in crate::sim::world) struct BuyerDemand {
    pub(in crate::sim::world) person_idx: usize,
    pub(in crate::sim::world) units: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(in crate::sim::world) struct PlannedIntent {
    pub(in crate::sim::world) buyer_country_idx: usize,
    pub(in crate::sim::world) person_idx: usize,
    pub(in crate::sim::world) pending_units: u32,
    pub(in crate::sim::world) quote: SellerQuote,
    pub(in crate::sim::world) essential: bool,
    /// Buyer's cash at planning time; used in parallel apply to compute essential debt
    /// without touching buyer country data.
    pub(in crate::sim::world) buyer_cash_cents: i64,
}

/// Buyer-side info collected during parallel apply, applied in the serial merge.
#[derive(Debug, Copy, Clone)]
pub(in crate::sim::world) struct BuyerApplyInfo {
    pub(in crate::sim::world) buyer_country_idx: usize,
    pub(in crate::sim::world) person_idx: usize,
    pub(in crate::sim::world) landed_price_cents: i64,
    pub(in crate::sim::world) imported: bool,
    /// 0 if not essential or cash was sufficient.
    pub(in crate::sim::world) essential_debt_cents: i64,
    /// true for non-essential buys: discretionary budget is deducted in the merge.
    pub(in crate::sim::world) deduct_discretionary: bool,
}

/// Per-decision outcome from the parallel apply phase.
#[derive(Debug, Copy, Clone)]
pub(in crate::sim::world) enum ParallelApplyOutcome {
    Bought(BuyerApplyInfo),
    InventoryUnavailable,
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

#[derive(Debug, Default, Clone, PartialEq)]
pub(in crate::sim::world) struct PlanningScratch {
    pub(in crate::sim::world) buyer_slots: Vec<BuyerSlot>,
    pub(in crate::sim::world) decisions: Vec<IntentDecision>,
    pub(in crate::sim::world) cheapest_quotes: Vec<Option<SellerQuote>>,
}

impl PlanningScratch {
    fn prepare_capacity(&mut self, max_slots: usize, country_count: usize) {
        if self.buyer_slots.capacity() < max_slots {
            self.buyer_slots
                .reserve(max_slots.saturating_sub(self.buyer_slots.capacity()));
        }
        if self.decisions.capacity() < max_slots {
            self.decisions
                .reserve(max_slots.saturating_sub(self.decisions.capacity()));
        }
        if self.cheapest_quotes.capacity() < country_count {
            self.cheapest_quotes
                .reserve(country_count.saturating_sub(self.cheapest_quotes.capacity()));
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(in crate::sim::world) struct RoundRobinScratch {
    pub(in crate::sim::world) pending_demands: Vec<Vec<u32>>,
    pub(in crate::sim::world) active_indices: Vec<Vec<usize>>,
    pub(in crate::sim::world) needs_met: Vec<Vec<u8>>,
    pub(in crate::sim::world) needs_unmet: Vec<Vec<u8>>,
    /// Count of (country, person) pairs with non-zero pending demand for the current good.
    /// Used to skip the buyer-slot scan when all demands are already satisfied.
    pub(in crate::sim::world) pending_non_zero: usize,
    pub(in crate::sim::world) planning: PlanningScratch,
}

impl RoundRobinScratch {
    pub(in crate::sim::world) fn prepare_for_good(
        &mut self,
        countries: &[crate::sim::country::Country],
        active_demands_by_country: &[Vec<BuyerDemand>],
    ) {
        let country_count = countries.len();
        self.pending_demands.resize_with(country_count, Vec::new);
        self.active_indices.resize_with(country_count, Vec::new);
        self.needs_met.resize_with(country_count, Vec::new);
        self.needs_unmet.resize_with(country_count, Vec::new);

        for (country_idx, country) in countries.iter().enumerate() {
            let person_count = country.people.len();

            let pending = &mut self.pending_demands[country_idx];
            if pending.capacity() < person_count {
                pending.reserve(person_count - pending.capacity());
            }
            pending.resize(person_count, 0);

            let active_indices = &mut self.active_indices[country_idx];
            for &person_idx in active_indices.iter() {
                if let Some(value) = pending.get_mut(person_idx) {
                    *value = 0;
                }
            }
            active_indices.clear();

            let needs_met = &mut self.needs_met[country_idx];
            if needs_met.capacity() < person_count {
                needs_met.reserve(person_count - needs_met.capacity());
            }
            needs_met.resize(person_count, 0);

            let needs_unmet = &mut self.needs_unmet[country_idx];
            if needs_unmet.capacity() < person_count {
                needs_unmet.reserve(person_count - needs_unmet.capacity());
            }
            needs_unmet.resize(person_count, 0);

            for demand in active_demands_by_country
                .get(country_idx)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                if demand.units == 0 || demand.person_idx >= person_count {
                    continue;
                }
                pending[demand.person_idx] = demand.units;
                needs_met[demand.person_idx] = 0;
                needs_unmet[demand.person_idx] = 0;
                active_indices.push(demand.person_idx);
            }
        }
        self.pending_non_zero = 0;
        for active_indices in &self.active_indices {
            self.pending_non_zero = self.pending_non_zero.saturating_add(active_indices.len());
        }
        let max_slots = self
            .active_indices
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            .max(country_count);
        self.planning.prepare_capacity(max_slots, country_count);
    }
}
