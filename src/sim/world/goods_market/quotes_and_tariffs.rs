use super::*;
use crate::sim::world::fx::CrossBorderFlowKind;

impl World {
    pub(in crate::sim::world) fn build_quote_templates_by_buyer(
        &self,
        good_id: GoodId,
        supplier_refs: &[SupplierRef],
        tariff_rates_by_good: &[Vec<f64>],
        import_tariff_rates_by_good: &[Vec<f64>],
    ) -> Vec<Vec<QuoteTemplate>> {
        let country_count = self.countries.len();
        let tariff_rates_by_seller = tariff_rates_by_good
            .get(good_id as usize)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let import_tariff_rates_by_buyer = import_tariff_rates_by_good
            .get(good_id as usize)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        (0..country_count)
            .map(|buyer_country_idx| {
                let mut templates = supplier_refs
                    .iter()
                    .map(|supplier| {
                        let (tariff_cents, import_tariff_cents) =
                            if supplier.seller_country_idx == buyer_country_idx {
                                (0, 0)
                            } else {
                                let export_rate = tariff_rates_by_seller
                                    .get(supplier.seller_country_idx)
                                    .copied()
                                    .unwrap_or(0.0);
                                let import_rate = import_tariff_rates_by_buyer
                                    .get(buyer_country_idx)
                                    .copied()
                                    .unwrap_or(0.0);
                                let base = from_cents(supplier.base_price_cents);
                                (to_cents(base * export_rate), to_cents(base * import_rate))
                            };
                        let landed_price_cents = supplier
                            .base_price_cents
                            .saturating_add(tariff_cents)
                            .saturating_add(import_tariff_cents);

                        QuoteTemplate {
                            seller_country_idx: supplier.seller_country_idx,
                            seller_company_idx: supplier.seller_company_idx,
                            base_price_cents: supplier.base_price_cents,
                            tariff_cents,
                            import_tariff_cents,
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

    pub(in crate::sim::world) fn execute_goods_purchase(
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
            let settlement = self.settle_cross_border_cash(
                CrossBorderFlowKind::GoodsImport,
                buyer_country_idx,
                quote.seller_country_idx,
                from_cents(quote.base_price_cents),
            );
            let (left, right) = self.countries.split_at_mut(quote.seller_country_idx);
            let buyer_country = &mut left[buyer_country_idx];
            let seller_country = &mut right[0];
            Self::apply_purchase_to_buyer(
                buyer_country,
                person_idx,
                quote.landed_price_cents,
                true,
            );
            Self::apply_tariff_to_importer(buyer_country, quote.import_tariff_cents);
            Self::apply_sale_to_seller_with_amount(
                seller_country,
                quote.seller_company_idx,
                settlement.payee_amount_local,
                false,
            );
            Self::apply_tariff_to_exporter(seller_country, quote.tariff_cents);
        } else {
            let settlement = self.settle_cross_border_cash(
                CrossBorderFlowKind::GoodsImport,
                buyer_country_idx,
                quote.seller_country_idx,
                from_cents(quote.base_price_cents),
            );
            let (left, right) = self.countries.split_at_mut(buyer_country_idx);
            let seller_country = &mut left[quote.seller_country_idx];
            let buyer_country = &mut right[0];
            Self::apply_purchase_to_buyer(
                buyer_country,
                person_idx,
                quote.landed_price_cents,
                true,
            );
            Self::apply_tariff_to_importer(buyer_country, quote.import_tariff_cents);
            Self::apply_sale_to_seller_with_amount(
                seller_country,
                quote.seller_company_idx,
                settlement.payee_amount_local,
                false,
            );
            Self::apply_tariff_to_exporter(seller_country, quote.tariff_cents);
        }
    }

    pub(in crate::sim::world) fn apply_purchase_to_buyer(
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

        country.adjust_household_cash_total_cents(-landed_price_cents);
        country.monthly_household_spending =
            round_cents(country.monthly_household_spending + amount);
        if imported {
            country.monthly_goods_import_spending =
                round_cents(country.monthly_goods_import_spending + amount);
        }
    }

    pub(in crate::sim::world) fn apply_sale_to_seller(
        country: &mut crate::sim::country::Country,
        company_idx: usize,
        base_price_cents: i64,
        domestic: bool,
    ) {
        Self::apply_sale_to_seller_with_amount(
            country,
            company_idx,
            from_cents(base_price_cents),
            domestic,
        );
    }

    pub(in crate::sim::world) fn apply_sale_to_seller_with_amount(
        country: &mut crate::sim::country::Country,
        company_idx: usize,
        amount: f64,
        domestic: bool,
    ) {
        let amount = round_cents(amount);
        let base_price_cents = to_cents(amount);

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

        country.adjust_company_cash_total_cents(base_price_cents);
        if !domestic {
            country.monthly_goods_export_revenue =
                round_cents(country.monthly_goods_export_revenue + amount);
        }
    }

    pub(in crate::sim::world) fn apply_tariff_to_exporter(
        country: &mut crate::sim::country::Country,
        tariff_cents: i64,
    ) {
        if tariff_cents <= 0 {
            return;
        }

        let amount = from_cents(tariff_cents);
        country.monthly_tariff_collected = round_cents(country.monthly_tariff_collected + amount);
        country.adjust_treasury_balance_cents(tariff_cents);
    }

    pub(in crate::sim::world) fn apply_tariff_to_importer(
        country: &mut crate::sim::country::Country,
        import_tariff_cents: i64,
    ) {
        if import_tariff_cents <= 0 {
            return;
        }

        let amount = from_cents(import_tariff_cents);
        country.monthly_import_tariff_collected =
            round_cents(country.monthly_import_tariff_collected + amount);
        country.adjust_treasury_balance_cents(import_tariff_cents);
    }

    pub(in crate::sim::world) fn fill_suppliers_by_good(
        &self,
        suppliers_by_good: &mut [Vec<SupplierRef>],
    ) {
        for (seller_country_idx, country) in self.countries.iter().enumerate() {
            for (seller_company_idx, company) in country.companies.iter().enumerate() {
                let good_idx = company.produced_good_id as usize;
                if good_idx >= GOODS_COUNT || good_idx >= suppliers_by_good.len() {
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
    }

    pub(in crate::sim::world) fn person_cash_cents(
        &self,
        buyer_country_idx: usize,
        person_idx: usize,
    ) -> i64 {
        self.countries
            .get(buyer_country_idx)
            .and_then(|country| country.people.get(person_idx))
            .map_or(0, |person| to_cents(person.cash_balance))
    }

    pub(in crate::sim::world) fn bump_unmet_need_events(
        &mut self,
        buyer_country_idx: usize,
        count: u32,
    ) {
        if let Some(country) = self.countries.get_mut(buyer_country_idx) {
            country.monthly_unmet_need_events = country
                .monthly_unmet_need_events
                .saturating_add(u64::from(count));
        }
    }

    pub(in crate::sim::world) fn bump_essential_debt_spending(
        &mut self,
        buyer_country_idx: usize,
        debt_cents: i64,
    ) {
        if debt_cents <= 0 {
            return;
        }

        if let Some(country) = self.countries.get_mut(buyer_country_idx) {
            country.monthly_essential_debt_spending =
                round_cents(country.monthly_essential_debt_spending + from_cents(debt_cents));
        }
    }
}
