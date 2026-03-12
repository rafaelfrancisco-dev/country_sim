use super::*;
use crate::sim::country::DAYS_PER_MONTH;
use crate::sim::essentials::{
    is_stockable_essential_good, monthly_requirement_for_good, stockable_essential_slot,
    target_buffer_units,
};

/// Number of goods processed in the global round-robin (non-durable, id < SUPERFICIAL_GOOD_START_ID).
pub(in crate::sim::world) const ESSENTIAL_ND_GOOD_COUNT: usize = 9;

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

#[derive(Debug, Clone, PartialEq)]
struct CountryGoodsRuntime {
    stockable_state_month: u64,
    person_count: usize,
    stockable_initial_deficits: Vec<[u32; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT]>,
    stockable_deficits: Vec<[u32; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT]>,
    stockable_targets: Vec<[u32; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT]>,
    stockable_active_buyers: [Vec<usize>; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT],
    stockable_active_positions: [Vec<usize>; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT],
    superficial_zero_stock_buyers: [Vec<usize>; SUPERFICIAL_GOOD_SLOT_COUNT],
    superficial_zero_stock_positions: [Vec<usize>; SUPERFICIAL_GOOD_SLOT_COUNT],
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

        while let Some(template) = buyer_templates.get(*cursor) {
            if world
                .countries
                .get(template.seller_country_idx)
                .and_then(|country| country.companies.get(template.seller_company_idx))
                .is_some_and(|company| company.inventory_units > 0)
            {
                return Some(SellerQuote {
                    seller_country_idx: template.seller_country_idx,
                    seller_company_idx: template.seller_company_idx,
                    base_price_cents: template.base_price_cents,
                    tariff_cents: template.tariff_cents,
                    import_tariff_cents: template.import_tariff_cents,
                    landed_price_cents: template.landed_price_cents,
                });
            }
            *cursor = cursor.saturating_add(1);
        }
        None
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

impl CountryGoodsRuntime {
    fn rebuild_from_country(&mut self, country: &crate::sim::country::Country) {
        let person_count = country.people.len();
        self.person_count = person_count;
        self.stockable_state_month = country.month;
        self.stockable_initial_deficits.resize(
            person_count,
            [0; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT],
        );
        self.stockable_deficits.resize(
            person_count,
            [0; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT],
        );
        self.stockable_targets.resize(
            person_count,
            [0; crate::sim::person::ESSENTIAL_STOCK_SLOT_COUNT],
        );

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
            for &good_id in &crate::sim::essentials::STOCKABLE_ESSENTIAL_GOOD_IDS {
                let Some(slot_idx) = stockable_essential_slot(good_id) else {
                    continue;
                };
                let target_units = stock_target_units(country, good_id, household_size);
                let current_units = person.essential_stock_units[slot_idx].max(0.0);
                let deficit = (target_units - current_units).ceil().max(0.0) as u32;
                self.stockable_targets[person_idx][slot_idx] = target_units.ceil().max(0.0) as u32;
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

    fn fill_stockable_demands(
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

    fn add_stockable_active_buyer(&mut self, person_idx: usize, slot_idx: usize) {
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

    fn remove_stockable_active_buyer(&mut self, person_idx: usize, slot_idx: usize) {
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

    fn add_superficial_zero_stock(&mut self, person_idx: usize, stock_slot: usize) {
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

    fn remove_superficial_zero_stock(&mut self, person_idx: usize, stock_slot: usize) {
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

fn stock_target_units(
    country: &crate::sim::country::Country,
    good_id: GoodId,
    household_size: usize,
) -> f32 {
    let essentials = &country.sim_params.essentials;
    monthly_requirement_for_good(essentials, good_id, household_size)
        + target_buffer_units(essentials, good_id, household_size)
}

fn current_day_in_month(day: u64) -> u64 {
    let remainder = day % DAYS_PER_MONTH;
    if remainder == 0 {
        DAYS_PER_MONTH
    } else {
        remainder
    }
}

fn stockable_release_phase(person_idx: usize, slot_idx: usize) -> u64 {
    hash64(((person_idx as u64) << 8) ^ slot_idx as u64) % DAYS_PER_MONTH
}
