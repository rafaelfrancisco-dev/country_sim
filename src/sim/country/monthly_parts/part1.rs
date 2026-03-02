use super::*;
use crate::sim::{
    bank::{DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS, max_principal_for_payment},
    good::{GOODS_COUNT, good_by_id},
    job::{common_job_types, salary_for_job},
    money::round_cents,
    params::{GovernmentBackfillParams, SwitchCadence},
    stock_market::{ListedEquity, StockMarket},
};
#[cfg(feature = "bench-trace")]
use crate::time_op;
use std::cmp::Ordering;

impl Country {
    #[cfg(test)]
    pub fn process_month(&mut self) {
        self.process_month_end();
        self.reset_month_trackers();
    }

    pub(crate) fn process_month_end(&mut self) {
        self.recompute_company_headcounts();

        self.ensure_company_payroll_liquidity();
        self.process_payroll_and_taxes();
        self.process_government_spending();
        self.process_stock_collateral_seizure();
        self.collect_monthly_loan_repayments();
        self.recompute_company_profitability();
        self.process_company_distress();
        self.process_end_of_month_loan_requests();
        self.update_company_growth_openings();
        self.process_startup_formation();
        self.process_essential_company_backfill();
        self.adjust_public_utility_opening_wages();
        self.process_monthly_job_switches();
        self.recompute_company_headcounts();
        self.close_company_months();
        self.process_stock_dividends();

        self.month = self.month.saturating_add(1);
        self.recompute_aggregates();
    }

    #[cfg(feature = "bench-trace")]
    pub(crate) fn process_month_end_traced(&mut self) -> crate::sim::trace::CountryMonthlyTrace {
        let total_started = ::std::time::Instant::now();

        let recompute_headcounts_before_ms = time_op!(self.recompute_company_headcounts());
        let payroll_liquidity_ms = time_op!(self.ensure_company_payroll_liquidity());
        let payroll_taxes_ms = time_op!(self.process_payroll_and_taxes());
        let gov_spending_ms = time_op!(self.process_government_spending());
        let stock_collateral_ms = time_op!(self.process_stock_collateral_seizure());
        let loan_repayments_ms = time_op!(self.collect_monthly_loan_repayments());
        let profitability_ms = time_op!(self.recompute_company_profitability());
        let company_distress_ms = time_op!(self.process_company_distress());
        let loan_requests_ms = time_op!(self.process_end_of_month_loan_requests());
        let growth_openings_ms = time_op!(self.update_company_growth_openings());
        let startup_formation_ms = time_op!(self.process_startup_formation());
        let essential_backfill_ms = time_op!(self.process_essential_company_backfill());
        let public_wage_adjust_ms = time_op!(self.adjust_public_utility_opening_wages());
        let job_switches_ms = time_op!(self.process_monthly_job_switches());
        let recompute_headcounts_after_ms = time_op!(self.recompute_company_headcounts());
        let close_company_months_ms = time_op!(self.close_company_months());
        let dividends_ms = time_op!(self.process_stock_dividends());

        self.month = self.month.saturating_add(1);

        let recompute_aggregates_ms = time_op!(self.recompute_aggregates());

        crate::sim::trace::CountryMonthlyTrace {
            total_ms: crate::sim::trace::duration_ms(total_started.elapsed()),
            recompute_headcounts_before_ms,
            payroll_liquidity_ms,
            payroll_taxes_ms,
            gov_spending_ms,
            stock_collateral_ms,
            loan_repayments_ms,
            profitability_ms,
            company_distress_ms,
            loan_requests_ms,
            growth_openings_ms,
            startup_formation_ms,
            essential_backfill_ms,
            public_wage_adjust_ms,
            job_switches_ms,
            recompute_headcounts_after_ms,
            close_company_months_ms,
            dividends_ms,
            recompute_aggregates_ms,
        }
    }

    pub(crate) fn reset_month_trackers(&mut self) {
        self.monthly_tax_collected = 0.0;
        self.monthly_household_spending = 0.0;
        self.monthly_tariff_collected = 0.0;
        self.monthly_goods_import_spending = 0.0;
        self.monthly_goods_export_revenue = 0.0;
        self.monthly_unmet_need_events = 0;
        self.monthly_essential_debt_spending = 0.0;
        self.monthly_gov_spending = 0.0;
        self.monthly_budget_balance = 0.0;
        self.stock_market.reset_mtd_trackers();

        self.people
            .par_chunks_mut(PEOPLE_CHUNK_SIZE)
            .for_each(|chunk| {
                for person in chunk {
                    person.reset_monthly_trackers();
                }
            });
        for company in &mut self.companies {
            company.open_month();
        }
    }

    fn ensure_company_payroll_liquidity(&mut self) {
        let company_count = self.companies.len();
        let payroll_by_chunk_cents = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                let mut local = vec![0_i64; company_count];
                for person in chunk {
                    let Some(company_id) = person.employer_company_id() else {
                        continue;
                    };
                    let company_idx = company_id as usize;
                    if let Some(total) = local.get_mut(company_idx) {
                        *total = total.saturating_add(to_cents(person.monthly_salary()));
                    }
                }
                local
            })
            .collect::<Vec<_>>();

        let mut payroll_requirements_cents = vec![0_i64; company_count];
        for chunk_totals in payroll_by_chunk_cents {
            for (company_idx, cents) in chunk_totals.into_iter().enumerate() {
                if let Some(total) = payroll_requirements_cents.get_mut(company_idx) {
                    *total = total.saturating_add(cents);
                }
            }
        }

        for (company_idx, company) in self.companies.iter_mut().enumerate() {
            let required_payroll = payroll_requirements_cents
                .get(company_idx)
                .copied()
                .map(from_cents)
                .unwrap_or(0.0);
            company.monthly_payroll_budget = required_payroll;

            if company.cash_balance >= required_payroll {
                continue;
            }

            let shortfall = round_cents(required_payroll - company.cash_balance);
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

    fn process_payroll_and_taxes(&mut self) {
        for person in &mut self.people {
            let gross_pay = if let Some(company_id) = person.employer_company_id() {
                let company_idx = company_id as usize;
                let salary = person.monthly_salary();
                if let Some(company) = self.companies.get_mut(company_idx) {
                    let payout = round_cents(salary.min(company.cash_balance.max(0.0)));
                    company.cash_balance = round_cents(company.cash_balance - payout);
                    company.last_month_payroll_paid =
                        round_cents(company.last_month_payroll_paid + payout);
                    payout
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let tax = monthly_income_tax(gross_pay).min(gross_pay);
            let net_pay = round_cents(gross_pay - tax);

            person.cash_balance = round_cents(person.cash_balance + net_pay);
            person.last_month_gross_pay = gross_pay;
            person.last_month_tax_paid = tax;
            person.last_month_net_pay = net_pay;

            self.treasury_balance = round_cents(self.treasury_balance + tax);
            self.monthly_tax_collected = round_cents(self.monthly_tax_collected + tax);
        }
    }

    fn collect_monthly_loan_repayments(&mut self) {
        for person in &mut self.people {
            let payment = self
                .bank
                .collect_monthly_payment(&mut person.cash_balance, &mut person.loan);
            person.last_month_loan_payment = payment.amount_paid;
            if payment.unpaid_amount > 0.0 {
                self.pending_events.push(SimEvent::DebtDistress {
                    country_name: self.name.clone(),
                    entity_kind: EntityKind::Person,
                    entity_id: person.id,
                    unpaid_amount: payment.unpaid_amount,
                    day: self.day,
                });
            }
            if payment.loan_ended {
                self.pending_events.push(SimEvent::LoanEnded {
                    country_name: self.name.clone(),
                    entity_kind: EntityKind::Person,
                    entity_id: person.id,
                    day: self.day,
                });
            }
        }

        for company in &mut self.companies {
            let payment = self
                .bank
                .collect_monthly_payment(&mut company.cash_balance, &mut company.loan);
            company.last_month_loan_payment = payment.amount_paid;
            company.missed_loan_payment_this_month = payment.unpaid_amount > 0.0;
            if payment.unpaid_amount > 0.0 {
                self.pending_events.push(SimEvent::DebtDistress {
                    country_name: self.name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    unpaid_amount: payment.unpaid_amount,
                    day: self.day,
                });
            }
            if payment.loan_ended {
                self.pending_events.push(SimEvent::LoanEnded {
                    country_name: self.name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    day: self.day,
                });
            }
        }
    }

    fn recompute_company_profitability(&mut self) {
        for company in &mut self.companies {
            company.recompute_profitability();
        }
    }

    fn process_company_distress(&mut self) {
        let params = self.sim_params.company_distress;
        let mut bankrupt_indices = Vec::new();

        for company_idx in 0..self.companies.len() {
            let missed = self.companies[company_idx].missed_loan_payment_this_month;
            if !missed {
                self.companies[company_idx].consecutive_missed_loan_payments = 0;
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
                let laid_off = self.layoff_people_by_rate(company_id, params.layoff_rate);
                if laid_off > 0 {
                    self.pending_events.push(SimEvent::CompanyLayoffs {
                        country_name: self.name.clone(),
                        company_id,
                        laid_off_count: laid_off,
                        day: self.day,
                    });
                }
                continue;
            }

            if misses >= params.bankruptcy_missed_payment_threshold {
                let company = &self.companies[company_idx];
                let has_capacity = self.company_has_extra_loan_capacity(company);
                if company.cash_balance <= 0.0 && !has_capacity {
                    // Attempt IPO rescue before bankruptcy
                    if self.sim_params.stock_market.enabled
                        && !self.stock_market.is_listed(company_id)
                    {
                        let rescued = self.attempt_ipo_rescue(company_idx);
                        if rescued {
                            // IPO succeeded â€” clear distress streak
                            self.companies[company_idx].consecutive_missed_loan_payments = 0;
                            continue;
                        }
                    }
                    bankrupt_indices.push(company_idx);
                    continue;
                }

                let laid_off = self.layoff_people_by_rate(company_id, params.layoff_rate);
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

        if !bankrupt_indices.is_empty() {
            self.resolve_company_bankruptcies(bankrupt_indices);
        }
    }

    fn apply_company_wage_cut(&mut self, company_id: u16, cut_rate: f64) {
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
            person.set_employment_with_salary(company_id, job_id, next_salary);
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

    fn layoff_people_by_rate(&mut self, company_id: u16, layoff_rate: f64) -> u16 {
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

    fn layoff_all_people(&mut self, company_id: u16) -> u16 {
        let mut laid_off = 0_u16;
        for person in &mut self.people {
            if person.employer_company_id() == Some(company_id) {
                person.clear_employment();
                laid_off = laid_off.saturating_add(1);
            }
        }
        laid_off
    }

    fn company_has_extra_loan_capacity(&self, company: &Company) -> bool {
        let monthly_income = company.average_monthly_revenue();
        if monthly_income <= 0.0 {
            return false;
        }

        let max_payment = round_cents(monthly_income * DEFAULT_MAX_DEBT_TO_INCOME.max(0.0));
        if max_payment <= 0.0 {
            return false;
        }

        let max_total_debt =
            max_principal_for_payment(max_payment, DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS);
        let existing_debt = company.loan.as_ref().map_or(0.0, |loan| loan.debt_balance);
        max_total_debt > existing_debt + 0.01
    }

