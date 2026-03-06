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

        // Housing stock invariants
        {
            let total = self.housing_stock_total;
            let expected_total = self
                .housing_stock_occupied_owned
                .saturating_add(self.housing_stock_occupied_rented)
                .saturating_add(self.housing_stock_vacant);
            if total != expected_total {
                violations.push(InvariantViolation {
                    code: "housing_stock_total_mismatch",
                    message: format!(
                        "housing_stock_total {total} != owned {} + rented {} + vacant {} (= {expected_total})",
                        self.housing_stock_occupied_owned,
                        self.housing_stock_occupied_rented,
                        self.housing_stock_vacant,
                    ),
                });
            }

            // Company rental_units_owned tracks total company-owned units (occupied + vacant),
            // so it can exceed housing_stock_occupied_rented (actual renters).
            let company_rentals: u32 = self.companies.iter().map(|c| c.rental_units_owned).sum();
            if company_rentals < self.housing_stock_occupied_rented {
                violations.push(InvariantViolation {
                    code: "housing_rental_units_undercount",
                    message: format!(
                        "Σ company.rental_units_owned {company_rentals} < housing_stock_occupied_rented {}",
                        self.housing_stock_occupied_rented,
                    ),
                });
            }
        }

        // Stock market invariants
        if self.sim_params.stock_market.enabled {
            for (cid, equity) in &self.stock_market.listed {
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
