use std::collections::HashMap;

use super::Country;

impl Country {
    pub fn currency_code(&self) -> &str {
        &self.currency.code
    }

    pub fn currency_name(&self) -> &str {
        &self.currency.name
    }

    pub fn currency_symbol(&self) -> &str {
        &self.currency.symbol
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
        self.fx_reserves_base
    }

    pub fn fx_reserves_by_source(&self) -> &HashMap<usize, f64> {
        &self.fx_reserves_by_source
    }

    pub fn fx_pressure(&self) -> f64 {
        self.fx_pressure
    }
}
