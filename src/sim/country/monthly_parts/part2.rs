    fn resolve_company_bankruptcies(&mut self, mut bankrupt_indices: Vec<usize>) {
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
            if let Some(company) = self.companies.get_mut(company_idx) {
                company.open_positions.clear();
                principal_written_off = self.bank.write_off_loan(&mut company.loan);
                company.cash_balance = 0.0;
            }

            // Delist stock before emitting bankruptcy event
            let shares_wiped = if self.sim_params.stock_market.enabled {
                self.stock_market.delist_company(company_id)
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
            let new_id = self.companies.len() as u16;
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

    fn update_company_growth_openings(&mut self) {
        let month_seed = self.month.saturating_add(1);
        let sim_params = self.sim_params.clone();
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.open_positions_from_growth(&sim_params, month_seed);
                }
            });
    }

    fn process_startup_formation(&mut self) {
        let formation = self.sim_params.company_formation;
        let mut startups_created = 0_u16;

        for job in common_job_types() {
            loop {
                if let Some(limit) = formation.max_startups_per_month
                    && startups_created >= limit
                {
                    return;
                }

                let formed = self.try_form_startup_for_job(job.id);
                if !formed {
                    break;
                }
                startups_created = startups_created.saturating_add(1);
            }
        }
    }

    fn process_essential_company_backfill(&mut self) {
        let backfill = self.sim_params.government_backfill;
        if !backfill.enabled {
            return;
        }

        let missing_goods = self.missing_essential_goods(backfill.essential_good_max_id_inclusive);
        for good_id in missing_goods {
            self.create_public_essential_company(good_id, backfill);
        }
    }

    fn missing_essential_goods(&self, max_good_id_inclusive: u8) -> Vec<u8> {
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

    fn create_public_essential_company(&mut self, good_id: u8, backfill: GovernmentBackfillParams) {
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
        company.is_public_utility = true;
        company.is_essential_backfill = true;

        let base_price_floor = good_by_id(good_id)
            .map(|good| good.base_unit_cost * (1.0 + self.sim_params.goods_market.min_margin_rate))
            .unwrap_or(0.0);
        company.unit_price =
            round_cents(self.average_price_for_good(good_id).max(base_price_floor));

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

    fn public_opening_offer(&self, job_id: u8, backfill: GovernmentBackfillParams) -> f64 {
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

    fn market_best_opening_salary_for_job(&self, job_id: u8) -> f64 {
        self.companies
            .iter()
            .flat_map(|company| company.open_positions.iter())
            .filter(|opening| opening.job_type_id == job_id)
            .map(|opening| opening.offered_monthly_salary)
            .fold(0.0, f64::max)
    }

    fn adjust_public_utility_opening_wages(&mut self) {
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

    fn try_form_startup_for_job(&mut self, founder_job_id: u8) -> bool {
