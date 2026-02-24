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
        let total_money_supply_cents = household_cents
            .saturating_add(company_cents)
            .saturating_add(to_cents(self.treasury_balance))
            .saturating_add(to_cents(self.bank.reserve_balance));
        self.total_money_supply = from_cents(total_money_supply_cents);

        let expected_money_supply_cents = to_cents(self.initial_money_supply)
            .saturating_add(to_cents(self.bank.created_money_outstanding));
        self.expected_money_supply = from_cents(expected_money_supply_cents);
        self.money_supply_gap =
            from_cents(total_money_supply_cents.saturating_sub(expected_money_supply_cents));
    }

    pub fn validate_money_invariants(&self) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        let expected = round_cents(self.initial_money_supply + self.bank.created_money_outstanding);
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
                message: format!("money gap {} diverges from computed {}", self.money_supply_gap, gap),
            });
        }

        violations
    }
}
