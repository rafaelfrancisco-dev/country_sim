use super::*;
impl Country {
    pub fn process_month(&mut self) {
        self.monthly_tax_collected = 0.0;
        self.monthly_household_spending = 0.0;
        self.recompute_company_headcounts();

        for company in &mut self.companies {
            company.open_month();
        }

        self.ensure_company_payroll_liquidity();
        self.process_payroll_and_taxes();
        self.collect_monthly_loan_repayments();
        self.process_household_spending();
        self.process_end_of_month_loan_requests();
        self.update_company_growth_openings();
        self.close_company_months();

        self.month = self.month.saturating_add(1);
        self.recompute_aggregates();
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
            }
        }
    }

    fn process_payroll_and_taxes(&mut self) {
        for person in &mut self.people {
            person.reset_monthly_trackers();

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
        }

        for company in &mut self.companies {
            let payment = self
                .bank
                .collect_monthly_payment(&mut company.cash_balance, &mut company.loan);
            company.last_month_loan_payment = payment.amount_paid;
        }
    }

    fn process_household_spending(&mut self) {
        if self.companies.is_empty() {
            return;
        }

        let month_seed = self.month.saturating_add(1);
        let company_count = self.companies.len();

        let spending_intents_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let mut intents = Vec::new();
                let base_idx = chunk_idx.saturating_mul(PEOPLE_CHUNK_SIZE);
                for (offset, person) in chunk.iter().enumerate() {
                    if person.cash_balance <= 0.0 {
                        continue;
                    }

                    let seed = hash64(
                        (u64::from(self.id) << 48)
                            ^ (month_seed << 24)
                            ^ u64::from(person.id)
                            ^ u64::from(person.name_seed),
                    );
                    let spend_percent = 5.0 + (seed % 11) as f64;
                    let spend_cents = to_cents(person.cash_balance * spend_percent / 100.0);
                    if spend_cents <= 0 {
                        continue;
                    }

                    let seller_idx =
                        (hash64(seed ^ 0x9e37_79b9_7f4a_7c15) as usize) % company_count;
                    intents.push((base_idx + offset, seller_idx, spend_cents));
                }
                intents
            })
            .collect::<Vec<_>>();

        let mut total_spend_cents = 0_i64;
        for chunk_intents in spending_intents_by_chunk {
            for (person_idx, seller_idx, spend_cents) in chunk_intents {
                let spend = from_cents(spend_cents);
                let Some(person) = self.people.get_mut(person_idx) else {
                    continue;
                };
                person.cash_balance = round_cents(person.cash_balance - spend);
                person.last_month_spending = spend;

                if let Some(company) = self.companies.get_mut(seller_idx) {
                    company.cash_balance = round_cents(company.cash_balance + spend);
                    company.last_month_sales = round_cents(company.last_month_sales + spend);
                }

                total_spend_cents = total_spend_cents.saturating_add(spend_cents);
            }
        }

        self.monthly_household_spending = from_cents(total_spend_cents);
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
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.close_month();
                }
            });
    }
}
