use super::*;
use crate::sim::{
    bank::{DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS, max_principal_for_payment},
    good::{GOODS_COUNT, good_by_id},
    job::{common_job_types, salary_for_job},
    money::round_cents,
    params::{GovernmentBackfillParams, SwitchCadence},
    stock_market::{ListedEquity, StockMarket},
};
use std::cmp::Ordering;
#[cfg(feature = "bench-trace")]
use crate::time_op;

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
                            // IPO succeeded — clear distress streak
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

    fn close_company_months(&mut self) {
        let market = self.sim_params.goods_market;
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.close_month(&market);
                }
            });
    }

    fn process_government_spending(&mut self) {
        let fiscal = self.sim_params.fiscal;
        let benefit_amount = round_cents(fiscal.monthly_unemployment_benefit());
        let basic_threshold = fiscal.basic_needs_cash_threshold;

        // Essential needs goods are IDs 0–4 (water, food, housing, electricity, medicine).
        // Mask covering the first 5 goods.
        const ESSENTIAL_NEEDS_MASK: u16 = 0b0001_1111;

        let mut total_spent: f64 = 0.0;
        let mut total_recipients: u32 = 0;

        for person in &mut self.people {
            let age = person.age_years();
            let is_working_age = age >= 18 && age <= 65;

            if !person.is_employed() && is_working_age && benefit_amount > 0.0 {
                // Pay unemployment benefit from treasury
                let payout = round_cents(benefit_amount.min(self.treasury_balance.max(0.0)));
                if payout > 0.0 {
                    person.cash_balance = round_cents(person.cash_balance + payout);
                    self.treasury_balance = round_cents(self.treasury_balance - payout);
                    total_spent = round_cents(total_spent + payout);
                    total_recipients = total_recipients.saturating_add(1);
                }
            } else if person.cash_balance < basic_threshold
                && (person.last_month_unmet_needs_mask & ESSENTIAL_NEEDS_MASK) != 0
            {
                // Top up to basic_threshold for people with unmet essential needs
                let top_up = round_cents(basic_threshold - person.cash_balance);
                let payout = round_cents(top_up.min(self.treasury_balance.max(0.0)));
                if payout > 0.0 {
                    person.cash_balance = round_cents(person.cash_balance + payout);
                    self.treasury_balance = round_cents(self.treasury_balance - payout);
                    total_spent = round_cents(total_spent + payout);
                    total_recipients = total_recipients.saturating_add(1);
                }
            }
        }

        self.monthly_gov_spending = round_cents(self.monthly_gov_spending + total_spent);
        self.monthly_budget_balance =
            round_cents(self.monthly_tax_collected - self.monthly_gov_spending);

        // If treasury went negative (shouldn't normally, but guard anyway), accumulate debt
        if self.treasury_balance < 0.0 {
            self.gov_debt = round_cents(self.gov_debt - self.treasury_balance);
            self.treasury_balance = 0.0;
        }

        if total_recipients > 0 && total_spent > 0.0 {
            self.pending_events.push(SimEvent::GovTransfer {
                country_name: self.name.clone(),
                total_recipients,
                total_amount: total_spent,
                day: self.day,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Stock market — IPO rescue
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Stock market — seed listings at simulation start
    // -----------------------------------------------------------------------

    /// Called once after world generation. Lists the top `initial_listing_count`
    /// companies (by cash balance) so there is immediate market activity.
    pub(crate) fn seed_initial_ipos(&mut self) {
        let params = self.sim_params.stock_market;
        if !params.enabled || params.initial_listing_count == 0 || params.ipo_initial_shares == 0 {
            return;
        }

        // Collect candidate indices sorted by cash balance descending.
        let mut candidates: Vec<usize> = (0..self.companies.len())
            .filter(|&i| !self.stock_market.is_listed(self.companies[i].id))
            .collect();
        candidates.sort_by(|&a, &b| {
            self.companies[b]
                .cash_balance
                .partial_cmp(&self.companies[a].cash_balance)
                .unwrap_or(Ordering::Equal)
        });
        candidates.truncate(params.initial_listing_count);

        for company_idx in candidates {
            self.do_initial_listing(company_idx);
        }
    }

    /// List a healthy company at book-value price, selling a small free float
    /// to investors. Cash raised goes into the company treasury.
    fn do_initial_listing(&mut self, company_idx: usize) {
        let params = self.sim_params.stock_market;
        let Some(company) = self.companies.get(company_idx) else {
            return;
        };

        let company_id = company.id;
        let company_name = company.name.clone();
        let cash_balance = company.cash_balance;
        let debt_balance = company.loan.as_ref().map_or(0.0, |l| l.debt_balance);
        let initial_shares = params.ipo_initial_shares;

        // Price purely on book value (no revenue history at day 0).
        let price_per_share =
            StockMarket::ipo_valuation_anchor(0.0, 0.0, cash_balance, debt_balance, initial_shares);
        if price_per_share < 0.01 {
            return;
        }

        // Float = fraction of initial_shares offered to investors.
        let float_shares = ((initial_shares as f64 * params.initial_float_fraction) as u64).max(1);

        // Collect person subscriptions.
        // Cap per-person allocation so ~20% of the population becomes shareholders:
        // target ~people/5 subscribers → per_participant_max = float_shares / target.
        // This gives the daily market a sufficient holder base to generate sell orders.
        let reserve_ratio = params.investor_cash_reserve_ratio;
        let target_subscribers = (self.people.len() / 5).max(1) as u64;
        let per_participant_max = (float_shares / target_subscribers).max(1);
        let mut shares_allocated = 0_u64;
        let mut raised = 0.0_f64;
        let mut subscriptions: Vec<(u32, u64, f64)> = Vec::new();

        for person in &self.people {
            if shares_allocated >= float_shares {
                break;
            }
            let investable = round_cents((person.cash_balance * (1.0 - reserve_ratio)).max(0.0));
            if investable < price_per_share {
                continue;
            }
            let affordable_shares = (investable / price_per_share) as u64;
            let want = affordable_shares.min(per_participant_max);
            let available = float_shares - shares_allocated;
            let alloc = want.min(available);
            if alloc == 0 {
                continue;
            }
            let cost = round_cents(alloc as f64 * price_per_share);
            subscriptions.push((person.id, alloc, cost));
            shares_allocated += alloc;
            raised = round_cents(raised + cost);
        }

        // Bank fills residual up to its equity headroom.
        let bank_reserve = self.bank.reserve_balance;
        let bank_equity_exposure = self.stock_market.bank_equity_exposure();
        let bank_headroom = round_cents(
            (bank_reserve * params.bank_max_equity_reserve_fraction - bank_equity_exposure)
                .max(0.0),
        );
        let mut bank_alloc = 0_u64;
        let mut bank_cost = 0.0_f64;
        if shares_allocated < float_shares && bank_headroom >= price_per_share {
            let remaining = float_shares - shares_allocated;
            let bank_affordable = (bank_headroom / price_per_share) as u64;
            bank_alloc = bank_affordable.min(remaining);
            if bank_alloc > 0 {
                bank_cost = round_cents(bank_alloc as f64 * price_per_share);
                shares_allocated += bank_alloc;
                raised = round_cents(raised + bank_cost);
            }
        }

        // Apply subscriptions regardless of uptake — healthy company always lists.
        for (person_id, alloc, cost) in &subscriptions {
            if let Some(person) = self.people.iter_mut().find(|p| p.id == *person_id) {
                person.cash_balance = round_cents(person.cash_balance - cost);
                self.stock_market
                    .person_portfolio_mut(*person_id)
                    .add_shares(company_id, *alloc, price_per_share);
            }
        }
        if bank_alloc > 0 {
            self.bank.reserve_balance = round_cents(self.bank.reserve_balance - bank_cost);
            self.stock_market
                .bank_portfolio
                .add_shares(company_id, bank_alloc, price_per_share);
        }

        // Register listing: founder shares + float sold.
        let total_shares = initial_shares + shares_allocated;
        let ticker = StockMarket::generate_ticker(company_id, &company_name);
        let equity = ListedEquity::new(
            company_id,
            ticker.clone(),
            price_per_share,
            total_shares,
            shares_allocated,
        );
        self.stock_market
            .insert_listing_preserve_index(company_id, equity);

        // Proceeds flow into company treasury (primary offering).
        if let Some(company) = self.companies.get_mut(company_idx) {
            company.cash_balance = round_cents(company.cash_balance + raised);
        }
        self.stock_market.ipo_raised_mtd = round_cents(self.stock_market.ipo_raised_mtd + raised);

        self.pending_events.push(SimEvent::StockIpo {
            country_name: self.name.clone(),
            company_id,
            company_name,
            ticker,
            shares_issued: shares_allocated,
            price_per_share,
            total_raised: raised,
            day: self.day,
        });
    }

    /// Attempt an IPO for a distressed company that cannot cover funding via
    /// debt. Returns true if the IPO succeeded and saved the company.
    fn attempt_ipo_rescue(&mut self, company_idx: usize) -> bool {
        let params = self.sim_params.stock_market;
        let Some(company) = self.companies.get(company_idx) else {
            return false;
        };

        let company_id = company.id;
        let company_name = company.name.clone();
        let monthly_revenue = company.average_monthly_revenue();
        let last_month_profit = company.last_month_profit;
        let cash_balance = company.cash_balance;
        let debt_balance = company.loan.as_ref().map_or(0.0, |l| l.debt_balance);
        let monthly_payroll = company.monthly_payroll_budget;

        // How much fresh cash does the company need?
        let funding_need = round_cents((-cash_balance).max(0.0).max(monthly_payroll * 2.0));
        if funding_need <= 0.0 {
            return false;
        }

        // Shares to issue (capped by dilution param)
        let initial_shares = params.ipo_initial_shares;
        if initial_shares == 0 {
            return false;
        }

        let price_per_share = StockMarket::ipo_valuation_anchor(
            monthly_revenue,
            last_month_profit,
            cash_balance,
            debt_balance,
            initial_shares,
        );

        // New shares = how many needed to raise `funding_need`
        let max_new_shares = ((initial_shares as f64 * params.ipo_max_dilution) as u64).max(1);
        let needed_shares = ((funding_need / price_per_share).ceil() as u64).max(1);
        let shares_to_issue = needed_shares.min(max_new_shares);

        let total_raise_target = round_cents(shares_to_issue as f64 * price_per_share);
        if total_raise_target <= 0.0 {
            return false;
        }

        // Collect subscription from people (those with enough surplus cash)
        let reserve_ratio = params.investor_cash_reserve_ratio;
        let mut raised = 0.0_f64;
        let mut shares_allocated = 0_u64;
        let mut subscriptions: Vec<(u32, u64, f64)> = Vec::new(); // (person_id, shares, cost)

        let remaining_shares = shares_to_issue;
        if remaining_shares > 0 {
            let per_participant_max = (remaining_shares / 10).max(1);
            for person in &self.people {
                if shares_allocated >= remaining_shares {
                    break;
                }
                let reserve = person.cash_balance * reserve_ratio;
                let investable = round_cents((person.cash_balance - reserve).max(0.0));
                if investable < price_per_share {
                    continue;
                }
                let affordable_shares = (investable / price_per_share) as u64;
                let want = affordable_shares.min(per_participant_max);
                let available = remaining_shares - shares_allocated;
                let alloc = want.min(available);
                if alloc == 0 {
                    continue;
                }
                let cost = round_cents(alloc as f64 * price_per_share);
                subscriptions.push((person.id, alloc, cost));
                shares_allocated += alloc;
                raised = round_cents(raised + cost);
            }
        }

        // Bank may fill residual up to reserve headroom
        let bank_reserve = self.bank.reserve_balance;
        let bank_equity_exposure = self.stock_market.bank_equity_exposure();
        let bank_headroom = round_cents(
            (bank_reserve * params.bank_max_equity_reserve_fraction - bank_equity_exposure)
                .max(0.0),
        );
        if shares_allocated < shares_to_issue && bank_headroom >= price_per_share {
            let remaining = shares_to_issue - shares_allocated;
            let bank_affordable = (bank_headroom / price_per_share) as u64;
            let bank_alloc = bank_affordable.min(remaining);
            if bank_alloc > 0 {
                let bank_cost = round_cents(bank_alloc as f64 * price_per_share);
                shares_allocated += bank_alloc;
                raised = round_cents(raised + bank_cost);
                // bank_portfolio add happens below after verifying success
                let _ = bank_alloc; // used below in success path
                let _ = bank_cost;
                // Store separately so we can apply below
                let bank_alloc_final = bank_alloc;
                let bank_cost_final = bank_cost;

                if raised >= total_raise_target * 0.8 {
                    // IPO success — apply bank subscription
                    self.bank.reserve_balance =
                        round_cents(self.bank.reserve_balance - bank_cost_final);
                    self.stock_market.bank_portfolio.add_shares(
                        company_id,
                        bank_alloc_final,
                        price_per_share,
                    );
                }
            }
        }

        // Require at least 80% subscribed to succeed
        if raised < total_raise_target * 0.8 {
            return false;
        }

        // Apply person subscriptions
        for (person_id, alloc, cost) in &subscriptions {
            if let Some(person) = self.people.iter_mut().find(|p| p.id == *person_id) {
                person.cash_balance = round_cents(person.cash_balance - cost);
                self.stock_market
                    .person_portfolio_mut(*person_id)
                    .add_shares(company_id, *alloc, price_per_share);
            }
        }

        // Register listing
        let total_shares = initial_shares + shares_allocated;
        let ticker = StockMarket::generate_ticker(company_id, &company_name);
        let equity = ListedEquity::new(
            company_id,
            ticker.clone(),
            price_per_share,
            total_shares,
            shares_allocated, // free float = newly issued shares
        );
        self.stock_market
            .insert_listing_preserve_index(company_id, equity);

        // Inject raised cash into company
        if let Some(company) = self.companies.get_mut(company_idx) {
            company.cash_balance = round_cents(company.cash_balance + raised);
        }

        self.stock_market.ipo_raised_mtd = round_cents(self.stock_market.ipo_raised_mtd + raised);

        self.pending_events.push(SimEvent::StockIpo {
            country_name: self.name.clone(),
            company_id,
            company_name,
            ticker,
            shares_issued: shares_allocated,
            price_per_share,
            total_raised: raised,
            day: self.day,
        });

        true
    }

    // -----------------------------------------------------------------------
    // Stock market — collateral seizure
    // -----------------------------------------------------------------------

    fn process_stock_collateral_seizure(&mut self) {
        if !self.sim_params.stock_market.enabled {
            return;
        }

        let discount = self.sim_params.stock_market.margin_liquidation_discount;

        let people_with_loans: Vec<(u32, f64)> = self
            .people
            .iter()
            .filter_map(|p| {
                let loan = p.loan.as_ref()?;
                Some((p.id, loan.monthly_payment))
            })
            .collect();

        for (person_id, monthly_payment) in people_with_loans {
            let Some(person) = self.people.iter_mut().find(|p| p.id == person_id) else {
                continue;
            };

            // Pledge all shares for borrowers
            if let Some(portfolio) = self.stock_market.person_portfolios.get_mut(&person_id) {
                portfolio.pledge_all();
            }

            // Estimate shortfall
            let cash = person.cash_balance.max(0.0);
            if cash >= monthly_payment {
                continue; // can cover — no forced liquidation
            }
            let shortfall = round_cents(monthly_payment - cash);

            // Sell pledged shares to cover shortfall
            let portfolio = match self.stock_market.person_portfolios.get_mut(&person_id) {
                Some(p) => p,
                None => continue,
            };

            let mut proceeds = 0.0_f64;
            let mut total_liquidated_shares = 0_u64;

            let company_ids: Vec<u16> = portfolio
                .positions
                .iter()
                .filter(|pos| pos.pledged_shares > 0)
                .map(|pos| pos.company_id)
                .collect();

            for cid in company_ids {
                if proceeds >= shortfall {
                    break;
                }
                let last_price = self
                    .stock_market
                    .listed
                    .get(&cid)
                    .map_or(0.0, |e| e.last_price);
                if last_price <= 0.0 {
                    continue;
                }
                let sale_price = round_cents(last_price * (1.0 - discount));
                let portfolio = match self.stock_market.person_portfolios.get_mut(&person_id) {
                    Some(p) => p,
                    None => break,
                };
                let pos = match portfolio.positions.iter().find(|p| p.company_id == cid) {
                    Some(p) => p,
                    None => continue,
                };
                let pledged = pos.pledged_shares;
                let remaining_need = round_cents(shortfall - proceeds);
                let shares_needed = ((remaining_need / sale_price).ceil() as u64).max(1);
                let shares_to_sell = shares_needed.min(pledged);
                if shares_to_sell == 0 {
                    continue;
                }
                let sale_proceeds = round_cents(shares_to_sell as f64 * sale_price);
                portfolio.remove_shares(cid, shares_to_sell, sale_price);
                proceeds = round_cents(proceeds + sale_proceeds);
                total_liquidated_shares = total_liquidated_shares.saturating_add(shares_to_sell);

                // Credit proceeds to person
                if let Some(p) = self.people.iter_mut().find(|p| p.id == person_id) {
                    p.cash_balance = round_cents(p.cash_balance + sale_proceeds);
                }
                // Bank receives shares — bank pays for them (reserve decreases)
                self.stock_market
                    .bank_portfolio
                    .add_shares(cid, shares_to_sell, sale_price);
                self.bank.reserve_balance =
                    round_cents(self.bank.reserve_balance - sale_proceeds).max(0.0);
            }

            if total_liquidated_shares > 0 {
                self.stock_market.forced_liquidations_mtd =
                    round_cents(self.stock_market.forced_liquidations_mtd + proceeds);
                self.pending_events.push(SimEvent::StockMarginCall {
                    country_name: self.name.clone(),
                    person_id,
                    shares_liquidated: total_liquidated_shares,
                    proceeds,
                    day: self.day,
                });
            }
        }

        self.stock_market.gc_portfolios();
    }

    // -----------------------------------------------------------------------
    // Stock market — dividends
    // -----------------------------------------------------------------------

    fn process_stock_dividends(&mut self) {
        if !self.sim_params.stock_market.enabled {
            return;
        }

        let payout_ratio = self.sim_params.stock_market.dividend_payout_ratio;
        let runway_months = self.sim_params.stock_market.dividend_min_runway_months;

        let company_ids: Vec<u16> = self.stock_market.listed.keys().copied().collect();

        for company_id in company_ids {
            let Some(company) = self.companies.get(company_id as usize) else {
                continue;
            };
            if company.last_month_profit <= 0.0 {
                continue;
            }

            let monthly_payroll = company.monthly_payroll_budget.max(0.0);
            let min_cash = round_cents(monthly_payroll * runway_months);
            let cash_excess = round_cents((company.cash_balance - min_cash).max(0.0));
            if cash_excess <= 0.0 {
                continue;
            }

            let dividend_pool =
                round_cents(company.last_month_profit * payout_ratio).min(cash_excess);
            if dividend_pool <= 0.0 {
                continue;
            }

            let equity = match self.stock_market.listed.get(&company_id) {
                Some(e) => e,
                None => continue,
            };
            let total_shares = equity.shares_outstanding;
            if total_shares == 0 {
                continue;
            }
            let per_share = round_cents(dividend_pool / total_shares as f64);
            if per_share <= 0.0 {
                continue;
            }

            // Collect all holders and their share counts
            let mut payments: Vec<(Option<u32>, u64)> = Vec::new(); // (Some(person_id) or None=bank, shares)
            for (&pid, portfolio) in &self.stock_market.person_portfolios {
                if let Some(pos) = portfolio.position(company_id) {
                    if pos.shares > 0 {
                        payments.push((Some(pid), pos.shares));
                    }
                }
            }
            if let Some(pos) = self.stock_market.bank_portfolio.position(company_id) {
                if pos.shares > 0 {
                    payments.push((None, pos.shares));
                }
            }

            let mut total_paid = 0.0_f64;
            for (holder, shares) in &payments {
                let gross = round_cents(*shares as f64 * per_share);
                if gross <= 0.0 {
                    continue;
                }
                match holder {
                    Some(pid) => {
                        // Household dividend — apply withholding tax (same bracket logic)
                        let tax = round_cents(gross * 0.15); // flat 15% withholding
                        let net = round_cents(gross - tax);
                        if let Some(person) = self.people.iter_mut().find(|p| p.id == *pid) {
                            person.cash_balance = round_cents(person.cash_balance + net);
                        }
                        self.treasury_balance = round_cents(self.treasury_balance + tax);
                        self.monthly_tax_collected = round_cents(self.monthly_tax_collected + tax);
                        if let Some(portfolio) = self.stock_market.person_portfolios.get_mut(pid) {
                            portfolio.dividends_mtd = round_cents(portfolio.dividends_mtd + net);
                        }
                        total_paid = round_cents(total_paid + gross);
                    }
                    None => {
                        // Bank dividend goes to reserves
                        self.bank.reserve_balance = round_cents(self.bank.reserve_balance + gross);
                        self.stock_market.bank_portfolio.dividends_mtd =
                            round_cents(self.stock_market.bank_portfolio.dividends_mtd + gross);
                        total_paid = round_cents(total_paid + gross);
                    }
                }
            }

            if total_paid <= 0.0 {
                continue;
            }

            // Deduct from company cash
            if let Some(company) = self.companies.get_mut(company_id as usize) {
                company.cash_balance = round_cents(company.cash_balance - total_paid);
            }

            self.stock_market.dividends_paid_mtd =
                round_cents(self.stock_market.dividends_paid_mtd + total_paid);

            let ticker = self
                .stock_market
                .listed
                .get(&company_id)
                .map(|e| e.ticker.clone())
                .unwrap_or_default();

            self.pending_events.push(SimEvent::StockDividend {
                country_name: self.name.clone(),
                company_id,
                ticker,
                total_amount: total_paid,
                per_share,
                day: self.day,
            });
        }
    }
}

fn pick_weighted_job(seed: u64, weighted_jobs: &[(u8, u16)]) -> Option<u8> {
    let total_weight: u64 = weighted_jobs
        .iter()
        .map(|(_, weight)| u64::from(*weight))
        .sum();
    if total_weight == 0 {
        return weighted_jobs.first().map(|(job_id, _)| *job_id);
    }

    let mut roll = hash64(seed) % total_weight;
    for (job_id, weight) in weighted_jobs {
        let weight = u64::from(*weight);
        if roll < weight {
            return Some(*job_id);
        }
        roll -= weight;
    }

    weighted_jobs.last().map(|(job_id, _)| *job_id)
}
