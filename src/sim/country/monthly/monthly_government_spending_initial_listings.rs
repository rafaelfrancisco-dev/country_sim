use super::*;

impl Country {
    pub(super) fn close_company_months(&mut self) {
        let mut market = self.sim_params.goods_market;
        let price_pressure = self.price_pressure_multiplier().max(0.01);
        market.price_raise_rate = (market.price_raise_rate * price_pressure).clamp(0.0, 1.0);
        market.price_cut_rate = (market.price_cut_rate / price_pressure).clamp(0.0, 0.95);
        let threshold_shift = self.sell_through_threshold_shift();
        market.sell_through_raise_threshold =
            (market.sell_through_raise_threshold + threshold_shift).clamp(0.0, 1.0);
        market.sell_through_cut_threshold =
            (market.sell_through_cut_threshold + threshold_shift).clamp(0.0, 1.0);
        if market.sell_through_cut_threshold > market.sell_through_raise_threshold {
            market.sell_through_cut_threshold =
                (market.sell_through_raise_threshold - 0.01).max(0.0);
        }

        let dynamic_base_index = self.public_utility_dynamic_base_index();
        let cap_multiplier = self
            .sim_params
            .government_backfill
            .public_utility_price_cap_multiplier
            .max(1.0);
        let population = self.people.len().max(1) as f64;
        let debt_per_capita = self.monthly_essential_debt_spending / population;
        let unmet_events_per_capita = self.monthly_unmet_need_events as f64 / population;
        let affordability_pressure =
            ((debt_per_capita / 80.0 + unmet_events_per_capita / 14.0) * 0.5).clamp(0.0, 1.0);
        let close_one = |company: &mut crate::sim::company::Company| {
            company.close_month(&market);
            if !company.is_public_utility {
                return;
            }
            let Some(good) = good_by_id(company.produced_good_id) else {
                return;
            };
            let min_price = crate::sim::company::minimum_viable_unit_price(
                company.produced_good_id,
                market.worker_effort_per_month,
                market.min_margin_rate,
            );
            let dynamic_base_cost = round_cents(good.base_unit_cost * dynamic_base_index);
            let max_price = round_cents((dynamic_base_cost * cap_multiplier).max(min_price));
            let anchor_price = round_cents((min_price + max_price) * 0.5);
            if company.unit_price > anchor_price {
                // Price relief intensifies under affordability stress, reducing runaway
                // debt while preserving endogenous month-to-month movement.
                let correction_strength = 0.10 + affordability_pressure * 0.25;
                let gap = company.unit_price - anchor_price;
                company.unit_price = round_cents(company.unit_price - gap * correction_strength);
            } else {
                // Mild upward drift when well below anchor to avoid hard pegging.
                let gap = anchor_price - company.unit_price;
                company.unit_price = round_cents(company.unit_price + gap * 0.05);
            }
            company.unit_price = round_cents(company.unit_price.clamp(min_price, max_price));
        };

        if self.use_inner_parallelism {
            self.companies
                .par_chunks_mut(COMPANY_CHUNK_SIZE)
                .for_each(|chunk| {
                    for company in chunk {
                        close_one(company);
                    }
                });
        } else {
            for company in &mut self.companies {
                close_one(company);
            }
        }
    }

    pub(super) fn process_government_spending(&mut self) {
        let fiscal = self.sim_params.fiscal;
        let base_benefit = round_cents(fiscal.monthly_unemployment_benefit());
        let austerity_benefit_amount = if self.austerity_active {
            round_cents(base_benefit * (1.0 - fiscal.austerity_benefit_reduction))
        } else {
            base_benefit
        };
        let basic_threshold = fiscal.basic_needs_cash_threshold;

        // Essential needs goods are IDs 0–4 (water, food, housing, electricity, medicine).
        // Mask covering the first 5 goods.
        const ESSENTIAL_NEEDS_MASK: u16 = 0b0001_1111;

        let mut total_spent: f64 = 0.0;
        let mut total_recipients: u32 = 0;

        for person in &mut self.people {
            let age = person.age_years();
            let is_working_age = (18..=65).contains(&age);
            let mut paid_unemployment_benefit = false;

            // Track consecutive unemployment months
            if person.is_employed() {
                person.months_unemployed = 0;
                person.months_on_benefits = 0;
            } else if is_working_age {
                person.months_unemployed = person.months_unemployed.saturating_add(1);
            }

            let within_duration_cap = person.months_unemployed <= fiscal.max_benefit_months;
            if !person.is_employed()
                && is_working_age
                && austerity_benefit_amount > 0.0
                && within_duration_cap
            {
                // Pay unemployment benefit from treasury
                let payout =
                    round_cents(austerity_benefit_amount.min(self.treasury_balance.max(0.0)));
                if payout > 0.0 {
                    person.cash_balance = round_cents(person.cash_balance + payout);
                    self.treasury_balance = round_cents(self.treasury_balance - payout);
                    total_spent = round_cents(total_spent + payout);
                    total_recipients = total_recipients.saturating_add(1);
                    paid_unemployment_benefit = true;
                }
            }

            if !person.is_employed() && is_working_age {
                if paid_unemployment_benefit {
                    person.months_on_benefits = person.months_on_benefits.saturating_add(1);
                } else {
                    person.months_on_benefits = 0;
                }
            }

            if person.cash_balance < basic_threshold
                && (person.last_month_unmet_needs_mask & ESSENTIAL_NEEDS_MASK) != 0
            {
                // Top up to basic_threshold for people with unmet essential needs (unconditional)
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
    pub(super) fn do_initial_listing(&mut self, company_idx: usize) {
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
        // target ~people/5 subscribers ? per_participant_max = float_shares / target.
        // This gives the daily market a sufficient holder base to generate sell orders.
        let reserve_ratio = params.investor_cash_reserve_ratio;
        let target_subscribers = (self.people.len() / 5).max(1) as u64;
        let per_participant_max = (float_shares / target_subscribers).max(1);
        let mut shares_allocated = 0_u64;
        let mut raised = 0.0_f64;
        let mut subscriptions: Vec<(u32, u64, f64)> = Vec::new();
        let security_id = crate::sim::stock_market::StockSecurityId {
            country_id: self.id,
            company_id,
        };

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
        let bank_reserve = self.primary_bank().reserve_balance;
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
                    .add_shares(security_id, *alloc, price_per_share);
            }
        }
        if bank_alloc > 0 {
            let primary_bank = self.primary_bank_mut();
            primary_bank.reserve_balance = round_cents(primary_bank.reserve_balance - bank_cost);
            self.stock_market
                .bank_portfolio
                .add_shares(security_id, bank_alloc, price_per_share);
        }

        // Register listing: founder shares + float sold.
        let total_shares = initial_shares + shares_allocated;
        let ticker = StockMarket::generate_ticker(company_id, &company_name);
        let equity = ListedEquity::new(
            company_id,
            company_id,
            ticker.clone(),
            price_per_share,
            total_shares,
            shares_allocated,
        );
        let ticker = self
            .stock_market
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
            reason: IpoReason::Seed,
            shares_issued: shares_allocated,
            primary_shares_issued: shares_allocated,
            secondary_shares_sold: 0,
            price_per_share,
            total_raised: raised,
            primary_raised: raised,
            secondary_raised: 0.0,
            founder_payout_total: 0.0,
            day: self.day,
        });
    }
}
