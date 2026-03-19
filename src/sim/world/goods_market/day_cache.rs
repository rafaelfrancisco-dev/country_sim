use super::*;
use super::day_cache_runtime::CountryGoodsRuntime;
use crate::sim::country::DAYS_PER_MONTH;
use crate::sim::essentials::{is_stockable_essential_good, stockable_essential_slot};

/// Number of goods processed in the global round-robin (non-durable, id < SUPERFICIAL_GOOD_START_ID).
pub(in crate::sim::world) const ESSENTIAL_ND_GOOD_COUNT: usize = 9;
const COMPETITIVE_QUOTE_BAND_BPS: i64 = 2_000;
const MAX_COMPETITIVE_QUOTES: usize = 12;

/// Good IDs processed in the global round-robin, in slot order.
pub(in crate::sim::world) const ESSENTIAL_ND_GOOD_IDS: [GoodId; ESSENTIAL_ND_GOOD_COUNT] =
    [0, 1, 3, 4, 5, 6, 7, 8, 9];

/// Maps good_id (0..GOODS_COUNT) -> round-robin slot.
pub(in crate::sim::world) const ESSENTIAL_ND_SLOT: [Option<usize>; GOODS_COUNT] = [
    Some(0), // 0  Water
    Some(1), // 1  Food
    None,    // 2  Housing (durable)
    Some(2), // 3  Electricity
    Some(3), // 4  Medicine
    Some(4), // 5  Clothing
    Some(5), // 6  Hygiene Products
    Some(6), // 7  Transport
    Some(7), // 8  Internet
    Some(8), // 9  Education Services
    None,    // 10 Household Goods
    None,    // 11 Electronics
    None,    // 12 Appliances
    None,    // 13 Cars
    None,    // 14 Entertainment
    None,    // 15 Raw Metals
    None,    // 16 Timber
    None,    // 17 Fuel
];

#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::sim::world) struct GoodsMarketDayCache {
    supplier_topology_signature: u64,
    quote_signature: u64,
    pub(in crate::sim::world) suppliers_by_good: Vec<Vec<SupplierRef>>,
    pub(in crate::sim::world) quote_templates_by_good: Vec<Vec<Vec<QuoteTemplate>>>,
    pub(in crate::sim::world) cheapest_quote_cursors: Vec<Vec<usize>>,
    pub(in crate::sim::world) round_robin: RoundRobinScratch,
    /// Indexed by essential slot, then country.
    pub(in crate::sim::world) active_demands_by_good: Vec<Vec<Vec<BuyerDemand>>>,
    pub(in crate::sim::world) discretionary_buyers_scratch: Vec<usize>,
    countries_runtime: Vec<CountryGoodsRuntime>,
}

impl GoodsMarketDayCache {
    pub(in crate::sim::world) fn prepare(&mut self, world: &World) {
        self.ensure_country_runtime(world);
        self.prepare_quote_cache(world);
        self.prepare_daily_demands(world);
        self.reset_daily_cursors(world.countries.len());
    }

    fn ensure_country_runtime(&mut self, world: &World) {
        self.countries_runtime
            .resize_with(world.countries.len(), CountryGoodsRuntime::default);

        for (country_idx, country) in world.countries.iter().enumerate() {
            let runtime = &mut self.countries_runtime[country_idx];
            if runtime.person_count != country.people.len()
                || runtime.stockable_state_month != country.month
            {
                runtime.rebuild_from_country(country);
            }
        }
    }

    fn prepare_quote_cache(&mut self, world: &World) {
        let topology_signature = supplier_topology_signature(world);
        let quote_signature = quote_signature(world);
        let country_count = world.countries.len();

        if self.suppliers_by_good.len() != GOODS_COUNT {
            self.suppliers_by_good = vec![Vec::new(); GOODS_COUNT];
        }
        if self.quote_templates_by_good.len() != GOODS_COUNT {
            self.quote_templates_by_good = vec![Vec::new(); GOODS_COUNT];
        }
        if self.cheapest_quote_cursors.len() != GOODS_COUNT {
            self.cheapest_quote_cursors = vec![vec![0; country_count]; GOODS_COUNT];
        } else {
            for cursors in &mut self.cheapest_quote_cursors {
                cursors.resize(country_count, 0);
            }
        }

        if self.supplier_topology_signature != topology_signature {
            for suppliers in &mut self.suppliers_by_good {
                suppliers.clear();
            }
            world.fill_suppliers_by_good(&mut self.suppliers_by_good);
            self.supplier_topology_signature = topology_signature;
            self.quote_signature = 0;
        }

        if self.quote_signature != quote_signature {
            for good_idx in 0..GOODS_COUNT {
                self.quote_templates_by_good[good_idx] = world.build_quote_templates_by_buyer(
                    good_idx as GoodId,
                    &self.suppliers_by_good[good_idx],
                    &world.tariff_rates_by_good,
                    &world.import_tariff_rates_by_good,
                );
            }
            self.quote_signature = quote_signature;
        }
    }

    fn prepare_daily_demands(&mut self, world: &World) {
        self.active_demands_by_good
            .resize_with(ESSENTIAL_ND_GOOD_COUNT, Vec::new);
        for slot in 0..ESSENTIAL_ND_GOOD_COUNT {
            self.active_demands_by_good[slot].resize_with(world.countries.len(), Vec::new);
            for country_demands in &mut self.active_demands_by_good[slot] {
                country_demands.clear();
            }
        }

        let day = world.day;
        let day_in_month = current_day_in_month(day);
        let goods = common_goods();
        for (country_idx, country) in world.countries.iter().enumerate() {
            for (slot, &good_id) in ESSENTIAL_ND_GOOD_IDS.iter().enumerate() {
                let demands = &mut self.active_demands_by_good[slot][country_idx];
                if is_stockable_essential_good(good_id) {
                    self.countries_runtime[country_idx].fill_stockable_demands(
                        good_id,
                        day_in_month,
                        demands,
                    );
                    continue;
                }

                for (person_idx, person) in country.people.iter().enumerate() {
                    let units = demand_units_for_person_good(
                        day,
                        country.id,
                        person.id,
                        person.name_seed,
                        good_id,
                        goods[good_id as usize].monthly_demand_factor,
                    );
                    if units == 0 {
                        continue;
                    }
                    demands.push(BuyerDemand { person_idx, units });
                }
            }
        }
    }

    fn reset_daily_cursors(&mut self, country_count: usize) {
        for cursors in &mut self.cheapest_quote_cursors {
            cursors.resize(country_count, 0);
            cursors.fill(0);
        }
    }

    pub(in crate::sim::world) fn prepare_round_robin_for_good(
        &mut self,
        countries: &[crate::sim::country::Country],
        good_id: GoodId,
    ) {
        let Some(slot) = ESSENTIAL_ND_SLOT[good_id as usize] else {
            return;
        };
        let active_demands = self
            .active_demands_by_good
            .get(slot)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        self.round_robin.prepare_for_good(countries, active_demands);
    }

    pub(in crate::sim::world) fn current_cheapest_quote(
        &mut self,
        world: &World,
        good_id: GoodId,
        buyer_country_idx: usize,
    ) -> Option<SellerQuote> {
        let buyer_templates = self
            .quote_templates_by_good
            .get(good_id as usize)?
            .get(buyer_country_idx)?;
        let cursor = self
            .cheapest_quote_cursors
            .get_mut(good_id as usize)?
            .get_mut(buyer_country_idx)?;

        let mut candidate_indices = Vec::with_capacity(MAX_COMPETITIVE_QUOTES);
        let mut price_limit = None;

        for (idx, template) in buyer_templates.iter().enumerate() {
            let in_stock = world
                .countries
                .get(template.seller_country_idx)
                .and_then(|country| country.companies.get(template.seller_company_idx))
                .is_some_and(|company| company.inventory_units > 0);
            if !in_stock {
                continue;
            }

            let current_limit = price_limit.get_or_insert_with(|| {
                template.landed_price_cents.saturating_add(
                    (template
                        .landed_price_cents
                        .saturating_mul(COMPETITIVE_QUOTE_BAND_BPS)
                        / 10_000)
                        .max(1),
                )
            });
            if template.landed_price_cents > *current_limit {
                break;
            }

            candidate_indices.push(idx);
            if candidate_indices.len() >= MAX_COMPETITIVE_QUOTES {
                break;
            }
        }

        if candidate_indices.is_empty() {
            return None;
        }

        let selected_idx = candidate_indices[*cursor % candidate_indices.len()];
        *cursor = cursor.wrapping_add(1);
        let template = buyer_templates.get(selected_idx)?;
        Some(SellerQuote {
            seller_country_idx: template.seller_country_idx,
            seller_company_idx: template.seller_company_idx,
            base_price_cents: template.base_price_cents,
            tariff_cents: template.tariff_cents,
            import_tariff_cents: template.import_tariff_cents,
            landed_price_cents: template.landed_price_cents,
        })
    }

    pub(in crate::sim::world) fn mark_superficial_zero_stock(
        &mut self,
        country_idx: usize,
        person_idx: usize,
        stock_slot: usize,
    ) {
        let Some(runtime) = self.countries_runtime.get_mut(country_idx) else {
            return;
        };
        runtime.add_superficial_zero_stock(person_idx, stock_slot);
    }

    pub(in crate::sim::world) fn mark_superficial_stock_replenished(
        &mut self,
        country_idx: usize,
        person_idx: usize,
        stock_slot: usize,
    ) {
        let Some(runtime) = self.countries_runtime.get_mut(country_idx) else {
            return;
        };
        runtime.remove_superficial_zero_stock(person_idx, stock_slot);
    }

    pub(in crate::sim::world) fn superficial_zero_stock_candidates(
        &self,
        country_idx: usize,
        stock_slot: usize,
    ) -> &[usize] {
        self.countries_runtime
            .get(country_idx)
            .map(|runtime| runtime.superficial_zero_stock_buyers[stock_slot].as_slice())
            .unwrap_or(&[])
    }

    pub(in crate::sim::world) fn record_stockable_purchase(
        &mut self,
        country_idx: usize,
        person_idx: usize,
        good_id: GoodId,
    ) {
        let Some(slot_idx) = stockable_essential_slot(good_id) else {
            return;
        };
        let Some(runtime) = self.countries_runtime.get_mut(country_idx) else {
            return;
        };
        if let Some(deficits) = runtime.stockable_deficits.get_mut(person_idx) {
            deficits[slot_idx] = deficits[slot_idx].saturating_sub(1);
            if deficits[slot_idx] == 0 {
                runtime.remove_stockable_active_buyer(person_idx, slot_idx);
            }
        }
    }
}

fn supplier_topology_signature(world: &World) -> u64 {
    let mut signature = 0_u64;
    for (country_idx, country) in world.countries.iter().enumerate() {
        signature ^= hash64(
            ((country_idx as u64) << 48)
                ^ ((country.companies.len() as u64) << 24)
                ^ u64::from(country.id),
        );
        for (company_idx, company) in country.companies.iter().enumerate() {
            signature ^= hash64(
                ((country_idx as u64) << 48)
                    ^ ((company_idx as u64) << 24)
                    ^ u64::from(company.produced_good_id)
                    ^ u64::from((to_cents(company.unit_price) > 0) as u8),
            );
        }
    }
    signature
}

fn quote_signature(world: &World) -> u64 {
    let mut signature = 0_u64;
    for (country_idx, country) in world.countries.iter().enumerate() {
        for (company_idx, company) in country.companies.iter().enumerate() {
            signature ^= hash64(
                ((country_idx as u64) << 48)
                    ^ ((company_idx as u64) << 24)
                    ^ (to_cents(company.unit_price) as u64),
            );
        }
    }
    for (good_idx, tariff_rates) in world.tariff_rates_by_good.iter().enumerate() {
        for (seller_country_idx, rate) in tariff_rates.iter().enumerate() {
            signature ^= hash64(
                ((good_idx as u64) << 48)
                    ^ ((seller_country_idx as u64) << 24)
                    ^ (to_cents(*rate * 10_000.0) as u64),
            );
        }
    }
    for (good_idx, import_rates) in world.import_tariff_rates_by_good.iter().enumerate() {
        for (buyer_country_idx, rate) in import_rates.iter().enumerate() {
            signature ^= hash64(
                ((good_idx as u64) << 48)
                    ^ ((buyer_country_idx as u64) << 24)
                    ^ (to_cents(*rate * 10_000.0) as u64)
                    ^ 0x8000_0000_0000_0000, // distinguish import from export
            );
        }
    }
    signature
}

fn current_day_in_month(day: u64) -> u64 {
    let remainder = day % DAYS_PER_MONTH;
    if remainder == 0 {
        DAYS_PER_MONTH
    } else {
        remainder
    }
}
