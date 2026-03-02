impl World {
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

        let day = self.day;
        scratch
            .pending_demands
            .iter_mut()
            .zip(self.countries.iter())
            .for_each(|(country_pending, country)| {
                let country_id = country.id;
                country_pending
                    .par_chunks_mut(PEOPLE_CHUNK_SIZE)
                    .zip(country.people.par_chunks(PEOPLE_CHUNK_SIZE))
                    .for_each(|(pending_chunk, people_chunk)| {
                        for (pending, person) in pending_chunk.iter_mut().zip(people_chunk.iter()) {
                            *pending = demand_units_for_person_good(
                                day,
                                country_id,
                                person.id,
                                person.name_seed,
                                good_id,
                                monthly_demand_factor,
                            );
                        }
                    });
            });

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

}
