use super::*;

impl Country {
    pub(super) fn process_auto_money_minting(&mut self) {
        let monetary = self.sim_params.monetary;
        let inflation_shortfall =
            (monetary.target_inflation_rate - self.monthly_inflation_rate).max(0.0);
        let raw_unemployment_gap =
            (self.labor_force_unemployment_rate() - monetary.natural_unemployment_rate).max(0.0);
        let inflation_overshoot_threshold =
            monetary.target_inflation_rate * monetary.mint_unemployment_inflation_cap_multiplier;
        let unemployment_gap = if self.monthly_inflation_rate > inflation_overshoot_threshold {
            0.0
        } else {
            raw_unemployment_gap
        };

        let mint_fraction_raw = inflation_shortfall * monetary.mint_inflation_gap_weight.max(0.0)
            + unemployment_gap * monetary.mint_unemployment_gap_weight.max(0.0);
        let mint_cap_fraction = monetary.mint_max_fraction_expected_supply.clamp(0.0, 1.0);
        let mint_fraction = mint_fraction_raw.clamp(0.0, mint_cap_fraction);

        self.last_mint_inflation_shortfall = inflation_shortfall;
        self.last_mint_unemployment_gap = unemployment_gap;
        self.last_mint_fraction = mint_fraction;

        if mint_fraction <= 0.0 {
            return;
        }

        let mint_amount = round_cents(self.expected_money_supply.max(0.0) * mint_fraction);
        if mint_amount <= 0.0 {
            return;
        }

        let primary_bank = self.primary_bank_mut();
        primary_bank.reserve_balance = round_cents(primary_bank.reserve_balance + mint_amount);
        self.monthly_money_minted = round_cents(self.monthly_money_minted + mint_amount);
        self.cumulative_money_minted = round_cents(self.cumulative_money_minted + mint_amount);

        self.pending_events.push(SimEvent::MoneyMinted {
            country_name: self.name.clone(),
            amount: mint_amount,
            inflation_shortfall,
            unemployment_gap,
            day: self.day,
        });
    }
}
