use super::*;

#[derive(Debug, Default)]
pub(in crate::sim::world) struct GoodsMarketDayCache {
    supplier_lists_ready: bool,
    pub(in crate::sim::world) suppliers_by_good: Vec<Vec<SupplierRef>>,
    pub(in crate::sim::world) quote_templates_by_good: Vec<Vec<Vec<QuoteTemplate>>>,
    pub(in crate::sim::world) cheapest_quote_cursors: Vec<Vec<usize>>,
    pub(in crate::sim::world) round_robin: RoundRobinScratch,
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
