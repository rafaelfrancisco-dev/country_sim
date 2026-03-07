use super::*;
use crate::sim::essentials::{
    is_stockable_essential_good, monthly_requirement_for_good, stockable_essential_slot,
    target_buffer_units,
};

/// Number of goods processed in the global round-robin (non-durable, id < SUPERFICIAL_GOOD_START_ID).
pub(in crate::sim::world) const ESSENTIAL_ND_GOOD_COUNT: usize = 9;

/// Good IDs processed in the global round-robin, in slot order.
/// Slot index corresponds to the column in `precomputed_essential_demands`.
pub(in crate::sim::world) const ESSENTIAL_ND_GOOD_IDS: [GoodId; ESSENTIAL_ND_GOOD_COUNT] =
    [0, 1, 3, 4, 5, 6, 7, 8, 9];

/// Maps good_id (0..GOODS_COUNT) → slot in precomputed_essential_demands.
/// None = durable (Housing) or superficial (id ≥ 10) — not in global round-robin.
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
    None,    // 10 Household Goods  (superficial)
    None,    // 11 Electronics      (superficial)
    None,    // 12 Appliances       (superficial)
    None,    // 13 Cars             (superficial)
    None,    // 14 Entertainment    (superficial)
];

#[derive(Debug, Default)]
pub(in crate::sim::world) struct GoodsMarketDayCache {
    supplier_lists_ready: bool,
    pub(in crate::sim::world) suppliers_by_good: Vec<Vec<SupplierRef>>,
    pub(in crate::sim::world) quote_templates_by_good: Vec<Vec<Vec<QuoteTemplate>>>,
    pub(in crate::sim::world) cheapest_quote_cursors: Vec<Vec<usize>>,
    pub(in crate::sim::world) round_robin: RoundRobinScratch,
    /// Flat demand buffer: [country_idx][person_idx * ESSENTIAL_ND_GOOD_COUNT + slot].
    /// Populated once per tick by `precompute_all_essential_demands` before the goods loop.
    pub(in crate::sim::world) precomputed_essential_demands: Vec<Vec<u32>>,
}

impl GoodsMarketDayCache {
    pub(in crate::sim::world) fn prepare(&mut self, world: &World) {
        let country_count = world.countries.len();
        if self.suppliers_by_good.len() != GOODS_COUNT {
            self.suppliers_by_good = vec![Vec::new(); GOODS_COUNT];
        } else {
            for suppliers in &mut self.suppliers_by_good {
                suppliers.clear();
            }
        }
        if self.quote_templates_by_good.len() != GOODS_COUNT {
            self.quote_templates_by_good = vec![Vec::new(); GOODS_COUNT];
        } else {
            for templates in &mut self.quote_templates_by_good {
                templates.clear();
            }
        }
        if self.cheapest_quote_cursors.len() != GOODS_COUNT {
            self.cheapest_quote_cursors = vec![vec![0; country_count]; GOODS_COUNT];
        } else {
            for cursors in &mut self.cheapest_quote_cursors {
                cursors.resize(country_count, 0);
                cursors.fill(0);
            }
        }

        world.fill_suppliers_by_good(&mut self.suppliers_by_good);
        let tariff_rates_by_good = world.tariff_rates_by_good_for_month();
        for good_idx in 0..GOODS_COUNT {
            self.quote_templates_by_good[good_idx] = world.build_quote_templates_by_buyer(
                good_idx as GoodId,
                &self.suppliers_by_good[good_idx],
                &tariff_rates_by_good,
            );
        }
        self.supplier_lists_ready = true;
    }

    /// Precompute initial demands for all 9 round-robin goods in one pass over the people array.
    ///
    /// Buffer layout: `precomputed_essential_demands[country_idx][person_idx * ESSENTIAL_ND_GOOD_COUNT + slot]`
    /// where `slot = ESSENTIAL_ND_SLOT[good_id]`.
    ///
    /// This replaces the per-good O(people) scan in `process_good_round_robin`, reducing 9 separate
    /// cold memory reads to a single warm pass shared across all goods.
    pub(in crate::sim::world) fn precompute_all_essential_demands(&mut self, world: &World) {
        let country_count = world.countries.len();
        self.precomputed_essential_demands
            .resize_with(country_count, Vec::new);
        for (c_idx, buf) in self.precomputed_essential_demands.iter_mut().enumerate() {
            let person_count = world.countries[c_idx].people.len();
            let required = person_count * ESSENTIAL_ND_GOOD_COUNT;
            if buf.len() != required {
                buf.resize(required, 0);
            }
        }

        let day = world.day;
        let goods = common_goods();

        self.precomputed_essential_demands
            .par_iter_mut()
            .zip(world.countries.par_iter())
            .for_each(|(buf, country)| {
                let country_id = country.id;
                for (p_idx, person) in country.people.iter().enumerate() {
                    let household_size = country.household_size_for_person(p_idx);
                    for (slot, &good_id) in ESSENTIAL_ND_GOOD_IDS.iter().enumerate() {
                        let good = &goods[good_id as usize];
                        let demand = if is_stockable_essential_good(good_id) {
                            compute_stockable_demand(
                                country,
                                household_size,
                                good_id,
                                &person.essential_stock_units,
                            )
                        } else {
                            demand_units_for_person_good(
                                day,
                                country_id,
                                person.id,
                                person.name_seed,
                                good_id,
                                good.monthly_demand_factor,
                            )
                        };
                        buf[p_idx * ESSENTIAL_ND_GOOD_COUNT + slot] = demand;
                    }
                }
            });
    }

    pub(in crate::sim::world) fn current_cheapest_quote(
        &mut self,
        world: &World,
        good_id: GoodId,
        buyer_country_idx: usize,
    ) -> Option<SellerQuote> {
        if !self.supplier_lists_ready {
            self.prepare(world);
        }
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
                    landed_price_cents: template.landed_price_cents,
                });
            }
            *cursor = cursor.saturating_add(1);
        }
        None
    }
}

/// Standalone version of `World::stockable_essential_purchase_gap` that works without `&World`.
/// Computes how many units of a stockable essential good a person needs to buy to hit their target.
fn compute_stockable_demand(
    country: &crate::sim::country::Country,
    household_size: usize,
    good_id: GoodId,
    essential_stock_units: &[f32],
) -> u32 {
    let Some(slot_idx) = stockable_essential_slot(good_id) else {
        return 0;
    };
    let essentials = &country.sim_params.essentials;
    let target_units = monthly_requirement_for_good(essentials, good_id, household_size)
        + target_buffer_units(essentials, good_id, household_size);
    let current_units = essential_stock_units
        .get(slot_idx)
        .copied()
        .unwrap_or(0.0)
        .max(0.0);
    (target_units - current_units).ceil().max(0.0) as u32
}
