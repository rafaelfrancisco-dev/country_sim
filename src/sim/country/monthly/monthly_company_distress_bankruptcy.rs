use super::*;

impl Country {
    pub(super) fn recompute_company_profitability(&mut self) {
        for company in &mut self.companies {
            company.recompute_profitability();
        }
    }

    pub(super) fn process_company_distress(&mut self) {
        let params = self.sim_params.company_distress;
        let mut bankrupt_indices = Vec::new();
        let labor_unemployment = self.labor_force_unemployment_rate();
        let unemployment_guard = params.countercyclical_layoff_unemployment_guard.max(0.0);
        let layoff_blocked = labor_unemployment >= unemployment_guard;
        let bankruptcy_blocked = labor_unemployment >= unemployment_guard;
        let miss_layoff_rate = params.layoff_rate.min(params.max_monthly_layoff_rate);
        let loss_layoff_rate = params
            .loss_trigger_layoff_rate
            .min(params.max_monthly_layoff_rate);

        for company_idx in 0..self.companies.len() {
            let missed = self.companies[company_idx].missed_loan_payment_this_month;
            let loss_months = self.companies[company_idx].consecutive_loss_months;
            if !missed {
                self.companies[company_idx].consecutive_missed_loan_payments = 0;
                if loss_months >= params.loss_distress_month_threshold {
                    let company_id = self.companies[company_idx].id;
                    if params.loss_trigger_wage_cut_rate > 0.0 {
                        self.apply_company_wage_cut(company_id, params.loss_trigger_wage_cut_rate);
                        self.pending_events.push(SimEvent::CompanyWageCut {
                            country_name: self.name.clone(),
                            company_id,
                            cut_rate: params.loss_trigger_wage_cut_rate,
                            day: self.day,
                        });
                    }
                    if !layoff_blocked {
                        let laid_off = self.layoff_people_by_rate(company_id, loss_layoff_rate);
                        if laid_off > 0 {
                            self.pending_events.push(SimEvent::CompanyLayoffs {
                                country_name: self.name.clone(),
                                company_id,
                                laid_off_count: laid_off,
                                day: self.day,
                            });
                        }
                    }
                }
                continue;
            }

            self.companies[company_idx].consecutive_missed_loan_payments = self.companies
                [company_idx]
                .consecutive_missed_loan_payments
                .saturating_add(1);

            let misses = self.companies[company_idx].consecutive_missed_loan_payments;
            let company_id = self.companies[company_idx].id;

            if misses == 1 {
                self.apply_company_wage_cut(company_id, params.wage_cut_rate);
                self.pending_events.push(SimEvent::CompanyWageCut {
                    country_name: self.name.clone(),
                    company_id,
                    cut_rate: params.wage_cut_rate,
                    day: self.day,
                });
                continue;
            }

            if misses == 2 {
                if !layoff_blocked {
                    let laid_off = self.layoff_people_by_rate(company_id, miss_layoff_rate);
                    if laid_off > 0 {
                        self.pending_events.push(SimEvent::CompanyLayoffs {
                            country_name: self.name.clone(),
                            company_id,
                            laid_off_count: laid_off,
                            day: self.day,
                        });
                    }
                }
                continue;
            }

            if misses >= params.bankruptcy_missed_payment_threshold {
                if bankruptcy_blocked {
                    self.companies[company_idx].consecutive_missed_loan_payments =
                        params.bankruptcy_missed_payment_threshold.saturating_sub(1);
                    continue;
                }

                let company = &self.companies[company_idx];
                let has_capacity = self.company_has_extra_loan_capacity(company);
                let deep_losses = loss_months >= params.bankruptcy_min_loss_months;
                if company.cash_balance <= 0.0 && !has_capacity && deep_losses {
                    // Attempt IPO rescue before bankruptcy
                    if self.sim_params.stock_market.enabled
                        && !self.stock_market.is_listed(company_id)
                    {
                        let rescued = self.attempt_ipo_rescue(company_idx);
                        if rescued {
                            // IPO succeeded — clear distress streak
                            self.companies[company_idx].consecutive_missed_loan_payments = 0;
                            continue;
                        }
                    }
                    bankrupt_indices.push(company_idx);
                    continue;
                }

                if !layoff_blocked {
                    let laid_off = self.layoff_people_by_rate(company_id, miss_layoff_rate);
                    if laid_off > 0 {
                        self.pending_events.push(SimEvent::CompanyLayoffs {
                            country_name: self.name.clone(),
                            company_id,
                            laid_off_count: laid_off,
                            day: self.day,
                        });
                    }
                }
            }
        }

        if !bankrupt_indices.is_empty() {
            self.resolve_company_bankruptcies(bankrupt_indices);
        }
    }

    pub(super) fn apply_company_wage_cut(&mut self, company_id: u16, cut_rate: f64) {
        let floor_ratio = self
            .sim_params
            .company_distress
            .min_wage_floor_ratio_to_catalog
            .clamp(0.0, 1.0);

        for person in &mut self.people {
            if person.employer_company_id() != Some(company_id) {
                continue;
            }

            let Some(job_id) = person.job_type_id() else {
                continue;
            };
            let floor_salary = salary_for_job(job_id) * floor_ratio;
            let next_salary =
                round_cents((person.monthly_salary() * (1.0 - cut_rate)).max(floor_salary));
            person.set_employment_with_salary(company_id, job_id, next_salary, self.day);
        }

        if let Some(company) = self.companies.get_mut(company_id as usize) {
            company.wage_offer_multiplier =
                (company.wage_offer_multiplier * (1.0 - cut_rate)).max(floor_ratio);
            for opening in &mut company.open_positions {
                let floor_salary = salary_for_job(opening.job_type_id) * floor_ratio;
                opening.offered_monthly_salary = round_cents(
                    (opening.offered_monthly_salary * (1.0 - cut_rate)).max(floor_salary),
                );
            }
        }
    }

    pub(super) fn layoff_people_by_rate(&mut self, company_id: u16, layoff_rate: f64) -> u16 {
        let mut employed = self
            .people
            .iter()
            .enumerate()
            .filter_map(|(person_idx, person)| {
                if person.employer_company_id() == Some(company_id) {
                    Some((person_idx, person.id, person.monthly_salary()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if employed.is_empty() {
            return 0;
        }

        employed.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });

        let workforce = employed.len();
        let layoff_count = ((workforce as f64 * layoff_rate).ceil() as usize)
            .max(1)
            .min(workforce);

        for (person_idx, _, _) in employed.into_iter().take(layoff_count) {
            if let Some(person) = self.people.get_mut(person_idx) {
                person.clear_employment();
            }
        }

        layoff_count as u16
    }

    pub(super) fn layoff_all_people(&mut self, company_id: u16) -> u16 {
        let mut laid_off = 0_u16;
        for person in &mut self.people {
            if person.employer_company_id() == Some(company_id) {
                person.clear_employment();
                laid_off = laid_off.saturating_add(1);
            }
        }
        laid_off
    }

    pub(super) fn company_has_extra_loan_capacity(&self, company: &Company) -> bool {
        let monthly_income = company.average_monthly_revenue();
        if monthly_income <= 0.0 {
            return false;
        }

        let debt_to_income_cap = self.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);
        let max_payment = round_cents(monthly_income * debt_to_income_cap.max(0.0));
        if max_payment <= 0.0 {
            return false;
        }

        let max_total_debt =
            max_principal_for_payment(max_payment, self.bank_loan_rate(), DEFAULT_LOAN_TERM_MONTHS);
        let existing_debt = company.loan.as_ref().map_or(0.0, |loan| loan.debt_balance);
        max_total_debt > existing_debt + 0.01
    }

    pub(crate) fn resolve_company_bankruptcies(&mut self, mut bankrupt_indices: Vec<usize>) {
        bankrupt_indices.sort_unstable();
        bankrupt_indices.dedup();

        for &company_idx in &bankrupt_indices {
            let Some(company) = self.companies.get(company_idx) else {
                continue;
            };
            let company_id = company.id;
            let company_name = company.name.clone();
            let laid_off = self.layoff_all_people(company_id);

            let mut principal_written_off = 0.0;
            let primary_bank_id = self.primary_bank().id;
            let mut loan_slot = None;
            if let Some(company) = self.companies.get_mut(company_idx) {
                company.open_positions.clear();
                loan_slot = company.loan.take();
                company.cash_balance = 0.0;
            }
            if let Some(bank) = self.bank_by_id_mut(primary_bank_id) {
                principal_written_off = bank.write_off_loan(&mut loan_slot);
            }
            if let Some(company) = self.companies.get_mut(company_idx) {
                company.loan = loan_slot;
            }

            // Delist stock before emitting bankruptcy event
            let shares_wiped = if self.sim_params.stock_market.enabled {
                self.stock_market.delist_company(self.id, company_id)
            } else {
                0
            };
            if shares_wiped > 0 {
                self.pending_events.push(SimEvent::StockDelisted {
                    country_name: self.name.clone(),
                    company_id,
                    company_name: company_name.clone(),
                    shares_wiped,
                    day: self.day,
                });
            }

            self.pending_events.push(SimEvent::CompanyBankrupt {
                country_name: self.name.clone(),
                company_id,
                company_name,
                laid_off_count: laid_off,
                principal_written_off,
                day: self.day,
            });
        }

        let mut removed = vec![false; self.companies.len()];
        for idx in &bankrupt_indices {
            if let Some(entry) = removed.get_mut(*idx) {
                *entry = true;
            }
        }

        let old_companies = std::mem::take(&mut self.companies);
        let mut old_to_new_vec = vec![None::<u16>; old_companies.len()];
        self.companies = Vec::with_capacity(old_companies.len());

        for (old_idx, mut company) in old_companies.into_iter().enumerate() {
            if removed.get(old_idx).copied().unwrap_or(false) {
                continue;
            }
            let old_company_id = company.id;
            let new_id = self.companies.len() as u16;
            if old_company_id != new_id {
                company.remember_historical_id(old_company_id);
            }
            company.id = new_id;
            old_to_new_vec[old_idx] = Some(new_id);
            self.companies.push(company);
        }

        for person in &mut self.people {
            let Some(old_company_id) = person.employer_company_id() else {
                continue;
            };
            match old_to_new_vec
                .get(old_company_id as usize)
                .copied()
                .flatten()
            {
                Some(new_company_id) => person.remap_employer_company_id(new_company_id),
                None => {
                    person.clear_employment();
                }
            }
        }

        // Remap stock market company IDs to match compacted company vec
        if self.sim_params.stock_market.enabled {
            use std::collections::HashMap;
            let old_to_new_map: HashMap<u16, u16> = old_to_new_vec
                .iter()
                .enumerate()
                .filter_map(|(old, new)| new.map(|n| (old as u16, n)))
                .collect();
            self.stock_market.remap_company_ids(&old_to_new_map);
        }
    }

    pub(super) fn labor_force_unemployment_rate(&self) -> f64 {
        let mut labor_force = 0_usize;
        let mut unemployed = 0_usize;

        for person in &self.people {
            if !person.is_labor_force_participant() {
                continue;
            }
            labor_force += 1;
            if !person.is_employed() {
                unemployed += 1;
            }
        }

        if labor_force == 0 {
            0.0
        } else {
            unemployed as f64 / labor_force as f64
        }
    }
}
