use crate::sim::bank::Bank;
use crate::sim::money::round_cents;

impl Bank {
    pub fn can_fund_withdrawal(&self, amount: f64) -> bool {
        round_cents(amount.max(0.0)) <= round_cents(self.reserve_balance.max(0.0)) + 0.01
    }

    pub fn liquidity_ratio(&self) -> f64 {
        if self.deposit_liabilities <= 0.0 {
            return 1.0;
        }
        (self.reserve_balance.max(0.0) / self.deposit_liabilities.max(0.01)).clamp(0.0, 10.0)
    }

    pub fn capital_ratio(&self) -> f64 {
        let assets = self.reserve_balance.max(0.0) + self.created_money_outstanding.max(0.0);
        let liabilities = self.deposit_liabilities.max(0.0);
        if assets <= 0.0 {
            return 0.0;
        }
        ((assets - liabilities) / assets).clamp(-1.0, 1.0)
    }

    pub fn health_score(&self) -> f64 {
        self.health_score_private()
    }

    pub fn health_score_private(&self) -> f64 {
        let liquidity = (self.liquidity_ratio() / 0.20).clamp(0.0, 1.0);
        let capital = ((self.capital_ratio() + 0.20) / 0.45).clamp(0.0, 1.0);
        let resolution_penalty = if self.is_in_resolution { 0.20 } else { 0.0 };
        let run_penalty = self.run_intensity.clamp(0.0, 1.0) * 0.30;
        (0.55 * liquidity + 0.45 * capital - resolution_penalty - run_penalty).clamp(0.0, 1.0)
    }

    pub fn health_score_public(
        &self,
        treasury_balance: f64,
        health_floor: f64,
        liquidity_weight: f64,
        capital_weight: f64,
        sovereign_support_weight: f64,
    ) -> f64 {
        let liquidity = (self.liquidity_ratio() / 0.15).clamp(0.0, 1.0);
        let capital = ((self.capital_ratio() + 0.15) / 0.35).clamp(0.0, 1.0);
        let support_base = self.deposit_liabilities.max(0.0) * 0.20 + 1.0;
        let sovereign_support = (treasury_balance.max(0.0) / support_base).clamp(0.0, 1.0);
        let weights_sum =
            (liquidity_weight + capital_weight + sovereign_support_weight).max(f64::EPSILON);
        let weighted = (liquidity * liquidity_weight
            + capital * capital_weight
            + sovereign_support * sovereign_support_weight)
            / weights_sum;
        let resolution_penalty = if self.is_in_resolution { 0.12 } else { 0.0 };
        let run_penalty = self.run_intensity.clamp(0.0, 1.0) * 0.18;
        let stress_adjusted = (weighted - resolution_penalty - run_penalty).clamp(0.0, 1.0);
        if self.is_in_resolution {
            stress_adjusted
        } else {
            stress_adjusted.max(health_floor.clamp(0.0, 1.0))
        }
    }

    pub fn update_run_intensity(&mut self, stress: f64) {
        let target = stress.clamp(0.0, 1.0);
        let smoothed = 0.65 * self.run_intensity + 0.35 * target;
        self.run_intensity = round_cents(smoothed.clamp(0.0, 1.0));
    }

    pub fn set_resolution_state(&mut self, in_resolution: bool) {
        if in_resolution {
            if self.is_in_resolution {
                self.months_in_resolution = self.months_in_resolution.saturating_add(1);
            } else {
                self.months_in_resolution = 1;
            }
        } else {
            self.months_in_resolution = 0;
        }
        self.is_in_resolution = in_resolution;
    }

    pub fn apply_resolution_signal(
        &mut self,
        resolution_required: bool,
        entry_months: u16,
        exit_months: u16,
    ) {
        if resolution_required {
            self.resolution_distress_streak = self.resolution_distress_streak.saturating_add(1);
            self.resolution_stable_streak = 0;
        } else {
            self.resolution_stable_streak = self.resolution_stable_streak.saturating_add(1);
            self.resolution_distress_streak = 0;
        }

        let entry_months = entry_months.max(1);
        let exit_months = exit_months.max(1);
        if self.is_in_resolution {
            if resolution_required {
                self.set_resolution_state(true);
            } else if self.resolution_stable_streak >= exit_months {
                self.set_resolution_state(false);
            }
            return;
        }

        if resolution_required && self.resolution_distress_streak >= entry_months {
            self.set_resolution_state(true);
        }
    }
}
