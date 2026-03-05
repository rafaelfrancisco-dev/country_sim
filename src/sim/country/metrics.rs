use super::*;
impl Country {
    pub fn recompute_aggregates(&mut self) {
        let household_chunks = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|person| to_cents(person.cash_balance))
                    .sum::<i64>()
            })
            .collect::<Vec<_>>();
        let household_cents = household_chunks
            .into_iter()
            .fold(0_i64, |acc, cents| acc.saturating_add(cents));

        let company_chunks = self
            .companies
            .par_chunks(COMPANY_CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|company| to_cents(company.cash_balance))
                    .sum::<i64>()
            })
            .collect::<Vec<_>>();
        let company_cents = company_chunks
            .into_iter()
            .fold(0_i64, |acc, cents| acc.saturating_add(cents));

        self.household_cash_total = from_cents(household_cents);
        self.company_cash_total = from_cents(company_cents);
        let bank_reserve_cents = to_cents(self.total_bank_reserves());
        let total_money_supply_cents = household_cents
            .saturating_add(company_cents)
            .saturating_add(to_cents(self.treasury_balance))
            .saturating_add(bank_reserve_cents);
        self.total_money_supply = from_cents(total_money_supply_cents);

        let expected_money_supply_cents = to_cents(self.initial_money_supply)
            .saturating_add(to_cents(self.total_bank_created_money()))
            .saturating_add(to_cents(self.cumulative_money_minted));
        self.expected_money_supply = from_cents(expected_money_supply_cents);
        self.money_supply_gap =
            from_cents(total_money_supply_cents.saturating_sub(expected_money_supply_cents));
    }

    pub fn validate_money_invariants(&self) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        let expected = round_cents(
            self.initial_money_supply
                + self.total_bank_created_money()
                + self.cumulative_money_minted,
        );
        if (self.expected_money_supply - expected).abs() > 0.01 {
            violations.push(InvariantViolation {
                code: "expected_supply_mismatch",
                message: format!(
                    "expected money supply {} diverges from computed {}",
                    self.expected_money_supply, expected
                ),
            });
        }

        let gap = round_cents(self.total_money_supply - self.expected_money_supply);
        if (self.money_supply_gap - gap).abs() > 0.01 {
            violations.push(InvariantViolation {
                code: "money_gap_mismatch",
                message: format!(
                    "money gap {} diverges from computed {}",
                    self.money_supply_gap, gap
                ),
            });
        }

        // Stock invariants
        if self.sim_params.stock_market.enabled {
            for (cid, equity) in &self.stock_market.listed {
                // Share conservation: sum(person holdings) + bank holdings == free_float_shares
                let mut total_person_shares = 0_u64;
                for portfolio in self.stock_market.person_portfolios.values() {
                    if let Some(pos) = portfolio.position(*cid) {
                        total_person_shares = total_person_shares.saturating_add(pos.shares);
                    }
                }
                let bank_shares = self
                    .stock_market
                    .bank_portfolio
                    .position(*cid)
                    .map_or(0, |p| p.shares);
                let total_held = total_person_shares.saturating_add(bank_shares);
                if total_held != equity.free_float_shares {
                    violations.push(InvariantViolation {
                        code: "stock_share_conservation",
                        message: format!(
                            "company {cid}: held shares {total_held} != free_float_shares {}",
                            equity.free_float_shares
                        ),
                    });
                }

                // No NaN/inf in price
                if !equity.last_price.is_finite() || equity.last_price <= 0.0 {
                    violations.push(InvariantViolation {
                        code: "stock_price_invalid",
                        message: format!(
                            "company {cid}: last_price {} is not a positive finite number",
                            equity.last_price
                        ),
                    });
                }
            }

            // Bank reserves must be finite
            if !self.total_bank_reserves().is_finite() {
                violations.push(InvariantViolation {
                    code: "bank_reserve_not_finite",
                    message: format!(
                        "aggregate bank reserve_balance {} is not finite",
                        self.total_bank_reserves()
                    ),
                });
            }
        }

        violations
    }
}
