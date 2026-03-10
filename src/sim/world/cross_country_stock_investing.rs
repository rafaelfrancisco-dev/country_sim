use super::*;
use crate::sim::money::round_cents;
use crate::sim::stock_market::StockSecurityId;
use crate::sim::world::fx::CrossBorderFlowKind;

impl World {
    /// Monthly: each bank allocates part of its reserves into listed equities (domestic and
    /// foreign).  Banks issue new shares in each company (private placement at last price) so
    /// both the `free_float_shares` and `bank_portfolio` stay consistent with the share
    /// conservation invariant (`total_held == free_float_shares`).
    pub(in crate::sim::world) fn process_cross_country_bank_stock_investing(&mut self) {
        // Snapshot every tradeable security: (listing_country_idx, listing_country_id,
        // company_id, current_company_id, last_price)
        let securities: Vec<(usize, u16, u16, u16, f64)> = self
            .countries
            .iter()
            .enumerate()
            .flat_map(|(listing_idx, country)| {
                if !country.sim_params().stock_market.enabled {
                    return vec![];
                }
                country
                    .stock_market()
                    .listed
                    .iter()
                    .filter(|(_, equity)| equity.last_price > 0.0 && equity.free_float_shares > 0)
                    .map(|(&company_id, equity)| {
                        (
                            listing_idx,
                            country.id(),
                            company_id,
                            equity.current_company_id,
                            equity.last_price,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        if securities.is_empty() {
            return;
        }
        let num_securities = securities.len();

        for investor_idx in 0..self.countries.len() {
            let params = self.countries[investor_idx].sim_params().stock_market;
            if !params.enabled || !params.cross_country_bank_investing_enabled {
                continue;
            }
            let invest_fraction = params.cross_country_bank_invest_fraction_monthly;
            if invest_fraction <= 0.0 {
                continue;
            }

            let investor_country_id = self.countries[investor_idx].id();
            let Some(bank_id) = self.countries[investor_idx].stock_market_bank_id() else {
                continue;
            };
            let bank_reserve = self.countries[investor_idx]
                .bank_by_id(bank_id)
                .map_or(0.0, |b| b.reserve_balance);
            if bank_reserve <= 0.0 {
                continue;
            }

            let bank_equity_budget = bank_reserve * params.bank_max_equity_reserve_fraction;
            let bank_exposure = self.market_bank_portfolio_value(investor_country_id);
            let headroom = (bank_equity_budget - bank_exposure).max(0.0);
            let monthly_budget = (bank_reserve * invest_fraction).min(headroom);
            if monthly_budget < 1.0 {
                continue;
            }

            // Equal allocation across all tradeable securities.
            let budget_per_security = round_cents(monthly_budget / num_securities as f64);
            // Equal target holdings per security.
            let target_per_security_base = bank_equity_budget / num_securities as f64;

            for &(listing_idx, listing_country_id, company_id, current_company_id, last_price) in
                &securities
            {
                if budget_per_security < last_price || last_price <= 0.0 {
                    continue;
                }

                let security_id = StockSecurityId {
                    country_id: listing_country_id,
                    company_id,
                };

                // Only buy up to the per-security target.
                let target_shares = (target_per_security_base / last_price).floor() as u64;
                let current_holding = self.countries[investor_idx]
                    .stock_market()
                    .bank_portfolio
                    .position(security_id)
                    .map_or(0, |p| p.shares);
                let shares_needed = target_shares.saturating_sub(current_holding);
                if shares_needed == 0 {
                    continue;
                }

                let shares = ((budget_per_security / last_price) as u64).min(shares_needed);
                if shares == 0 {
                    continue;
                }

                let cost_listing_currency = round_cents(shares as f64 * last_price);

                // FX-convert cost to investor's local currency.
                let cost_local = if investor_idx == listing_idx {
                    cost_listing_currency
                } else {
                    let base = self.base_amount_from_local(listing_idx, cost_listing_currency);
                    self.local_amount_from_base(investor_idx, base)
                };

                // Verify bank has sufficient reserves.
                if !self.countries[investor_idx]
                    .bank_by_id(bank_id)
                    .is_some_and(|b| b.reserve_balance >= cost_local)
                {
                    continue;
                }

                // Cross-border FX settlement (no-op for same-country).
                if investor_idx != listing_idx {
                    self.settle_cross_border_cash(
                        CrossBorderFlowKind::StockTrade,
                        investor_idx,
                        listing_idx,
                        cost_local,
                    );
                }

                // Deduct from bank reserve.
                if let Some(bank) = self.countries[investor_idx].bank_by_id_mut(bank_id) {
                    bank.reserve_balance =
                        round_cents((bank.reserve_balance - cost_local).max(0.0));
                }

                // Add to bank portfolio.
                self.countries[investor_idx]
                    .stock_market_mut()
                    .bank_portfolio
                    .add_shares(security_id, shares, last_price);

                // Issue new shares (private placement): update equity and company cash so
                // the share-conservation invariant (total_held == free_float_shares) holds.
                if let Some(equity) = self.countries[listing_idx]
                    .stock_market_mut()
                    .listed
                    .get_mut(&company_id)
                {
                    equity.shares_outstanding = equity.shares_outstanding.saturating_add(shares);
                    equity.free_float_shares = equity.free_float_shares.saturating_add(shares);
                }
                // Company receives the proceeds (capital injection).
                if let Some(company) = self.countries[listing_idx]
                    .companies_mut()
                    .get_mut(current_company_id as usize)
                {
                    company.cash_balance =
                        round_cents(company.cash_balance + cost_listing_currency);
                }
            }
        }
    }
}
