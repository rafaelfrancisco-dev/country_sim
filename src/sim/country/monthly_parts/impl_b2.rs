impl Country {
    fn try_form_startup_for_job(&mut self, founder_job_id: u8) -> bool {
        let formation = self.sim_params.company_formation;
        let Some(high_margin_good_id) = self.pick_high_margin_good(formation.high_margin_threshold)
        else {
            return false;
        };

        let mut founder_pool = self
            .people
            .iter()
            .enumerate()
            .filter_map(|(idx, person)| {
                let job_id = person.job_type_id()?;
                if job_id != founder_job_id {
                    return None;
                }

                let reserve_cash =
                    person.monthly_salary() * formation.founder_personal_reserve_months;
                let investable = round_cents((person.cash_balance - reserve_cash).max(0.0));
                if investable <= 0.0 {
                    return None;
                }

                Some((idx, person.id, investable))
            })
            .collect::<Vec<_>>();

        if founder_pool.len() < formation.min_founders_same_profession {
            return false;
        }

        founder_pool.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });
        founder_pool.truncate(formation.min_founders_same_profession);

        let total_investable = round_cents(founder_pool.iter().map(|(_, _, amount)| *amount).sum());
        if total_investable <= 0.0 {
            return false;
        }

        let founder_openings = formation
            .initial_openings_founder_profession
            .min(formation.initial_openings_total);
        let mixed_openings = formation.initial_openings_total - founder_openings;

        let founder_salary = salary_for_job(founder_job_id);
        let mut opening_plan = Vec::with_capacity(formation.initial_openings_total as usize);
        opening_plan.push((founder_job_id, founder_openings, founder_salary));

        let mixed_weighted_jobs = common_job_types()
            .iter()
            .filter(|job| job.id != founder_job_id)
            .map(|job| (job.id, job.assignment_weight))
            .collect::<Vec<_>>();

        for idx in 0..mixed_openings {
            let seed = hash64(
                (u64::from(self.id) << 48)
                    ^ (self.month << 24)
                    ^ (u64::from(self.next_startup_serial) << 8)
                    ^ u64::from(idx),
            );
            let Some(job_id) = pick_weighted_job(seed, &mixed_weighted_jobs) else {
                continue;
            };
            opening_plan.push((job_id, 1, salary_for_job(job_id)));
        }

        let monthly_opening_payroll = round_cents(
            opening_plan
                .iter()
                .map(|(_, slots, salary)| f64::from(*slots) * *salary)
                .sum(),
        );
        let required_capital =
            round_cents(monthly_opening_payroll * formation.startup_runway_months);
        if required_capital <= 0.0 || total_investable + 0.01 < required_capital {
            return false;
        }

        let company_id = self.companies.len() as u16;
        let company_name = format!("{} Startup {}", self.name, self.next_startup_serial);
        self.next_startup_serial = self.next_startup_serial.saturating_add(1);

        let mut startup = Company::new(
            company_id,
            company_name.clone(),
            common_job_types()
                .iter()
                .map(|job| job.id)
                .collect::<Vec<_>>(),
            high_margin_good_id,
        );

        let avg_price = self.average_price_for_good(high_margin_good_id);
        let margin_floor = good_by_id(high_margin_good_id)
            .map(|good| good.base_unit_cost * (1.0 + self.sim_params.goods_market.min_margin_rate))
            .unwrap_or(0.0);
        startup.unit_price = round_cents((avg_price * 0.95).max(margin_floor));

        for (job_id, slots, salary) in opening_plan {
            startup.add_opening_with_salary(job_id, slots, salary);
        }

        let mut remaining = required_capital;
        for (founder_idx, (_, _, investable)) in founder_pool.iter().copied().enumerate() {
            let contribution = if founder_idx + 1 == founder_pool.len() {
                remaining
            } else {
                let share = round_cents(required_capital * (investable / total_investable));
                share.min(remaining)
            };
            remaining = round_cents((remaining - contribution).max(0.0));

            if let Some((person_idx, _, _)) = founder_pool.get(founder_idx).copied()
                && let Some(person) = self.people.get_mut(person_idx)
            {
                person.cash_balance = round_cents((person.cash_balance - contribution).max(0.0));
            }
        }

        startup.cash_balance = round_cents(required_capital);
        self.companies.push(startup);

        for (person_idx, _, _) in founder_pool {
            let Some(person) = self.people.get_mut(person_idx) else {
                continue;
            };
            let previous_salary = person.monthly_salary();
            let previous = person.clear_employment();
            if let Some((old_company_id, old_job_id)) = previous
                && let Some(old_company) = self.companies.get_mut(old_company_id as usize)
            {
                old_company.add_opening_with_salary(old_job_id, 1, previous_salary);
            }
            person.set_employment_with_salary(company_id, founder_job_id, founder_salary);
        }

        self.pending_events.push(SimEvent::CompanyFounded {
            country_name: self.name.clone(),
            company_id,
            company_name,
            founders: formation.min_founders_same_profession as u16,
            job_type_id: founder_job_id,
            produced_good_id: high_margin_good_id,
            day: self.day,
        });

        true
    }

    fn pick_high_margin_good(&self, threshold: f64) -> Option<u8> {
        let mut total_sales = [0.0_f64; GOODS_COUNT];
        let mut total_profit = [0.0_f64; GOODS_COUNT];

        for company in &self.companies {
            let good_idx = company.produced_good_id as usize;
            if good_idx >= GOODS_COUNT {
                continue;
            }
            total_sales[good_idx] = round_cents(total_sales[good_idx] + company.last_month_sales);
            total_profit[good_idx] =
                round_cents(total_profit[good_idx] + company.last_month_profit);
        }

        let mut best: Option<(u8, f64)> = None;
        for good_id in 0..GOODS_COUNT as u8 {
            let sales = total_sales[good_id as usize];
            if sales <= 0.0 {
                continue;
            }
            let margin = total_profit[good_id as usize] / sales;
            if margin < threshold {
                continue;
            }

            match best {
                None => best = Some((good_id, margin)),
                Some((current_good_id, current_margin)) => {
                    if margin > current_margin
                        || ((margin - current_margin).abs() < f64::EPSILON
                            && good_id < current_good_id)
                    {
                        best = Some((good_id, margin));
                    }
                }
            }
        }

        best.map(|(good_id, _)| good_id)
    }

    fn average_price_for_good(&self, good_id: u8) -> f64 {
        let mut total = 0.0;
        let mut count = 0_u32;

        for company in &self.companies {
            if company.produced_good_id == good_id {
                total += company.unit_price;
                count = count.saturating_add(1);
            }
        }

        if count == 0 {
            let min_price = good_by_id(good_id)
                .map(|good| {
                    good.base_unit_cost * (1.0 + self.sim_params.goods_market.min_margin_rate)
                })
                .unwrap_or(0.0);
            return round_cents(min_price);
        }

        round_cents(total / f64::from(count))
    }

    fn process_monthly_job_switches(&mut self) {
        if self.sim_params.labor_mobility.switch_cadence != SwitchCadence::MonthlyOnce {
            return;
        }

        let raise_threshold = self
            .sim_params
            .labor_mobility
            .switch_raise_threshold
            .max(0.0);
        for person_idx in 0..self.people.len() {
            let (person_id, current_company_id, current_job_id, current_salary) = {
                let person = &self.people[person_idx];
                let Some(company_id) = person.employer_company_id() else {
                    continue;
                };
                let Some(job_id) = person.job_type_id() else {
                    continue;
                };
                (person.id, company_id, job_id, person.monthly_salary())
            };

            let min_target_salary = current_salary * (1.0 + raise_threshold);
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

                person.set_employment_with_salary(target_company_idx as u16, job_id, new_salary);
                self.pending_events.push(SimEvent::JobSwitched {
                    country_name: self.name.clone(),
                    person_id,
                    from_company_id: current_company_id,
                    to_company_id: target_company_idx as u16,
                    monthly_salary: new_salary,
                    day: self.day,
                });
            }
        }
    }

    fn best_switch_opening(
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

    pub(super) fn process_end_of_month_loan_requests(&mut self) {
        for person in &mut self.people {
            if person.cash_balance >= 0.0 {
                continue;
            }

            let shortfall = round_cents(-person.cash_balance);
            let monthly_income = person.monthly_salary();
            let issued = self.bank.originate_loan(
                shortfall,
                monthly_income,
                DEFAULT_MAX_DEBT_TO_INCOME,
                &mut person.loan,
            );

            if issued > 0.0 {
                person.cash_balance = round_cents(person.cash_balance + issued);
                person.last_month_new_loan = round_cents(person.last_month_new_loan + issued);
                self.pending_events.push(SimEvent::LoanEmitted {
                    country_name: self.name.clone(),
                    entity_kind: EntityKind::Person,
                    entity_id: person.id,
                    principal: issued,
                    day: self.day,
                });
            }
        }

        for company in &mut self.companies {
            if company.cash_balance >= 0.0 {
                continue;
            }

            let shortfall = round_cents(-company.cash_balance);
            let monthly_income = company.average_monthly_revenue();
            let issued = self.bank.originate_loan(
                shortfall,
                monthly_income,
                DEFAULT_MAX_DEBT_TO_INCOME,
                &mut company.loan,
            );

            if issued > 0.0 {
                company.cash_balance = round_cents(company.cash_balance + issued);
                company.last_month_new_loan = round_cents(company.last_month_new_loan + issued);
                self.pending_events.push(SimEvent::LoanEmitted {
                    country_name: self.name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    principal: issued,
                    day: self.day,
                });
            }
        }
    }

}
