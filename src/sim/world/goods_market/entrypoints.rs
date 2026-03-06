use super::*;

impl World {
    pub(in crate::sim::world) fn run_daily_goods_economy(&mut self) {
        self.produce_goods_daily();
        self.consume_superficial_stock_daily();
        self.process_global_goods_market();
        self.process_discretionary_superficial_purchases();
        self.countries
            .par_iter_mut()
            .for_each(|country| country.recompute_aggregates());
    }

    pub(in crate::sim::world) fn produce_goods_daily(&mut self) {
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

                if let Some(&last_sold) = company.sold_units_history.last()
                    && last_sold > 0
                {
                    let months_of_inventory = company.inventory_units as f64 / last_sold as f64;
                    if months_of_inventory > market.excess_inventory_months {
                        continue;
                    }
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

    pub(in crate::sim::world) fn process_global_goods_market(&mut self) {
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

        for good in common_goods()
            .iter()
            .filter(|g| g.id < SUPERFICIAL_GOOD_START_ID && !g.is_durable)
        {
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

    pub(in crate::sim::world) fn consume_superficial_stock_daily(&mut self) {
        // Hoist the static per-slot computation out of the 5M-person loop.
        let daily_usages: [f32; SUPERFICIAL_GOOD_SLOT_COUNT] = std::array::from_fn(|slot_idx| {
            let good_id = SUPERFICIAL_GOOD_IDS[slot_idx];
            (common_goods()[good_id as usize].monthly_demand_factor / 30.0).max(0.0) as f32
        });

        for country in &mut self.countries {
            country
                .people
                .par_chunks_mut(PEOPLE_CHUNK_SIZE)
                .for_each(|chunk| {
                    for person in chunk {
                        for (slot_idx, &daily_usage) in daily_usages.iter().enumerate() {
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
                            person.superficial_stock_units[slot_idx] = person
                                .superficial_stock_units[slot_idx]
                                .saturating_sub(consume_units);
                        }
                    }
                });
        }
    }

    pub(in crate::sim::world) fn process_discretionary_superficial_purchases(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        let suppliers_by_good = self.build_suppliers_by_good();
        let tariff_rates_by_good = self.tariff_rates_by_good_for_month();

        for good_id in SUPERFICIAL_GOOD_IDS {
            let quote_templates_by_buyer = self.build_quote_templates_by_buyer(
                good_id,
                &suppliers_by_good[good_id as usize],
                &tariff_rates_by_good,
            );
            let stock_slot = usize::from(good_id - SUPERFICIAL_GOOD_START_ID);

            for buyer_country_idx in 0..self.countries.len() {
                // Pre-fetch cheapest quote for snapshot affordability check.
                let Some(snapshot_quote) =
                    self.find_cheapest_supplier(&quote_templates_by_buyer, buyer_country_idx)
                else {
                    continue;
                };
                if snapshot_quote.landed_price_cents <= 0 {
                    continue;
                }

                // Phase A — parallel planning: each person independently decides to buy.
                // Uses per-chunk thread_rng(); discretionary purchases are non-deterministic.
                let buyers: Vec<usize> = self.countries[buyer_country_idx]
                    .people
                    .par_chunks(PEOPLE_CHUNK_SIZE)
                    .enumerate()
                    .flat_map(|(chunk_idx, chunk)| {
                        let mut rng = rand::thread_rng();
                        let base_idx = chunk_idx * PEOPLE_CHUNK_SIZE;
                        chunk
                            .iter()
                            .enumerate()
                            .filter_map(|(offset, person)| {
                                if person.superficial_stock_units[stock_slot] > 0 {
                                    return None;
                                }
                                let cash_cents = to_cents(person.cash_balance);
                                if cash_cents < snapshot_quote.landed_price_cents {
                                    return None;
                                }
                                let affordability_ratio =
                                    cash_cents as f64 / snapshot_quote.landed_price_cents as f64;
                                let probability =
                                    ((affordability_ratio - 2.0) / 40.0).clamp(0.0, 0.05);
                                if probability <= 0.0 || !rng.gen_bool(probability) {
                                    return None;
                                }
                                Some(base_idx + offset)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();

                // Phase B — sequential apply: re-fetch current cheapest supplier per purchase
                // so inventory depletion is respected, then execute.
                for person_idx in buyers {
                    let Some(current_quote) =
                        self.find_cheapest_supplier(&quote_templates_by_buyer, buyer_country_idx)
                    else {
                        break;
                    };
                    let cash_cents = self.person_cash_cents(buyer_country_idx, person_idx);
                    if cash_cents < current_quote.landed_price_cents {
                        continue;
                    }

                    self.execute_goods_purchase(buyer_country_idx, person_idx, current_quote);
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

    pub(in crate::sim::world) fn adjust_dynamic_country_tariffs(&mut self) {
        for country in &mut self.countries {
            let mut exported_units_by_good = [0_u32; GOODS_COUNT];
            let mut inventory_by_good = [0_u32; GOODS_COUNT];
            let mut unmet_need_by_good = [false; GOODS_COUNT];
            let baseline_tariff = country
                .sim_params
                .goods_market
                .tariff_rate
                .clamp(0.0, MAX_DYNAMIC_TARIFF_RATE);

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
                for (good_idx, unmet) in unmet_need_by_good.iter_mut().enumerate().take(GOODS_COUNT)
                {
                    let bit = 1_u16 << (good_idx as u8);
                    if person.last_month_unmet_needs_mask & bit != 0 {
                        *unmet = true;
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
                } else {
                    let current = country.export_tariffs_by_good[good_idx];
                    if current > baseline_tariff {
                        country.export_tariffs_by_good[good_idx] =
                            (current - DYNAMIC_TARIFF_DECREASE_STEP).max(baseline_tariff);
                    } else if current < baseline_tariff {
                        country.export_tariffs_by_good[good_idx] =
                            (current + DYNAMIC_TARIFF_DECREASE_STEP).min(baseline_tariff);
                    }
                }
            }
        }
        self.invalidate_tariff_rate_lookup_cache();
    }

    pub(in crate::sim::world) fn initial_discretionary_budgets(&self) -> Vec<Vec<i64>> {
        self.countries
            .par_iter()
            .map(|country| {
                let month = country.month.saturating_add(1);
                let country_id = country.id;
                let spend_multiplier = country.discretionary_spend_multiplier();
                let mut country_budgets = Vec::with_capacity(country.people.len());

                for person in &country.people {
                    let budget_seed = hash64(
                        (u64::from(country_id) << 48)
                            ^ (month << 24)
                            ^ u64::from(person.id)
                            ^ u64::from(person.name_seed),
                    );
                    let spend_percent = (5.0 + (budget_seed % 11) as f64) * spend_multiplier;
                    country_budgets.push(to_cents(
                        person.cash_balance.max(0.0) * spend_percent / 100.0 / 30.0,
                    ));
                }

                country_budgets
            })
            .collect()
    }

    pub(in crate::sim::world) fn tariff_rates_by_good_for_month(&self) -> Arc<Vec<Vec<f64>>> {
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

    pub(in crate::sim::world) fn rebuild_tariff_rates_by_good(&self) -> Vec<Vec<f64>> {
        let country_count = self.countries.len();
        let mut rates_by_good = vec![vec![0.0; country_count]; GOODS_COUNT];

        for (seller_country_idx, country) in self.countries.iter().enumerate() {
            for (good_idx, rates) in rates_by_good.iter_mut().enumerate().take(GOODS_COUNT) {
                rates[seller_country_idx] = country
                    .export_tariffs_by_good
                    .get(good_idx)
                    .copied()
                    .unwrap_or(0.0);
            }
        }

        rates_by_good
    }

    pub(in crate::sim::world) fn invalidate_tariff_rate_lookup_cache(&self) {
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
}
