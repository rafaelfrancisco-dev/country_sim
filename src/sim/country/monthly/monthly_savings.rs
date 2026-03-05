use super::*;

impl Country {
    pub(super) fn process_savings_interest(&mut self) {
        self.ensure_entity_deposit_assignments();

        let min_balance = self.sim_params.monetary.savings_min_balance;
        let bank_ids = std::iter::once(self.bank.id)
            .chain(self.banks.iter().map(|bank| bank.id))
            .collect::<Vec<_>>();
        let mut total_paid = 0.0_f64;

        for bank_id in bank_ids {
            let deposit_rate = self.deposit_rate_for(bank_id);
            if deposit_rate <= 0.0 {
                continue;
            }
            let monthly_rate = deposit_rate / 12.0;

            let mut people_interest = Vec::new();
            for (idx, person) in self.people.iter().enumerate() {
                if person.deposit_bank_id != Some(bank_id) || person.cash_balance <= min_balance {
                    continue;
                }
                let gross = round_cents(person.cash_balance * monthly_rate);
                if gross > 0.0 {
                    people_interest.push((idx, gross));
                }
            }

            let mut company_interest = Vec::new();
            for (idx, company) in self.companies.iter().enumerate() {
                if company.deposit_bank_id != Some(bank_id) || company.cash_balance <= min_balance {
                    continue;
                }
                let gross = round_cents(company.cash_balance * monthly_rate);
                if gross > 0.0 {
                    company_interest.push((idx, gross));
                }
            }

            let treasury_interest =
                if self.treasury_bank_id == Some(bank_id) && self.treasury_balance > min_balance {
                    round_cents(self.treasury_balance * monthly_rate)
                } else {
                    0.0
                };

            let people_gross: f64 = people_interest.iter().map(|(_, v)| *v).sum();
            let companies_gross: f64 = company_interest.iter().map(|(_, v)| *v).sum();
            let total_gross = round_cents(people_gross + companies_gross + treasury_interest);
            if total_gross <= 0.0 {
                continue;
            }

            let available = self
                .bank_by_id(bank_id)
                .map(|bank| bank.reserve_balance.max(0.0))
                .unwrap_or(0.0);
            let paid_for_bank = round_cents(total_gross.min(available));
            if paid_for_bank <= 0.0 {
                continue;
            }

            let scale = paid_for_bank / total_gross;
            if let Some(bank) = self.bank_by_id_mut(bank_id) {
                bank.reserve_balance = round_cents(bank.reserve_balance - paid_for_bank);
                bank.last_month_deposit_interest_expense =
                    round_cents(bank.last_month_deposit_interest_expense + paid_for_bank);
            }

            for (idx, gross) in people_interest {
                let paid = round_cents(gross * scale);
                if paid <= 0.0 {
                    continue;
                }
                if let Some(person) = self.people.get_mut(idx) {
                    person.cash_balance = round_cents(person.cash_balance + paid);
                    person.last_month_savings_interest =
                        round_cents(person.last_month_savings_interest + paid);
                    total_paid = round_cents(total_paid + paid);
                }
            }

            for (idx, gross) in company_interest {
                let paid = round_cents(gross * scale);
                if paid <= 0.0 {
                    continue;
                }
                if let Some(company) = self.companies.get_mut(idx) {
                    company.cash_balance = round_cents(company.cash_balance + paid);
                    company.last_month_savings_interest =
                        round_cents(company.last_month_savings_interest + paid);
                    total_paid = round_cents(total_paid + paid);
                }
            }

            if treasury_interest > 0.0 {
                let paid = round_cents(treasury_interest * scale);
                if paid > 0.0 {
                    self.treasury_balance = round_cents(self.treasury_balance + paid);
                    total_paid = round_cents(total_paid + paid);
                }
            }
        }

        self.monthly_savings_interest_paid = total_paid;
        self.recompute_bank_deposit_liabilities();
    }

    pub(super) fn process_competitive_deposit_switching(&mut self) {
        self.ensure_entity_deposit_assignments();
        self.recompute_bank_deposit_liabilities();

        let Some(target_bank_id) = self.preferred_private_bank_id() else {
            return;
        };
        let target_health = self.bank_health_score(target_bank_id);
        let target_rate = self.deposit_rate_for(target_bank_id);
        let min_balance = self.sim_params.monetary.savings_min_balance.max(0.0);

        let mut person_moves: Vec<usize> = Vec::new();
        for (idx, person) in self.people.iter().enumerate() {
            let Some(source_bank_id) = person.deposit_bank_id else {
                continue;
            };
            if source_bank_id == target_bank_id || person.cash_balance <= min_balance {
                continue;
            }
            let source_health = self.bank_health_score(source_bank_id);
            let source_rate = self.deposit_rate_for(source_bank_id);
            let health_edge = (target_health - source_health).clamp(-1.0, 1.0);
            let rate_edge = (target_rate - source_rate).clamp(-0.05, 0.05);
            let source_bias = if source_bank_id == self.bank.id {
                0.07
            } else {
                0.03
            };
            let impulse = (0.50 + 0.35 * health_edge + 3.5 * rate_edge).clamp(0.0, 1.0);
            let switch_prob = (source_bias * impulse).clamp(0.0, 0.18);
            let roll = competitive_switch_roll(
                self.month,
                self.day,
                u64::from(person.id),
                source_bank_id,
                target_bank_id,
                11,
            );
            if roll <= switch_prob {
                person_moves.push(idx);
            }
        }

        let mut company_moves: Vec<usize> = Vec::new();
        for (idx, company) in self.companies.iter().enumerate() {
            let Some(source_bank_id) = company.deposit_bank_id else {
                continue;
            };
            if source_bank_id == target_bank_id || company.cash_balance <= min_balance {
                continue;
            }
            let source_health = self.bank_health_score(source_bank_id);
            let source_rate = self.deposit_rate_for(source_bank_id);
            let health_edge = (target_health - source_health).clamp(-1.0, 1.0);
            let rate_edge = (target_rate - source_rate).clamp(-0.05, 0.05);
            let source_bias = if source_bank_id == self.bank.id {
                0.04
            } else {
                0.02
            };
            let impulse = (0.45 + 0.40 * health_edge + 3.0 * rate_edge).clamp(0.0, 1.0);
            let switch_prob = (source_bias * impulse).clamp(0.0, 0.12);
            let roll = competitive_switch_roll(
                self.month,
                self.day,
                u64::from(company.id),
                source_bank_id,
                target_bank_id,
                12,
            );
            if roll <= switch_prob {
                company_moves.push(idx);
            }
        }

        for idx in person_moves {
            self.move_person_deposit_bank(idx, target_bank_id);
        }
        for idx in company_moves {
            self.move_company_deposit_bank(idx, target_bank_id);
        }
        self.recompute_bank_deposit_liabilities();
    }
}

fn competitive_switch_roll(
    month: u64,
    day: u64,
    entity_id: u64,
    from_bank_id: u16,
    to_bank_id: u16,
    salt: u64,
) -> f64 {
    let mut x = month
        .wrapping_mul(2_000_029)
        .wrapping_add(day.wrapping_mul(17_117))
        .wrapping_add(entity_id.wrapping_mul(2_654_435_761))
        .wrapping_add(u64::from(from_bank_id).wrapping_mul(12_289))
        .wrapping_add(u64::from(to_bank_id).wrapping_mul(6_151))
        .wrapping_add(salt);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    ((x % 1_000_000) as f64) / 1_000_000.0
}
