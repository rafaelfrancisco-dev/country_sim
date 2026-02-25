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
        self.collect_monthly_loan_repayments();
        self.process_end_of_month_loan_requests();
        self.update_company_growth_openings();
        self.close_company_months();

        self.month = self.month.saturating_add(1);
        self.recompute_aggregates();
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

    fn update_company_growth_openings(&mut self) {
        let month_seed = self.month.saturating_add(1);
        let sim_params = self.sim_params.clone();
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.recompute_profitability();
                    company.open_positions_from_growth(&sim_params, month_seed);
                }
            });
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

        if total_recipients > 0 {
            self.pending_events.push(SimEvent::GovTransfer {
                country_name: self.name.clone(),
                total_recipients,
                total_amount: total_spent,
                day: self.day,
            });
        }
    }
}
