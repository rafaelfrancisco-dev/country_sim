use super::*;

impl Country {
    /// Attempt an IPO for a distressed company that cannot cover funding via
    /// debt. Returns true if the IPO succeeded and saved the company.
    pub(super) fn attempt_ipo_rescue(&mut self, company_idx: usize) -> bool {
        self.attempt_ipo(company_idx, IpoReason::Distress)
    }

    pub(super) fn process_growth_ipos(&mut self) {
        let params = self.sim_params.stock_market;
        if !params.enabled || params.ipo_max_new_listings_per_month == 0 {
            return;
        }

        let mut candidates: Vec<(usize, f64)> = self
            .companies
            .iter()
            .enumerate()
            .filter_map(|(company_idx, company)| {
                if self.stock_market.is_listed(company.id)
                    || company.is_public_utility
                    || company.age_months < params.ipo_min_company_age_months
                    || company.ipo_cooldown_months > 0
                {
                    return None;
                }
                if company.consecutive_profitable_months < params.ipo_growth_min_profitable_months {
                    return None;
                }
                let revenue = company.average_monthly_revenue();
                if revenue < params.ipo_growth_min_monthly_revenue {
                    return None;
                }
                let payroll = company.monthly_payroll_budget.max(1.0);
                let runway = company.cash_balance.max(0.0) / payroll;
                if runway < params.ipo_growth_min_cash_runway_months
                    || runway > params.ipo_growth_max_cash_runway_months
                {
                    return None;
                }

                let score = revenue + company.last_month_profit.max(0.0) * 0.5 + payroll * 0.25;
                Some((company_idx, score))
            })
            .collect();
        if candidates.is_empty() {
            candidates = self
                .companies
                .iter()
                .enumerate()
                .filter_map(|(company_idx, company)| {
                    if self.stock_market.is_listed(company.id)
                        || company.is_public_utility
                        || company.age_months < params.ipo_min_company_age_months
                        || company.ipo_cooldown_months > 0
                    {
                        return None;
                    }
                    let revenue = company.average_monthly_revenue();
                    if revenue <= 0.0 {
                        return None;
                    }
                    let score = revenue + company.cash_balance.max(0.0) * 0.05;
                    Some((company_idx, score))
                })
                .collect();
        }
        candidates.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        candidates.truncate(params.ipo_max_new_listings_per_month as usize);

        for (company_idx, _) in candidates {
            let _ = self.attempt_ipo(company_idx, IpoReason::Growth);
        }
    }

    pub(super) fn attempt_ipo(&mut self, company_idx: usize, reason: IpoReason) -> bool {
        let params = self.sim_params.stock_market;
        if !params.enabled || params.ipo_initial_shares == 0 {
            return false;
        }
        let Some(company) = self.companies.get(company_idx) else {
            return false;
        };
        if self.stock_market.is_listed(company.id) || company.ipo_cooldown_months > 0 {
            return false;
        }

        let company_id = company.id;
        let company_name = company.name.clone();
        let monthly_revenue = company.average_monthly_revenue();
        let last_month_profit = company.last_month_profit;
        let cash_balance = company.cash_balance;
        let debt_balance = company.loan.as_ref().map_or(0.0, |l| l.debt_balance);
        let monthly_payroll = company.monthly_payroll_budget.max(0.0);
        let founder_shares_total: u64 = company.founder_stakes.iter().map(|s| s.shares_owned).sum();
        let pre_ipo_shares = if founder_shares_total > 0 {
            founder_shares_total
        } else {
            params.ipo_initial_shares
        };
        if pre_ipo_shares == 0 {
            return false;
        }

        let funding_need = match reason {
            IpoReason::Distress => round_cents((-cash_balance).max(0.0).max(monthly_payroll * 2.0)),
            IpoReason::Growth => round_cents(
                (monthly_payroll * params.ipo_primary_raise_target_runway_months - cash_balance)
                    .max(monthly_payroll * 0.75)
                    .max(0.0),
            ),
            IpoReason::Seed => 0.0,
        };
        if !matches!(reason, IpoReason::Seed) && funding_need <= 0.0 {
            return false;
        }

        let price_per_share = StockMarket::ipo_valuation_anchor(
            monthly_revenue,
            last_month_profit,
            cash_balance,
            debt_balance,
            pre_ipo_shares,
        );
        if price_per_share <= 0.0 {
            return false;
        }

        let max_primary = ((pre_ipo_shares as f64 * params.ipo_max_dilution) as u64).max(1);
        let mut primary_shares_to_offer = ((funding_need / price_per_share).ceil() as u64).max(1);
        primary_shares_to_offer = primary_shares_to_offer.min(max_primary);

        let mut secondary_shares_to_offer = 0_u64;
        let mut founder_total_offer = 0_u64;
        if matches!(reason, IpoReason::Growth) && founder_shares_total > 0 {
            let cap = params.ipo_founder_secondary_cap_fraction.clamp(0.0, 1.0);
            founder_total_offer = company
                .founder_stakes
                .iter()
                .map(|stake| {
                    ((stake.shares_owned as f64 * cap).floor() as u64).min(stake.shares_owned)
                })
                .sum();
            secondary_shares_to_offer =
                founder_total_offer.min((primary_shares_to_offer / 2).max(1));
        }

        let total_shares_to_offer =
            primary_shares_to_offer.saturating_add(secondary_shares_to_offer);
        if total_shares_to_offer == 0 {
            return false;
        }
        let offer_value_target = round_cents(total_shares_to_offer as f64 * price_per_share);
        if offer_value_target <= 0.0 {
            return false;
        }

        let reserve_ratio = params.investor_cash_reserve_ratio.clamp(0.0, 1.0);
        let per_participant_max = (total_shares_to_offer / 10).max(1);
        let mut subscriptions: Vec<(u32, u64, f64)> = Vec::new();
        let mut shares_allocated = 0_u64;
        let mut raised = 0.0_f64;

        for person in &self.people {
            if shares_allocated >= total_shares_to_offer {
                break;
            }
            let investable = round_cents((person.cash_balance * (1.0 - reserve_ratio)).max(0.0));
            if investable < price_per_share {
                continue;
            }
            let affordable_shares = (investable / price_per_share) as u64;
            let want = affordable_shares.min(per_participant_max);
            let available = total_shares_to_offer - shares_allocated;
            let alloc = want.min(available);
            if alloc == 0 {
                continue;
            }
            let cost = round_cents(alloc as f64 * price_per_share);
            subscriptions.push((person.id, alloc, cost));
            shares_allocated += alloc;
            raised = round_cents(raised + cost);
        }

        let bank_reserve = self.bank.reserve_balance;
        let bank_equity_exposure = self.stock_market.bank_equity_exposure();
        let bank_headroom = round_cents(
            (bank_reserve * params.bank_max_equity_reserve_fraction - bank_equity_exposure)
                .max(0.0),
        );
        let mut bank_alloc = 0_u64;
        let mut bank_cost = 0.0_f64;
        if shares_allocated < total_shares_to_offer && bank_headroom >= price_per_share {
            let remaining = total_shares_to_offer - shares_allocated;
            let bank_affordable = (bank_headroom / price_per_share) as u64;
            bank_alloc = bank_affordable.min(remaining);
            if bank_alloc > 0 {
                bank_cost = round_cents(bank_alloc as f64 * price_per_share);
                shares_allocated += bank_alloc;
                raised = round_cents(raised + bank_cost);
            }
        }

        let fill_ratio = raised / offer_value_target;
        let min_fill = match reason {
            IpoReason::Growth => params.ipo_min_fill_ratio_growth,
            IpoReason::Distress => params.ipo_min_fill_ratio_distress,
            IpoReason::Seed => 1.0,
        };
        if fill_ratio + 1e-9 < min_fill {
            if let Some(company) = self.companies.get_mut(company_idx) {
                company.failed_ipo_attempts = company.failed_ipo_attempts.saturating_add(1);
                company.ipo_cooldown_months = params.ipo_retry_cooldown_months;
            }
            return false;
        }

        for (person_id, alloc, cost) in &subscriptions {
            if let Some(person) = self.people.iter_mut().find(|p| p.id == *person_id) {
                person.cash_balance = round_cents((person.cash_balance - cost).max(0.0));
                self.stock_market
                    .person_portfolio_mut(*person_id)
                    .add_shares(company_id, *alloc, price_per_share);
            }
        }
        if bank_alloc > 0 {
            self.bank.reserve_balance =
                round_cents((self.bank.reserve_balance - bank_cost).max(0.0));
            self.stock_market
                .bank_portfolio
                .add_shares(company_id, bank_alloc, price_per_share);
        }

        let shares_sold = shares_allocated;
        let primary_shares_sold = shares_sold.min(primary_shares_to_offer);
        let secondary_shares_sold = shares_sold.saturating_sub(primary_shares_sold);
        let primary_raised = round_cents(primary_shares_sold as f64 * price_per_share);
        let secondary_raised = round_cents(secondary_shares_sold as f64 * price_per_share);
        let total_raised = round_cents(primary_raised + secondary_raised);
        let shares_outstanding = pre_ipo_shares.saturating_add(primary_shares_sold);

        let mut founder_payout_total = 0.0_f64;
        if secondary_shares_sold > 0 && founder_total_offer > 0 {
            let cap = params.ipo_founder_secondary_cap_fraction.clamp(0.0, 1.0);
            let mut remaining_shares = secondary_shares_sold;
            let mut remaining_cash = secondary_raised;
            let plan = self
                .companies
                .get(company_idx)
                .map(|c| {
                    c.founder_stakes
                        .iter()
                        .enumerate()
                        .map(|(idx, stake)| {
                            let offered = ((stake.shares_owned as f64 * cap).floor() as u64)
                                .min(stake.shares_owned);
                            (idx, stake.person_id, offered)
                        })
                        .filter(|(_, _, offered)| *offered > 0)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let offered_total: u64 = plan.iter().map(|(_, _, offered)| *offered).sum();

            for (plan_idx, (stake_idx, person_id, offered)) in plan.iter().copied().enumerate() {
                if remaining_shares == 0 || offered_total == 0 {
                    break;
                }
                let sold_shares = if plan_idx + 1 == plan.len() {
                    remaining_shares
                } else {
                    (((secondary_shares_sold as f64) * (offered as f64 / offered_total as f64))
                        .round() as u64)
                        .min(remaining_shares)
                };
                if sold_shares == 0 {
                    continue;
                }
                let payout = if plan_idx + 1 == plan.len() {
                    remaining_cash
                } else {
                    round_cents(sold_shares as f64 * price_per_share).min(remaining_cash)
                };

                if let Some(company) = self.companies.get_mut(company_idx)
                    && let Some(stake) = company.founder_stakes.get_mut(stake_idx)
                {
                    stake.shares_owned = stake.shares_owned.saturating_sub(sold_shares);
                }
                if let Some(person) = self.people.iter_mut().find(|p| p.id == person_id) {
                    person.cash_balance = round_cents(person.cash_balance + payout);
                }

                remaining_shares = remaining_shares.saturating_sub(sold_shares);
                remaining_cash = round_cents((remaining_cash - payout).max(0.0));
                founder_payout_total = round_cents(founder_payout_total + payout);
            }
        }

        // Register retained founder shares in person_portfolios so founders can trade them.
        let mut total_retained_founder_shares = 0_u64;
        if let Some(company) = self.companies.get(company_idx) {
            let stakes_snapshot: Vec<(u32, u64)> = company
                .founder_stakes
                .iter()
                .filter(|s| s.shares_owned > 0)
                .map(|s| (s.person_id, s.shares_owned))
                .collect();
            for (person_id, retained) in stakes_snapshot {
                self.stock_market
                    .person_portfolio_mut(person_id)
                    .add_shares(company_id, retained, price_per_share);
                total_retained_founder_shares =
                    total_retained_founder_shares.saturating_add(retained);
            }
        }

        let ticker = StockMarket::generate_ticker(company_id, &company_name);
        let equity = ListedEquity::new(
            company_id,
            ticker.clone(),
            price_per_share,
            shares_outstanding,
            shares_sold,
        );
        self.stock_market
            .insert_listing_preserve_index(company_id, equity);

        // Expand free float to include retained founder shares (now freely tradeable).
        if total_retained_founder_shares > 0 {
            if let Some(equity) = self.stock_market.listed.get_mut(&company_id) {
                equity.free_float_shares = equity
                    .free_float_shares
                    .saturating_add(total_retained_founder_shares);
            }
        }

        if let Some(company) = self.companies.get_mut(company_idx) {
            company.cash_balance = round_cents(company.cash_balance + primary_raised);
            company.failed_ipo_attempts = 0;
            company.ipo_cooldown_months = 0;
        }
        self.stock_market.ipo_raised_mtd =
            round_cents(self.stock_market.ipo_raised_mtd + total_raised);
        self.pending_events.push(SimEvent::StockIpo {
            country_name: self.name.clone(),
            company_id,
            company_name,
            ticker,
            reason,
            shares_issued: shares_sold,
            primary_shares_issued: primary_shares_sold,
            secondary_shares_sold,
            price_per_share,
            total_raised,
            primary_raised,
            secondary_raised,
            founder_payout_total,
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
