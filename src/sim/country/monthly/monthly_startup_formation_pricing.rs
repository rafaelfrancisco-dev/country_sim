use super::*;

mod labor_dynamics;

impl Country {
    pub(super) fn try_form_startup_for_job(&mut self, founder_job_id: u8) -> bool {
        let formation = self.sim_params.company_formation;
        let startup_loan_fraction = (formation.startup_bank_loan_max_fraction
            * self.loan_appetite_multiplier())
        .clamp(0.0, 1.0);
        let debt_to_income_cap = self.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);
        if self.companies.len() >= u16::MAX as usize {
            return false;
        }

        let Some(high_margin_good_id) = self.pick_startup_target_good(
            formation.high_margin_threshold,
            formation.necessity_startup_sales_floor,
        ) else {
            return false;
        };

        let mut founder_pool = self
            .people
            .iter()
            .enumerate()
            .filter_map(|(idx, person)| {
                if !person.can_seek_job() {
                    return None;
                }
                let matches_job = person.job_type_id() == Some(founder_job_id);
                let eligible_unemployed = formation.allow_unemployed_founders
                    && !person.is_employed()
                    && job_is_eligible_for_habilitation(founder_job_id, person.habilitation);
                if !matches_job && !eligible_unemployed {
                    return None;
                }

                let reserve_salary = person.monthly_salary().max(salary_for_job(founder_job_id));
                let reserve_cash = reserve_salary * formation.founder_personal_reserve_months;
                let mut investable = round_cents((person.cash_balance - reserve_cash).max(0.0));
                if investable <= 0.0 {
                    if formation.startup_bank_loan_enabled && formation.allow_unemployed_founders {
                        investable = 1.0;
                    } else {
                        return None;
                    }
                }

                Some((idx, person.id, investable))
            })
            .collect::<Vec<_>>();

        if founder_pool.len() < formation.min_founders {
            return false;
        }

        founder_pool.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });
        founder_pool.truncate(formation.max_founders.max(formation.min_founders));
        if founder_pool.len() < formation.min_founders {
            return false;
        }

        let total_investable = round_cents(founder_pool.iter().map(|(_, _, amount)| *amount).sum());
        if total_investable <= 0.0 {
            return false;
        }

        let labor_slack =
            (self.labor_force_unemployment_rate() - formation.labor_slack_startup_trigger).max(0.0);
        let dynamic_openings = formation
            .startup_openings_min
            .saturating_add((labor_slack * 20.0).round() as u16);
        let max_openings = formation
            .startup_openings_max
            .max(formation.startup_openings_min);
        let target_openings = dynamic_openings.clamp(formation.startup_openings_min, max_openings);

        let mut selected_opening_plan: Option<Vec<(u8, u16, f64)>> = None;
        let mut selected_required_capital = 0.0_f64;
        let mut selected_expected_revenue = 0.0_f64;

        for openings_total in (formation.startup_openings_min..=target_openings).rev() {
            let founder_openings = formation
                .initial_openings_founder_profession
                .max(1)
                .min(openings_total);
            let mixed_openings = openings_total.saturating_sub(founder_openings);
            let founder_salary = salary_for_job(founder_job_id);
            let mut opening_plan = Vec::with_capacity(openings_total as usize);
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
            if monthly_opening_payroll <= 0.0 {
                continue;
            }
            let expected_revenue = self.expected_revenue_for_good(high_margin_good_id);
            if expected_revenue <= 0.0 {
                continue;
            }
            if monthly_opening_payroll
                > round_cents(expected_revenue * formation.startup_max_payroll_to_revenue.max(0.0))
            {
                continue;
            }

            let required_capital =
                round_cents(monthly_opening_payroll * formation.startup_runway_months);
            let max_loan = if formation.startup_bank_loan_enabled {
                round_cents(required_capital * startup_loan_fraction)
            } else {
                0.0
            };
            if total_investable + max_loan + 0.01 < required_capital {
                continue;
            }

            selected_opening_plan = Some(opening_plan);
            selected_required_capital = required_capital;
            selected_expected_revenue = expected_revenue;
            break;
        }

        let Some(opening_plan) = selected_opening_plan else {
            return false;
        };
        let founder_salary = salary_for_job(founder_job_id);
        let required_capital = selected_required_capital;
        let expected_revenue = selected_expected_revenue;

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
        startup.deposit_bank_id = self
            .preferred_private_bank_id()
            .or(self.public_bank_id_or_first());

        let avg_price = self.average_price_for_good(high_margin_good_id);
        let margin_floor = good_by_id(high_margin_good_id)
            .map(|good| good.base_unit_cost * (1.0 + self.sim_params.goods_market.min_margin_rate))
            .unwrap_or(0.0);
        startup.unit_price = round_cents((avg_price * 0.95).max(margin_floor));

        for (job_id, slots, salary) in &opening_plan {
            startup.add_opening_with_salary(*job_id, *slots, *salary);
        }

        let mut startup_loan = 0.0_f64;
        let founder_required = round_cents(required_capital.min(total_investable));
        if founder_required + 0.01 < required_capital && formation.startup_bank_loan_enabled {
            let shortfall = round_cents(required_capital - founder_required);
            let max_loan = round_cents(required_capital * startup_loan_fraction);
            let desired_loan = shortfall.min(max_loan);
            if desired_loan > 0.0 {
                startup_loan = self.bank.originate_loan(
                    desired_loan,
                    expected_revenue,
                    debt_to_income_cap,
                    self.bank_loan_rate(),
                    &mut startup.loan,
                );
            }
        }
        let founder_capital_needed = round_cents((required_capital - startup_loan).max(0.0));
        if founder_capital_needed > total_investable + 0.01 {
            return false;
        }

        let mut remaining = founder_capital_needed;
        let mut founder_contributions: Vec<(usize, u32, f64)> =
            Vec::with_capacity(founder_pool.len());
        for (founder_idx, (_, _, investable)) in founder_pool.iter().copied().enumerate() {
            let contribution = if founder_idx + 1 == founder_pool.len() {
                founder_capital_needed.min(remaining)
            } else {
                let share = round_cents(founder_capital_needed * (investable / total_investable));
                share.min(remaining)
            };
            remaining = round_cents((remaining - contribution).max(0.0));

            if let Some((person_idx, _, _)) = founder_pool.get(founder_idx).copied()
                && let Some(person) = self.people.get_mut(person_idx)
            {
                person.cash_balance = round_cents((person.cash_balance - contribution).max(0.0));
                founder_contributions.push((person_idx, person.id, contribution));
            }
        }

        startup.cash_balance = round_cents(founder_capital_needed + startup_loan);
        let founder_share_base = self.sim_params.stock_market.ipo_initial_shares.max(1);
        let mut remaining_shares = founder_share_base;
        for (idx, (_, person_id, contribution)) in founder_contributions.iter().copied().enumerate()
        {
            let shares_owned = if idx + 1 == founder_contributions.len() {
                remaining_shares
            } else if founder_capital_needed <= 0.0 {
                let even = founder_share_base / founder_contributions.len() as u64;
                even.min(remaining_shares)
            } else {
                let proportional = ((founder_share_base as f64)
                    * (contribution / founder_capital_needed))
                    .round() as u64;
                proportional.max(1).min(remaining_shares)
            };
            remaining_shares = remaining_shares.saturating_sub(shares_owned);
            startup.founder_stakes.push(FounderStake {
                person_id,
                shares_owned,
                cash_contributed: contribution,
            });
        }
        self.companies.push(startup);

        let founders_count = founder_pool.len() as u16;
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
            person.set_employment_with_salary(company_id, founder_job_id, founder_salary, self.day);
        }

        self.pending_events.push(SimEvent::CompanyFounded {
            country_name: self.name.clone(),
            company_id,
            company_name,
            founders: founders_count,
            job_type_id: founder_job_id,
            produced_good_id: high_margin_good_id,
            day: self.day,
        });

        true
    }

    pub(super) fn pick_startup_target_good(
        &self,
        margin_threshold: f64,
        necessity_sales_floor: f64,
    ) -> Option<u8> {
        let mut sales_by_good = [0.0_f64; GOODS_COUNT];
        let mut producer_count = [0_u32; GOODS_COUNT];
        let mut profit_by_good = [0.0_f64; GOODS_COUNT];
        for company in &self.companies {
            let good_idx = company.produced_good_id as usize;
            if good_idx >= GOODS_COUNT {
                continue;
            }
            sales_by_good[good_idx] =
                round_cents(sales_by_good[good_idx] + company.last_month_sales);
            profit_by_good[good_idx] =
                round_cents(profit_by_good[good_idx] + company.last_month_profit);
            producer_count[good_idx] = producer_count[good_idx].saturating_add(1);
        }

        let mut best_necessity: Option<(u8, f64)> = None;
        for good_id in 0..=4_u8 {
            let sales = sales_by_good[good_id as usize];
            if sales < necessity_sales_floor {
                continue;
            }
            let producers = producer_count[good_id as usize].max(1);
            let score = sales / f64::from(producers);
            match best_necessity {
                None => best_necessity = Some((good_id, score)),
                Some((_, best_score)) if score > best_score => {
                    best_necessity = Some((good_id, score))
                }
                _ => {}
            }
        }
        if let Some((good_id, _)) = best_necessity {
            return Some(good_id);
        }

        self.pick_high_margin_good(margin_threshold)
    }

    pub(super) fn expected_revenue_for_good(&self, good_id: u8) -> f64 {
        let mut total_sales = 0.0_f64;
        let mut producers = 0_u32;
        for company in &self.companies {
            if company.produced_good_id != good_id {
                continue;
            }
            total_sales = round_cents(total_sales + company.last_month_sales);
            producers = producers.saturating_add(1);
        }

        let observed_avg = if producers == 0 {
            0.0
        } else {
            round_cents(total_sales / f64::from(producers))
        };

        let Some(good) = good_by_id(good_id) else {
            return observed_avg;
        };
        let market_price = self
            .average_price_for_good(good_id)
            .max(good.base_unit_cost);
        let active_population = self
            .people
            .iter()
            .filter(|person| person.can_seek_job())
            .count();
        if active_population == 0 {
            return observed_avg;
        }
        let demand_units = active_population as f64 * good.monthly_demand_factor.max(0.0);
        let incumbent_count = producers.max(1) as f64;
        let entrant_share = (1.0 / (incumbent_count + 1.0)).clamp(0.05, 0.25);
        let demand_proxy = round_cents(demand_units * market_price * entrant_share);

        round_cents(observed_avg.max(demand_proxy))
    }

    pub(super) fn pick_high_margin_good(&self, threshold: f64) -> Option<u8> {
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

    pub(super) fn average_price_for_good(&self, good_id: u8) -> f64 {
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
}
