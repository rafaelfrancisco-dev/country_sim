use super::*;

impl Country {
    /// Recompute the CPI price index using the essential-goods basket (goods 0–7).
    /// Updates `cpi_index`, `last_month_cpi_index`, and `monthly_inflation_rate`.
    pub(super) fn recompute_cpi(&mut self) {
        let mut weighted_sum = 0.0_f64;
        let mut weight_total = 0.0_f64;

        for good_id in 0u8..=7u8 {
            let Some(good) = good_by_id(good_id) else {
                continue;
            };
            if good.base_unit_cost <= 0.0 {
                continue;
            }
            let avg_price = self.average_price_for_good(good_id);
            let relative_price = avg_price / good.base_unit_cost;
            weighted_sum += relative_price * good.base_unit_cost;
            weight_total += good.base_unit_cost;
        }

        let new_cpi = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            1.0
        };

        let old_cpi = self.cpi_index;
        self.last_month_cpi_index = old_cpi;
        self.cpi_index = new_cpi;
        if old_cpi > 0.0 {
            // Annualized month-over-month rate
            self.monthly_inflation_rate = (new_cpi / old_cpi).powf(12.0) - 1.0;
        }
    }

    /// Pay monthly coupon on outstanding bonds and handle maturity if the countdown hits zero.
    pub(super) fn process_bond_coupon_and_maturity(&mut self) {
        if self.bonds_outstanding <= 0.0 {
            return;
        }

        let bank_fraction = self.sim_params.monetary.bank_primary_dealer_fraction;
        let monthly_coupon = round_cents(self.bonds_outstanding * self.bond_coupon_rate / 12.0);
        let bank_coupon = round_cents(monthly_coupon * bank_fraction);
        let retail_coupon = round_cents(monthly_coupon - bank_coupon);

        // Treasury pays total coupon
        self.treasury_balance = round_cents(self.treasury_balance - monthly_coupon);
        self.monthly_coupon_paid = round_cents(self.monthly_coupon_paid + monthly_coupon);

        // Bank portion flows to bank reserve
        self.bank.reserve_balance = round_cents(self.bank.reserve_balance + bank_coupon);

        // Retail portion distributed evenly to the top-N wealthiest people
        if retail_coupon > 0.0 {
            self.distribute_to_top_n_people(retail_coupon);
        }

        self.pending_events.push(SimEvent::CouponPaid {
            country_name: self.name.clone(),
            total_coupon: monthly_coupon,
            day: self.day,
        });

        // Countdown to maturity
        if self.bond_months_to_maturity > 0 {
            self.bond_months_to_maturity -= 1;
        }

        // Handle maturity
        if self.bond_months_to_maturity == 0 {
            let face_value = self.bonds_outstanding;
            let available_treasury = self.treasury_balance.max(0.0);

            if available_treasury >= face_value {
                // Full repayment
                self.treasury_balance = round_cents(self.treasury_balance - face_value);
                let bank_repay = round_cents(face_value * bank_fraction);
                let retail_repay = round_cents(face_value - bank_repay);
                self.bank.reserve_balance = round_cents(self.bank.reserve_balance + bank_repay);
                if retail_repay > 0.0 {
                    self.distribute_to_top_n_people(retail_repay);
                }
                self.pending_events.push(SimEvent::BondMatured {
                    country_name: self.name.clone(),
                    face_value,
                    day: self.day,
                });
            } else {
                // Partial treasury + monetized shortfall
                let shortfall = round_cents(face_value - available_treasury);
                self.treasury_balance = round_cents(self.treasury_balance - available_treasury);
                let monetized = self.bank.fund_government_deficit(shortfall);
                self.gov_debt = round_cents(self.gov_debt + monetized);
                let total_paid = round_cents(available_treasury + monetized);
                let bank_repay = round_cents(total_paid * bank_fraction);
                let retail_repay = round_cents(total_paid - bank_repay);
                self.bank.reserve_balance = round_cents(self.bank.reserve_balance + bank_repay);
                if retail_repay > 0.0 {
                    self.distribute_to_top_n_people(retail_repay);
                }
                self.pending_events.push(SimEvent::BondDefault {
                    country_name: self.name.clone(),
                    shortfall,
                    monetized,
                    day: self.day,
                });
            }

            self.monthly_bonds_matured = round_cents(self.monthly_bonds_matured + face_value);
            self.bonds_outstanding = 0.0;
            self.bond_coupon_rate = 0.0;
            self.bond_months_to_maturity = 0;
        }
    }

    /// Issue sovereign bonds if the treasury is at or below the threshold and budget is in deficit.
    pub(super) fn maybe_issue_bonds(&mut self) {
        if self.bonds_outstanding > 0.0 {
            return;
        }
        let threshold = self.sim_params.monetary.bond_issuance_treasury_threshold;
        if self.treasury_balance > threshold {
            return;
        }
        if self.monthly_budget_balance >= 0.0 {
            return;
        }

        let deficit_coverage = (-self.monthly_budget_balance).abs();
        let treasury_restore = (threshold - self.treasury_balance).max(0.0);
        let face_value = round_cents(deficit_coverage + treasury_restore);
        if face_value <= 0.0 {
            return;
        }

        let monetary = self.sim_params.monetary;
        let coupon_rate = (self.policy_rate + monetary.sovereign_spread)
            .clamp(monetary.rate_floor, monetary.rate_ceiling);
        let bank_fraction = monetary.bank_primary_dealer_fraction;
        let bank_portion = round_cents(face_value * bank_fraction);
        let retail_portion = round_cents(face_value - bank_portion);

        // Bank provides its portion; reserve decreases
        let bank_funded = if self.bank.reserve_balance >= bank_portion {
            self.bank.reserve_balance = round_cents(self.bank.reserve_balance - bank_portion);
            bank_portion
        } else {
            let available = self.bank.reserve_balance.max(0.0);
            self.bank.reserve_balance = 0.0;
            available
        };
        self.treasury_balance = round_cents(self.treasury_balance + bank_funded);

        // Retail portion deducted from top-N wealthiest people
        let retail_funded = if retail_portion > 0.0 {
            self.collect_from_top_n_people(retail_portion, monetary.bond_retail_top_n)
        } else {
            0.0
        };
        self.treasury_balance = round_cents(self.treasury_balance + retail_funded);

        let total_funded = round_cents(bank_funded + retail_funded);
        if total_funded <= 0.0 {
            return;
        }

        self.bonds_outstanding = total_funded;
        self.bond_coupon_rate = coupon_rate;
        self.bond_months_to_maturity = monetary.bond_maturity_months;
        self.monthly_bonds_issued = round_cents(self.monthly_bonds_issued + total_funded);

        self.pending_events.push(SimEvent::BondIssued {
            country_name: self.name.clone(),
            face_value: total_funded,
            coupon_rate,
            day: self.day,
        });
    }

    /// Distribute `amount` evenly among the top-N wealthiest people (by cash balance).
    fn distribute_to_top_n_people(&mut self, amount: f64) {
        let top_n = self.sim_params.monetary.bond_retail_top_n;
        let mut indices: Vec<usize> = (0..self.people.len()).collect();
        indices.sort_by(|&a, &b| {
            self.people[b]
                .cash_balance
                .partial_cmp(&self.people[a].cash_balance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        indices.truncate(top_n);
        if indices.is_empty() {
            return;
        }
        let per_person = round_cents(amount / indices.len() as f64);
        for idx in indices {
            if let Some(person) = self.people.get_mut(idx) {
                person.cash_balance = round_cents(person.cash_balance + per_person);
            }
        }
    }

    /// Collect `amount` evenly from the top-N wealthiest people (capped by their cash balance).
    /// Returns the total actually collected.
    fn collect_from_top_n_people(&mut self, amount: f64, top_n: usize) -> f64 {
        let mut indices: Vec<usize> = (0..self.people.len()).collect();
        indices.sort_by(|&a, &b| {
            self.people[b]
                .cash_balance
                .partial_cmp(&self.people[a].cash_balance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        indices.truncate(top_n);
        if indices.is_empty() {
            return 0.0;
        }
        let per_person = round_cents(amount / indices.len() as f64);
        let mut total_collected = 0.0;
        for idx in indices {
            if let Some(person) = self.people.get_mut(idx) {
                let contribution = per_person.min(person.cash_balance.max(0.0));
                person.cash_balance = round_cents(person.cash_balance - contribution);
                total_collected = round_cents(total_collected + contribution);
            }
        }
        total_collected
    }
}
