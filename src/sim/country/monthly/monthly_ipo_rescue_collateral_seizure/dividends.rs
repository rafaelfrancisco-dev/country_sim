use std::collections::HashMap;

use super::*;

impl Country {
    pub(in crate::sim::country::monthly) fn process_stock_dividends(&mut self) {
        if !self.sim_params.stock_market.enabled {
            return;
        }

        let payout_ratio = self.sim_params.stock_market.dividend_payout_ratio;
        let runway_months = self.sim_params.stock_market.dividend_min_runway_months;

        // Build O(1) person lookup once for all companies.
        let person_idx_by_id: HashMap<u32, usize> = self
            .people
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id, i))
            .collect();

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
                if let Some(pos) = portfolio.position(company_id)
                    && pos.shares > 0
                {
                    payments.push((Some(pid), pos.shares));
                }
            }
            if let Some(pos) = self.stock_market.bank_portfolio.position(company_id)
                && pos.shares > 0
            {
                payments.push((None, pos.shares));
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
                        if let Some(&idx) = person_idx_by_id.get(pid) {
                            if let Some(person) = self.people.get_mut(idx) {
                                person.cash_balance = round_cents(person.cash_balance + net);
                            }
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
                        self.primary_bank_mut().reserve_balance =
                            round_cents(self.primary_bank_mut().reserve_balance + gross);
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

            // Record DPS in the trailing-12-month ring buffer for valuation.
            if let Some(equity) = self.stock_market.listed.get_mut(&company_id) {
                equity.record_monthly_dps(per_share);
            }

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
