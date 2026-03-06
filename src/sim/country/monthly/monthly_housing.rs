use super::*;
use crate::sim::{
    bank::MortgageAccount,
    bank::payment_for,
    event::SimEvent,
    money::round_cents,
    person::HousingStatus,
};

const HOUSING_GOOD_ID: u8 = 2;

impl Country {
    /// Drain housing company inventory into the housing stock pool.
    pub(super) fn process_housing_construction(&mut self) {
        let mut new_units: u32 = 0;
        for company in &mut self.companies {
            if company.produced_good_id != HOUSING_GOOD_ID {
                continue;
            }
            let built = company.inventory_units;
            if built == 0 {
                continue;
            }
            company.inventory_units = 0;
            company.rental_units_owned = company.rental_units_owned.saturating_add(built);
            new_units = new_units.saturating_add(built);
        }

        if new_units > 0 {
            self.housing_stock_total = self.housing_stock_total.saturating_add(new_units);
            self.housing_stock_occupied_rented =
                self.housing_stock_occupied_rented.saturating_add(new_units);
            self.monthly_housing_units_built =
                self.monthly_housing_units_built.saturating_add(new_units);
            // New units go directly to rental stock (already occupied by company ownership).
            // Vacancy is unchanged because they are claimed immediately.
        }
    }

    /// Adjust housing price index based on vacancy deviation from target.
    pub(super) fn update_housing_price_index(&mut self) {
        let params = self.sim_params.housing;
        if self.housing_stock_total == 0 || self.housing_price_index <= 0.0 {
            return;
        }

        let vacancy_rate = self.housing_vacancy_rate();
        let deviation = vacancy_rate - params.target_vacancy_rate;
        // If vacancy < target (shortage) → prices rise; if vacancy > target (surplus) → prices fall.
        let change = (-deviation * 2.0).clamp(
            -params.max_monthly_price_change,
            params.max_monthly_price_change,
        );

        let old_index = self.housing_price_index;
        self.housing_price_index = round_cents(self.housing_price_index * (1.0 + change));
        self.monthly_rent = round_cents(self.housing_price_index * params.rent_as_fraction_of_price);

        if change.abs() > 0.005 {
            self.pending_events.push(SimEvent::HousingPriceChanged {
                country_name: self.name.clone(),
                new_price_index: self.housing_price_index,
                change_pct: (self.housing_price_index - old_index) / old_index.max(1.0),
                day: self.day,
            });
        }
    }

    /// Collect monthly rent from renters and credit landlord companies.
    pub(super) fn collect_rent(&mut self) {
        let monthly_rent = self.monthly_rent;
        if monthly_rent <= 0.0 {
            return;
        }

        // Collect-then-apply: gather (person_idx, company_id, amount_paid) without holding &mut
        struct RentEntry {
            person_idx: usize,
            company_id: u16,
            amount_due: f64,
            amount_paid: f64,
        }

        let mut entries: Vec<RentEntry> = Vec::new();
        for (person_idx, person) in self.people.iter().enumerate() {
            let landlord_company_id = match &person.housing {
                HousingStatus::Renter {
                    landlord_company_id,
                    landlord_country_id,
                } if *landlord_country_id == self.id => *landlord_company_id,
                _ => continue,
            };
            let available = person.cash_balance.max(0.0);
            let amount_paid = round_cents(monthly_rent.min(available));
            entries.push(RentEntry {
                person_idx,
                company_id: landlord_company_id,
                amount_due: monthly_rent,
                amount_paid,
            });
        }

        let country_name = self.name.clone();
        let day = self.day;
        let mut pending_events = Vec::new();
        let mut total_rent_collected = 0.0_f64;

        for entry in entries {
            let person = &mut self.people[entry.person_idx];
            person.cash_balance = round_cents(person.cash_balance - entry.amount_paid);
            person.last_month_rent_paid = round_cents(person.last_month_rent_paid + entry.amount_paid);

            if let Some(company) = self.companies.get_mut(entry.company_id as usize) {
                company.cash_balance = round_cents(company.cash_balance + entry.amount_paid);
                company.last_month_rent_revenue =
                    round_cents(company.last_month_rent_revenue + entry.amount_paid);
            }

            total_rent_collected = round_cents(total_rent_collected + entry.amount_paid);

            if entry.amount_paid < entry.amount_due {
                pending_events.push(SimEvent::RentUnpaid {
                    country_name: country_name.clone(),
                    person_id: self.people[entry.person_idx].id,
                    amount_due: entry.amount_due,
                    amount_paid: entry.amount_paid,
                    day,
                });
            }
        }

        self.monthly_rent_collected = round_cents(self.monthly_rent_collected + total_rent_collected);
        self.pending_events.extend(pending_events);
    }

    /// Collect monthly mortgage payments from mortgaged homeowners.
    pub(super) fn collect_mortgage_payments(&mut self) {
        let country_name = self.name.clone();
        let day = self.day;
        let foreclosure_threshold = self.sim_params.housing.foreclosure_missed_months;

        struct MortgageEntry {
            person_idx: usize,
            interest_paid: f64,
            principal_paid: f64,
            missed: bool,
            paid_off: bool,
            foreclosed: bool,
            remaining_balance: f64,
        }

        let mut entries: Vec<MortgageEntry> = Vec::new();

        for (person_idx, person) in self.people.iter().enumerate() {
            let HousingStatus::Mortgaged { mortgage } = &person.housing else {
                continue;
            };

            let monthly_rate = mortgage.annual_rate / 12.0;
            let interest_due = round_cents(mortgage.principal_balance * monthly_rate);
            let scheduled = round_cents(
                mortgage
                    .monthly_payment
                    .min(mortgage.principal_balance + interest_due),
            );
            let available = person.cash_balance.max(0.0);
            let amount_paid = round_cents(scheduled.min(available));
            let missed = amount_paid < scheduled;

            let interest_paid = round_cents(amount_paid.min(interest_due));
            let principal_paid = round_cents((amount_paid - interest_paid).max(0.0));
            let new_balance = round_cents(
                (mortgage.principal_balance + interest_due - amount_paid).max(0.0),
            );

            let new_months_missed = if missed {
                mortgage.months_missed.saturating_add(1)
            } else {
                0
            };

            let foreclosed = new_months_missed >= foreclosure_threshold;
            let paid_off = !foreclosed && new_balance <= 0.0;

            entries.push(MortgageEntry {
                person_idx,
                interest_paid,
                principal_paid,
                missed,
                paid_off,
                foreclosed,
                remaining_balance: new_balance,
            });
        }

        // Bank update: (bank_id, interest_added, principal_reduced, credit_loss)
        struct BankUpdate {
            bank_id: u16,
            reserve_add: f64,
            interest_income_add: f64,
            created_money_reduce: f64,
            principal_repaid_add: f64,
            credit_loss_add: f64,
        }

        let mut pending_events = Vec::new();
        let mut total_mortgage = 0.0_f64;
        let mut bank_updates: Vec<BankUpdate> = Vec::new();
        let mut foreclosed_count: u32 = 0;

        for entry in entries {
            let person = &mut self.people[entry.person_idx];
            let person_id = person.id;

            let HousingStatus::Mortgaged { mortgage } = &mut person.housing else {
                continue;
            };

            let payment = round_cents(entry.interest_paid + entry.principal_paid);
            person.cash_balance = round_cents(person.cash_balance - payment);
            person.last_month_mortgage_payment =
                round_cents(person.last_month_mortgage_payment + payment);
            total_mortgage = round_cents(total_mortgage + payment);

            // Update collateral value to current price index
            mortgage.collateral_value = self.housing_price_index;

            if entry.foreclosed {
                let written_off = round_cents(mortgage.principal_balance.max(0.0));
                let lender_bank_id = mortgage.lender_bank_id;

                bank_updates.push(BankUpdate {
                    bank_id: lender_bank_id,
                    reserve_add: 0.0,
                    interest_income_add: 0.0,
                    created_money_reduce: written_off,
                    principal_repaid_add: 0.0,
                    credit_loss_add: written_off,
                });

                person.housing = HousingStatus::Unhoused;
                foreclosed_count = foreclosed_count.saturating_add(1);

                pending_events.push(SimEvent::Foreclosure {
                    country_name: country_name.clone(),
                    person_id,
                    principal_written_off: written_off,
                    day,
                });
            } else if entry.paid_off {
                let lender_bank_id = mortgage.lender_bank_id;
                bank_updates.push(BankUpdate {
                    bank_id: lender_bank_id,
                    reserve_add: entry.interest_paid,
                    interest_income_add: entry.interest_paid,
                    created_money_reduce: entry.principal_paid,
                    principal_repaid_add: entry.principal_paid,
                    credit_loss_add: 0.0,
                });

                person.housing = HousingStatus::Owner;
                pending_events.push(SimEvent::MortgagePaidOff {
                    country_name: country_name.clone(),
                    person_id,
                    day,
                });
            } else {
                // Normal payment
                let lender_bank_id = mortgage.lender_bank_id;
                bank_updates.push(BankUpdate {
                    bank_id: lender_bank_id,
                    reserve_add: entry.interest_paid,
                    interest_income_add: entry.interest_paid,
                    created_money_reduce: entry.principal_paid,
                    principal_repaid_add: entry.principal_paid,
                    credit_loss_add: 0.0,
                });

                mortgage.principal_balance = entry.remaining_balance;
                if entry.missed {
                    mortgage.months_missed = mortgage.months_missed.saturating_add(1);
                } else {
                    mortgage.months_missed = 0;
                }
                if mortgage.months_remaining > 0 {
                    mortgage.months_remaining -= 1;
                }
            }
        }

        // Apply bank updates (no simultaneous borrow with people)
        for update in bank_updates {
            if let Some(bank) = self.bank_by_id_mut(update.bank_id) {
                bank.reserve_balance = round_cents(bank.reserve_balance + update.reserve_add);
                bank.last_month_interest_income =
                    round_cents(bank.last_month_interest_income + update.interest_income_add);
                bank.created_money_outstanding = round_cents(
                    (bank.created_money_outstanding - update.created_money_reduce).max(0.0),
                );
                bank.total_principal_repaid =
                    round_cents(bank.total_principal_repaid + update.principal_repaid_add);
                bank.last_month_credit_losses =
                    round_cents(bank.last_month_credit_losses + update.credit_loss_add);
            }
        }

        // Update housing stock for foreclosures
        if foreclosed_count > 0 {
            self.housing_stock_occupied_owned =
                self.housing_stock_occupied_owned.saturating_sub(foreclosed_count);
            self.housing_stock_vacant =
                self.housing_stock_vacant.saturating_add(foreclosed_count);
            self.monthly_foreclosures =
                self.monthly_foreclosures.saturating_add(foreclosed_count);
        }

        self.monthly_mortgage_payments =
            round_cents(self.monthly_mortgage_payments + total_mortgage);
        self.pending_events.extend(pending_events);
    }

    /// Allow eligible renters to purchase their home via a bank mortgage.
    pub(super) fn process_renter_to_owner_transitions(&mut self) {
        let params = self.sim_params.housing;
        let price_index = self.housing_price_index;
        if price_index <= 0.0 {
            return;
        }

        let down_payment_required = round_cents(price_index * params.mortgage_down_payment_rate);
        let principal = round_cents(price_index - down_payment_required);
        let mortgage_rate = (self.policy_rate + params.mortgage_spread_over_policy).clamp(
            self.sim_params.monetary.rate_floor,
            self.sim_params.monetary.rate_ceiling,
        );
        let monthly_payment = payment_for(principal, mortgage_rate, params.mortgage_term_months);
        let month = self.month;
        let day = self.day;
        let country_name = self.name.clone();

        struct PurchaseEntry {
            person_idx: usize,
            landlord_company_id: u16,
            lender_bank_id: u16,
        }

        let mut purchases: Vec<PurchaseEntry> = Vec::new();

        for (person_idx, person) in self.people.iter().enumerate() {
            let landlord_company_id = match &person.housing {
                HousingStatus::Renter {
                    landlord_company_id,
                    landlord_country_id,
                } if *landlord_country_id == self.id => *landlord_company_id,
                _ => continue,
            };

            // Eligibility checks
            if person.cash_balance < down_payment_required {
                continue;
            }
            let monthly_income = person.agreed_monthly_salary.max(0.0);
            if monthly_income <= 0.0 {
                continue;
            }
            let dti = monthly_payment / monthly_income;
            if dti > params.mortgage_max_dti {
                continue;
            }

            // Deterministic probability roll
            let roll_seed = hash64(month.wrapping_mul(0x9e37_79b9) ^ u64::from(person.id));
            let threshold = (params.renter_purchase_probability * 10_000.0) as u64;
            if roll_seed % 10_000 >= threshold {
                continue;
            }

            let lender_bank_id = self
                .preferred_lender_bank_id()
                .unwrap_or_else(|| self.primary_bank().id);

            purchases.push(PurchaseEntry {
                person_idx,
                landlord_company_id,
                lender_bank_id,
            });
        }

        let mut pending_events = Vec::new();

        // Phase 1: update people and companies (no bank borrow)
        for entry in &purchases {
            let person = &mut self.people[entry.person_idx];
            let person_id = person.id;

            // Pay down payment
            person.cash_balance = round_cents(person.cash_balance - down_payment_required);

            // Update person housing status
            person.housing = HousingStatus::Mortgaged {
                mortgage: MortgageAccount::new(
                    principal,
                    mortgage_rate,
                    params.mortgage_term_months,
                    entry.lender_bank_id,
                    price_index,
                ),
            };

            pending_events.push(SimEvent::HomePurchased {
                country_name: country_name.clone(),
                person_id,
                price: price_index,
                day,
            });
        }

        // Phase 2: update companies (no people/bank borrow)
        for entry in &purchases {
            if let Some(company) = self.companies.get_mut(entry.landlord_company_id as usize) {
                company.cash_balance = round_cents(company.cash_balance + down_payment_required);
                company.rental_units_owned = company.rental_units_owned.saturating_sub(1);
            }
        }

        // Phase 3: update banks (no people borrow)
        for entry in &purchases {
            if let Some(bank) = self.bank_by_id_mut(entry.lender_bank_id) {
                bank.created_money_outstanding =
                    round_cents(bank.created_money_outstanding + principal);
                bank.total_originated = round_cents(bank.total_originated + principal);
            }
        }

        // Phase 4: update housing stock counters
        let purchase_count = purchases.len() as u32;
        self.housing_stock_occupied_rented =
            self.housing_stock_occupied_rented.saturating_sub(purchase_count);
        self.housing_stock_occupied_owned =
            self.housing_stock_occupied_owned.saturating_add(purchase_count);
        self.monthly_home_purchases = self.monthly_home_purchases.saturating_add(purchase_count);

        self.pending_events.extend(pending_events);
    }
}
