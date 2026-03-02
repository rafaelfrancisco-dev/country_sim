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

        // Essential needs goods are IDs 0â€“4 (water, food, housing, electricity, medicine).
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
    // Stock market â€” IPO rescue
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Stock market â€” seed listings at simulation start
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
        // target ~people/5 subscribers â†’ per_participant_max = float_shares / target.
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

        // Apply subscriptions regardless of uptake â€” healthy company always lists.
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
                    // IPO success â€” apply bank subscription
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
    // Stock market â€” collateral seizure
    // -----------------------------------------------------------------------

