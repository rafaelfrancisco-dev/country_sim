use super::*;
use crate::sim::money::round_cents;

#[derive(Debug, Clone, Copy)]
enum DividendHolder {
    Person { country_idx: usize, person_id: u32 },
    Bank { country_idx: usize },
}

impl World {
    pub(crate) fn market_bank_portfolio_value(&self, country_id: u16) -> f64 {
        let Some(country) = self
            .countries()
            .iter()
            .find(|country| country.id() == country_id)
        else {
            return 0.0;
        };

        round_cents(
            country
                .stock_market()
                .bank_portfolio
                .positions
                .iter()
                .map(|position| {
                    self.stock_security_price(position.security_id)
                        .map_or(0.0, |price| position.market_value(price))
                })
                .sum(),
        )
    }

    pub(crate) fn stock_security_price(
        &self,
        security_id: crate::sim::stock_market::StockSecurityId,
    ) -> Option<f64> {
        self.countries().iter().find_map(|country| {
            (country.id() == security_id.country_id)
                .then(|| country.stock_market().listed.get(&security_id.company_id))
                .flatten()
                .map(|equity| equity.last_price)
        })
    }

    pub(in crate::sim::world) fn process_cross_market_stock_dividends(&mut self) {
        for listing_country_idx in 0..self.countries.len() {
            let payout_ratio = self.countries[listing_country_idx]
                .sim_params
                .stock_market
                .dividend_payout_ratio;
            let runway_months = self.countries[listing_country_idx]
                .sim_params
                .stock_market
                .dividend_min_runway_months;
            let company_ids = self.countries[listing_country_idx]
                .stock_market
                .listed
                .keys()
                .copied()
                .collect::<Vec<_>>();

            for company_id in company_ids {
                let Some((listing_country_id, current_company_id, ticker, total_shares)) = self
                    .countries[listing_country_idx]
                    .stock_market
                    .listed
                    .get(&company_id)
                    .map(|equity| {
                        (
                            self.countries[listing_country_idx].id(),
                            equity.current_company_id,
                            equity.ticker.clone(),
                            equity.shares_outstanding,
                        )
                    })
                else {
                    continue;
                };
                let Some(company) = self.countries[listing_country_idx]
                    .companies
                    .get(current_company_id as usize)
                else {
                    continue;
                };
                if company.last_month_profit <= 0.0 || total_shares == 0 {
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

                let per_share = round_cents(dividend_pool / total_shares as f64);
                if per_share <= 0.0 {
                    continue;
                }

                let security_id = crate::sim::stock_market::StockSecurityId {
                    country_id: listing_country_id,
                    company_id,
                };
                let mut payments = Vec::new();
                for (holder_country_idx, holder_country) in self.countries.iter().enumerate() {
                    for (&person_id, portfolio) in &holder_country.stock_market.person_portfolios {
                        if let Some(position) = portfolio.position(security_id)
                            && position.shares > 0
                        {
                            payments.push((
                                DividendHolder::Person {
                                    country_idx: holder_country_idx,
                                    person_id,
                                },
                                position.shares,
                            ));
                        }
                    }
                    if let Some(position) = holder_country
                        .stock_market
                        .bank_portfolio
                        .position(security_id)
                        && position.shares > 0
                    {
                        payments.push((
                            DividendHolder::Bank {
                                country_idx: holder_country_idx,
                            },
                            position.shares,
                        ));
                    }
                }

                let mut total_paid = 0.0_f64;
                for (holder, shares) in payments {
                    let gross = round_cents(shares as f64 * per_share);
                    if gross <= 0.0 {
                        continue;
                    }
                    match holder {
                        DividendHolder::Person {
                            country_idx,
                            person_id,
                        } => {
                            let tax = round_cents(gross * 0.15);
                            let net = round_cents(gross - tax);
                            if let Some(person_idx) = self.countries[country_idx]
                                .people
                                .iter()
                                .position(|person| person.id == person_id)
                                && let Some(person) =
                                    self.countries[country_idx].people.get_mut(person_idx)
                            {
                                person.cash_balance = round_cents(person.cash_balance + net);
                            }
                            if let Some(portfolio) = self.countries[country_idx]
                                .stock_market
                                .person_portfolios
                                .get_mut(&person_id)
                            {
                                portfolio.dividends_mtd =
                                    round_cents(portfolio.dividends_mtd + net);
                            }
                            self.countries[listing_country_idx].treasury_balance = round_cents(
                                self.countries[listing_country_idx].treasury_balance + tax,
                            );
                            self.countries[listing_country_idx].monthly_tax_collected = round_cents(
                                self.countries[listing_country_idx].monthly_tax_collected + tax,
                            );
                            total_paid = round_cents(total_paid + gross);
                        }
                        DividendHolder::Bank { country_idx } => {
                            let stock_bank_id = self.countries[country_idx]
                                .stock_market_bank_id()
                                .unwrap_or(self.countries[country_idx].primary_bank().id);
                            if let Some(bank) =
                                self.countries[country_idx].bank_by_id_mut(stock_bank_id)
                            {
                                bank.reserve_balance = round_cents(bank.reserve_balance + gross);
                            }
                            self.countries[country_idx]
                                .stock_market
                                .bank_portfolio
                                .dividends_mtd = round_cents(
                                self.countries[country_idx]
                                    .stock_market
                                    .bank_portfolio
                                    .dividends_mtd
                                    + gross,
                            );
                            total_paid = round_cents(total_paid + gross);
                        }
                    }
                }

                if total_paid <= 0.0 {
                    continue;
                }

                if let Some(company) = self.countries[listing_country_idx]
                    .companies
                    .get_mut(current_company_id as usize)
                {
                    company.cash_balance = round_cents(company.cash_balance - total_paid);
                }
                self.countries[listing_country_idx]
                    .stock_market
                    .dividends_paid_mtd = round_cents(
                    self.countries[listing_country_idx]
                        .stock_market
                        .dividends_paid_mtd
                        + total_paid,
                );
                if let Some(equity) = self.countries[listing_country_idx]
                    .stock_market
                    .listed
                    .get_mut(&company_id)
                {
                    equity.record_monthly_dps(per_share);
                }
                let country_name = self.countries[listing_country_idx].name.clone();
                let day = self.countries[listing_country_idx].day;
                self.countries[listing_country_idx]
                    .pending_events
                    .push(SimEvent::StockDividend {
                        country_name,
                        company_id: current_company_id,
                        ticker,
                        total_amount: total_paid,
                        per_share,
                        day,
                    });
            }
        }
    }

    pub(in crate::sim::world) fn cleanup_stale_cross_market_positions(&mut self) {
        for country_idx in 0..self.countries.len() {
            let stale_person_positions = self.countries[country_idx]
                .stock_market
                .person_portfolios
                .iter()
                .map(|(&person_id, portfolio)| {
                    let stale = portfolio
                        .positions
                        .iter()
                        .filter_map(|position| {
                            (!self.security_exists(position.security_id))
                                .then_some(position.security_id)
                        })
                        .collect::<Vec<_>>();
                    (person_id, stale)
                })
                .collect::<Vec<_>>();

            for (person_id, positions) in stale_person_positions {
                if let Some(portfolio) = self.countries[country_idx]
                    .stock_market
                    .person_portfolios
                    .get_mut(&person_id)
                {
                    for security_id in positions {
                        portfolio.wipe_security(security_id);
                    }
                }
            }

            let stale_bank_positions = self.countries[country_idx]
                .stock_market
                .bank_portfolio
                .positions
                .iter()
                .filter_map(|position| {
                    (!self.security_exists(position.security_id)).then_some(position.security_id)
                })
                .collect::<Vec<_>>();
            for security_id in stale_bank_positions {
                self.countries[country_idx]
                    .stock_market
                    .bank_portfolio
                    .wipe_security(security_id);
            }
            self.countries[country_idx].stock_market.gc_portfolios();
        }
    }

    fn security_exists(&self, security_id: crate::sim::stock_market::StockSecurityId) -> bool {
        self.countries.iter().any(|country| {
            country.id() == security_id.country_id
                && country
                    .stock_market
                    .listed
                    .contains_key(&security_id.company_id)
        })
    }
}
