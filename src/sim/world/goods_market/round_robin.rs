use super::*;
use crate::sim::essentials::{
    is_essential_good, is_stockable_essential_good, stockable_essential_slot,
};

/// Wraps a raw pointer so it can cross rayon task boundaries.
/// Safety is upheld by the caller through disjoint-partition invariants.
struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}
impl<T> Copy for SendPtr<T> {}
impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> SendPtr<T> {
    /// Returns the inner raw pointer.
    ///
    /// Accessed via a method so closures capture the whole `SendPtr` (which is `Send + Sync`)
    /// rather than the inner `*mut T` field directly (which is neither `Send` nor `Sync`).
    #[inline(always)]
    fn get(self) -> *mut T {
        self.0
    }
}

impl World {
    pub(in crate::sim::world) fn process_good_round_robin(
        &mut self,
        good_id: GoodId,
        necessity_rank: u8,
        start_country_idx: usize,
        discretionary_budgets: &mut [Vec<i64>],
        day_cache: &mut GoodsMarketDayCache,
    ) {
        let country_count = self.countries.len();
        day_cache.prepare_round_robin_for_good(&self.countries, good_id);

        let essential = is_essential_good(good_id) || necessity_rank <= 4;
        let deferred_unmet = is_stockable_essential_good(good_id);
        let mut round = 0_u64;
        let needs_mask_bit = 1_u16 << good_id;

        loop {
            {
                let scratch = &mut day_cache.round_robin;
                if scratch.pending_non_zero == 0 {
                    break;
                }
                if round == 0 {
                    scratch.planning.buyer_slots.clear();
                    for country_rr_order in 0..country_count {
                        let buyer_country_idx =
                            (start_country_idx + country_rr_order) % country_count;
                        for &person_idx in &scratch.active_indices[buyer_country_idx] {
                            if scratch.pending_demands[buyer_country_idx][person_idx] == 0 {
                                continue;
                            }
                            scratch.planning.buyer_slots.push(BuyerSlot {
                                round,
                                person_idx,
                                buyer_country_idx,
                                country_rr_order,
                            });
                        }
                    }
                    scratch
                        .planning
                        .buyer_slots
                        .sort_unstable_by_key(|slot| (slot.person_idx, slot.country_rr_order));
                } else {
                    let pending = &scratch.pending_demands;
                    scratch.planning.buyer_slots.retain(|slot| {
                        pending
                            .get(slot.buyer_country_idx)
                            .and_then(|v| v.get(slot.person_idx))
                            .copied()
                            .unwrap_or(0)
                            > 0
                    });
                    // Update round field on retained slots.
                    for slot in &mut scratch.planning.buyer_slots {
                        slot.round = round;
                    }
                }
            }
            if day_cache.round_robin.planning.buyer_slots.is_empty() {
                break;
            }

            let mut cheapest_quotes = Vec::with_capacity(country_count);
            for buyer_country_idx in 0..country_count {
                cheapest_quotes.push(day_cache.current_cheapest_quote(
                    self,
                    good_id,
                    buyer_country_idx,
                ));
            }

            {
                let scratch = &mut day_cache.round_robin;
                scratch
                    .planning
                    .decisions
                    .resize(scratch.planning.buyer_slots.len(), IntentDecision::None);
                scratch.planning.cheapest_quotes.clear();
                scratch.planning.cheapest_quotes.extend(cheapest_quotes);

                let pending_demands = &scratch.pending_demands;
                let buyer_slots = &scratch.planning.buyer_slots;
                let cheapest_quotes = &scratch.planning.cheapest_quotes;
                scratch
                    .planning
                    .decisions
                    .par_iter_mut()
                    .zip(buyer_slots.par_iter())
                    .for_each(|(decision, slot)| {
                        *decision = self.plan_intent_for_slot(
                            slot,
                            pending_demands,
                            discretionary_budgets,
                            cheapest_quotes,
                            essential,
                        );
                    });
            }

            // ---- Step A: Partition decisions by seller_country_idx ----
            let country_count = self.countries.len();
            let mut partitions: Vec<Vec<usize>> = vec![Vec::new(); country_count];
            {
                let decisions = &day_cache.round_robin.planning.decisions;
                for (idx, decision) in decisions.iter().enumerate() {
                    if let IntentDecision::Candidate(intent) = decision {
                        partitions[intent.quote.seller_country_idx].push(idx);
                    }
                }
            }

            // ---- Step B: Parallel apply — seller-side mutations only ----
            // SAFETY:
            //   1. `self.countries` is not resized or moved during the rayon scope.
            //   2. Each rayon task receives a distinct `seller_country_idx` (partition invariant).
            //   3. No task reads or writes buyer countries — all buyer info is in
            //      PlannedIntent::buyer_cash_cents, captured at planning time.
            //   4. Within each partition the inner loop is sequential, so inventory
            //      decrements within one seller country are race-free.
            //   5. The serial merge runs after all rayon tasks complete with exclusive &mut self.
            let countries_ptr = SendPtr(self.countries.as_mut_ptr());
            let decisions_ptr =
                SendPtr(day_cache.round_robin.planning.decisions.as_ptr() as *mut IntentDecision);
            let decisions_len = day_cache.round_robin.planning.decisions.len();

            let partition_outcomes: Vec<Vec<(usize, ParallelApplyOutcome)>> = partitions
                .par_iter()
                .enumerate()
                .filter(|(_, group)| !group.is_empty())
                .map(move |(seller_country_idx, group)| {
                    let seller_country: &mut crate::sim::country::Country =
                        unsafe { &mut *countries_ptr.get().add(seller_country_idx) };
                    let decisions: &[IntentDecision] = unsafe {
                        std::slice::from_raw_parts(
                            decisions_ptr.get() as *const IntentDecision,
                            decisions_len,
                        )
                    };
                    let mut results = Vec::with_capacity(group.len());
                    for &decision_idx in group {
                        let IntentDecision::Candidate(intent) = decisions[decision_idx] else {
                            continue;
                        };
                        let outcome = if seller_country
                            .companies
                            .get(intent.quote.seller_company_idx)
                            .is_none_or(|c| c.inventory_units == 0)
                        {
                            ParallelApplyOutcome::InventoryUnavailable
                        } else {
                            World::apply_sale_to_seller(
                                seller_country,
                                intent.quote.seller_company_idx,
                                intent.quote.base_price_cents,
                                intent.buyer_country_idx == seller_country_idx,
                            );
                            World::apply_tariff_to_exporter(
                                seller_country,
                                intent.quote.tariff_cents,
                            );
                            let essential_debt_cents = if intent.essential
                                && intent.buyer_cash_cents < intent.quote.landed_price_cents
                            {
                                intent
                                    .quote
                                    .landed_price_cents
                                    .saturating_sub(intent.buyer_cash_cents.max(0))
                            } else {
                                0
                            };
                            ParallelApplyOutcome::Bought(BuyerApplyInfo {
                                buyer_country_idx: intent.buyer_country_idx,
                                person_idx: intent.person_idx,
                                landed_price_cents: intent.quote.landed_price_cents,
                                imported: intent.buyer_country_idx != seller_country_idx,
                                essential_debt_cents,
                                deduct_discretionary: !intent.essential,
                            })
                        };
                        results.push((decision_idx, outcome));
                    }
                    results
                })
                .collect();

            // ---- Step C: Serial merge ----
            // Handle IntentDecision::Unmet (no seller involved, not captured in partitions).
            let mut progress = false;
            {
                let scratch = &mut day_cache.round_robin;
                for decision_idx in 0..scratch.planning.decisions.len() {
                    if let IntentDecision::Unmet {
                        buyer_country_idx,
                        person_idx,
                        pending_units,
                    } = scratch.planning.decisions[decision_idx]
                    {
                        if !deferred_unmet {
                            scratch.needs_unmet[buyer_country_idx][person_idx] = 1;
                            if let Some(country) = self.countries.get_mut(buyer_country_idx) {
                                country.monthly_unmet_need_events = country
                                    .monthly_unmet_need_events
                                    .saturating_add(u64::from(pending_units));
                            }
                        }
                        scratch.pending_demands[buyer_country_idx][person_idx] = 0;
                        scratch.pending_non_zero -= 1;
                    }
                }
            }

            // Apply outcomes from the parallel phase.
            for partition_results in partition_outcomes {
                for (_, outcome) in partition_results {
                    match outcome {
                        ParallelApplyOutcome::InventoryUnavailable => {
                            // Leave pending to be retried next round.
                        }
                        ParallelApplyOutcome::Bought(info) => {
                            // Deduct discretionary budget (non-essential only).
                            if info.deduct_discretionary {
                                if let Some(budget) = discretionary_budgets
                                    .get_mut(info.buyer_country_idx)
                                    .and_then(|v| v.get_mut(info.person_idx))
                                {
                                    *budget = budget.saturating_sub(info.landed_price_cents);
                                }
                            }
                            // Essential debt spending.
                            if info.essential_debt_cents > 0 {
                                self.bump_essential_debt_spending(
                                    info.buyer_country_idx,
                                    info.essential_debt_cents,
                                );
                            }
                            // Apply buyer-side mutations (person cash + country aggregates).
                            World::apply_purchase_to_buyer(
                                &mut self.countries[info.buyer_country_idx],
                                info.person_idx,
                                info.landed_price_cents,
                                info.imported,
                            );
                            // Update scratch.
                            {
                                let scratch = &mut day_cache.round_robin;
                                let pending = &mut scratch.pending_demands[info.buyer_country_idx]
                                    [info.person_idx];
                                *pending = pending.saturating_sub(1);
                                if *pending == 0 {
                                    scratch.pending_non_zero -= 1;
                                }
                                if !deferred_unmet {
                                    scratch.needs_met[info.buyer_country_idx][info.person_idx] = 1;
                                }
                            }
                            if deferred_unmet {
                                if let Some(slot_idx) = stockable_essential_slot(good_id)
                                    && let Some(person) = self
                                        .countries
                                        .get_mut(info.buyer_country_idx)
                                        .and_then(|country| country.people.get_mut(info.person_idx))
                                {
                                    person.essential_stock_units[slot_idx] += 1.0;
                                }
                                day_cache.record_stockable_purchase(
                                    info.buyer_country_idx,
                                    info.person_idx,
                                    good_id,
                                );
                            }
                            progress = true;
                        }
                    }
                }
            }

            if !progress {
                let scratch = &mut day_cache.round_robin;
                if deferred_unmet {
                    self.clear_remaining_pending(&mut scratch.pending_demands);
                } else {
                    self.finalize_remaining_pending_as_unmet(
                        &mut scratch.pending_demands,
                        &mut scratch.needs_unmet,
                    );
                }
                break;
            }

            round = round.saturating_add(1);
        }

        let scratch = &day_cache.round_robin;
        for country_idx in 0..country_count {
            let Some(country) = self.countries.get_mut(country_idx) else {
                continue;
            };
            if deferred_unmet {
                continue;
            }
            let needs_met = &scratch.needs_met[country_idx];
            let needs_unmet = &scratch.needs_unmet[country_idx];
            for &person_idx in &scratch.active_indices[country_idx] {
                let Some(person) = country.people.get_mut(person_idx) else {
                    continue;
                };
                if needs_met[person_idx] != 0 {
                    person.last_month_needs_met_mask |= needs_mask_bit;
                }
                if needs_unmet[person_idx] != 0 {
                    person.last_month_unmet_needs_mask |= needs_mask_bit;
                }
            }
        }
    }

    pub(in crate::sim::world) fn plan_intent_for_slot(
        &self,
        slot: &BuyerSlot,
        pending_demands: &[Vec<u32>],
        discretionary_budgets: &[Vec<i64>],
        cheapest_quotes: &[Option<SellerQuote>],
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

        let Some(quote) = cheapest_quotes
            .get(slot.buyer_country_idx)
            .copied()
            .flatten()
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
            buyer_cash_cents: cash_cents,
        })
    }

    pub(in crate::sim::world) fn can_afford_purchase(
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

    pub(in crate::sim::world) fn finalize_remaining_pending_as_unmet(
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

    fn clear_remaining_pending(&mut self, pending_demands: &mut [Vec<u32>]) {
        for country_pending in pending_demands {
            for pending_units in country_pending {
                *pending_units = 0;
            }
        }
    }
}
