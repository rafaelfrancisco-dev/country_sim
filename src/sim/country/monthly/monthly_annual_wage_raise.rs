use super::*;
use std::collections::HashMap;

impl Country {
    pub(super) fn process_annual_wage_raises(&mut self) {
        let annual = self.sim_params.annual_wage_raise;
        if !annual.enabled {
            return;
        }

        let cadence = u64::from(annual.cadence_months.max(1));
        let next_month = self.month.saturating_add(1);
        if !next_month.is_multiple_of(cadence) {
            return;
        }

        let company_count = self.companies.len();
        if company_count == 0 {
            return;
        }

        let job_types = common_job_types();
        if job_types.is_empty() {
            return;
        }

        let job_count = job_types.len();
        let mut job_index = [usize::MAX; 256];
        for (idx, job) in job_types.iter().enumerate() {
            job_index[job.id as usize] = idx;
        }

        let company_idx_by_id = self
            .companies
            .iter()
            .enumerate()
            .map(|(idx, company)| (company.id, idx))
            .collect::<HashMap<_, _>>();

        let mut contract_sum = vec![vec![0.0_f64; job_count]; company_count];
        let mut contract_count = vec![vec![0_u32; job_count]; company_count];
        let mut opening_sum = vec![vec![0.0_f64; job_count]; company_count];
        let mut opening_slots = vec![vec![0_u32; job_count]; company_count];
        let mut company_market_best = vec![vec![0.0_f64; job_count]; company_count];

        for person in &self.people {
            let Some(company_id) = person.employer_company_id() else {
                continue;
            };
            let Some(job_id) = person.job_type_id() else {
                continue;
            };
            let Some(&company_idx) = company_idx_by_id.get(&company_id) else {
                continue;
            };
            let job_idx = job_index[job_id as usize];
            if job_idx == usize::MAX {
                continue;
            }

            let salary = person.monthly_salary().max(0.0);
            contract_sum[company_idx][job_idx] += salary;
            contract_count[company_idx][job_idx] =
                contract_count[company_idx][job_idx].saturating_add(1);
            company_market_best[company_idx][job_idx] =
                company_market_best[company_idx][job_idx].max(salary);
        }

        for (company_idx, company) in self.companies.iter().enumerate() {
            for opening in &company.open_positions {
                if opening.slots == 0 {
                    continue;
                }
                let job_idx = job_index[opening.job_type_id as usize];
                if job_idx == usize::MAX {
                    continue;
                }

                let slots = u32::from(opening.slots);
                opening_sum[company_idx][job_idx] +=
                    opening.offered_monthly_salary.max(0.0) * f64::from(slots);
                opening_slots[company_idx][job_idx] =
                    opening_slots[company_idx][job_idx].saturating_add(slots);
                company_market_best[company_idx][job_idx] = company_market_best[company_idx]
                    [job_idx]
                    .max(opening.offered_monthly_salary.max(0.0));
            }
        }

        let min_history_months = usize::from(annual.min_history_months.max(1));
        let prior_history_months = min_history_months.saturating_sub(1);
        let min_cash_runway_months = annual.min_cash_runway_months.max(0.0);

        let mut raise_rate_by_company = vec![0.0_f64; company_count];
        for (company_idx, company) in self.companies.iter().enumerate() {
            if company.monthly_payroll_budget <= 0.0 {
                continue;
            }
            if company.profit_history.len() < prior_history_months {
                continue;
            }

            let recent_history_sum: f64 = company
                .profit_history
                .iter()
                .rev()
                .take(prior_history_months)
                .sum();
            let avg_profit = round_cents(
                (recent_history_sum + company.last_month_profit) / min_history_months as f64,
            );
            if avg_profit <= 0.0 {
                continue;
            }
            let profit_to_payroll_ratio =
                avg_profit / company.monthly_payroll_budget.max(f64::EPSILON);
            if profit_to_payroll_ratio + 0.000_001 < annual.min_profit_to_payroll_ratio.max(0.0) {
                continue;
            }

            let required_cash =
                round_cents(company.monthly_payroll_budget.max(0.0) * min_cash_runway_months);
            if company.cash_balance + 0.01 < required_cash {
                continue;
            }

            let mut raise_rate = 0.0_f64;
            for job_idx in 0..job_count {
                let internal_salary = if contract_count[company_idx][job_idx] > 0 {
                    round_cents(
                        contract_sum[company_idx][job_idx]
                            / f64::from(contract_count[company_idx][job_idx]),
                    )
                } else if opening_slots[company_idx][job_idx] > 0 {
                    round_cents(
                        opening_sum[company_idx][job_idx]
                            / f64::from(opening_slots[company_idx][job_idx]),
                    )
                } else {
                    0.0
                };

                if internal_salary <= 0.0 {
                    continue;
                }

                let current_is_public = company.is_public_utility;
                let external_salary = company_market_best
                    .iter()
                    .enumerate()
                    .filter(|(other_idx, _)| *other_idx != company_idx)
                    .filter(|(other_idx, _)| {
                        current_is_public || !self.companies[*other_idx].is_public_utility
                    })
                    .map(|(_, other_market_best)| other_market_best[job_idx])
                    .fold(0.0_f64, f64::max);

                if external_salary <= internal_salary + 0.01 {
                    continue;
                }

                let ratio =
                    (external_salary / internal_salary - 1.0) * annual.market_gap_convergence;
                if ratio.is_finite() {
                    raise_rate = raise_rate.max(ratio.max(0.0));
                }
            }

            if let Some(max_raise_rate) = annual.max_raise_rate {
                raise_rate = raise_rate.min(max_raise_rate.max(0.0));
            }
            if raise_rate > 0.0 {
                raise_rate_by_company[company_idx] = raise_rate;
            }
        }

        for person in &mut self.people {
            let Some(company_id) = person.employer_company_id() else {
                continue;
            };
            let Some(job_id) = person.job_type_id() else {
                continue;
            };
            let Some(&company_idx) = company_idx_by_id.get(&company_id) else {
                continue;
            };
            let raise_rate = raise_rate_by_company[company_idx];
            if raise_rate <= 0.0 {
                continue;
            }

            let next_salary = round_cents(person.monthly_salary() * (1.0 + raise_rate));
            person.set_employment_with_salary(company_id, job_id, next_salary, self.day);
        }

        for (company_idx, company) in self.companies.iter_mut().enumerate() {
            let raise_rate = raise_rate_by_company[company_idx];
            if raise_rate <= 0.0 {
                continue;
            }

            let multiplier = 1.0 + raise_rate;
            company.wage_offer_multiplier *= multiplier;
            for opening in &mut company.open_positions {
                opening.offered_monthly_salary =
                    round_cents(opening.offered_monthly_salary * multiplier);
            }
        }
    }
}
