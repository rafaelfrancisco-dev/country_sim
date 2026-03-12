use std::collections::HashMap;

use super::Country;

impl Country {
    pub fn currency_code(&self) -> &str {
        &self.currency_spec.code
    }

    pub fn currency_name(&self) -> &str {
        &self.currency_spec.name
    }

    pub fn currency_symbol(&self) -> &str {
        &self.currency_spec.symbol
    }

    pub fn currency_idx(&self) -> usize {
        self.currency_idx
    }

    pub fn is_currency_issuer(&self) -> bool {
        self.is_currency_issuer
    }

    pub fn monthly_goods_import_spending_base(&self) -> f64 {
        self.monthly_goods_import_spending_base
    }

    pub fn monthly_goods_export_revenue_base(&self) -> f64 {
        self.monthly_goods_export_revenue_base
    }

    pub fn monthly_fx_financial_inflows_base(&self) -> f64 {
        self.monthly_fx_financial_inflows_base
    }

    pub fn monthly_fx_financial_outflows_base(&self) -> f64 {
        self.monthly_fx_financial_outflows_base
    }

    pub fn monthly_fx_net_flow_base(&self) -> f64 {
        self.monthly_fx_net_flow_base
    }

    pub fn fx_rate_local_per_base(&self) -> f64 {
        self.fx_rate_local_per_base
    }

    pub fn fx_reserves_base(&self) -> f64 {
        self.public_bank()
            .map(|bank| bank.fx_reserve_assets_base)
            .unwrap_or(0.0)
    }

    pub fn fx_reserves_by_source(&self) -> &HashMap<usize, f64> {
        self.public_bank()
            .map(|bank| &bank.fx_reserve_assets_by_source)
            .unwrap_or_else(|| {
                self.banks
                    .first()
                    .map(|bank| &bank.fx_reserve_assets_by_source)
                    .expect("country should always have at least one bank")
            })
    }

    pub fn fx_pressure(&self) -> f64 {
        self.fx_pressure
    }
}
