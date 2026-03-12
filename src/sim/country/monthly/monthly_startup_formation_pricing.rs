use super::*;
use crate::sim::company::minimum_viable_unit_price;
use crate::sim::good::{industry_job_types_for_good, production_job_for_good, recipe_for_good};

mod labor_dynamics;

impl Country {
    pub(super) fn try_form_startup_for_good_with_cache(
        &mut self,
        target_good_id: u8,
        startup_loan_fraction: f64,
        debt_to_income_cap: f64,
    ) -> bool {
        let formation = self.sim_params.company_formation;
        if self.companies.len() >= u16::MAX as usize {
            return false;
        }

        let Some(founder_job_id) = production_job_for_good(target_good_id) else {
            return false;
        };
        let allowed_job_types = industry_job_types_for_good(target_good_id);
        if !allowed_job_types.contains(&founder_job_id) {
            return false;
        }

        let founder_salary = salary_for_job(founder_job_id);
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

                let reserve_salary = person.monthly_salary().max(founder_salary);
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

        let founders_count = founder_pool.len() as u16;
        let founder_payroll = round_cents(f64::from(founders_count) * founder_salary);
        let expected_revenue = self.expected_revenue_for_good(target_good_id);
        if expected_revenue <= 0.0 {
            return false;
        }
        if founder_payroll
            > round_cents(expected_revenue * formation.startup_max_payroll_to_revenue.max(0.0))
        {
            return false;
        }

        let company_id = self.companies.len() as u16;
        let company_name = format!("{} Startup {}", self.name, self.next_startup_serial);
        self.next_startup_serial = self.next_startup_serial.saturating_add(1);

        let mut startup = Company::new(
            company_id,
            company_name.clone(),
            allowed_job_types.to_vec(),
            target_good_id,
        );
        startup.deposit_bank_id = self
            .preferred_private_bank_id()
            .or(self.public_bank_id_or_first());

        let avg_price = self.average_price_for_good(target_good_id);
        let margin_floor = minimum_viable_unit_price(
            target_good_id,
            self.sim_params.goods_market.worker_effort_per_month,
            self.sim_params.goods_market.min_margin_rate,
        );
        startup.unit_price = round_cents((avg_price * 0.95).max(margin_floor));

        let projected_input_cost = self.estimate_startup_monthly_input_cost(
            target_good_id,
            founders_count.into(),
            startup.unit_price,
            expected_revenue,
        );
        let projected_facility_cost =
            self.estimate_startup_monthly_facility_cost(founders_count.into());
        let projected_operating_cost =
            round_cents(founder_payroll + projected_input_cost + projected_facility_cost);
        if projected_operating_cost <= 0.0 {
            return false;
        }

        let required_capital =
            round_cents(projected_operating_cost * formation.startup_runway_months.max(0.0));
        let founder_required = round_cents(required_capital.min(total_investable));

        let lender_id = self
            .preferred_lender_bank_id()
            .unwrap_or(self.primary_bank().id);
        let loan_rate = self.bank_loan_rate_for(lender_id);
        let mut desired_loan = 0.0_f64;
        let mut projected_loan_payment = 0.0_f64;
        if founder_required + 0.01 < required_capital && formation.startup_bank_loan_enabled {
            let shortfall = round_cents(required_capital - founder_required);
            let max_loan = round_cents(required_capital * startup_loan_fraction);
            let monthly_income_cap = round_cents(expected_revenue * debt_to_income_cap.max(0.0));
            let principal_capacity =
                max_principal_for_payment(monthly_income_cap, loan_rate, DEFAULT_LOAN_TERM_MONTHS);
            desired_loan = round_cents(shortfall.min(max_loan).min(principal_capacity));
            if desired_loan > 0.0 {
                projected_loan_payment = round_cents(payment_for(
                    desired_loan,
                    loan_rate,
                    DEFAULT_LOAN_TERM_MONTHS,
                ));
            }
        }
        if total_investable + desired_loan + 0.01 < required_capital {
            return false;
        }

        let projected_profit = self.estimate_startup_monthly_profit(
            target_good_id,
            founders_count.into(),
            startup.unit_price,
            expected_revenue,
            founder_payroll,
            projected_input_cost,
            projected_facility_cost,
            projected_loan_payment,
        );
        if projected_profit < 0.0 {
            return false;
        }

        if desired_loan > 0.0 {
            let use_public_bank = self.primary_bank().id == lender_id;
            let private_bank_idx = if use_public_bank {
                None
            } else {
                self.banks.iter().position(|b| b.id == lender_id)
            };
            let bank = match private_bank_idx {
                Some(bank_idx) if !use_public_bank => &mut self.banks[bank_idx],
                _ => self.primary_bank_mut(),
            };
            let startup_loan = bank.originate_loan(
                desired_loan,
                expected_revenue,
                debt_to_income_cap,
                loan_rate,
                &mut startup.loan,
            );
            if startup_loan + 0.01 < desired_loan {
                return false;
            }
        }

        let startup_loan = startup.loan.as_ref().map_or(0.0, |loan| loan.debt_balance);
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

        for (person_idx, _, _) in founder_pool {
            let Some(person) = self.people.get_mut(person_idx) else {
                continue;
            };
            let previous_salary = person.monthly_salary();
            let previous = person.clear_employment();
            if let Some(crate::sim::person::ClearedEmployment::Company {
                company_id: old_company_id,
                job_type_id: old_job_id,
            }) = previous
                && let Some(old_company) = self.companies.get_mut(old_company_id as usize)
            {
                old_company.add_opening_with_salary(old_job_id, 1, previous_salary);
            }
            person.set_employment_with_salary(company_id, founder_job_id, founder_salary, self.day);
        }

        self.pending_events.push(SimEvent::CompanyFounded {
            country_name: self.name.clone(),
            company_id,
            company_name: company_name.into(),
            founders: founders_count,
            job_type_id: founder_job_id,
            produced_good_id: target_good_id,
            day: self.day,
        });

        true
    }

    pub(super) fn startup_candidate_goods(
        &self,
        margin_threshold: f64,
        necessity_sales_floor: f64,
    ) -> Vec<u8> {
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

        let mut ranked = Vec::new();
        let mut best_necessity: Option<(u8, f64)> = None;
        for good_id in 0..=4_u8 {
            if production_job_for_good(good_id).is_none() {
                continue;
            }
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
            ranked.push(good_id);
        }

        let mut high_margin = Vec::new();
        for good_id in 0..GOODS_COUNT as u8 {
            if ranked.contains(&good_id) || production_job_for_good(good_id).is_none() {
                continue;
            }
            let sales = sales_by_good[good_id as usize];
            if sales <= 0.0 {
                continue;
            }
            let margin = profit_by_good[good_id as usize] / sales;
            if margin < margin_threshold {
                continue;
            }
            high_margin.push((good_id, margin));
        }
        high_margin.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked.extend(high_margin.into_iter().map(|(good_id, _)| good_id));
        ranked
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

        let observed_entrant_revenue = if producers == 0 {
            0.0
        } else {
            round_cents(total_sales / (f64::from(producers) + 1.0))
        };

        let Some(good) = good_by_id(good_id) else {
            return observed_entrant_revenue;
        };
        let market_price = self
            .average_price_for_good(good_id)
            .max(minimum_viable_unit_price(
                good_id,
                self.sim_params.goods_market.worker_effort_per_month,
                self.sim_params.goods_market.min_margin_rate,
            ));
        let active_population = self
            .people
            .iter()
            .filter(|person| person.can_seek_job())
            .count();
        if active_population == 0 {
            return observed_entrant_revenue;
        }
        let demand_units = active_population as f64 * good.monthly_demand_factor.max(0.0);
        let incumbent_count = producers.max(1) as f64;
        let entrant_share = (1.0 / (incumbent_count + 1.0)).clamp(0.05, 0.25);
        let demand_proxy = round_cents(demand_units * market_price * entrant_share);

        let company_demand_proxy = self.company_input_demand_proxy_value(good_id, market_price);

        round_cents(
            observed_entrant_revenue
                .max(demand_proxy)
                .max(company_demand_proxy),
        )
    }

    fn estimate_startup_monthly_output_units(&self, good_id: u8, staffed_workers: u32) -> f64 {
        let Some(good) = good_by_id(good_id) else {
            return 0.0;
        };
        if staffed_workers == 0 || good.effort_per_unit <= 0.0 {
            return 0.0;
        }

        (f64::from(staffed_workers) * self.sim_params.goods_market.worker_effort_per_month
            / good.effort_per_unit)
            .max(0.0)
    }

    fn estimate_startup_monthly_input_cost(
        &self,
        good_id: u8,
        staffed_workers: u32,
        unit_price: f64,
        expected_revenue: f64,
    ) -> f64 {
        if unit_price <= 0.0 {
            return 0.0;
        }

        let sellable_units = self
            .estimate_startup_monthly_output_units(good_id, staffed_workers)
            .min(expected_revenue / unit_price);
        round_cents(
            recipe_for_good(good_id)
                .inputs()
                .map(|(input_good_id, required_per_unit)| {
                    let unit_cost = self
                        .average_price_for_good(input_good_id)
                        .max(good_by_id(input_good_id).map_or(0.0, |good| good.base_unit_cost));
                    sellable_units * f64::from(required_per_unit) * unit_cost
                })
                .sum(),
        )
    }

    fn estimate_startup_monthly_facility_cost(&self, staffed_headcount: u32) -> f64 {
        if staffed_headcount == 0 || self.monthly_rent <= 0.0 {
            return 0.0;
        }

        round_cents(self.monthly_rent * f64::from(staffed_headcount.div_ceil(12).max(1)))
    }

    fn estimate_startup_monthly_profit(
        &self,
        good_id: u8,
        staffed_workers: u32,
        unit_price: f64,
        expected_revenue: f64,
        payroll: f64,
        input_cost: f64,
        facility_cost: f64,
        projected_loan_payment: f64,
    ) -> f64 {
        let capacity_revenue = round_cents(
            self.estimate_startup_monthly_output_units(good_id, staffed_workers) * unit_price,
        );
        let realizable_revenue = round_cents(expected_revenue.min(capacity_revenue));
        round_cents(
            realizable_revenue - payroll - input_cost - facility_cost - projected_loan_payment,
        )
    }

    fn company_input_demand_proxy_value(&self, input_good_id: u8, market_price: f64) -> f64 {
        let total_required_units = self
            .companies
            .iter()
            .map(|company| {
                let required_per_unit = company
                    .recipe_for_output_good()
                    .units_required_for(input_good_id);
                if required_per_unit == 0 {
                    return 0.0;
                }

                let output_units = company
                    .last_month_units_produced
                    .max(company.last_completed_month_units_produced)
                    .max(company.last_month_units_sold)
                    .max(company.last_completed_month_units_sold);
                f64::from(output_units) * f64::from(required_per_unit)
            })
            .sum::<f64>();

        round_cents(total_required_units * market_price)
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
            let min_price = minimum_viable_unit_price(
                good_id,
                self.sim_params.goods_market.worker_effort_per_month,
                self.sim_params.goods_market.min_margin_rate,
            );
            return round_cents(min_price);
        }

        round_cents(total / f64::from(count))
    }
}
