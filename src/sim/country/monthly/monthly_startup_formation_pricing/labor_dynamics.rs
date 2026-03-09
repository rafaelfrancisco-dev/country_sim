use super::*;
use crate::sim::country::runtime::CachedSwitchOpening;

const JOB_SWITCH_COHORTS_PER_MONTH: u64 = 30;
const JOB_SWITCH_SALARY_BUCKET: f64 = 100.0;

impl Country {
    pub(in crate::sim::country) fn process_daily_job_switches(&mut self) {
        if self.sim_params.labor_mobility.switch_cadence != SwitchCadence::MonthlyOnce {
            return;
        }

        self.runtime.prepare_monthly_switch_cache(self.month);
        let active_cohort = self.day.saturating_sub(1) % JOB_SWITCH_COHORTS_PER_MONTH;
        let employed_ids = self.employed_ids().to_vec();
        for person_id in employed_ids {
            if hash64((u64::from(self.id) << 48) ^ u64::from(person_id) ^ 0x5107_c04f_0712_u64)
                % JOB_SWITCH_COHORTS_PER_MONTH
                != active_cohort
            {
                continue;
            }
            let Some(person_idx) = self.runtime_person_idx(person_id) else {
                continue;
            };
            self.process_job_switch_for_person(person_idx);
        }
    }

    pub(in crate::sim::country::monthly) fn process_monthly_job_switches(&mut self) {
        if self.sim_params.labor_mobility.switch_cadence != SwitchCadence::MonthlyOnce {
            return;
        }
        if self.day < JOB_SWITCH_COHORTS_PER_MONTH {
            let employed_ids = self.employed_ids().to_vec();
            self.runtime.prepare_monthly_switch_cache(self.month);
            for person_id in employed_ids {
                let Some(person_idx) = self.runtime_person_idx(person_id) else {
                    continue;
                };
                self.process_job_switch_for_person(person_idx);
            }
        }
        self.runtime.clear_monthly_switch_cache();
    }

    fn process_job_switch_for_person(&mut self, person_idx: usize) {
        let raise_threshold = self
            .sim_params
            .labor_mobility
            .switch_raise_threshold
            .max(0.0);
        let (person_id, current_job_id, current_salary) = {
            let Some(person) = self.people.get(person_idx) else {
                return;
            };
            if !person.is_employed() {
                return;
            }
            let Some(job_id) = person.job_type_id() else {
                return;
            };
            (person.id, job_id, person.monthly_salary())
        };
        let min_target_salary = current_salary * (1.0 + raise_threshold);

        if let Some(current_company_id) = self.people[person_idx].employer_company_id() {
            let Some(target) = self.best_switch_opening_cached(
                current_company_id,
                current_job_id,
                min_target_salary,
            ) else {
                return;
            };
            let Some((job_id, new_salary)) = self
                .companies
                .get_mut(target.company_idx)
                .and_then(|company| company.consume_opening(target.opening_idx))
            else {
                self.runtime.clear_monthly_switch_cache();
                return;
            };
            if let Some(old_company) = self.companies.get_mut(current_company_id as usize) {
                old_company.add_opening_with_salary(current_job_id, 1, current_salary);
            }
            if let Some(person) = self.people.get_mut(person_idx) {
                if person.employer_company_id() != Some(current_company_id)
                    || person.job_type_id() != Some(current_job_id)
                {
                    if let Some(target_company) = self.companies.get_mut(target.company_idx) {
                        target_company.add_opening_with_salary(job_id, 1, new_salary);
                    }
                    return;
                }
                person.set_employment_with_salary(
                    target.company_idx as u16,
                    job_id,
                    new_salary,
                    self.day,
                );
                self.refresh_person_runtime_membership(person_idx);
                self.pending_events.push(SimEvent::JobSwitched {
                    country_name: self.name.clone(),
                    person_id,
                    from_company_id: current_company_id,
                    to_company_id: target.company_idx as u16,
                    monthly_salary: new_salary,
                    day: self.day,
                });
                self.runtime.clear_monthly_switch_cache();
            }
            return;
        }

        if let Some(current_bank_id) = self.people[person_idx].employer_bank_id() {
            let Some(target) =
                self.best_switch_opening_cached(u16::MAX, current_job_id, min_target_salary)
            else {
                return;
            };
            let Some((job_id, new_salary)) = self
                .companies
                .get_mut(target.company_idx)
                .and_then(|company| company.consume_opening(target.opening_idx))
            else {
                self.runtime.clear_monthly_switch_cache();
                return;
            };
            let bank_opt = if self.primary_bank().id == current_bank_id {
                Some(self.primary_bank_mut())
            } else {
                self.banks.iter_mut().find(|b| b.id == current_bank_id)
            };
            if let Some(bank) = bank_opt {
                bank.add_opening_with_salary(current_job_id, 1, current_salary);
            }
            if let Some(person) = self.people.get_mut(person_idx) {
                if person.employer_bank_id() != Some(current_bank_id)
                    || person.job_type_id() != Some(current_job_id)
                {
                    if let Some(target_company) = self.companies.get_mut(target.company_idx) {
                        target_company.add_opening_with_salary(job_id, 1, new_salary);
                    }
                    return;
                }
                person.set_employment_with_salary(
                    target.company_idx as u16,
                    job_id,
                    new_salary,
                    self.day,
                );
                self.refresh_person_runtime_membership(person_idx);
                self.pending_events.push(SimEvent::JobSwitched {
                    country_name: self.name.clone(),
                    person_id,
                    from_company_id: u16::MAX,
                    to_company_id: target.company_idx as u16,
                    monthly_salary: new_salary,
                    day: self.day,
                });
                self.runtime.clear_monthly_switch_cache();
            }
        }
    }

    fn best_switch_opening_cached(
        &mut self,
        current_company_id: u16,
        job_type_id: u8,
        min_salary: f64,
    ) -> Option<CachedSwitchOpening> {
        let threshold_bucket = ((min_salary / JOB_SWITCH_SALARY_BUCKET).floor() as i64).max(0);
        if let Some(cached) = self
            .runtime
            .cached_switch_opening(job_type_id, threshold_bucket)
            .flatten()
            && self.switch_opening_still_valid(cached, current_company_id, job_type_id, min_salary)
        {
            return Some(cached);
        }
        let best = self.best_switch_opening(current_company_id, job_type_id, min_salary);
        self.runtime
            .store_switch_opening(job_type_id, threshold_bucket, best);
        best
    }

    pub(super) fn best_switch_opening(
        &self,
        current_company_id: u16,
        job_type_id: u8,
        min_salary: f64,
    ) -> Option<CachedSwitchOpening> {
        let mut best: Option<CachedSwitchOpening> = None;

        for (company_idx, company) in self.companies.iter().enumerate() {
            if company.id == current_company_id {
                continue;
            }

            for (opening_idx, opening) in company.open_positions.iter().enumerate() {
                if opening.slots == 0 || opening.job_type_id != job_type_id {
                    continue;
                }
                if opening.offered_monthly_salary + 0.01 < min_salary {
                    continue;
                }

                match best {
                    None => {
                        best = Some(CachedSwitchOpening {
                            company_idx,
                            opening_idx,
                            offered_salary: opening.offered_monthly_salary,
                            company_id: company.id,
                        });
                    }
                    Some(current_best) => {
                        if opening.offered_monthly_salary > current_best.offered_salary
                            || ((opening.offered_monthly_salary - current_best.offered_salary)
                                .abs()
                                < 0.01
                                && company.id < current_best.company_id)
                        {
                            best = Some(CachedSwitchOpening {
                                company_idx,
                                opening_idx,
                                offered_salary: opening.offered_monthly_salary,
                                company_id: company.id,
                            });
                        }
                    }
                }
            }
        }

        best
    }

    fn switch_opening_still_valid(
        &self,
        cached: CachedSwitchOpening,
        current_company_id: u16,
        job_type_id: u8,
        min_salary: f64,
    ) -> bool {
        self.companies
            .get(cached.company_idx)
            .filter(|company| company.id != current_company_id)
            .and_then(|company| company.open_positions.get(cached.opening_idx))
            .is_some_and(|opening| {
                opening.slots > 0
                    && opening.job_type_id == job_type_id
                    && opening.offered_monthly_salary + 0.01 >= min_salary
            })
    }

    pub(in crate::sim::country) fn process_end_of_month_loan_requests(&mut self) {
        let lender_id = self
            .preferred_lender_bank_id()
            .unwrap_or(self.primary_bank().id);
        let loan_rate = self.bank_loan_rate_for(lender_id);
        let debt_to_income_cap = self.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);
        let lender_bank_idx = self.banks.iter().position(|bank| bank.id == lender_id);
        let country_name = self.name.clone();
        let day = self.day;
        let banks = &mut self.banks;
        let mut pending_events = Vec::new();
        for person in &mut self.people {
            if person.cash_balance >= 0.0 {
                continue;
            }

            let shortfall = round_cents(-person.cash_balance);
            let monthly_income = person.monthly_salary();
            let Some(bank_idx) = lender_bank_idx else {
                continue;
            };
            let bank = &mut banks[bank_idx];
            let issued = bank.originate_loan(
                shortfall,
                monthly_income,
                debt_to_income_cap,
                loan_rate,
                &mut person.loan,
            );

            if issued > 0.0 {
                person.cash_balance = round_cents(person.cash_balance + issued);
                person.last_month_new_loan = round_cents(person.last_month_new_loan + issued);
                pending_events.push(SimEvent::LoanEmitted {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Person,
                    entity_id: person.id,
                    principal: issued,
                    day,
                });
            }
        }

        for company in &mut self.companies {
            if company.cash_balance >= 0.0 {
                continue;
            }

            let shortfall = round_cents(-company.cash_balance);
            let monthly_income = company.average_monthly_revenue();
            let Some(bank_idx) = lender_bank_idx else {
                continue;
            };
            let bank = &mut banks[bank_idx];
            let issued = bank.originate_loan(
                shortfall,
                monthly_income,
                debt_to_income_cap,
                loan_rate,
                &mut company.loan,
            );

            if issued > 0.0 {
                company.cash_balance = round_cents(company.cash_balance + issued);
                company.last_month_new_loan = round_cents(company.last_month_new_loan + issued);
                pending_events.push(SimEvent::LoanEmitted {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    principal: issued,
                    day,
                });
            }
        }
        self.pending_events.extend(pending_events);
    }
}
