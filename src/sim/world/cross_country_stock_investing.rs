use super::*;
use crate::sim::{bank::BankSecurityId, money::round_cents};

impl World {
    /// Monthly: let all banks allocate part of their reserves to listed equities
    /// in foreign countries.
    pub(in crate::sim::world) fn process_cross_country_bank_stock_investing(&mut self) {
        if self.countries.len() < 2 {
            return;
        }

        let country_count = self.countries.len();
        for home_idx in 0..country_count {
            let Some(home_country) = self.countries.get(home_idx) else {
                continue;
            };
            let params = home_country.sim_params.stock_market;
            if !params.enabled || !params.cross_country_bank_investing_enabled {
                continue;
            }

            let invest_fraction = params.cross_country_bank_invest_fraction_monthly.clamp(0.0, 1.0);
            if invest_fraction <= 0.0 {
                continue;
            }

            let target_indices = (0..country_count)
                .filter(|idx| {
                    *idx != home_idx
                        && self
                            .countries
                            .get(*idx)
                            .is_some_and(|country| !country.stock_market.listed.is_empty())
                })
                .collect::<Vec<_>>();
            if target_indices.is_empty() {
                continue;
            }

            let bank_ids = std::iter::once(home_country.bank.id)
                .chain(home_country.banks.iter().map(|bank| bank.id))
                .collect::<Vec<_>>();

            for bank_id in bank_ids {
                let reserve_balance = self
                    .countries
                    .get(home_idx)
                    .and_then(|country| country.bank_by_id(bank_id))
                    .map_or(0.0, |bank| bank.reserve_balance.max(0.0));
                let created_money_outstanding = self
                    .countries
                    .get(home_idx)
                    .and_then(|country| country.bank_by_id(bank_id))
                    .map_or(0.0, |bank| bank.created_money_outstanding.max(0.0));
                let deployable_capacity = round_cents(reserve_balance + created_money_outstanding * 0.10);
                if deployable_capacity <= 0.0 {
                    continue;
                }

                let total_budget = round_cents(deployable_capacity * invest_fraction);
                if total_budget <= 0.0 {
                    continue;
                }
                let per_country_budget = round_cents(total_budget / target_indices.len() as f64);
                if per_country_budget <= 0.0 {
                    continue;
                }

                for &target_idx in &target_indices {
                    let Some((company_id, price)) = self
                        .countries
                        .get(target_idx)
                        .and_then(|country| {
                            country
                                .stock_market
                                .listed
                                .values()
                                .filter(|equity| equity.last_price > 0.0)
                                .min_by(|a, b| {
                                    a.last_price
                                        .partial_cmp(&b.last_price)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .map(|equity| (equity.company_id, equity.last_price))
                        })
                    else {
                        continue;
                    };
                    if price <= 0.0 {
                        continue;
                    }

                    let mut shares = (per_country_budget / price).floor() as u64;
                    if shares == 0 && deployable_capacity + 0.01 >= price {
                        shares = 1;
                    }
                    if shares == 0 {
                        continue;
                    }
                    let cost = round_cents(shares as f64 * price);
                    if cost <= 0.0 {
                        continue;
                    }

                    let Some((home_country_mut, target_country_mut)) =
                        self.country_pair_mut(home_idx, target_idx)
                    else {
                        continue;
                    };

                    let available_home_reserve = home_country_mut
                        .bank_by_id(bank_id)
                        .map_or(0.0, |bank| bank.reserve_balance.max(0.0));
                    if available_home_reserve + 0.01 < cost {
                        let shortfall = round_cents((cost - available_home_reserve).max(0.0));
                        if let Some(bank) = home_country_mut.bank_by_id_mut(bank_id) {
                            let minted = bank.fund_government_deficit(shortfall);
                            bank.reserve_balance = round_cents(bank.reserve_balance + minted);
                        }
                    }

                    let target_country_id = target_country_mut.id;
                    if let Some(bank) = home_country_mut.bank_by_id_mut(bank_id) {
                        bank.reserve_balance = round_cents((bank.reserve_balance - cost).max(0.0));
                        bank.add_stock_shares(
                            BankSecurityId {
                                country_id: target_country_id,
                                company_id,
                            },
                            shares,
                            price,
                        );
                    }

                    let settlement_bank_id = target_country_mut
                        .stock_market_bank_id()
                        .unwrap_or(target_country_mut.bank.id);
                    if let Some(settlement_bank) = target_country_mut.bank_by_id_mut(settlement_bank_id)
                    {
                        settlement_bank.reserve_balance =
                            round_cents(settlement_bank.reserve_balance + cost);
                    }

                    if let Some(equity) = target_country_mut.stock_market.listed.get_mut(&company_id) {
                        equity.record_trade(shares, price);
                    }
                }
            }
        }
    }

    fn country_pair_mut(
        &mut self,
        first: usize,
        second: usize,
    ) -> Option<(&mut Country, &mut Country)> {
        if first == second {
            return None;
        }
        if first < second {
            let (left, right) = self.countries.split_at_mut(second);
            Some((&mut left[first], &mut right[0]))
        } else {
            let (left, right) = self.countries.split_at_mut(first);
            Some((&mut right[0], &mut left[second]))
        }
    }
}
