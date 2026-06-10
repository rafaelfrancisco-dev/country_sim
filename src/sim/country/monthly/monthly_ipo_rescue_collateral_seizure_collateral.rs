use super::*;

impl Country {
    pub(crate) fn process_stock_collateral_seizure(&mut self) {
        if !self.sim_params.stock_market.enabled {
            return;
        }

        let discount = self.sim_params.stock_market.margin_liquidation_discount;
        let people_with_loans: Vec<(u32, f64)> = self
            .people
            .iter()
            .filter_map(|p| p.loan.as_ref().map(|loan| (p.id, loan.monthly_payment)))
            .collect();

        // Pledges are recomputed from scratch every month: release everything first so
        // holdings unlock once a loan is repaid, then re-pledge for current borrowers.
        for portfolio in self.stock_market.person_portfolios.values_mut() {
            for position in &mut portfolio.positions {
                position.pledged_shares = 0;
            }
        }

        // The bank pays cash for seized shares; it can only buy what its reserves cover.
        let mut bank_reserve_remaining = self.primary_bank().reserve_balance.max(0.0);

        for (person_id, monthly_payment) in people_with_loans {
            let Some(person_idx) = self.runtime.person_idx(person_id) else {
                continue;
            };
            let person = &mut self.people[person_idx];

            if let Some(portfolio) = self.stock_market.person_portfolios.get_mut(&person_id) {
                portfolio.pledge_all();
            }

            let cash = person.cash_balance.max(0.0);
            if cash >= monthly_payment {
                continue;
            }
            let shortfall = round_cents(monthly_payment - cash);
            let Some(portfolio) = self.stock_market.person_portfolios.get_mut(&person_id) else {
                continue;
            };

            let mut proceeds = 0.0_f64;
            let mut total_liquidated_shares = 0_u64;
            let company_ids: Vec<u16> = portfolio
                .positions
                .iter()
                .filter(|pos| pos.pledged_shares > 0 && pos.security_id.country_id == self.id)
                .map(|pos| pos.security_id.company_id)
                .collect();

            for cid in company_ids {
                if proceeds >= shortfall {
                    break;
                }
                let last_price = self
                    .stock_market
                    .listed
                    .get(&cid)
                    .map_or(0.0, |e| e.last_price);
                if last_price <= 0.0 {
                    continue;
                }
                let sale_price = round_cents(last_price * (1.0 - discount));
                let Some(portfolio) = self.stock_market.person_portfolios.get_mut(&person_id)
                else {
                    break;
                };
                let Some(pos) = portfolio.positions.iter().find(|p| {
                    p.security_id
                        == crate::sim::stock_market::StockSecurityId {
                            country_id: self.id,
                            company_id: cid,
                        }
                }) else {
                    continue;
                };
                let remaining_need = round_cents(shortfall - proceeds);
                let shares_needed = ((remaining_need / sale_price).ceil() as u64).max(1);
                let bank_affordable_shares = if sale_price > 0.0 {
                    (bank_reserve_remaining / sale_price).floor() as u64
                } else {
                    0
                };
                let shares_to_sell = shares_needed
                    .min(pos.pledged_shares)
                    .min(bank_affordable_shares);
                if shares_to_sell == 0 {
                    continue;
                }
                let sale_proceeds = round_cents(shares_to_sell as f64 * sale_price);
                portfolio.remove_shares(
                    crate::sim::stock_market::StockSecurityId {
                        country_id: self.id,
                        company_id: cid,
                    },
                    shares_to_sell,
                    sale_price,
                );
                proceeds = round_cents(proceeds + sale_proceeds);
                total_liquidated_shares = total_liquidated_shares.saturating_add(shares_to_sell);

                if let Some(person_idx) = self.runtime.person_idx(person_id) {
                    self.people[person_idx].cash_balance =
                        round_cents(self.people[person_idx].cash_balance + sale_proceeds);
                }
                self.stock_market.bank_portfolio.add_shares(
                    crate::sim::stock_market::StockSecurityId {
                        country_id: self.id,
                        company_id: cid,
                    },
                    shares_to_sell,
                    sale_price,
                );
                bank_reserve_remaining =
                    round_cents((bank_reserve_remaining - sale_proceeds).max(0.0));
                let primary_bank = self.primary_bank_mut();
                primary_bank.reserve_balance =
                    round_cents(primary_bank.reserve_balance - sale_proceeds).max(0.0);
            }

            if total_liquidated_shares > 0 {
                self.stock_market.forced_liquidations_mtd =
                    round_cents(self.stock_market.forced_liquidations_mtd + proceeds);
                self.pending_events.push(SimEvent::StockMarginCall {
                    country_name: self.name.clone(),
                    person_id,
                    shares_liquidated: total_liquidated_shares,
                    proceeds,
                    day: self.day,
                });
            }
        }

        self.stock_market.gc_portfolios();
    }
}
