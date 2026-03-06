use super::*;
use crate::sim::bank::BankKind;
#[path = "monthly_bank_runs_support.rs"]
mod support;
use support::{resolve_run_claim, run_switch_probability, stable_switch_roll};

impl Country {
    pub(crate) fn process_bank_runs(&mut self) {
        self.ensure_entity_deposit_assignments();
        self.recompute_bank_deposit_liabilities();

        let params = self.sim_params.banking;
        let bank_ids = self.banks.iter().map(|bank| bank.id).collect::<Vec<_>>();
        let run_threshold = params.run_health_threshold.clamp(0.0, 1.0);
        let run_decay = (1.0 - params.run_intensity_decay_monthly.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let terminal_resolution_months = params.terminal_resolution_months.max(1);
        let mut terminal_failures: Vec<u16> = Vec::new();

        for bank_id in bank_ids {
            let Some(bank) = self.bank_by_id(bank_id) else {
                continue;
            };
            let kind = bank.kind;
            let bank_name = bank.name.clone();
            if kind != BankKind::Private {
                if let Some(bank) = self.bank_by_id_mut(bank_id) {
                    bank.update_run_intensity(0.0);
                    bank.set_resolution_state(false);
                }
                continue;
            }
            let health = self.bank_health_score(bank_id);
            if health >= run_threshold {
                let mut resolved_bank_name: Option<String> = None;
                if let Some(bank) = self.bank_by_id_mut(bank_id) {
                    let was_in_resolution = bank.is_in_resolution;
                    bank.update_run_intensity((bank.run_intensity * run_decay).max(0.0));
                    if bank.run_intensity <= 0.01 {
                        bank.apply_resolution_signal(
                            false,
                            params.resolution_entry_consecutive_months,
                            params.resolution_exit_consecutive_months,
                        );
                    }
                    if was_in_resolution && !bank.is_in_resolution {
                        resolved_bank_name = Some(bank.name.clone());
                    }
                }
                if let Some(bank_name) = resolved_bank_name {
                    self.pending_events.push(SimEvent::BankResolved {
                        country_name: self.name.clone(),
                        bank_id,
                        bank_name,
                        day: self.day,
                    });
                }
                continue;
            }

            self.pending_events.push(SimEvent::BankRunStarted {
                country_name: self.name.clone(),
                bank_id,
                bank_name: bank_name.clone(),
                health,
                day: self.day,
            });

            let stress = if run_threshold <= f64::EPSILON {
                1.0
            } else {
                ((run_threshold - health) / run_threshold).clamp(0.0, 1.0)
            };
            let private_mult = params.private_bank_failure_sensitivity.max(0.1);
            let withdraw_fraction =
                (params.run_withdraw_fraction * stress * private_mult).clamp(0.0, 1.0);
            if withdraw_fraction <= 0.0 {
                let mut resolved_bank_name: Option<String> = None;
                if let Some(bank) = self.bank_by_id_mut(bank_id) {
                    let was_in_resolution = bank.is_in_resolution;
                    bank.update_run_intensity((bank.run_intensity * run_decay).max(0.0));
                    if bank.run_intensity <= 0.01 {
                        bank.apply_resolution_signal(
                            false,
                            params.resolution_entry_consecutive_months,
                            params.resolution_exit_consecutive_months,
                        );
                    }
                    if was_in_resolution && !bank.is_in_resolution {
                        resolved_bank_name = Some(bank.name.clone());
                    }
                }
                if let Some(bank_name) = resolved_bank_name {
                    self.pending_events.push(SimEvent::BankResolved {
                        country_name: self.name.clone(),
                        bank_id,
                        bank_name,
                        day: self.day,
                    });
                }
                continue;
            }

            let switch_fraction = params.run_switch_fraction.clamp(0.0, 1.0);
            let target_bank_id = self.healthiest_alternative_bank_id(bank_id);
            let source_rate = self.deposit_rate_for(bank_id);
            let (target_health, target_rate) = target_bank_id
                .map(|id| (self.bank_health_score(id), self.deposit_rate_for(id)))
                .unwrap_or((health, source_rate));
            let available_reserve = self
                .bank_by_id(bank_id)
                .map(|bank| bank.reserve_balance.max(0.0))
                .unwrap_or(0.0);
            let requested_total = self.total_run_requested_for_bank(bank_id, withdraw_fraction);
            if requested_total <= 0.0 {
                let mut resolved_bank_name: Option<String> = None;
                if let Some(bank) = self.bank_by_id_mut(bank_id) {
                    let was_in_resolution = bank.is_in_resolution;
                    bank.update_run_intensity((bank.run_intensity * run_decay).max(0.0));
                    if bank.run_intensity <= 0.01 {
                        bank.apply_resolution_signal(
                            false,
                            params.resolution_entry_consecutive_months,
                            params.resolution_exit_consecutive_months,
                        );
                    }
                    if was_in_resolution && !bank.is_in_resolution {
                        resolved_bank_name = Some(bank.name.clone());
                    }
                }
                if let Some(bank_name) = resolved_bank_name {
                    self.pending_events.push(SimEvent::BankResolved {
                        country_name: self.name.clone(),
                        bank_id,
                        bank_name,
                        day: self.day,
                    });
                }
                continue;
            }
            let shortfall_ratio = ((requested_total - available_reserve).max(0.0)
                / requested_total.max(0.01))
            .clamp(0.0, 1.0);
            if shortfall_ratio < params.run_min_uninsured_share_trigger.clamp(0.0, 1.0) {
                let mut resolved_bank_name: Option<String> = None;
                if let Some(bank) = self.bank_by_id_mut(bank_id) {
                    let was_in_resolution = bank.is_in_resolution;
                    bank.update_run_intensity((bank.run_intensity * run_decay).max(0.0));
                    if bank.run_intensity <= 0.01 {
                        bank.apply_resolution_signal(
                            false,
                            params.resolution_entry_consecutive_months,
                            params.resolution_exit_consecutive_months,
                        );
                    }
                    if was_in_resolution && !bank.is_in_resolution {
                        resolved_bank_name = Some(bank.name.clone());
                    }
                }
                if let Some(bank_name) = resolved_bank_name {
                    self.pending_events.push(SimEvent::BankResolved {
                        country_name: self.name.clone(),
                        bank_id,
                        bank_name,
                        day: self.day,
                    });
                }
                continue;
            }
            let available_ratio = (available_reserve / requested_total).clamp(0.0, 1.0);

            let mut entities_affected = 0_u32;
            let mut haircut_loss_total = 0.0_f64;
            let mut treasury_support_total = 0.0_f64;
            let mut paid_from_bank_total = 0.0_f64;
            let mut person_switches: Vec<(usize, u16)> = Vec::new();
            let mut company_switches: Vec<(usize, u16)> = Vec::new();
            let mut treasury_switch: Option<u16> = None;
            let person_switch_prob = run_switch_probability(
                switch_fraction,
                stress,
                health,
                target_health,
                source_rate,
                target_rate,
                1.0,
            );
            let company_switch_prob = run_switch_probability(
                switch_fraction,
                stress,
                health,
                target_health,
                source_rate,
                target_rate,
                0.85,
            );
            let treasury_switch_prob = run_switch_probability(
                switch_fraction,
                stress,
                health,
                target_health,
                source_rate,
                target_rate,
                0.65,
            );

            for (idx, person) in self.people.iter_mut().enumerate() {
                if person.deposit_bank_id != Some(bank_id) || person.cash_balance <= 0.0 {
                    continue;
                }
                let requested = round_cents(person.cash_balance * withdraw_fraction);
                if requested <= 0.0 {
                    continue;
                }
                entities_affected = entities_affected.saturating_add(1);
                let (paid_from_bank, treasury_support, loss) = resolve_run_claim(
                    requested,
                    available_ratio,
                    params.deposit_insurance_cap.max(0.0),
                    params.uninsured_haircut_rate.clamp(0.0, 1.0),
                );
                paid_from_bank_total = round_cents(paid_from_bank_total + paid_from_bank);
                treasury_support_total = round_cents(treasury_support_total + treasury_support);
                haircut_loss_total = round_cents(haircut_loss_total + loss);
                if loss > 0.0 {
                    person.cash_balance = round_cents((person.cash_balance - loss).max(0.0));
                    self.pending_events.push(SimEvent::BankDepositHaircut {
                        country_name: self.name.clone(),
                        bank_id,
                        entity_kind: EntityKind::Person,
                        entity_id: person.id,
                        amount_lost: loss,
                        day: self.day,
                    });
                }
                if switch_fraction > 0.0
                    && let Some(target_id) = target_bank_id
                    && target_id != bank_id
                {
                    let switch_roll = stable_switch_roll(
                        self.month,
                        self.day,
                        u64::from(person.id),
                        bank_id,
                        target_id,
                        1,
                    );
                    if switch_roll <= person_switch_prob {
                        person_switches.push((idx, target_id));
                    }
                }
            }

            for (idx, company) in self.companies.iter_mut().enumerate() {
                if company.deposit_bank_id != Some(bank_id) || company.cash_balance <= 0.0 {
                    continue;
                }
                let requested = round_cents(company.cash_balance * withdraw_fraction);
                if requested <= 0.0 {
                    continue;
                }
                entities_affected = entities_affected.saturating_add(1);
                let (paid_from_bank, treasury_support, loss) = resolve_run_claim(
                    requested,
                    available_ratio,
                    params.deposit_insurance_cap.max(0.0),
                    params.uninsured_haircut_rate.clamp(0.0, 1.0),
                );
                paid_from_bank_total = round_cents(paid_from_bank_total + paid_from_bank);
                treasury_support_total = round_cents(treasury_support_total + treasury_support);
                haircut_loss_total = round_cents(haircut_loss_total + loss);
                if loss > 0.0 {
                    company.cash_balance = round_cents((company.cash_balance - loss).max(0.0));
                    self.pending_events.push(SimEvent::BankDepositHaircut {
                        country_name: self.name.clone(),
                        bank_id,
                        entity_kind: EntityKind::Company,
                        entity_id: u32::from(company.id),
                        amount_lost: loss,
                        day: self.day,
                    });
                }
                if switch_fraction > 0.0
                    && let Some(target_id) = target_bank_id
                    && target_id != bank_id
                {
                    let switch_roll = stable_switch_roll(
                        self.month,
                        self.day,
                        u64::from(company.id),
                        bank_id,
                        target_id,
                        2,
                    );
                    if switch_roll <= company_switch_prob {
                        company_switches.push((idx, target_id));
                    }
                }
            }

            if self.treasury_bank_id == Some(bank_id) && self.treasury_balance > 0.0 {
                let requested = round_cents(self.treasury_balance * withdraw_fraction);
                let (paid_from_bank, treasury_support, loss) = resolve_run_claim(
                    requested,
                    available_ratio,
                    params.deposit_insurance_cap.max(0.0),
                    params.uninsured_haircut_rate.clamp(0.0, 1.0),
                );
                paid_from_bank_total = round_cents(paid_from_bank_total + paid_from_bank);
                treasury_support_total = round_cents(treasury_support_total + treasury_support);
                haircut_loss_total = round_cents(haircut_loss_total + loss);
                self.treasury_balance = round_cents((self.treasury_balance - loss).max(0.0));
                if loss > 0.0 {
                    self.pending_events.push(SimEvent::BankDepositHaircut {
                        country_name: self.name.clone(),
                        bank_id,
                        entity_kind: EntityKind::Country,
                        entity_id: u32::from(self.id),
                        amount_lost: loss,
                        day: self.day,
                    });
                }
                if switch_fraction > 0.0
                    && let Some(target_id) = target_bank_id
                    && target_id != bank_id
                {
                    let treasury_roll = stable_switch_roll(
                        self.month,
                        self.day,
                        u64::from(self.id),
                        bank_id,
                        target_id,
                        3,
                    );
                    if treasury_roll <= treasury_switch_prob {
                        treasury_switch = Some(target_id);
                    }
                }
            }

            self.pending_events.push(SimEvent::BankRunWithdrawal {
                country_name: self.name.clone(),
                bank_id,
                entities_affected,
                requested: requested_total,
                haircut_loss: haircut_loss_total,
                day: self.day,
            });

            for (person_idx, target_id) in person_switches {
                self.move_person_deposit_bank(person_idx, target_id);
            }
            for (company_idx, target_id) in company_switches {
                self.move_company_deposit_bank(company_idx, target_id);
            }
            if let Some(target_id) = treasury_switch {
                self.move_treasury_deposit_bank(target_id);
            }

            self.recompute_bank_deposit_liabilities();
            let post_reserve = self
                .bank_by_id(bank_id)
                .map(|bank| bank.reserve_balance.max(0.0))
                .unwrap_or(0.0);
            let post_deposit_liabilities = self
                .bank_by_id(bank_id)
                .map(|bank| bank.deposit_liabilities.max(0.0))
                .unwrap_or(0.0);
            let post_liquidity = if post_deposit_liabilities <= 0.0 {
                1.0
            } else {
                (post_reserve / post_deposit_liabilities).clamp(0.0, 10.0)
            };
            let resolution_required = paid_from_bank_total + 0.01 < requested_total
                && post_liquidity < params.resolution_liquidity_floor.max(0.0);
            let was_in_resolution = self
                .bank_by_id(bank_id)
                .is_some_and(|bank| bank.is_in_resolution);

            if let Some(bank) = self.bank_by_id_mut(bank_id) {
                bank.apply_resolution_signal(
                    resolution_required,
                    params.resolution_entry_consecutive_months,
                    params.resolution_exit_consecutive_months,
                );
                bank.update_run_intensity(stress);
            }

            let now_in_resolution = self
                .bank_by_id(bank_id)
                .is_some_and(|bank| bank.is_in_resolution);
            if !was_in_resolution && now_in_resolution && treasury_support_total > 0.0 {
                if self.treasury_balance + 0.01 < treasury_support_total {
                    let shortfall =
                        round_cents((treasury_support_total - self.treasury_balance).max(0.0));
                    self.treasury_balance = 0.0;
                    let monetized = self.primary_bank_mut().fund_government_deficit(shortfall);
                    self.gov_debt = round_cents(self.gov_debt + monetized);
                    self.treasury_balance = round_cents(self.treasury_balance + monetized);
                }
                self.treasury_balance =
                    round_cents((self.treasury_balance - treasury_support_total).max(0.0));
                self.pending_events.push(SimEvent::BankResolutionStarted {
                    country_name: self.name.clone(),
                    bank_id,
                    treasury_support: treasury_support_total,
                    day: self.day,
                });
            }

            if self
                .bank_by_id(bank_id)
                .map(|bank| {
                    bank.is_in_resolution && bank.months_in_resolution >= terminal_resolution_months
                })
                .unwrap_or(false)
            {
                terminal_failures.push(bank_id);
            }
        }

        terminal_failures.sort_unstable();
        terminal_failures.dedup();
        for failed_bank_id in terminal_failures {
            self.close_private_bank_terminally(failed_bank_id);
        }

        self.recompute_bank_deposit_liabilities();
    }
    fn healthiest_alternative_bank_id(&self, exclude_bank_id: u16) -> Option<u16> {
        self.all_banks()
            .filter(|bank| bank.id != exclude_bank_id && !bank.is_in_resolution)
            .max_by(|a, b| {
                self.bank_health_score(a.id)
                    .partial_cmp(&self.bank_health_score(b.id))
                    .unwrap_or(Ordering::Equal)
            })
            .map(|bank| bank.id)
    }

    fn total_run_requested_for_bank(&self, bank_id: u16, withdraw_fraction: f64) -> f64 {
        let people_requested = self
            .people
            .iter()
            .filter(|person| person.deposit_bank_id == Some(bank_id))
            .map(|person| round_cents(person.cash_balance.max(0.0) * withdraw_fraction))
            .sum::<f64>();
        let companies_requested = self
            .companies
            .iter()
            .filter(|company| company.deposit_bank_id == Some(bank_id))
            .map(|company| round_cents(company.cash_balance.max(0.0) * withdraw_fraction))
            .sum::<f64>();
        let treasury_requested = if self.treasury_bank_id == Some(bank_id) {
            round_cents(self.treasury_balance.max(0.0) * withdraw_fraction)
        } else {
            0.0
        };
        round_cents(people_requested + companies_requested + treasury_requested)
    }
}
