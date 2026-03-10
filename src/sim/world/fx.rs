use super::World;
use crate::sim::{event::SimEvent, money::round_cents};

const MIN_FX_RATE_LOCAL_PER_BASE: f64 = 0.25;
const MAX_FX_RATE_LOCAL_PER_BASE: f64 = 4.0;
const MAX_MONTHLY_FX_MOVE: f64 = 0.08;

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
        for (country_idx, country) in self.countries.iter_mut().enumerate() {
            let rate = 0.85 + (country_idx % 8) as f64 * 0.12;
            country.fx_rate_local_per_base =
                rate.clamp(MIN_FX_RATE_LOCAL_PER_BASE, MAX_FX_RATE_LOCAL_PER_BASE);
            let supply_base = if country.fx_rate_local_per_base > 0.0 {
                country.total_money_supply() / country.fx_rate_local_per_base
            } else {
                0.0
            };
            country.fx_reserves_base = round_cents((supply_base * 0.20).max(5_000.0));
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
        if payer_country_idx == payee_country_idx {
            return settlement;
        }

        {
            let payer = &mut self.countries[payer_country_idx];
            let old_total = payer.fx_reserves_base;
            payer.fx_reserves_base =
                round_cents((old_total - settlement.payer_amount_base).max(0.0));
            // Reduce per-source holdings proportionally to the spend.
            if old_total > 0.0 {
                let fraction_remaining = payer.fx_reserves_base / old_total;
                for v in payer.fx_reserves_by_source.values_mut() {
                    *v = round_cents(*v * fraction_remaining);
                }
            }
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
            payee.fx_reserves_base =
                round_cents(payee.fx_reserves_base + settlement.payer_amount_base);
            // Credit the inflow to the payer's source bucket.
            let entry = payee
                .fx_reserves_by_source
                .entry(payer_country_idx)
                .or_insert(0.0);
            *entry = round_cents(*entry + settlement.payer_amount_base);
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
        for country in &mut self.countries {
            let expected_reserves =
                (country.expected_money_supply() / country.fx_rate_local_per_base).max(1.0) * 0.15;
            let reserve_gap = if expected_reserves > 0.0 {
                (expected_reserves - country.fx_reserves_base) / expected_reserves
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

            if (new_rate - old_rate).abs() >= 0.01 {
                country.push_pending_event(SimEvent::FxRateChanged {
                    country_name: country.name().into(),
                    currency_code: country.currency.code.clone(),
                    old_rate,
                    new_rate,
                    pressure: country.fx_pressure,
                    reserves_base: country.fx_reserves_base,
                    day: country.day(),
                });
            }

            if reserve_gap > 0.35 {
                country.push_pending_event(SimEvent::FxReservePressure {
                    country_name: country.name().into(),
                    currency_code: country.currency.code.clone(),
                    reserves_base: country.fx_reserves_base,
                    reserve_gap: reserve_gap.clamp(0.0, 5.0),
                    pressure: country.fx_pressure,
                    day: country.day(),
                });
            }
        }
    }
}
