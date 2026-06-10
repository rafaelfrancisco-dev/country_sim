#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    pub code: &'static str,
    pub message: String,
}

impl super::World {
    pub fn validate_invariants(&self) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        if self.country_history.len() != self.countries.len() {
            violations.push(InvariantViolation {
                code: "country_history_len",
                message: format!(
                    "country history length {} does not match countries {}",
                    self.country_history.len(),
                    self.countries.len()
                ),
            });
        }

        let expected_month_points = (self.day / super::super::country::DAYS_PER_MONTH + 1) as usize;
        if self.world_history.len() > super::HISTORY_LIMIT {
            violations.push(InvariantViolation {
                code: "world_history_limit",
                message: format!(
                    "world history length {} exceeds limit {}",
                    self.world_history.len(),
                    super::HISTORY_LIMIT
                ),
            });
        }
        if !self.world_history.is_empty() && self.world_history.len() > expected_month_points {
            violations.push(InvariantViolation {
                code: "world_history_count",
                message: format!(
                    "world history length {} exceeds expected points {}",
                    self.world_history.len(),
                    expected_month_points
                ),
            });
        }

        for country in &self.countries {
            let mut seen_person_ids = std::collections::HashSet::new();
            for person in &country.people {
                if !seen_person_ids.insert(person.id) {
                    violations.push(InvariantViolation {
                        code: "duplicate_person_id",
                        message: format!(
                            "country {} contains duplicate person_id {}",
                            country.id(),
                            person.id
                        ),
                    });
                }
            }

            for portfolio in country.stock_market.person_portfolios.values() {
                for position in &portfolio.positions {
                    if position.pledged_shares > position.shares {
                        violations.push(InvariantViolation {
                            code: "stock_pledge_exceeds_holding",
                            message: format!(
                                "country {} security ({}, {}): pledged {} > held {}",
                                country.id(),
                                position.security_id.country_id,
                                position.security_id.company_id,
                                position.pledged_shares,
                                position.shares
                            ),
                        });
                    }
                }
            }

            for violation in country.validate_money_invariants() {
                violations.push(InvariantViolation {
                    code: violation.code,
                    message: format!("country {}: {}", country.id(), violation.message),
                });
            }
        }

        let mut held_by_security = std::collections::HashMap::new();
        for country in &self.countries {
            for portfolio in country.stock_market.person_portfolios.values() {
                for position in &portfolio.positions {
                    *held_by_security
                        .entry(position.security_id)
                        .or_insert(0_u64) = held_by_security
                        .get(&position.security_id)
                        .copied()
                        .unwrap_or(0)
                        .saturating_add(position.shares);
                }
            }
            for position in &country.stock_market.bank_portfolio.positions {
                *held_by_security
                    .entry(position.security_id)
                    .or_insert(0_u64) = held_by_security
                    .get(&position.security_id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(position.shares);
            }
        }

        for country in &self.countries {
            for (&company_id, equity) in &country.stock_market.listed {
                let security_id = crate::sim::stock_market::StockSecurityId {
                    country_id: country.id(),
                    company_id,
                };
                let total_held = held_by_security.get(&security_id).copied().unwrap_or(0);
                if total_held != equity.free_float_shares {
                    violations.push(InvariantViolation {
                        code: "stock_share_conservation",
                        message: format!(
                            "security ({}, {}): held shares {} != free_float_shares {}",
                            security_id.country_id,
                            security_id.company_id,
                            total_held,
                            equity.free_float_shares
                        ),
                    });
                }
            }
        }

        violations
    }
}
