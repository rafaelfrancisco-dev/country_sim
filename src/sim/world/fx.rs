use super::World;
use crate::sim::{
    bank::Bank,
    event::SimEvent,
    money::{from_cents, round_cents, to_cents},
};

const MIN_FX_RATE_LOCAL_PER_BASE: f64 = 0.25;
const MAX_FX_RATE_LOCAL_PER_BASE: f64 = 4.0;
const MAX_MONTHLY_FX_MOVE: f64 = 0.08;
const MIN_INITIAL_FX_RESERVES_BASE: f64 = 10_000.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CrossBorderFlowKind {
    GoodsImport,
    ForeignDepositOpen,
    ForeignDepositInterest,
    ForeignDepositRecall,
    StockTrade,
    StockDividend,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FxSettlement {
    pub payer_amount_local: f64,
    pub payer_amount_base: f64,
    pub payee_amount_local: f64,
    pub fx_rate_payee_per_payer: f64,
}

impl World {
    pub(crate) fn initialize_fx_state(&mut self) {
        let country_count = self.countries.len();

        let mut issuer_rates: std::collections::HashMap<usize, f64> =
            std::collections::HashMap::new();
        let mut currency_issuers: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();

        for (country_idx, country) in self.countries.iter().enumerate() {
            if country.is_currency_issuer {
                let rate = 0.85 + (country_idx % 8) as f64 * 0.12;
                let fx_rate = rate.clamp(MIN_FX_RATE_LOCAL_PER_BASE, MAX_FX_RATE_LOCAL_PER_BASE);
                issuer_rates.insert(country.currency_idx, fx_rate);
                currency_issuers.insert(country.currency_idx, country_idx);
            }
        }

        // Pre-compute currency indices so the mutable loop below can reference them
        // without a simultaneous immutable borrow of self.countries.
        let currency_idx_by_pos: Vec<usize> =
            self.countries.iter().map(|c| c.currency_idx).collect();

        for (country_idx, country) in self.countries.iter_mut().enumerate() {
            country.fx_rate_local_per_base = issuer_rates
                .get(&country.currency_idx)
                .copied()
                .unwrap_or(1.0);

            let supply_base = if country.fx_rate_local_per_base > 0.0 {
                country.total_money_supply() / country.fx_rate_local_per_base
            } else {
                0.0
            };
            let fx_reserve_assets_base =
                round_cents((supply_base * 0.20).max(MIN_INITIAL_FX_RESERVES_BASE));
            let mut fx_reserve_assets_by_source = std::collections::HashMap::new();
            // Only hold reserves for countries that use a genuinely different currency.
            // Holding reserves in your own shared currency is meaningless.
            let own_currency_idx = country.currency_idx;
            let foreign_currency_sources: Vec<usize> = (0..country_count)
                .filter(|&src| src != country_idx && currency_idx_by_pos[src] != own_currency_idx)
                .collect();
            let foreign_currency_count = foreign_currency_sources.len();
            if foreign_currency_count > 0 {
                let total_reserve_cents = to_cents(fx_reserve_assets_base);
                let share_cents = total_reserve_cents / foreign_currency_count as i64;
                let mut remainder_cents = total_reserve_cents % foreign_currency_count as i64;

                for source_idx in foreign_currency_sources {
                    let mut amount_cents = share_cents;
                    if remainder_cents > 0 {
                        amount_cents += 1;
                        remainder_cents -= 1;
                    }

                    if amount_cents > 0 {
                        fx_reserve_assets_by_source.insert(source_idx, from_cents(amount_cents));
                    }
                }
            }
            let counterpart_local =
                round_cents(fx_reserve_assets_base * country.fx_rate_local_per_base);
            country.set_public_bank_fx_reserves(
                fx_reserve_assets_base,
                fx_reserve_assets_by_source,
                counterpart_local,
            );
            country.fx_pressure = 0.0;
        }
    }

    pub(crate) fn base_amount_from_local(&self, country_idx: usize, amount_local: f64) -> f64 {
        let rate = self
            .countries
            .get(country_idx)
            .map(|country| country.fx_rate_local_per_base)
            .unwrap_or(1.0)
            .max(0.0001);
        round_cents(amount_local / rate)
    }

    pub(crate) fn local_amount_from_base(&self, country_idx: usize, amount_base: f64) -> f64 {
        let rate = self
            .countries
            .get(country_idx)
            .map(|country| country.fx_rate_local_per_base)
            .unwrap_or(1.0)
            .max(0.0001);
        round_cents(amount_base * rate)
    }

    pub(crate) fn convert_between_countries(
        &self,
        from_country_idx: usize,
        to_country_idx: usize,
        amount_local: f64,
    ) -> FxSettlement {
        let payer_amount_local = round_cents(amount_local.max(0.0));

        let from_currency_idx = self.countries.get(from_country_idx).map(|c| c.currency_idx);
        let to_currency_idx = self.countries.get(to_country_idx).map(|c| c.currency_idx);

        if from_currency_idx.is_some() && from_currency_idx == to_currency_idx {
            return FxSettlement {
                payer_amount_local,
                payer_amount_base: payer_amount_local,
                payee_amount_local: payer_amount_local,
                fx_rate_payee_per_payer: 1.0,
            };
        }

        let payer_amount_base = self.base_amount_from_local(from_country_idx, payer_amount_local);
        let payee_amount_local = self.local_amount_from_base(to_country_idx, payer_amount_base);
        let fx_rate_payee_per_payer = if payer_amount_local > 0.0 {
            payee_amount_local / payer_amount_local
        } else {
            1.0
        };

        FxSettlement {
            payer_amount_local,
            payer_amount_base,
            payee_amount_local,
            fx_rate_payee_per_payer,
        }
    }

    pub(crate) fn settle_cross_border_cash(
        &mut self,
        kind: CrossBorderFlowKind,
        payer_country_idx: usize,
        payee_country_idx: usize,
        payer_amount_local: f64,
    ) -> FxSettlement {
        let settlement = self.convert_between_countries(
            payer_country_idx,
            payee_country_idx,
            payer_amount_local,
        );

        let from_currency_idx = self
            .countries
            .get(payer_country_idx)
            .map(|c| c.currency_idx);
        let to_currency_idx = self
            .countries
            .get(payee_country_idx)
            .map(|c| c.currency_idx);

        if payer_country_idx == payee_country_idx
            || (from_currency_idx.is_some() && from_currency_idx == to_currency_idx)
        {
            return settlement;
        }

        let payer_available_base = self.countries[payer_country_idx]
            .public_bank()
            .map(|bank| bank.fx_reserve_assets_base)
            .unwrap_or(0.0);
        if payer_available_base + 0.01 < settlement.payer_amount_base {
            let payer = &mut self.countries[payer_country_idx];
            payer.push_pending_event(SimEvent::FxReservePressure {
                country_name: payer.name().into(),
                currency_code: payer.currency_code().to_string().into(),
                reserves_base: payer.fx_reserves_base(),
                reserve_gap: if settlement.payer_amount_base > 0.0 {
                    ((settlement.payer_amount_base - payer_available_base)
                        / settlement.payer_amount_base)
                        .clamp(0.0, 5.0)
                } else {
                    0.0
                },
                pressure: payer.fx_pressure(),
                day: payer.day(),
            });
            return FxSettlement {
                payer_amount_local: 0.0,
                payer_amount_base: 0.0,
                payee_amount_local: 0.0,
                fx_rate_payee_per_payer: settlement.fx_rate_payee_per_payer,
            };
        }

        {
            let payer = &mut self.countries[payer_country_idx];
            let bank = payer.primary_bank_mut();
            spend_fx_reserves(bank, payee_country_idx, settlement.payer_amount_base);
            bank.absorb_managed_fx_liquidity(settlement.payer_amount_local);
            match kind {
                CrossBorderFlowKind::GoodsImport => {
                    payer.monthly_goods_import_spending_base = round_cents(
                        payer.monthly_goods_import_spending_base + settlement.payer_amount_base,
                    );
                }
                _ => {
                    payer.monthly_fx_financial_outflows_base = round_cents(
                        payer.monthly_fx_financial_outflows_base + settlement.payer_amount_base,
                    );
                }
            }
            payer.monthly_fx_net_flow_base =
                round_cents(payer.monthly_fx_net_flow_base - settlement.payer_amount_base);
        }

        {
            let payee = &mut self.countries[payee_country_idx];
            let bank = payee.primary_bank_mut();
            bank.fx_reserve_assets_base =
                round_cents(bank.fx_reserve_assets_base + settlement.payer_amount_base);
            let entry = bank
                .fx_reserve_assets_by_source
                .entry(payer_country_idx)
                .or_insert(0.0);
            *entry = round_cents(*entry + settlement.payer_amount_base);
            bank.issue_managed_fx_liquidity(settlement.payee_amount_local);
            match kind {
                CrossBorderFlowKind::GoodsImport => {
                    payee.monthly_goods_export_revenue_base = round_cents(
                        payee.monthly_goods_export_revenue_base + settlement.payer_amount_base,
                    );
                }
                _ => {
                    payee.monthly_fx_financial_inflows_base = round_cents(
                        payee.monthly_fx_financial_inflows_base + settlement.payer_amount_base,
                    );
                }
            }
            payee.monthly_fx_net_flow_base =
                round_cents(payee.monthly_fx_net_flow_base + settlement.payer_amount_base);
        }

        settlement
    }

    pub(super) fn update_fx_rates(&mut self) {
        // Pass 1: update issuer countries and record their new rates.
        let mut issuer_rates: std::collections::HashMap<usize, f64> =
            std::collections::HashMap::new();

        for country in &mut self.countries {
            if !country.is_currency_issuer {
                continue;
            }

            let expected_reserves =
                (country.expected_money_supply() / country.fx_rate_local_per_base).max(1.0) * 0.15;
            let reserve_gap = if expected_reserves > 0.0 {
                (expected_reserves - country.fx_reserves_base()) / expected_reserves
            } else {
                0.0
            };
            let external_balance = if expected_reserves > 0.0 {
                -country.monthly_fx_net_flow_base / expected_reserves
            } else {
                0.0
            };
            let inflation_drag = country.monthly_inflation_rate().clamp(-0.10, 0.30) * 0.1;
            let raw_pressure =
                (reserve_gap * 0.55 + external_balance * 0.35 + inflation_drag).clamp(-1.0, 1.0);
            country.fx_pressure = raw_pressure;

            let move_frac = (raw_pressure * MAX_MONTHLY_FX_MOVE)
                .clamp(-MAX_MONTHLY_FX_MOVE, MAX_MONTHLY_FX_MOVE);
            let old_rate = country.fx_rate_local_per_base;
            let new_rate = round_cents(
                (old_rate * (1.0 + move_frac))
                    .clamp(MIN_FX_RATE_LOCAL_PER_BASE, MAX_FX_RATE_LOCAL_PER_BASE),
            );
            country.fx_rate_local_per_base = new_rate;
            issuer_rates.insert(country.currency_idx, new_rate);

            if (new_rate - old_rate).abs() >= 0.01 {
                country.push_pending_event(SimEvent::FxRateChanged {
                    country_name: country.name().into(),
                    currency_code: country.currency_spec.code.clone(),
                    old_rate,
                    new_rate,
                    pressure: country.fx_pressure,
                    reserves_base: country.fx_reserves_base(),
                    day: country.day(),
                });
            }

            if reserve_gap > 0.35 {
                country.push_pending_event(SimEvent::FxReservePressure {
                    country_name: country.name().into(),
                    currency_code: country.currency_spec.code.clone(),
                    reserves_base: country.fx_reserves_base(),
                    reserve_gap: reserve_gap.clamp(0.0, 5.0),
                    pressure: country.fx_pressure,
                    day: country.day(),
                });
            }
        }

        // Pass 2: non-issuer countries inherit the issuer's updated rate exactly.
        for country in &mut self.countries {
            if country.is_currency_issuer {
                continue;
            }
            if let Some(&issuer_rate) = issuer_rates.get(&country.currency_idx) {
                country.fx_rate_local_per_base = issuer_rate;
                country.fx_pressure = 0.0;
            }
        }
    }
}

fn spend_fx_reserves(bank: &mut Bank, target_country_idx: usize, amount_base: f64) {
    let old_total = bank.fx_reserve_assets_base;
    bank.fx_reserve_assets_base = round_cents((old_total - amount_base).max(0.0));
    let actual_spent = old_total - bank.fx_reserve_assets_base;
    if actual_spent <= 0.0 {
        return;
    }

    let target_held = bank
        .fx_reserve_assets_by_source
        .get(&target_country_idx)
        .copied()
        .unwrap_or(0.0);
    let from_target = actual_spent.min(target_held);
    if from_target > 0.0 {
        let entry = bank
            .fx_reserve_assets_by_source
            .entry(target_country_idx)
            .or_insert(0.0);
        *entry = round_cents(*entry - from_target);
    }

    let leftover = actual_spent - from_target;
    if leftover > 0.0 {
        let remaining_pool = old_total - from_target;
        if remaining_pool > 0.0 {
            let fraction = (remaining_pool - leftover) / remaining_pool;
            for (&source_idx, value) in &mut bank.fx_reserve_assets_by_source {
                if source_idx != target_country_idx {
                    *value = round_cents(*value * fraction);
                }
            }
        }
    }
    bank.fx_reserve_assets_by_source
        .retain(|_, amount| amount.abs() >= 0.01);
}
