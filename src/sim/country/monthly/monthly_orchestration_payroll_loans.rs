use super::*;

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
        self.process_bond_coupon_and_maturity(); // [D]
        self.process_stock_collateral_seizure();
        self.collect_monthly_loan_repayments();
        self.recompute_company_profitability();
        self.process_growth_ipos();
        self.process_company_distress();
        self.process_end_of_month_loan_requests();
        self.update_company_growth_openings();
        self.process_startup_formation();
        self.process_essential_company_backfill();
        self.maybe_issue_bonds(); // [E]
        self.adjust_public_utility_opening_wages();
        self.process_monthly_job_switches();
        self.recompute_company_headcounts();
        self.close_company_months();
        self.process_stock_dividends();
        self.recompute_cpi(); // [F]

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
        let bond_coupon_ms = time_op!(self.process_bond_coupon_and_maturity());
        let stock_collateral_ms = time_op!(self.process_stock_collateral_seizure());
        let loan_repayments_ms = time_op!(self.collect_monthly_loan_repayments());
        let profitability_ms = time_op!(self.recompute_company_profitability());
        let _growth_ipos_ms = time_op!(self.process_growth_ipos());
        let company_distress_ms = time_op!(self.process_company_distress());
        let loan_requests_ms = time_op!(self.process_end_of_month_loan_requests());
        let growth_openings_ms = time_op!(self.update_company_growth_openings());
        let startup_formation_ms = time_op!(self.process_startup_formation());
        let essential_backfill_ms = time_op!(self.process_essential_company_backfill());
        let bond_issuance_ms = time_op!(self.maybe_issue_bonds());
        let public_wage_adjust_ms = time_op!(self.adjust_public_utility_opening_wages());
        let job_switches_ms = time_op!(self.process_monthly_job_switches());
        let recompute_headcounts_after_ms = time_op!(self.recompute_company_headcounts());
        let close_company_months_ms = time_op!(self.close_company_months());
        let dividends_ms = time_op!(self.process_stock_dividends());
        let cpi_ms = time_op!(self.recompute_cpi());

        self.month = self.month.saturating_add(1);

        let recompute_aggregates_ms = time_op!(self.recompute_aggregates());

        crate::sim::trace::CountryMonthlyTrace {
            total_ms: crate::sim::trace::duration_ms(total_started.elapsed()),
            recompute_headcounts_before_ms,
            payroll_liquidity_ms,
            payroll_taxes_ms,
            gov_spending_ms,
            bond_coupon_ms,
            stock_collateral_ms,
            loan_repayments_ms,
            profitability_ms,
            company_distress_ms,
            loan_requests_ms,
            growth_openings_ms,
            startup_formation_ms,
            essential_backfill_ms,
            bond_issuance_ms,
            public_wage_adjust_ms,
            job_switches_ms,
            recompute_headcounts_after_ms,
            close_company_months_ms,
            dividends_ms,
            cpi_ms,
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
        self.monthly_hires_total = 0;
        self.monthly_gov_spending = 0.0;
        self.monthly_budget_balance = 0.0;
        self.monthly_coupon_paid = 0.0;
        self.monthly_bonds_issued = 0.0;
        self.monthly_bonds_matured = 0.0;
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

    pub(super) fn ensure_company_payroll_liquidity(&mut self) {
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

        let loan_rate = self.bank_loan_rate();

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
                loan_rate,
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

    pub(super) fn process_payroll_and_taxes(&mut self) {
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

    pub(super) fn collect_monthly_loan_repayments(&mut self) {
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
}
