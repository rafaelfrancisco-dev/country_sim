use super::*;

impl World {
    pub(in crate::sim::world) fn run_daily_goods_economy(&mut self) {
        self.produce_goods_daily();
        let mut goods_runtime = std::mem::take(&mut self.goods_runtime);
        self.consume_superficial_stock_daily(&mut goods_runtime);
        goods_runtime.prepare(self);
        self.process_global_goods_market(&mut goods_runtime);
        self.process_discretionary_superficial_purchases(&mut goods_runtime);
        self.goods_runtime = goods_runtime;
    }

    pub(in crate::sim::world) fn produce_goods_daily(&mut self) {
        self.countries.par_iter_mut().for_each(|country| {
            let market = country.sim_params.goods_market;
            let daily_effort_per_worker = market.worker_effort_per_month / 30.0;

            for company in &mut country.companies {
                let Some(good) = common_goods().get(company.produced_good_id as usize) else {
                    continue;
                };
                if good.effort_per_unit <= 0.0 {
                    continue;
                }

                if let Some(&last_sold) = company.sold_units_history.back()
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
        });
    }

    pub(in crate::sim::world) fn process_global_goods_market(
        &mut self,
        day_cache: &mut GoodsMarketDayCache,
    ) {
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

        for good in common_goods()
            .iter()
            .filter(|g| g.id < SUPERFICIAL_GOOD_START_ID && !g.is_durable)
        {
            self.process_good_round_robin(
                good.id,
                good.necessity_rank,
                start_country_idx,
                &mut discretionary_budgets,
                day_cache,
            );
        }
    }

    pub(in crate::sim::world) fn consume_superficial_stock_daily(
        &mut self,
        day_cache: &mut GoodsMarketDayCache,
    ) {
        // Hoist the static per-slot computation out of the 5M-person loop.
        let daily_usages: [f32; SUPERFICIAL_GOOD_SLOT_COUNT] = std::array::from_fn(|slot_idx| {
            let good_id = SUPERFICIAL_GOOD_IDS[slot_idx];
            (common_goods()[good_id as usize].monthly_demand_factor / 30.0).max(0.0) as f32
        });

        for (country_idx, country) in self.countries.iter_mut().enumerate() {
            for (person_idx, person) in country.people.iter_mut().enumerate() {
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
                    let had_stock = person.superficial_stock_units[slot_idx] > 0;
                    person.superficial_stock_units[slot_idx] =
                        person.superficial_stock_units[slot_idx].saturating_sub(consume_units);
                    if had_stock && person.superficial_stock_units[slot_idx] == 0 {
                        day_cache.mark_superficial_zero_stock(country_idx, person_idx, slot_idx);
                    }
                }
            }
        }
    }

    pub(in crate::sim::world) fn process_discretionary_superficial_purchases(
        &mut self,
        day_cache: &mut GoodsMarketDayCache,
    ) {
        if self.countries.is_empty() {
            return;
        }

        for good_id in SUPERFICIAL_GOOD_IDS {
            let stock_slot = usize::from(good_id - SUPERFICIAL_GOOD_START_ID);
            let active_cohort = self.day.saturating_sub(1) % 7;

            for buyer_country_idx in 0..self.countries.len() {
                let Some(snapshot_quote) =
                    day_cache.current_cheapest_quote(self, good_id, buyer_country_idx)
                else {
                    continue;
                };
                if snapshot_quote.landed_price_cents <= 0 {
                    continue;
                }

                // Phase A — parallel planning: each person independently decides to buy.
                let mut buyers = std::mem::take(&mut day_cache.discretionary_buyers_scratch);
                buyers.clear();
                for &person_idx in
                    day_cache.superficial_zero_stock_candidates(buyer_country_idx, stock_slot)
                {
                    let Some(person) = self.countries[buyer_country_idx].people.get(person_idx)
                    else {
                        continue;
                    };
                    if hash64(
                        (u64::from(self.countries[buyer_country_idx].id) << 48)
                            ^ u64::from(person.id)
                            ^ u64::from(good_id),
                    ) % 7
                        != active_cohort
                    {
                        continue;
                    }
                    let cash_cents = to_cents(person.cash_balance);
                    if cash_cents < snapshot_quote.landed_price_cents {
                        continue;
                    }
                    let affordability_ratio =
                        cash_cents as f64 / snapshot_quote.landed_price_cents as f64;
                    let probability = ((affordability_ratio - 2.0) / 40.0).clamp(0.0, 0.05);
                    if probability <= 0.0 {
                        continue;
                    }
                    let buy_roll = unit_interval(
                        (self.day << 40)
                            ^ (u64::from(self.countries[buyer_country_idx].id) << 24)
                            ^ (u64::from(person.id) << 8)
                            ^ u64::from(good_id),
                    );
                    if buy_roll < probability {
                        buyers.push(person_idx);
                    }
                }

                // Phase B — sequential apply: re-fetch current cheapest supplier per purchase
                // so inventory depletion is respected, then execute.
                for person_idx in &buyers {
                    let person_idx = *person_idx;
                    let Some(current_quote) =
                        day_cache.current_cheapest_quote(self, good_id, buyer_country_idx)
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
                        if person.superficial_stock_units[stock_slot] == 0 {
                            day_cache.mark_superficial_stock_replenished(
                                buyer_country_idx,
                                person_idx,
                                stock_slot,
                            );
                        }
                        person.superficial_stock_units[stock_slot] =
                            person.superficial_stock_units[stock_slot].saturating_add(1);
                    }
                }
                day_cache.discretionary_buyers_scratch = buyers;
            }
        }
    }

    pub(in crate::sim::world) fn adjust_dynamic_country_tariffs(&mut self) {
        self.countries.par_iter_mut().for_each(|country| {
            let mut exported_units_by_good = [0_u32; GOODS_COUNT];
            let mut domestic_sold_by_good = [0_u32; GOODS_COUNT];
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
                domestic_sold_by_good[good_idx] = domestic_sold_by_good[good_idx]
                    .saturating_add(company.last_month_domestic_units_sold);
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
                // Low stock: inventory covers less than half a month of domestic demand.
                let low_stock =
                    inventory_by_good[good_idx] < domestic_sold_by_good[good_idx].saturating_div(2);
                let unmet_domestic_need = unmet_need_by_good[good_idx];

                // Export tariff: rises when exporting AND domestic supply is tight.
                if exported && (low_stock || unmet_domestic_need) {
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

                // Import tariff: rises to protect domestic industry when there is excess stock
                // (foreign goods crowding out local producers); falls when needs go unmet.
                let excess_domestic_stock =
                    inventory_by_good[good_idx] > domestic_sold_by_good[good_idx].saturating_mul(2);
                if unmet_domestic_need {
                    // Consumers need more goods — lower barriers to allow cheaper imports.
                    let current = country.import_tariffs_by_good[good_idx];
                    if current > baseline_tariff {
                        country.import_tariffs_by_good[good_idx] =
                            (current - DYNAMIC_TARIFF_INCREASE_STEP).max(baseline_tariff);
                    } else if current < baseline_tariff {
                        country.import_tariffs_by_good[good_idx] =
                            (current + DYNAMIC_TARIFF_INCREASE_STEP).min(baseline_tariff);
                    }
                } else if excess_domestic_stock {
                    // Domestic producers are sitting on excess inventory — raise import barriers.
                    country.import_tariffs_by_good[good_idx] =
                        (country.import_tariffs_by_good[good_idx] + DYNAMIC_TARIFF_INCREASE_STEP)
                            .min(MAX_DYNAMIC_TARIFF_RATE);
                } else {
                    let current = country.import_tariffs_by_good[good_idx];
                    if current > baseline_tariff {
                        country.import_tariffs_by_good[good_idx] =
                            (current - DYNAMIC_TARIFF_DECREASE_STEP).max(baseline_tariff);
                    } else if current < baseline_tariff {
                        country.import_tariffs_by_good[good_idx] =
                            (current + DYNAMIC_TARIFF_DECREASE_STEP).min(baseline_tariff);
                    }
                }
            }
        });
        self.rebuild_tariff_matrix_in_place();
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

    pub(crate) fn build_tariff_rates_from_countries(
        countries: &[crate::sim::country::Country],
    ) -> Vec<Vec<f64>> {
        let country_count = countries.len();
        let mut rates_by_good = vec![vec![0.0; country_count]; GOODS_COUNT];
        for (seller_country_idx, country) in countries.iter().enumerate() {
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

    pub(crate) fn build_import_tariff_rates_from_countries(
        countries: &[crate::sim::country::Country],
    ) -> Vec<Vec<f64>> {
        let country_count = countries.len();
        let mut rates_by_good = vec![vec![0.0; country_count]; GOODS_COUNT];
        for (buyer_country_idx, country) in countries.iter().enumerate() {
            for (good_idx, rates) in rates_by_good.iter_mut().enumerate().take(GOODS_COUNT) {
                rates[buyer_country_idx] = country
                    .import_tariffs_by_good
                    .get(good_idx)
                    .copied()
                    .unwrap_or(0.0);
            }
        }
        rates_by_good
    }

    pub(in crate::sim::world) fn rebuild_tariff_matrix_in_place(&mut self) {
        let country_count = self.countries.len();
        if self.tariff_rates_by_good.len() != GOODS_COUNT {
            self.tariff_rates_by_good = vec![vec![0.0; country_count]; GOODS_COUNT];
        }
        for (good_idx, rates) in self
            .tariff_rates_by_good
            .iter_mut()
            .enumerate()
            .take(GOODS_COUNT)
        {
            rates.resize(country_count, 0.0);
            for (seller_country_idx, country) in self.countries.iter().enumerate() {
                rates[seller_country_idx] = country
                    .export_tariffs_by_good
                    .get(good_idx)
                    .copied()
                    .unwrap_or(0.0);
            }
        }

        if self.import_tariff_rates_by_good.len() != GOODS_COUNT {
            self.import_tariff_rates_by_good = vec![vec![0.0; country_count]; GOODS_COUNT];
        }
        for (good_idx, rates) in self
            .import_tariff_rates_by_good
            .iter_mut()
            .enumerate()
            .take(GOODS_COUNT)
        {
            rates.resize(country_count, 0.0);
            for (buyer_country_idx, country) in self.countries.iter().enumerate() {
                rates[buyer_country_idx] = country
                    .import_tariffs_by_good
                    .get(good_idx)
                    .copied()
                    .unwrap_or(0.0);
            }
        }
    }
}
