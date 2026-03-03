use super::*;

impl Country {
    /// Attempt an IPO for a distressed company that cannot cover funding via
    /// debt. Returns true if the IPO succeeded and saved the company.
    pub(super) fn attempt_ipo_rescue(&mut self, company_idx: usize) -> bool {
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

    pub(super) fn process_stock_collateral_seizure(&mut self) {
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

    pub(super) fn process_stock_dividends(&mut self) {
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
