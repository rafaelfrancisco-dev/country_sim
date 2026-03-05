use super::*;
use crate::sim::bank::BankKind;

impl Country {
    pub(super) fn update_company_growth_openings(&mut self) {
        let month_seed = self.month.saturating_add(1);
        let mut sim_params = self.sim_params.clone();
        let expansion_mult = self.expansion_multiplier().max(0.01);
        let inverse_expansion = (1.0 / expansion_mult).max(0.01);
        sim_params.company_growth.min_profitable_months =
            ((sim_params.company_growth.min_profitable_months as f64 * inverse_expansion)
                .round()
                .clamp(1.0, u8::MAX as f64)) as u8;
        sim_params.company_growth.min_cash_reserve =
            round_cents((sim_params.company_growth.min_cash_reserve * inverse_expansion).max(0.0));
        sim_params.company_growth.payroll_reserve_multiplier =
            (sim_params.company_growth.payroll_reserve_multiplier * inverse_expansion).max(0.0);
        sim_params.company_growth.profit_per_opening = round_cents(
            (sim_params.company_growth.profit_per_opening * inverse_expansion).max(1.0),
        );
        sim_params.company_growth.max_monthly_openings =
            ((sim_params.company_growth.max_monthly_openings as f64 * expansion_mult)
                .round()
                .clamp(1.0, u16::MAX as f64)) as u16;

        let labor_demand = self.sim_params.labor_demand;
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.open_positions_from_growth(&sim_params, month_seed);
                    company.ensure_baseline_openings(labor_demand, month_seed);
                }
            });
    }

    pub(super) fn process_startup_formation(&mut self) {
        let formation = self.sim_params.company_formation;
        let Some(limit) = formation.max_startups_per_month else {
            return;
        };
        if limit == 0 {
            return;
        }

        let mut labor_force = 0_usize;
        for person in &self.people {
            if person.is_labor_force_participant() {
                labor_force = labor_force.saturating_add(1);
            }
        }
        if labor_force == 0 {
            return;
        }

        let unemployment = self.labor_force_unemployment_rate();
        let pressure = (unemployment - formation.labor_slack_startup_trigger).max(0.0);
        let dynamic_budget =
            1_u16.saturating_add(((pressure * labor_force as f64) / 100.0).round() as u16);
        let mut startup_budget = dynamic_budget.min(limit);
        if unemployment >= formation.labor_slack_startup_trigger + 0.03 {
            startup_budget = startup_budget.saturating_add(6).min(limit);
        }

        if pressure <= f64::EPSILON {
            startup_budget = startup_budget.min(1);
        }
        startup_budget = ((startup_budget as f64) * self.expansion_multiplier())
            .round()
            .clamp(0.0, limit as f64) as u16;
        if startup_budget == 0 {
            return;
        }

        let jobs = common_job_types();
        if jobs.is_empty() {
            return;
        }

        let mut startups_created = 0_u16;
        let mut job_cursor = (self.month as usize) % jobs.len();
        let mut failed_attempts = 0_usize;
        let max_attempts = (startup_budget as usize)
            .saturating_mul(jobs.len())
            .max(jobs.len());

        while startups_created < startup_budget && failed_attempts < max_attempts {
            let job = jobs[job_cursor % jobs.len()];
            if self.try_form_startup_for_job(job.id) {
                startups_created = startups_created.saturating_add(1);
                failed_attempts = 0;
            } else {
                failed_attempts = failed_attempts.saturating_add(1);
            }
            job_cursor = job_cursor.saturating_add(1);
        }
    }

    pub(crate) fn process_bank_formation(&mut self) {
        let params = self.sim_params.banking;
        if params.max_new_banks_per_month == 0 {
            return;
        }

        let formation_budget = usize::from(params.max_new_banks_per_month);
        let market_capacity = self.private_bank_capacity_target();
        let mut formed = 0_usize;
        while formed < formation_budget {
            let private_count = std::iter::once(&self.bank)
                .chain(self.banks.iter())
                .filter(|bank| bank.kind == BankKind::Private)
                .count();
            if private_count >= market_capacity {
                break;
            }
            if let Some((bank_id, seed_capital)) = self.form_private_bank_from_founders() {
                // Give new private bank its initial job openings.
                let bank_employee_job_types = params.bank_employee_job_types;
                let bank_wage_multiplier = params.bank_wage_multiplier.max(1.0);
                if let Some(new_bank) = self.bank_by_id_mut(bank_id) {
                    let target = new_bank.target_employee_count(
                        new_bank.deposit_liabilities,
                        params.bank_min_employees,
                        params.bank_max_employees,
                        params.bank_employees_per_deposit_unit,
                    );
                    for i in 0..target {
                        let job_id =
                            bank_employee_job_types[i as usize % bank_employee_job_types.len()];
                        let salary = round_cents(salary_for_job(job_id) * bank_wage_multiplier);
                        new_bank.add_opening_with_salary(job_id, 1, salary);
                    }
                }

                self.pending_events.push(SimEvent::BankFounded {
                    country_name: self.name.clone(),
                    bank_id,
                    bank_name: self
                        .bank_by_id(bank_id)
                        .map(|bank| bank.name.clone())
                        .unwrap_or_else(|| format!("{} Private Bank", self.name)),
                    kind: BankKind::Private,
                    seed_capital,
                    day: self.day,
                });
                formed = formed.saturating_add(1);
            } else {
                break;
            }
        }

        // There is always exactly ONE public bank per country (self.bank, id=0).
        // Recapitalize when it has meaningful deposit obligations it can't meet.
        // Guard: require deposit_liabilities > 10% of recapitalize_amount so we
        // don't false-trigger on a bank that simply has no loans/reserves yet.
        let min_deposits_to_recapitalize = params.public_bank_recapitalize_amount * 0.1;
        if self.bank.deposit_liabilities >= min_deposits_to_recapitalize
            && self.bank_health_score(self.bank.id)
                < (params.public_health_floor + 0.05).clamp(0.0, 1.0)
        {
            self.recapitalize_public_bank(params.public_bank_recapitalize_amount);
        }

        self.ensure_entity_deposit_assignments();
    }

    fn private_bank_capacity_target(&self) -> usize {
        let target_people = self
            .sim_params
            .banking
            .target_people_per_private_bank
            .max(1);
        self.people
            .len()
            .saturating_add(target_people.saturating_sub(1))
            / target_people
    }

    /// Inject capital into the single public bank from treasury / monetization.
    fn recapitalize_public_bank(&mut self, amount: f64) {
        let amount = round_cents(amount.max(0.0));
        if amount <= 0.0 {
            return;
        }
        let treasury_funding = round_cents(amount.min(self.treasury_balance.max(0.0)));
        self.treasury_balance = round_cents(self.treasury_balance - treasury_funding);
        let deficit_need = round_cents((amount - treasury_funding).max(0.0));
        let deficit_funding = if deficit_need > 0.0 {
            self.bank.fund_government_deficit(deficit_need)
        } else {
            0.0
        };
        if deficit_funding > 0.0 {
            self.gov_debt = round_cents(self.gov_debt + deficit_funding);
        }
        let total = round_cents(treasury_funding + deficit_funding);
        if total > 0.0 {
            self.bank.reserve_balance = round_cents(self.bank.reserve_balance + total);
            self.bank.equity_capital = round_cents(self.bank.equity_capital + total);
        }
    }

    fn form_private_bank_from_founders(&mut self) -> Option<(u16, f64)> {
        let params = self.sim_params.banking;
        let founder_count = params.min_private_bank_founders.max(1);
        let minimum_seed = round_cents(params.min_private_bank_seed_capital.max(1.0));
        let contribution_target = round_cents(minimum_seed / founder_count as f64);
        let founder_cash_floor = params.min_founder_cash_after_contribution.max(0.0);
        if contribution_target <= 0.0 {
            return None;
        }

        let mut founder_indices = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, person)| person.cash_balance >= contribution_target + founder_cash_floor)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        if founder_indices.len() < founder_count {
            return None;
        }

        founder_indices.truncate(founder_count);
        let mover_limit = founder_count.saturating_mul(10);
        let mover_indices = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, person)| person.cash_balance > contribution_target)
            .map(|(idx, _)| idx)
            .take(mover_limit)
            .collect::<Vec<_>>();
        let projected_migrated_deposits = mover_indices
            .iter()
            .map(|idx| {
                let Some(person) = self.people.get(*idx) else {
                    return 0.0;
                };
                let founder_outflow = if founder_indices.contains(idx) {
                    contribution_target
                } else {
                    0.0
                };
                round_cents((person.cash_balance - founder_outflow).max(0.0))
            })
            .sum::<f64>();
        if projected_migrated_deposits > 0.0 {
            let projected_ratio = round_cents(minimum_seed / projected_migrated_deposits.max(0.01));
            if projected_ratio + 1e-6 < params.min_private_seed_to_deposits_ratio.max(0.0) {
                return None;
            }
        }

        let mut seed_capital = 0.0_f64;
        for idx in &founder_indices {
            if let Some(person) = self.people.get_mut(*idx) {
                let amount = round_cents(contribution_target.min(person.cash_balance.max(0.0)));
                person.cash_balance = round_cents(person.cash_balance - amount);
                seed_capital = round_cents(seed_capital + amount);
            }
        }
        if seed_capital + 0.01 < minimum_seed {
            return None;
        }

        let bank_id = self.create_bank(BankKind::Private, seed_capital)?;
        for idx in mover_indices {
            self.move_person_deposit_bank(idx, bank_id);
        }
        // Migrate some companies to give the new bank loan customers.
        let public_id = self.public_bank_id.unwrap_or(self.bank.id);
        let company_movers: Vec<usize> = self.companies.iter().enumerate()
            .filter(|(_, c)| c.deposit_bank_id == Some(public_id))
            .map(|(idx, _)| idx).take(founder_count).collect();
        for idx in company_movers { self.move_company_deposit_bank(idx, bank_id); }
        Some((bank_id, seed_capital))
    }

    pub(super) fn process_essential_company_backfill(&mut self) {
        let backfill = self.sim_params.government_backfill;
        if !backfill.enabled {
            return;
        }

        let missing_goods = self.missing_essential_goods(backfill.essential_good_max_id_inclusive);
        for good_id in missing_goods {
            self.create_public_essential_company(good_id, backfill);
        }
        self.spawn_countercyclical_public_employers(backfill);
    }

    pub(super) fn missing_essential_goods(&self, max_good_id_inclusive: u8) -> Vec<u8> {
        let max_good_id = max_good_id_inclusive.min(GOODS_COUNT.saturating_sub(1) as u8);
        (0..=max_good_id)
            .filter(|good_id| {
                !self
                    .companies
                    .iter()
                    .any(|company| company.produced_good_id == *good_id)
            })
            .collect()
    }

    pub(crate) fn create_public_essential_company(
        &mut self,
        good_id: u8,
        backfill: GovernmentBackfillParams,
    ) {
        if self.companies.len() >= u16::MAX as usize {
            return;
        }

        let opening_count = backfill.openings_total.max(1);
        let manufacturing_openings = backfill.manufacturing_openings.min(opening_count);
        let mixed_openings = opening_count.saturating_sub(manufacturing_openings);

        let company_id = self.companies.len() as u16;
        let company_name = format!("{} Utility {}", self.name, self.next_startup_serial);
        self.next_startup_serial = self.next_startup_serial.saturating_add(1);

        let mut company = Company::new(
            company_id,
            company_name.clone(),
            common_job_types()
                .iter()
                .map(|job| job.id)
                .collect::<Vec<_>>(),
            good_id,
        );
        company.deposit_bank_id = self.public_bank_id_or_first();
        company.is_public_utility = true;
        company.is_essential_backfill = true;

        let candidate_price = self.average_price_for_good(good_id);
        if let Some((min_price, max_price)) = self.public_utility_price_bounds_for_good(good_id) {
            company.unit_price = round_cents(candidate_price.clamp(min_price, max_price));
        } else {
            company.unit_price = round_cents(candidate_price.max(0.0));
        }

        const MANUFACTURING_OPERATOR_JOB_ID: u8 = 5;
        if manufacturing_openings > 0 {
            let offer = self.public_opening_offer(MANUFACTURING_OPERATOR_JOB_ID, backfill);
            company.add_opening_with_salary(
                MANUFACTURING_OPERATOR_JOB_ID,
                manufacturing_openings,
                offer,
            );
        }

        let mixed_weighted_jobs = common_job_types()
            .iter()
            .filter(|job| job.id != MANUFACTURING_OPERATOR_JOB_ID)
            .map(|job| (job.id, job.assignment_weight))
            .collect::<Vec<_>>();
        for idx in 0..mixed_openings {
            let seed = hash64(
                (u64::from(self.id) << 48)
                    ^ (self.month << 24)
                    ^ (u64::from(self.next_startup_serial) << 8)
                    ^ (u64::from(good_id) << 2)
                    ^ u64::from(idx),
            );
            let Some(job_id) = pick_weighted_job(seed, &mixed_weighted_jobs) else {
                continue;
            };
            let offer = self.public_opening_offer(job_id, backfill);
            company.add_opening_with_salary(job_id, 1, offer);
        }

        let monthly_opening_payroll = round_cents(
            company
                .open_positions
                .iter()
                .map(|opening| f64::from(opening.slots) * opening.offered_monthly_salary)
                .sum(),
        );
        if monthly_opening_payroll <= 0.0 {
            return;
        }

        let required_capital =
            round_cents(monthly_opening_payroll * backfill.runway_months.max(0.0));
        let treasury_funding = round_cents(required_capital.min(self.treasury_balance.max(0.0)));
        self.treasury_balance = round_cents(self.treasury_balance - treasury_funding);

        let deficit_need = round_cents((required_capital - treasury_funding).max(0.0));
        let deficit_funding = if deficit_need > 0.0 {
            self.bank.fund_government_deficit(deficit_need)
        } else {
            0.0
        };
        if deficit_funding > 0.0 {
            self.gov_debt = round_cents(self.gov_debt + deficit_funding);
        }

        let total_funding = round_cents(treasury_funding + deficit_funding);
        if total_funding <= 0.0 {
            return;
        }

        company.cash_balance = total_funding;
        self.companies.push(company);

        self.monthly_gov_spending = round_cents(self.monthly_gov_spending + total_funding);
        self.monthly_budget_balance =
            round_cents(self.monthly_tax_collected - self.monthly_gov_spending);

        self.pending_events.push(SimEvent::GovCompanyFunded {
            country_name: self.name.clone(),
            company_id,
            produced_good_id: good_id,
            total_funding,
            treasury_funding,
            deficit_funding,
            day: self.day,
        });
    }

    fn spawn_countercyclical_public_employers(&mut self, backfill: GovernmentBackfillParams) {
        let mut labor_force = 0_usize;
        for person in &self.people {
            if person.is_labor_force_participant() {
                labor_force = labor_force.saturating_add(1);
            }
        }
        if labor_force == 0 {
            return;
        }

        let unemployment = self.labor_force_unemployment_rate();
        let target_unemployment = (self
            .sim_params
            .company_distress
            .countercyclical_layoff_unemployment_guard
            + 0.08)
            .clamp(0.0, 0.20);
        if unemployment <= target_unemployment {
            return;
        }

        let excess = (unemployment - target_unemployment).max(0.0);
        let desired_jobs = ((labor_force as f64 * excess * 0.40).ceil() as u16).clamp(1, 600);
        let openings_per_company = backfill.openings_total.max(1);
        let companies_needed = ((f64::from(desired_jobs) / f64::from(openings_per_company)).ceil()
            as u16)
            .clamp(1, 30);
        let essential_span = u16::from(
            backfill
                .essential_good_max_id_inclusive
                .saturating_add(1)
                .max(1),
        );

        for offset in 0..companies_needed {
            if self.companies.len() >= u16::MAX as usize {
                break;
            }
            let good_id = (offset % essential_span) as u8;
            self.create_public_essential_company(good_id, backfill);
        }
    }

    pub(super) fn public_opening_offer(
        &self,
        job_id: u8,
        backfill: GovernmentBackfillParams,
    ) -> f64 {
        let catalog = salary_for_job(job_id);
        if catalog <= 0.0 {
            return 0.0;
        }

        let market_best = self.market_best_opening_salary_for_job(job_id).max(catalog);
        let switch_component = self
            .sim_params
            .labor_mobility
            .switch_raise_threshold
            .max(0.0)
            + backfill.switch_incentive_buffer.max(0.0);
        let switch_target = market_best * (1.0 + switch_component);
        let base_offer = (catalog * backfill.base_wage_multiplier.max(0.0)).max(switch_target);
        let cap = catalog * backfill.max_wage_multiplier.max(1.0);
        round_cents(base_offer.min(cap))
    }

    pub(super) fn market_best_opening_salary_for_job(&self, job_id: u8) -> f64 {
        self.companies
            .iter()
            .flat_map(|company| company.open_positions.iter())
            .filter(|opening| opening.job_type_id == job_id)
            .map(|opening| opening.offered_monthly_salary)
            .fold(0.0, f64::max)
    }

    pub(super) fn adjust_public_utility_opening_wages(&mut self) {
        let backfill = self.sim_params.government_backfill;
        if !backfill.enabled {
            return;
        }

        let escalation_multiplier = 1.0 + backfill.vacancy_escalation_rate.max(0.0);
        let mut updates: Vec<(usize, usize, f64)> = Vec::new();
        for (company_idx, company) in self.companies.iter().enumerate() {
            if !company.is_public_utility || company.open_positions.is_empty() {
                continue;
            }

            for (opening_idx, opening) in company.open_positions.iter().enumerate() {
                let catalog = salary_for_job(opening.job_type_id);
                if catalog <= 0.0 {
                    continue;
                }

                let market_best = self
                    .market_best_opening_salary_for_job(opening.job_type_id)
                    .max(catalog);
                let switch_component = self
                    .sim_params
                    .labor_mobility
                    .switch_raise_threshold
                    .max(0.0)
                    + backfill.switch_incentive_buffer.max(0.0);
                let switch_target = market_best * (1.0 + switch_component);
                let base_offer =
                    (catalog * backfill.base_wage_multiplier.max(0.0)).max(switch_target);
                let cap = catalog * backfill.max_wage_multiplier.max(1.0);
                let escalated = opening.offered_monthly_salary * escalation_multiplier;
                let next_salary = round_cents(escalated.max(base_offer).min(cap));
                if next_salary > opening.offered_monthly_salary + 0.009 {
                    updates.push((company_idx, opening_idx, next_salary));
                }
            }
        }

        for (company_idx, opening_idx, next_salary) in updates {
            if let Some(company) = self.companies.get_mut(company_idx)
                && let Some(opening) = company.open_positions.get_mut(opening_idx)
            {
                opening.offered_monthly_salary = next_salary;
            }
        }
    }
}
