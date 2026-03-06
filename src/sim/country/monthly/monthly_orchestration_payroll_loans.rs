use super::*;

impl Country {
    #[cfg(test)]
    pub fn process_month(&mut self) {
        self.process_month_end();
        self.reset_month_trackers();
    }

    pub(crate) fn process_month_end(&mut self) {
        self.compute_monthly_gdp_and_austerity();
        self.ensure_entity_deposit_assignments();
        self.recompute_company_headcounts();
        self.recompute_bank_headcounts();
        self.update_bank_openings();

        self.ensure_company_payroll_liquidity();
        self.process_payroll_and_taxes();
        self.process_corporate_taxes();
        self.process_government_spending();
        // Housing market (runs after payroll so people have income to pay rent/mortgages)
        self.process_housing_construction();
        self.update_housing_price_index();
        self.collect_rent();
        self.collect_mortgage_payments();
        self.process_renter_to_owner_transitions();
        self.process_bond_coupon_and_maturity(); // [D]
        self.process_stock_collateral_seizure();
        self.collect_monthly_loan_repayments();
        self.process_bank_runs();
        self.process_savings_interest();
        self.process_competitive_deposit_switching();
        self.recompute_company_profitability();
        self.process_annual_wage_raises();
        self.process_growth_ipos();
        self.process_listed_company_secondary_offerings();
        self.process_company_distress();
        self.process_end_of_month_loan_requests();
        self.update_company_growth_openings();
        self.process_startup_formation();
        self.process_essential_company_backfill();
        self.process_bank_formation();
        self.maybe_issue_bonds(); // [E]
        self.adjust_public_utility_opening_wages();
        self.process_monthly_job_switches();
        self.process_monthly_labor_churn();
        self.recompute_company_headcounts();
        self.recompute_bank_headcounts();
        self.close_company_months();
        for bank in self.all_banks_mut() {
            bank.close_month();
        }
        self.recompute_cpi(); // [F]
        self.recompute_bank_deposit_liabilities();
        self.process_auto_money_minting();

        self.month = self.month.saturating_add(1);
        self.recompute_aggregates();
    }

    #[cfg(feature = "bench-trace")]
    pub(crate) fn process_month_end_traced(&mut self) -> crate::sim::trace::CountryMonthlyTrace {
        let total_started = ::std::time::Instant::now();

        self.compute_monthly_gdp_and_austerity();
        let recompute_headcounts_before_ms = time_op!(self.recompute_company_headcounts());
        let payroll_liquidity_ms = time_op!(self.ensure_company_payroll_liquidity());
        let payroll_taxes_ms = time_op!(self.process_payroll_and_taxes());
        self.process_corporate_taxes();
        let gov_spending_ms = time_op!(self.process_government_spending());
        let bond_coupon_ms = time_op!(self.process_bond_coupon_and_maturity());
        let stock_collateral_ms = time_op!(self.process_stock_collateral_seizure());
        let loan_repayments_ms = time_op!(self.collect_monthly_loan_repayments());
        time_op!(self.process_bank_runs());
        let savings_interest_ms = time_op!(self.process_savings_interest());
        let _deposit_switching_ms = time_op!(self.process_competitive_deposit_switching());
        let profitability_ms = time_op!(self.recompute_company_profitability());
        let _annual_wage_raise_ms = time_op!(self.process_annual_wage_raises());
        let _growth_ipos_ms = time_op!(self.process_growth_ipos());
        let secondary_offerings_ms = time_op!(self.process_listed_company_secondary_offerings());
        let company_distress_ms = time_op!(self.process_company_distress());
        let loan_requests_ms = time_op!(self.process_end_of_month_loan_requests());
        let growth_openings_ms = time_op!(self.update_company_growth_openings());
        let startup_formation_ms = time_op!(self.process_startup_formation());
        let essential_backfill_ms = time_op!(self.process_essential_company_backfill());
        time_op!(self.process_bank_formation());
        let bond_issuance_ms = time_op!(self.maybe_issue_bonds());
        let public_wage_adjust_ms = time_op!(self.adjust_public_utility_opening_wages());
        let job_switches_ms = time_op!(self.process_monthly_job_switches());
        time_op!(self.process_monthly_labor_churn());
        let recompute_headcounts_after_ms = time_op!(self.recompute_company_headcounts());
        let close_company_months_ms = time_op!(self.close_company_months());
        for bank in self.all_banks_mut() {
            bank.close_month();
        }
        let dividends_ms = 0.0;
        let cpi_ms = time_op!(self.recompute_cpi());
        time_op!(self.recompute_bank_deposit_liabilities());
        self.process_auto_money_minting();

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
            savings_interest_ms,
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
            secondary_offerings_ms,
            recompute_aggregates_ms,
        }
    }

    pub(crate) fn reset_month_trackers(&mut self) {
        self.monthly_tax_collected = 0.0;
        self.monthly_corporate_tax_collected = 0.0;
        self.monthly_gdp = 0.0;
        // austerity_active is NOT reset here — persists until recomputed at start of next month
        self.monthly_household_spending = 0.0;
        self.monthly_tariff_collected = 0.0;
        self.monthly_goods_import_spending = 0.0;
        self.monthly_goods_export_revenue = 0.0;
        self.monthly_unmet_need_events = 0;
        self.monthly_essential_debt_spending = 0.0;
        self.monthly_hires_total = 0;
        self.monthly_private_banks_founded = 0;
        self.monthly_private_banks_closed = 0;
        self.monthly_gov_spending = 0.0;
        self.monthly_budget_balance = 0.0;
        self.monthly_coupon_paid = 0.0;
        self.monthly_bonds_issued = 0.0;
        self.monthly_bonds_matured = 0.0;
        self.monthly_savings_interest_paid = 0.0;
        self.monthly_money_minted = 0.0;
        self.monthly_rent_collected = 0.0;
        self.monthly_mortgage_payments = 0.0;
        self.monthly_home_purchases = 0;
        self.monthly_foreclosures = 0;
        self.monthly_housing_units_built = 0;
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
        for bank in self.all_banks_mut() {
            bank.open_month();
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

        let lender_id = self
            .preferred_lender_bank_id()
            .unwrap_or(self.primary_bank().id);
        let loan_rate = self.bank_loan_rate_for(lender_id);
        let debt_to_income_cap = self.effective_debt_to_income_cap(DEFAULT_MAX_DEBT_TO_INCOME);
        let lender_bank_idx = self.banks.iter().position(|bank| bank.id == lender_id);
        let country_name = self.name.clone();
        let day = self.day;
        let mut pending_events = Vec::new();

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
            let Some(bank_idx) = lender_bank_idx else {
                continue;
            };
            let bank = &mut self.banks[bank_idx];
            let issued = bank.originate_loan(
                shortfall,
                monthly_income,
                debt_to_income_cap,
                loan_rate,
                &mut company.loan,
            );
            if issued > 0.0 {
                company.cash_balance = round_cents(company.cash_balance + issued);
                company.last_month_new_loan = round_cents(company.last_month_new_loan + issued);
                pending_events.push(SimEvent::LoanEmitted {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    principal: issued,
                    day,
                });
            }
        }
        self.pending_events.extend(pending_events);
    }

    pub(super) fn compute_monthly_gdp_and_austerity(&mut self) {
        let gdp: f64 = self.companies.iter().map(|c| c.last_month_sales).sum();
        self.monthly_gdp = round_cents(gdp);
        let fiscal = self.sim_params.fiscal;
        let annual_gdp = self.monthly_gdp * 12.0;
        self.austerity_active = if annual_gdp > 0.0 {
            self.gov_debt / annual_gdp > fiscal.austerity_debt_gdp_threshold
        } else {
            false
        };
    }

    pub(super) fn process_corporate_taxes(&mut self) {
        let tax = self.sim_params.tax;
        let fiscal = self.sim_params.fiscal;
        let surcharge = if self.austerity_active {
            fiscal.austerity_corporate_tax_surcharge
        } else {
            0.0
        };

        let rate_1 = (tax.corp_rate_1 + surcharge).clamp(0.0, 1.0);
        let rate_2 = (tax.corp_rate_2 + surcharge).clamp(0.0, 1.0);
        let rate_3 = (tax.corp_rate_3 + surcharge).clamp(0.0, 1.0);

        let mut total_collected: f64 = 0.0;
        for company in &mut self.companies {
            let profit = company.last_month_profit;
            if profit <= 0.0 {
                continue;
            }
            // Tiered bracket logic on prior-month profit
            let tax_owed = if profit <= tax.corp_bracket_1_max {
                profit * rate_1
            } else if profit <= tax.corp_bracket_2_max {
                tax.corp_bracket_1_max * rate_1 + (profit - tax.corp_bracket_1_max) * rate_2
            } else {
                tax.corp_bracket_1_max * rate_1
                    + (tax.corp_bracket_2_max - tax.corp_bracket_1_max) * rate_2
                    + (profit - tax.corp_bracket_2_max) * rate_3
            };
            let tax_owed = round_cents(tax_owed);
            let actual = round_cents(tax_owed.min(company.cash_balance.max(0.0)));
            if actual <= 0.0 {
                continue;
            }
            company.cash_balance = round_cents(company.cash_balance - actual);
            total_collected = round_cents(total_collected + actual);
        }

        self.treasury_balance = round_cents(self.treasury_balance + total_collected);
        self.monthly_corporate_tax_collected =
            round_cents(self.monthly_corporate_tax_collected + total_collected);
        self.monthly_tax_collected = round_cents(self.monthly_tax_collected + total_collected);
    }

    pub(super) fn process_payroll_and_taxes(&mut self) {
        let tax_params = self.sim_params.tax;
        let companies = &mut self.companies;
        let banks = &mut self.banks;
        let mut total_tax = 0.0;
        for person in &mut self.people {
            let gross_pay = if let Some(company_id) = person.employer_company_id() {
                let company_idx = company_id as usize;
                let salary = person.monthly_salary();
                if let Some(company) = companies.get_mut(company_idx) {
                    let payout = round_cents(salary.min(company.cash_balance.max(0.0)));
                    company.cash_balance = round_cents(company.cash_balance - payout);
                    company.last_month_payroll_paid =
                        round_cents(company.last_month_payroll_paid + payout);
                    payout
                } else {
                    0.0
                }
            } else if let Some(bank_id) = person.employer_bank_id() {
                let salary = person.monthly_salary();
                let bank_opt = banks.iter_mut().find(|bank| bank.id == bank_id);
                if let Some(bank) = bank_opt {
                    // Pay from reserves; cap payout if bank is short
                    let payout = round_cents(salary.min(bank.reserve_balance.max(0.0)));
                    bank.reserve_balance = round_cents(bank.reserve_balance - payout);
                    bank.last_month_payroll_paid =
                        round_cents(bank.last_month_payroll_paid + payout);
                    payout
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let tax = monthly_income_tax_with_params(gross_pay, tax_params).min(gross_pay);
            let net_pay = round_cents(gross_pay - tax);

            person.cash_balance = round_cents(person.cash_balance + net_pay);
            person.last_month_gross_pay = gross_pay;
            person.last_month_tax_paid = tax;
            person.last_month_net_pay = net_pay;

            total_tax = round_cents(total_tax + tax);
        }

        self.treasury_balance = round_cents(self.treasury_balance + total_tax);
        self.monthly_tax_collected = round_cents(self.monthly_tax_collected + total_tax);
    }

    pub(super) fn collect_monthly_loan_repayments(&mut self) {
        let country_name = self.name.clone();
        let day = self.day;
        let banks = &mut self.banks;
        let mut pending_events = Vec::new();

        for person_idx in 0..self.people.len() {
            let lender_bank_id = self.people[person_idx]
                .loan
                .as_ref()
                .map(|loan| loan.lender_bank_id);
            let Some(lender_bank_id) = lender_bank_id else {
                continue;
            };

            let mut cash_balance = self.people[person_idx].cash_balance;
            let mut loan = self.people[person_idx].loan.take();
            let Some(bank) = banks.iter_mut().find(|bank| bank.id == lender_bank_id) else {
                self.people[person_idx].loan = loan;
                continue;
            };
            let payment = bank.collect_monthly_payment(&mut cash_balance, &mut loan);

            let person = &mut self.people[person_idx];
            person.cash_balance = cash_balance;
            person.loan = loan;
            person.last_month_loan_payment = payment.amount_paid;
            if payment.unpaid_amount > 0.0 {
                pending_events.push(SimEvent::DebtDistress {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Person,
                    entity_id: person.id,
                    unpaid_amount: payment.unpaid_amount,
                    day,
                });
            }
            if payment.loan_ended {
                pending_events.push(SimEvent::LoanEnded {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Person,
                    entity_id: person.id,
                    day,
                });
            }
        }

        for company_idx in 0..self.companies.len() {
            let lender_bank_id = self.companies[company_idx]
                .loan
                .as_ref()
                .map(|loan| loan.lender_bank_id);
            let Some(lender_bank_id) = lender_bank_id else {
                continue;
            };

            let mut cash_balance = self.companies[company_idx].cash_balance;
            let mut loan = self.companies[company_idx].loan.take();
            let Some(bank) = banks.iter_mut().find(|bank| bank.id == lender_bank_id) else {
                self.companies[company_idx].loan = loan;
                continue;
            };
            let payment = bank.collect_monthly_payment(&mut cash_balance, &mut loan);

            let company = &mut self.companies[company_idx];
            company.cash_balance = cash_balance;
            company.loan = loan;
            company.last_month_loan_payment = payment.amount_paid;
            company.missed_loan_payment_this_month = payment.unpaid_amount > 0.0;
            if payment.unpaid_amount > 0.0 {
                pending_events.push(SimEvent::DebtDistress {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    unpaid_amount: payment.unpaid_amount,
                    day,
                });
            }
            if payment.loan_ended {
                pending_events.push(SimEvent::LoanEnded {
                    country_name: country_name.clone(),
                    entity_kind: EntityKind::Company,
                    entity_id: u32::from(company.id),
                    day,
                });
            }
        }
        self.pending_events.extend(pending_events);
    }
}
