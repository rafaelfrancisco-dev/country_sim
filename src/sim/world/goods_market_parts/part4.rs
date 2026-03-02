impl World {
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
