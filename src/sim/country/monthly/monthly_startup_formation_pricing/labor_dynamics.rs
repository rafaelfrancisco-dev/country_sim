use super::*;

impl Country {
    pub(in crate::sim::country::monthly) fn process_monthly_job_switches(&mut self) {
        if self.sim_params.labor_mobility.switch_cadence != SwitchCadence::MonthlyOnce {
            return;
        }

        let raise_threshold = self
            .sim_params
            .labor_mobility
            .switch_raise_threshold
            .max(0.0);
        for person_idx in 0..self.people.len() {
            let (person_id, current_job_id, current_salary) = {
                let person = &self.people[person_idx];
                if !person.is_employed() {
                    continue;
                }
                let Some(job_id) = person.job_type_id() else {
                    continue;
                };
                (person.id, job_id, person.monthly_salary())
            };

            let min_target_salary = current_salary * (1.0 + raise_threshold);

            // Company → Company or Bank → Company switch
            if let Some(current_company_id) = self.people[person_idx].employer_company_id() {
                let Some((target_company_idx, target_opening_idx)) =
                    self.best_switch_opening(current_company_id, current_job_id, min_target_salary)
                else {
                    continue;
                };

                let Some((job_id, new_salary)) = self
                    .companies
                    .get_mut(target_company_idx)
                    .and_then(|company| company.consume_opening(target_opening_idx))
                else {
                    continue;
                };

                // Give back opening to old employer
                if let Some(old_company) = self.companies.get_mut(current_company_id as usize) {
                    old_company.add_opening_with_salary(current_job_id, 1, current_salary);
                }

                if let Some(person) = self.people.get_mut(person_idx) {
                    if person.employer_company_id() != Some(current_company_id)
                        || person.job_type_id() != Some(current_job_id)
                    {
                        if let Some(target_company) = self.companies.get_mut(target_company_idx) {
                            target_company.add_opening_with_salary(job_id, 1, new_salary);
                        }
                        continue;
                    }

                    person.set_employment_with_salary(
                        target_company_idx as u16,
                        job_id,
                        new_salary,
                        self.day,
                    );
                    self.pending_events.push(SimEvent::JobSwitched {
                        country_name: self.name.clone(),
                        person_id,
                        from_company_id: current_company_id,
                        to_company_id: target_company_idx as u16,
                        monthly_salary: new_salary,
                        day: self.day,
                    });
                }
                continue;
            }

            // EmployedAtBank: see if any company offers a raise
            if let Some(current_bank_id) = self.people[person_idx].employer_bank_id() {
                // Find a dummy company_id=u16::MAX as sentinel for best_switch_opening
                // but we need a function that ignores bank employer. Reuse best_switch_opening
                // with a sentinel company_id that won't match anything.
                let Some((target_company_idx, target_opening_idx)) =
                    self.best_switch_opening(u16::MAX, current_job_id, min_target_salary)
                else {
                    continue;
                };

                let Some((job_id, new_salary)) = self
                    .companies
                    .get_mut(target_company_idx)
                    .and_then(|company| company.consume_opening(target_opening_idx))
                else {
                    continue;
                };

                // Return opening to old bank
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
                        if let Some(target_company) = self.companies.get_mut(target_company_idx) {
                            target_company.add_opening_with_salary(job_id, 1, new_salary);
                        }
                        continue;
                    }

                    person.set_employment_with_salary(
                        target_company_idx as u16,
                        job_id,
                        new_salary,
                        self.day,
                    );
                    self.pending_events.push(SimEvent::JobSwitched {
                        country_name: self.name.clone(),
                        person_id,
                        from_company_id: u16::MAX,
                        to_company_id: target_company_idx as u16,
                        monthly_salary: new_salary,
                        day: self.day,
                    });
                }
            }
        }
    }

    pub(super) fn best_switch_opening(
        &self,
        current_company_id: u16,
        job_type_id: u8,
        min_salary: f64,
    ) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, f64, u16)> = None;

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
                        best = Some((
                            company_idx,
                            opening_idx,
                            opening.offered_monthly_salary,
                            company.id,
                        ));
                    }
                    Some((_, _, best_salary, best_company_id)) => {
                        if opening.offered_monthly_salary > best_salary
                            || ((opening.offered_monthly_salary - best_salary).abs() < 0.01
                                && company.id < best_company_id)
                        {
                            best = Some((
                                company_idx,
                                opening_idx,
                                opening.offered_monthly_salary,
                                company.id,
                            ));
                        }
                    }
                }
            }
        }

        best.map(|(company_idx, opening_idx, _, _)| (company_idx, opening_idx))
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
