use super::*;

impl Country {
    pub(super) fn process_listed_company_secondary_offerings(&mut self) {
        let params = self.sim_params.stock_market;
        if !params.enabled {
            return;
        }

        let trigger = params.secondary_offering_missed_payment_trigger;
        let discount = params.secondary_offering_discount.clamp(0.0, 1.0);
        let max_dilution = params.secondary_offering_max_dilution.clamp(0.0, 1.0);
        let reserve_ratio = params.investor_cash_reserve_ratio.clamp(0.0, 1.0);

        let listed_ids: Vec<u16> = self.stock_market.listed.keys().copied().collect();

        for company_id in listed_ids {
            let Some(company) = self.companies.get(company_id as usize) else {
                continue;
            };
            if company.consecutive_missed_loan_payments < trigger {
                continue;
            }
            let last_price = self
                .stock_market
                .listed
                .get(&company_id)
                .map_or(0.0, |e| e.last_price);
            if last_price <= 0.0 {
                continue;
            }
            let shares_outstanding = self
                .stock_market
                .listed
                .get(&company_id)
                .map_or(0, |e| e.shares_outstanding);
            if shares_outstanding == 0 {
                continue;
            }

            let offer_price = round_cents(last_price * (1.0 - discount));
            if offer_price <= 0.0 {
                continue;
            }

            let total_to_offer = ((shares_outstanding as f64 * max_dilution).floor() as u64).max(1);
            let per_participant_max = (total_to_offer / 10).max(1);
            let mut subscriptions: Vec<(u32, u64, f64)> = Vec::new();
            let mut shares_allocated = 0_u64;
            let mut raised = 0.0_f64;

            for person in &self.people {
                if shares_allocated >= total_to_offer {
                    break;
                }
                let investable =
                    round_cents((person.cash_balance * (1.0 - reserve_ratio)).max(0.0));
                if investable < offer_price {
                    continue;
                }
                let affordable_shares = (investable / offer_price) as u64;
                let want = affordable_shares.min(per_participant_max);
                let available = total_to_offer - shares_allocated;
                let alloc = want.min(available);
                if alloc == 0 {
                    continue;
                }
                let cost = round_cents(alloc as f64 * offer_price);
                subscriptions.push((person.id, alloc, cost));
                shares_allocated += alloc;
                raised = round_cents(raised + cost);
            }

            let bank_equity_exposure = self.stock_market.bank_equity_exposure();
            let bank_headroom = round_cents(
                (self.bank.reserve_balance * params.bank_max_equity_reserve_fraction
                    - bank_equity_exposure)
                    .max(0.0),
            );
            let mut bank_alloc = 0_u64;
            let mut bank_cost = 0.0_f64;
            if shares_allocated < total_to_offer && bank_headroom >= offer_price {
                let remaining = total_to_offer - shares_allocated;
                let bank_affordable = (bank_headroom / offer_price) as u64;
                bank_alloc = bank_affordable.min(remaining);
                if bank_alloc > 0 {
                    bank_cost = round_cents(bank_alloc as f64 * offer_price);
                    shares_allocated += bank_alloc;
                    raised = round_cents(raised + bank_cost);
                }
            }

            if shares_allocated == 0 {
                continue;
            }

            for (person_id, alloc, cost) in &subscriptions {
                if let Some(person) = self.people.iter_mut().find(|p| p.id == *person_id) {
                    person.cash_balance = round_cents((person.cash_balance - cost).max(0.0));
                    self.stock_market
                        .person_portfolio_mut(*person_id)
                        .add_shares(company_id, *alloc, offer_price);
                }
            }
            if bank_alloc > 0 {
                self.bank.reserve_balance =
                    round_cents((self.bank.reserve_balance - bank_cost).max(0.0));
                self.stock_market
                    .bank_portfolio
                    .add_shares(company_id, bank_alloc, offer_price);
            }

            if let Some(equity) = self.stock_market.listed.get_mut(&company_id) {
                equity.shares_outstanding =
                    equity.shares_outstanding.saturating_add(shares_allocated);
                equity.free_float_shares =
                    equity.free_float_shares.saturating_add(shares_allocated);
            }

            if let Some(company) = self.companies.get_mut(company_id as usize) {
                company.cash_balance = round_cents(company.cash_balance + raised);
            }

            self.stock_market.ipo_raised_mtd =
                round_cents(self.stock_market.ipo_raised_mtd + raised);

            let ticker = self
                .stock_market
                .listed
                .get(&company_id)
                .map(|e| e.ticker.clone())
                .unwrap_or_default();
            self.pending_events.push(SimEvent::StockSecondaryOffering {
                country_name: self.name.clone(),
                company_id,
                ticker,
                shares_issued: shares_allocated,
                price: offer_price,
                total_raised: raised,
                day: self.day,
            });
        }
    }
}
