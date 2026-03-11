use super::Country;

impl Country {
    pub fn last_completed_month_tax_collected(&self) -> f64 {
        self.last_completed_month_tax_collected
    }

    pub fn last_completed_month_household_spending(&self) -> f64 {
        self.last_completed_month_household_spending
    }

    pub fn last_completed_month_tariff_collected(&self) -> f64 {
        self.last_completed_month_tariff_collected
    }

    pub fn last_completed_month_gov_spending(&self) -> f64 {
        self.last_completed_month_gov_spending
    }

    pub fn last_completed_month_budget_balance(&self) -> f64 {
        self.last_completed_month_budget_balance
    }

    pub fn last_completed_month_coupon_paid(&self) -> f64 {
        self.last_completed_month_coupon_paid
    }

    pub fn last_completed_month_bonds_issued(&self) -> f64 {
        self.last_completed_month_bonds_issued
    }

    pub fn last_completed_month_bonds_matured(&self) -> f64 {
        self.last_completed_month_bonds_matured
    }

    pub fn last_completed_month_savings_interest_paid(&self) -> f64 {
        self.last_completed_month_savings_interest_paid
    }

    pub fn last_completed_month_fx_net_flow_base(&self) -> f64 {
        self.last_completed_month_fx_net_flow_base
    }

    pub fn last_completed_month_rent_collected(&self) -> f64 {
        self.last_completed_month_rent_collected
    }

    pub fn last_completed_month_mortgage_payments(&self) -> f64 {
        self.last_completed_month_mortgage_payments
    }

    pub fn last_completed_month_home_purchases(&self) -> u32 {
        self.last_completed_month_home_purchases
    }

    pub fn last_completed_month_foreclosures(&self) -> u32 {
        self.last_completed_month_foreclosures
    }

    pub fn last_completed_month_essential_debt_spending(&self) -> f64 {
        self.last_completed_month_essential_debt_spending
    }

    pub fn last_completed_month_hires_total(&self) -> u64 {
        self.last_completed_month_hires_total
    }

    pub fn last_completed_month_money_minted(&self) -> f64 {
        self.last_completed_month_money_minted
    }
}
