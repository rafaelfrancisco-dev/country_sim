use super::*;
use crate::sim::bank::BankKind;

#[path = "monthly_growth_backfill_public_wages_helpers.rs"]
mod helpers;

impl Country {
    pub(super) fn update_company_growth_openings(&mut self) {
        let month_seed = self.month.saturating_add(1);
        let expansion_mult = self.expansion_multiplier().max(0.01);
        let inverse_expansion = (1.0 / expansion_mult).max(0.01);
        let mut company_growth = self.sim_params.company_growth;
        company_growth.min_profitable_months = ((company_growth.min_profitable_months as f64
            * inverse_expansion)
            .round()
            .clamp(1.0, u8::MAX as f64)) as u8;
        company_growth.min_cash_reserve =
            round_cents((company_growth.min_cash_reserve * inverse_expansion).max(0.0));
        company_growth.payroll_reserve_multiplier =
            (company_growth.payroll_reserve_multiplier * inverse_expansion).max(0.0);
        company_growth.profit_per_opening =
            round_cents((company_growth.profit_per_opening * inverse_expansion).max(1.0));
        company_growth.max_monthly_openings = ((company_growth.max_monthly_openings as f64
            * expansion_mult)
            .round()
            .clamp(1.0, u16::MAX as f64)) as u16;

        let labor_demand = self.sim_params.labor_demand;
        if self.use_inner_parallelism {
            self.companies
                .par_chunks_mut(COMPANY_CHUNK_SIZE)
                .for_each(|chunk| {
                    for company in chunk {
                        company.open_positions_from_growth(&company_growth, month_seed);
                        company.ensure_baseline_openings(labor_demand, month_seed);
                    }
                });
        } else {
            for company in &mut self.companies {
                company.open_positions_from_growth(&company_growth, month_seed);
                company.ensure_baseline_openings(labor_demand, month_seed);
            }
        }
    }

    pub(super) fn process_startup_formation(&mut self) {
        let formation = self.sim_params.company_formation;
        let Some(limit) = formation.max_startups_per_month else {
            return;
        };
        if limit == 0 {
            return;
        }

        let labor_force = self
            .people
            .iter()
            .filter(|person| person.is_labor_force_participant())
            .count();
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

        let candidate_goods = self.startup_candidate_goods(
            formation.high_margin_threshold,
            formation.necessity_startup_sales_floor,
        );
        if candidate_goods.is_empty() {
            return;
        }

        let formation = self.sim_params.company_formation;
        let startup_loan_fraction = (formation.startup_bank_loan_max_fraction
            * self.loan_appetite_multiplier())
        .clamp(0.0, 1.0);
        let debt_to_income_cap = self.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);

        let mut startups_created = 0_u16;
        let mut failed_attempts = 0_usize;
        let max_attempts = (startup_budget as usize)
            .saturating_mul(candidate_goods.len())
            .max(candidate_goods.len());

        while startups_created < startup_budget && failed_attempts < max_attempts {
            let mut formed = false;
            for &good_id in &candidate_goods {
                if self.try_form_startup_for_good_with_cache(
                    good_id,
                    startup_loan_fraction,
                    debt_to_income_cap,
                ) {
                    startups_created = startups_created.saturating_add(1);
                    failed_attempts = 0;
                    formed = true;
                    break;
                }
            }
            if !formed {
                failed_attempts = failed_attempts.saturating_add(1);
            }
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
            let private_count = std::iter::once(self.primary_bank())
                .chain(self.banks.iter())
                .filter(|bank| bank.kind == BankKind::Private)
                .count();
            if private_count >= market_capacity {
                break;
            }
            if let Some((bank_id, seed_capital)) = self.form_private_bank_from_founders() {
                let bank_employee_job_types = params.bank_employee_job_types;
                let bank_wage_multiplier = params.bank_wage_multiplier.max(1.0);
                if let Some(new_bank) = self.bank_by_id_mut(bank_id) {
                    let target = new_bank.target_employee_count(
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
                        .unwrap_or_else(|| format!("{} Private Bank", self.name).into()),
                    kind: BankKind::Private,
                    seed_capital,
                    day: self.day,
                });
                formed = formed.saturating_add(1);
            } else {
                break;
            }
        }

        let min_deposits_to_recapitalize = params.public_bank_recapitalize_amount * 0.1;
        if self.primary_bank().deposit_liabilities >= min_deposits_to_recapitalize
            && self.bank_health_score(self.primary_bank().id)
                < (params.public_health_floor + 0.05).clamp(0.0, 1.0)
        {
            self.recapitalize_public_bank(params.public_bank_recapitalize_amount);
        }

        self.ensure_entity_deposit_assignments();
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
        self.expand_essential_capacity_under_shortage(backfill);
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
}
