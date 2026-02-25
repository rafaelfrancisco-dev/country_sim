use super::World;
use crate::sim::{
    good::{GOODS_COUNT, GoodId, common_goods},
    money::{from_cents, round_cents, to_cents},
    person::SUPERFICIAL_GOOD_SLOT_COUNT,
};
use rand::Rng;
use rayon::prelude::*;
use std::{cell::RefCell, sync::Arc};

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

impl World {
    pub(super) fn run_daily_goods_economy(&mut self) {
        self.produce_goods_daily();
        self.consume_superficial_stock_daily();
        self.process_global_goods_market();
        self.process_discretionary_superficial_purchases();
        for country in &mut self.countries {
            country.recompute_aggregates();
        }
    }

    fn produce_goods_daily(&mut self) {
        for country in &mut self.countries {
            let market = country.sim_params.goods_market;
            let daily_effort_per_worker = market.worker_effort_per_month / 30.0;

            for company in &mut country.companies {
                let Some(good) = common_goods().get(company.produced_good_id as usize) else {
                    continue;
                };
                if good.effort_per_unit <= 0.0 {
                    continue;
                }

                company.production_effort_carry +=
                    company.manufacturing_employee_count as f64 * daily_effort_per_worker;
                let produced = (company.production_effort_carry / good.effort_per_unit)
                    .floor()
                    .max(0.0) as u32;
                if produced == 0 {
                    continue;
                }

                company.production_effort_carry -= produced as f64 * good.effort_per_unit;
                company.inventory_units = company.inventory_units.saturating_add(produced);
                company.last_month_units_produced =
                    company.last_month_units_produced.saturating_add(produced);
            }
        }
    }

    pub(super) fn process_global_goods_market(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        let country_count = self.countries.len();
        let cycle_seed = self
            .countries
            .first()
            .map_or(0, |country| country.month.saturating_add(1));
        let start_country_idx = (cycle_seed as usize) % country_count;

        let mut discretionary_budgets = self.initial_discretionary_budgets();
        let suppliers_by_good = self.build_suppliers_by_good();
        let tariff_rates_by_good = self.tariff_rates_by_good_for_month();
        let mut scratch = RoundRobinScratch::default();

        for good in common_goods() {
            let quote_templates_by_buyer = self.build_quote_templates_by_buyer(
                good.id,
                &suppliers_by_good[good.id as usize],
                &tariff_rates_by_good,
            );
            scratch.prepare_for_world(&self.countries);
            self.process_good_round_robin(
                good.id,
                good.necessity_rank,
                start_country_idx,
                &mut discretionary_budgets,
                &quote_templates_by_buyer,
                &mut scratch,
            );
        }
    }

    fn consume_superficial_stock_daily(&mut self) {
        for country in &mut self.countries {
            for person in &mut country.people {
                for (slot_idx, good_id) in SUPERFICIAL_GOOD_IDS.iter().copied().enumerate() {
                    let daily_usage = (common_goods()[good_id as usize].monthly_demand_factor
                        / 30.0)
                        .max(0.0) as f32;
                    if daily_usage <= 0.0 {
                        continue;
                    }

                    person.superficial_stock_consumption_carry[slot_idx] += daily_usage;
                    let consume_units =
                        person.superficial_stock_consumption_carry[slot_idx].floor() as u16;
                    if consume_units == 0 {
                        continue;
                    }

                    person.superficial_stock_consumption_carry[slot_idx] -=
                        f32::from(consume_units);
                    person.superficial_stock_units[slot_idx] =
                        person.superficial_stock_units[slot_idx].saturating_sub(consume_units);
                }
            }
        }
    }

    fn process_discretionary_superficial_purchases(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        let suppliers_by_good = self.build_suppliers_by_good();
        let tariff_rates_by_good = self.tariff_rates_by_good_for_month();
        let mut rng = rand::thread_rng();

        for good_id in SUPERFICIAL_GOOD_IDS {
            let quote_templates_by_buyer = self.build_quote_templates_by_buyer(
                good_id,
                &suppliers_by_good[good_id as usize],
                &tariff_rates_by_good,
            );
            let stock_slot = usize::from(good_id - SUPERFICIAL_GOOD_START_ID);

            for buyer_country_idx in 0..self.countries.len() {
                let person_count = self.countries[buyer_country_idx].people.len();
                for person_idx in 0..person_count {
                    let stock_units = self.countries[buyer_country_idx].people[person_idx]
                        .superficial_stock_units[stock_slot];
                    if stock_units > 0 {
                        continue;
                    }

                    let Some(quote) =
                        self.find_cheapest_supplier(&quote_templates_by_buyer, buyer_country_idx)
                    else {
                        continue;
                    };
                    if quote.landed_price_cents <= 0 {
                        continue;
                    }

                    let cash_cents = self.person_cash_cents(buyer_country_idx, person_idx);
                    if cash_cents < quote.landed_price_cents {
                        continue;
                    }

                    let affordability_ratio = cash_cents as f64 / quote.landed_price_cents as f64;
                    let probability = ((affordability_ratio - 2.0) / 40.0).clamp(0.0, 0.05);
                    if probability <= 0.0 || !rng.gen_bool(probability) {
                        continue;
                    }

                    self.execute_goods_purchase(buyer_country_idx, person_idx, quote);
                    if let Some(person) = self
                        .countries
                        .get_mut(buyer_country_idx)
                        .and_then(|country| country.people.get_mut(person_idx))
                    {
                        person.superficial_stock_units[stock_slot] =
                            person.superficial_stock_units[stock_slot].saturating_add(1);
                    }
                }
            }
        }
    }

    pub(super) fn adjust_dynamic_country_tariffs(&mut self) {
        for country in &mut self.countries {
            let mut exported_units_by_good = [0_u32; GOODS_COUNT];
            let mut inventory_by_good = [0_u32; GOODS_COUNT];
            let mut unmet_need_by_good = [false; GOODS_COUNT];

            for company in &country.companies {
                let good_idx = company.produced_good_id as usize;
                if good_idx >= GOODS_COUNT {
                    continue;
                }

                exported_units_by_good[good_idx] = exported_units_by_good[good_idx]
                    .saturating_add(company.last_month_export_units_sold);
                inventory_by_good[good_idx] =
                    inventory_by_good[good_idx].saturating_add(company.inventory_units);
            }

            for person in &country.people {
                for good_idx in 0..GOODS_COUNT {
                    let bit = 1_u16 << (good_idx as u8);
                    if person.last_month_unmet_needs_mask & bit != 0 {
                        unmet_need_by_good[good_idx] = true;
                    }
                }
            }

            for good_idx in 0..GOODS_COUNT {
                let exported = exported_units_by_good[good_idx] > 0;
                let depleted_domestic_stock = inventory_by_good[good_idx] == 0;
                let unmet_domestic_need = unmet_need_by_good[good_idx];

                if exported && depleted_domestic_stock && unmet_domestic_need {
                    country.export_tariffs_by_good[good_idx] =
                        (country.export_tariffs_by_good[good_idx] + DYNAMIC_TARIFF_INCREASE_STEP)
                            .min(MAX_DYNAMIC_TARIFF_RATE);
                }
            }
        }
        self.invalidate_tariff_rate_lookup_cache();
    }

    fn initial_discretionary_budgets(&self) -> Vec<Vec<i64>> {
        self.countries
            .par_iter()
            .map(|country| {
                let month = country.month.saturating_add(1);
                let country_id = country.id;
                let mut country_budgets = Vec::with_capacity(country.people.len());

                for person in &country.people {
                    let budget_seed = hash64(
                        (u64::from(country_id) << 48)
                            ^ (month << 24)
                            ^ u64::from(person.id)
                            ^ u64::from(person.name_seed),
                    );
                    let spend_percent = 5.0 + (budget_seed % 11) as f64;
                    country_budgets.push(to_cents(
                        person.cash_balance.max(0.0) * spend_percent / 100.0 / 30.0,
                    ));
                }

                country_budgets
            })
            .collect()
    }

    fn tariff_rates_by_good_for_month(&self) -> Arc<Vec<Vec<f64>>> {
        let world_ptr = self as *const Self as usize;
        let month_stamp = self.countries.first().map_or(0, |country| country.month);
        let country_count = self.countries.len();

        TARIFF_RATE_LOOKUP_CACHE.with(|cache| {
            let mut cached = cache.borrow_mut();
            let needs_refresh = cached.as_ref().is_none_or(|entry| {
                entry.world_ptr != world_ptr
                    || entry.month_stamp != month_stamp
                    || entry.country_count != country_count
            });

            if needs_refresh {
                *cached = Some(TariffRateLookupCache {
                    world_ptr,
                    month_stamp,
                    country_count,
                    rates_by_good: Arc::new(self.rebuild_tariff_rates_by_good()),
                });
            }

            cached
                .as_ref()
                .map(|entry| entry.rates_by_good.clone())
                .unwrap_or_else(|| Arc::new(vec![vec![0.0; country_count]; GOODS_COUNT]))
        })
    }

    fn rebuild_tariff_rates_by_good(&self) -> Vec<Vec<f64>> {
        let country_count = self.countries.len();
        let mut rates_by_good = vec![vec![0.0; country_count]; GOODS_COUNT];

        for (seller_country_idx, country) in self.countries.iter().enumerate() {
            for good_idx in 0..GOODS_COUNT {
                rates_by_good[good_idx][seller_country_idx] = country
                    .export_tariffs_by_good
                    .get(good_idx)
                    .copied()
                    .unwrap_or(0.0);
            }
        }

        rates_by_good
    }

    fn invalidate_tariff_rate_lookup_cache(&self) {
        let world_ptr = self as *const Self as usize;
        TARIFF_RATE_LOOKUP_CACHE.with(|cache| {
            let mut cached = cache.borrow_mut();
            if cached
                .as_ref()
                .is_some_and(|entry| entry.world_ptr == world_ptr)
            {
                *cached = None;
            }
        });
    }

    fn process_good_round_robin(
        &mut self,
        good_id: GoodId,
        necessity_rank: u8,
        start_country_idx: usize,
        discretionary_budgets: &mut [Vec<i64>],
        quote_templates_by_buyer: &[Vec<QuoteTemplate>],
        scratch: &mut RoundRobinScratch,
    ) {
        let country_count = self.countries.len();
        let monthly_demand_factor = common_goods()[good_id as usize].monthly_demand_factor;

        for (country_idx, country) in self.countries.iter().enumerate() {
            let country_id = country.id;
            let country_pending = &mut scratch.pending_demands[country_idx];
            for (person_idx, person) in country.people.iter().enumerate() {
                country_pending[person_idx] = demand_units_for_person_good(
                    self.day,
                    country_id,
                    person.id,
                    person.name_seed,
                    good_id,
                    monthly_demand_factor,
                );
            }
        }

        let essential = necessity_rank <= ESSENTIAL_GOODS_COUNT;
        let mut round = 0_u64;

        loop {
            scratch.planning.buyer_slots.clear();
            for person_idx in 0..scratch.max_people {
                for country_rr_order in 0..country_count {
                    let buyer_country_idx = (start_country_idx + country_rr_order) % country_count;
                    let country_pending = &scratch.pending_demands[buyer_country_idx];
                    if person_idx >= country_pending.len() || country_pending[person_idx] == 0 {
                        continue;
                    }
                    scratch.planning.buyer_slots.push(BuyerSlot {
                        // Deterministic total order key:
                        // (round, person_idx, country_rr_order)
                        round,
                        person_idx,
                        buyer_country_idx,
                        country_rr_order,
                    });
                }
            }
            if scratch.planning.buyer_slots.is_empty() {
                break;
            }

            scratch
                .planning
                .decisions
                .resize(scratch.planning.buyer_slots.len(), IntentDecision::None);

            let world_ref: &World = self;
            let pending_demands = &scratch.pending_demands;
            let buyer_slots = &scratch.planning.buyer_slots;
            scratch
                .planning
                .decisions
                .par_iter_mut()
                .zip(buyer_slots.par_iter())
                .for_each(|(decision, slot)| {
                    *decision = world_ref.plan_intent_for_slot(
                        slot,
                        pending_demands,
                        discretionary_budgets,
                        quote_templates_by_buyer,
                        essential,
                    );
                });

            let mut progress = false;
            let mut prev_key: Option<(u64, usize, usize)> = None;
            for decision_idx in 0..scratch.planning.decisions.len() {
                let slot = scratch.planning.buyer_slots[decision_idx];
                let key = (slot.round, slot.person_idx, slot.country_rr_order);
                debug_assert!(prev_key.is_none_or(|prev| prev <= key));
                prev_key = Some(key);

                match scratch.planning.decisions[decision_idx] {
                    IntentDecision::None => {}
                    IntentDecision::Unmet {
                        buyer_country_idx,
                        person_idx,
                        pending_units,
                    } => {
                        scratch.needs_unmet[buyer_country_idx][person_idx] = 1;
                        self.bump_unmet_need_events(buyer_country_idx, pending_units);
                        scratch.pending_demands[buyer_country_idx][person_idx] = 0;
                    }
                    IntentDecision::Candidate(intent) => {
                        match self.apply_planned_intent(intent, discretionary_budgets) {
                            ApplyPlannedIntentOutcome::Bought => {
                                let pending = &mut scratch.pending_demands
                                    [intent.buyer_country_idx][intent.person_idx];
                                *pending = pending.saturating_sub(1);
                                scratch.needs_met[intent.buyer_country_idx][intent.person_idx] = 1;
                                progress = true;
                            }
                            ApplyPlannedIntentOutcome::InventoryUnavailable => {
                                // Leave pending to be retried next round.
                            }
                            ApplyPlannedIntentOutcome::Unmet => {
                                scratch.needs_unmet[intent.buyer_country_idx][intent.person_idx] =
                                    1;
                                self.bump_unmet_need_events(
                                    intent.buyer_country_idx,
                                    intent.pending_units,
                                );
                                scratch.pending_demands[intent.buyer_country_idx]
                                    [intent.person_idx] = 0;
                            }
                        }
                    }
                }
            }

            if !progress {
                self.finalize_remaining_pending_as_unmet(
                    &mut scratch.pending_demands,
                    &mut scratch.needs_unmet,
                );
                break;
            }

            round = round.saturating_add(1);
        }

        for country_idx in 0..country_count {
            let person_count = scratch.pending_demands[country_idx].len();
            for person_idx in 0..person_count {
                let need_met = scratch.needs_met[country_idx][person_idx] != 0;
                let need_unmet = scratch.needs_unmet[country_idx][person_idx] != 0;
                self.mark_person_need_result(
                    country_idx,
                    person_idx,
                    good_id,
                    need_met,
                    need_unmet,
                );
            }
        }
    }

    fn plan_intent_for_slot(
        &self,
        slot: &BuyerSlot,
        pending_demands: &[Vec<u32>],
        discretionary_budgets: &[Vec<i64>],
        quote_templates_by_buyer: &[Vec<QuoteTemplate>],
        essential: bool,
    ) -> IntentDecision {
        let pending_units = pending_demands
            .get(slot.buyer_country_idx)
            .and_then(|country_pending| country_pending.get(slot.person_idx))
            .copied()
            .unwrap_or(0);
        if pending_units == 0 {
            return IntentDecision::None;
        }

        let Some(quote) =
            self.find_cheapest_supplier(quote_templates_by_buyer, slot.buyer_country_idx)
        else {
            return IntentDecision::Unmet {
                buyer_country_idx: slot.buyer_country_idx,
                person_idx: slot.person_idx,
                pending_units,
            };
        };

        let budget_cents = discretionary_budgets
            .get(slot.buyer_country_idx)
            .and_then(|country_budgets| country_budgets.get(slot.person_idx))
            .copied()
            .unwrap_or(0);
        let cash_cents = self.person_cash_cents(slot.buyer_country_idx, slot.person_idx);

        if !Self::can_afford_purchase(
            essential,
            budget_cents,
            cash_cents,
            quote.landed_price_cents,
        ) {
            return IntentDecision::Unmet {
                buyer_country_idx: slot.buyer_country_idx,
                person_idx: slot.person_idx,
                pending_units,
            };
        }

        IntentDecision::Candidate(PlannedIntent {
            buyer_country_idx: slot.buyer_country_idx,
            person_idx: slot.person_idx,
            pending_units,
            quote,
            essential,
        })
    }

    fn can_afford_purchase(
        essential: bool,
        discretionary_budget_cents: i64,
        buyer_cash_cents: i64,
        landed_price_cents: i64,
    ) -> bool {
        if essential {
            return true;
        }
        discretionary_budget_cents >= landed_price_cents && buyer_cash_cents >= landed_price_cents
    }

    fn apply_planned_intent(
        &mut self,
        intent: PlannedIntent,
        discretionary_budgets: &mut [Vec<i64>],
    ) -> ApplyPlannedIntentOutcome {
        if !self.seller_has_inventory(
            intent.quote.seller_country_idx,
            intent.quote.seller_company_idx,
        ) {
            return ApplyPlannedIntentOutcome::InventoryUnavailable;
        }

        let buyer_cash_cents = self.person_cash_cents(intent.buyer_country_idx, intent.person_idx);
        let Some(discretionary_budget_cents) = discretionary_budgets
            .get_mut(intent.buyer_country_idx)
            .and_then(|country_budgets| country_budgets.get_mut(intent.person_idx))
        else {
            return ApplyPlannedIntentOutcome::Unmet;
        };

        if !intent.essential {
            if *discretionary_budget_cents < intent.quote.landed_price_cents
                || buyer_cash_cents < intent.quote.landed_price_cents
            {
                return ApplyPlannedIntentOutcome::Unmet;
            }
            *discretionary_budget_cents =
                discretionary_budget_cents.saturating_sub(intent.quote.landed_price_cents);
        } else if buyer_cash_cents < intent.quote.landed_price_cents {
            let covered_by_cash = buyer_cash_cents.max(0);
            let debt_cents = intent
                .quote
                .landed_price_cents
                .saturating_sub(covered_by_cash);
            self.bump_essential_debt_spending(intent.buyer_country_idx, debt_cents);
        }

        self.execute_goods_purchase(intent.buyer_country_idx, intent.person_idx, intent.quote);
        ApplyPlannedIntentOutcome::Bought
    }

    fn finalize_remaining_pending_as_unmet(
        &mut self,
        pending_demands: &mut [Vec<u32>],
        needs_unmet: &mut [Vec<u8>],
    ) {
        for (country_idx, country_pending) in pending_demands.iter_mut().enumerate() {
            for person_idx in 0..country_pending.len() {
                let pending_units = country_pending[person_idx];
                if pending_units == 0 {
                    continue;
                }
                needs_unmet[country_idx][person_idx] = 1;
                self.bump_unmet_need_events(country_idx, pending_units);
                country_pending[person_idx] = 0;
            }
        }
    }

    fn find_cheapest_supplier(
        &self,
        quote_templates_by_buyer: &[Vec<QuoteTemplate>],
        buyer_country_idx: usize,
    ) -> Option<SellerQuote> {
        let buyer_templates = quote_templates_by_buyer.get(buyer_country_idx)?;
        for template in buyer_templates {
            let Some(country) = self.countries.get(template.seller_country_idx) else {
                continue;
            };
            let Some(company) = country.companies.get(template.seller_company_idx) else {
                continue;
            };
            if company.inventory_units == 0 {
                continue;
            }

            return Some(SellerQuote {
                seller_country_idx: template.seller_country_idx,
                seller_company_idx: template.seller_company_idx,
                base_price_cents: template.base_price_cents,
                tariff_cents: template.tariff_cents,
                landed_price_cents: template.landed_price_cents,
            });
        }
        None
    }

    fn seller_has_inventory(&self, seller_country_idx: usize, seller_company_idx: usize) -> bool {
        self.countries
            .get(seller_country_idx)
            .and_then(|country| country.companies.get(seller_company_idx))
            .is_some_and(|company| company.inventory_units > 0)
    }

    fn build_suppliers_by_good(&self) -> Vec<Vec<SupplierRef>> {
        let mut suppliers_by_good = vec![Vec::new(); GOODS_COUNT];

        for (seller_country_idx, country) in self.countries.iter().enumerate() {
            for (seller_company_idx, company) in country.companies.iter().enumerate() {
                let good_idx = company.produced_good_id as usize;
                if good_idx >= GOODS_COUNT {
                    continue;
                }

                let base_price_cents = to_cents(company.unit_price);
                if base_price_cents <= 0 {
                    continue;
                }

                suppliers_by_good[good_idx].push(SupplierRef {
                    seller_country_idx,
                    seller_company_idx,
                    base_price_cents,
                });
            }
        }

        suppliers_by_good
    }

    fn build_quote_templates_by_buyer(
        &self,
        good_id: GoodId,
        supplier_refs: &[SupplierRef],
        tariff_rates_by_good: &[Vec<f64>],
    ) -> Vec<Vec<QuoteTemplate>> {
        let country_count = self.countries.len();
        let tariff_rates_by_seller = tariff_rates_by_good
            .get(good_id as usize)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        (0..country_count)
            .map(|buyer_country_idx| {
                let mut templates = supplier_refs
                    .iter()
                    .map(|supplier| {
                        let tariff_cents = if supplier.seller_country_idx == buyer_country_idx {
                            0
                        } else {
                            let tariff_rate = tariff_rates_by_seller
                                .get(supplier.seller_country_idx)
                                .copied()
                                .unwrap_or(0.0);
                            to_cents(from_cents(supplier.base_price_cents) * tariff_rate)
                        };
                        let landed_price_cents =
                            supplier.base_price_cents.saturating_add(tariff_cents);

                        QuoteTemplate {
                            seller_country_idx: supplier.seller_country_idx,
                            seller_company_idx: supplier.seller_company_idx,
                            base_price_cents: supplier.base_price_cents,
                            tariff_cents,
                            landed_price_cents,
                        }
                    })
                    .collect::<Vec<_>>();

                templates.sort_by_key(|template| {
                    (
                        template.landed_price_cents,
                        template.base_price_cents,
                        template.seller_country_idx,
                        template.seller_company_idx,
                    )
                });
                templates
            })
            .collect()
    }

    fn execute_goods_purchase(
        &mut self,
        buyer_country_idx: usize,
        person_idx: usize,
        quote: SellerQuote,
    ) {
        if buyer_country_idx == quote.seller_country_idx {
            let Some(country) = self.countries.get_mut(buyer_country_idx) else {
                return;
            };
            Self::apply_purchase_to_buyer(country, person_idx, quote.landed_price_cents, false);
            Self::apply_sale_to_seller(
                country,
                quote.seller_company_idx,
                quote.base_price_cents,
                true,
            );
            return;
        }

        if buyer_country_idx < quote.seller_country_idx {
            let (left, right) = self.countries.split_at_mut(quote.seller_country_idx);
            let buyer_country = &mut left[buyer_country_idx];
            let seller_country = &mut right[0];
            Self::apply_purchase_to_buyer(
                buyer_country,
                person_idx,
                quote.landed_price_cents,
                true,
            );
            Self::apply_sale_to_seller(
                seller_country,
                quote.seller_company_idx,
                quote.base_price_cents,
                false,
            );
            Self::apply_tariff_to_exporter(seller_country, quote.tariff_cents);
        } else {
            let (left, right) = self.countries.split_at_mut(buyer_country_idx);
            let seller_country = &mut left[quote.seller_country_idx];
            let buyer_country = &mut right[0];
            Self::apply_purchase_to_buyer(
                buyer_country,
                person_idx,
                quote.landed_price_cents,
                true,
            );
            Self::apply_sale_to_seller(
                seller_country,
                quote.seller_company_idx,
                quote.base_price_cents,
                false,
            );
            Self::apply_tariff_to_exporter(seller_country, quote.tariff_cents);
        }
    }

    fn apply_purchase_to_buyer(
        country: &mut crate::sim::country::Country,
        person_idx: usize,
        landed_price_cents: i64,
        imported: bool,
    ) {
        let amount = from_cents(landed_price_cents);

        if let Some(person) = country.people.get_mut(person_idx) {
            person.cash_balance = round_cents(person.cash_balance - amount);
            person.last_month_spending = round_cents(person.last_month_spending + amount);
            person.last_month_goods_spending =
                round_cents(person.last_month_goods_spending + amount);
            if imported {
                person.last_month_imported_goods_spending =
                    round_cents(person.last_month_imported_goods_spending + amount);
            }
            person.last_month_goods_units_bought =
                person.last_month_goods_units_bought.saturating_add(1);
        }

        country.monthly_household_spending =
            round_cents(country.monthly_household_spending + amount);
        if imported {
            country.monthly_goods_import_spending =
                round_cents(country.monthly_goods_import_spending + amount);
        }
    }

    fn apply_sale_to_seller(
        country: &mut crate::sim::country::Country,
        company_idx: usize,
        base_price_cents: i64,
        domestic: bool,
    ) {
        let amount = from_cents(base_price_cents);

        if let Some(company) = country.companies.get_mut(company_idx) {
            company.inventory_units = company.inventory_units.saturating_sub(1);
            company.cash_balance = round_cents(company.cash_balance + amount);
            company.last_month_sales = round_cents(company.last_month_sales + amount);
            company.last_month_units_sold = company.last_month_units_sold.saturating_add(1);
            if domestic {
                company.last_month_domestic_units_sold =
                    company.last_month_domestic_units_sold.saturating_add(1);
            } else {
                company.last_month_export_units_sold =
                    company.last_month_export_units_sold.saturating_add(1);
            }
        }

        if !domestic {
            country.monthly_goods_export_revenue =
                round_cents(country.monthly_goods_export_revenue + amount);
        }
    }

    fn apply_tariff_to_exporter(country: &mut crate::sim::country::Country, tariff_cents: i64) {
        if tariff_cents <= 0 {
            return;
        }

        let amount = from_cents(tariff_cents);
        country.monthly_tariff_collected = round_cents(country.monthly_tariff_collected + amount);
        country.treasury_balance = round_cents(country.treasury_balance + amount);
    }

    fn person_cash_cents(&self, buyer_country_idx: usize, person_idx: usize) -> i64 {
        self.countries
            .get(buyer_country_idx)
            .and_then(|country| country.people.get(person_idx))
            .map_or(0, |person| to_cents(person.cash_balance))
    }

    fn mark_person_need_result(
        &mut self,
        buyer_country_idx: usize,
        person_idx: usize,
        good_id: GoodId,
        need_met: bool,
        need_unmet: bool,
    ) {
        let bit = 1_u16 << good_id;

        let Some(country) = self.countries.get_mut(buyer_country_idx) else {
            return;
        };
        let Some(person) = country.people.get_mut(person_idx) else {
            return;
        };

        if need_met {
            person.last_month_needs_met_mask |= bit;
        }
        if need_unmet {
            person.last_month_unmet_needs_mask |= bit;
        }
    }

    fn bump_unmet_need_events(&mut self, buyer_country_idx: usize, count: u32) {
        if let Some(country) = self.countries.get_mut(buyer_country_idx) {
            country.monthly_unmet_need_events = country
                .monthly_unmet_need_events
                .saturating_add(u64::from(count));
        }
    }

    fn bump_essential_debt_spending(&mut self, buyer_country_idx: usize, debt_cents: i64) {
        if debt_cents <= 0 {
            return;
        }

        if let Some(country) = self.countries.get_mut(buyer_country_idx) {
            country.monthly_essential_debt_spending =
                round_cents(country.monthly_essential_debt_spending + from_cents(debt_cents));
        }
    }
}

fn demand_units_for_person_good(
    day: u64,
    country_id: u16,
    person_id: u32,
    name_seed: u32,
    good_id: GoodId,
    monthly_demand_factor: f64,
) -> u32 {
    let daily_demand_factor = (monthly_demand_factor / 30.0).max(0.0);
    let integer_part = daily_demand_factor.floor().max(0.0) as u32;
    let fractional_part = (daily_demand_factor - f64::from(integer_part)).clamp(0.0, 1.0);

    if fractional_part <= 0.0 {
        return integer_part;
    }

    let seed = hash64(
        (day << 40)
            ^ (u64::from(country_id) << 24)
            ^ (u64::from(person_id) << 8)
            ^ u64::from(good_id)
            ^ u64::from(name_seed),
    );

    integer_part + u32::from(unit_interval(seed) < fractional_part)
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}
