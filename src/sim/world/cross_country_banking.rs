use super::World;
use crate::sim::{
    bank::BankId,
    event::SimEvent,
    money::round_cents,
    person::ForeignDeposit,
};

/// Per-country snapshot used during cross-country deposit processing.
struct CountryDepositInfo {
    id: u16,
    name: String,
    best_bank_id: Option<BankId>,
    annual_deposit_rate: f64,
}

impl World {
    /// Monthly: process cross-country deposits.
    ///
    /// 1. Pay interest on existing foreign deposits (debit foreign bank reserves, credit person).
    /// 2. Recall foreign deposits when person is in distress.
    /// 3. Open new foreign deposits when a foreign bank offers a meaningfully better rate.
    pub fn process_cross_country_deposits(&mut self) {
        if self.countries.len() < 2 {
            return;
        }

        // Snapshot of each country's best deposit bank and rate (read-only pass).
        let infos: Vec<CountryDepositInfo> = self
            .countries
            .iter()
            .map(|c| {
                let best_bank_id = c.public_bank_id_or_first();
                let annual_rate = best_bank_id
                    .map(|id| c.deposit_rate_for(id))
                    .unwrap_or(0.0);
                CountryDepositInfo {
                    id: c.id,
                    name: c.name.clone(),
                    best_bank_id,
                    annual_deposit_rate: annual_rate,
                }
            })
            .collect();

        // ---------------------------------------------------------------
        // Step 1 & 2: Interest + distress recall.
        // Collect (home_c_idx, person_idx, interest, foreign_c_idx, recall).
        // ---------------------------------------------------------------
        struct InterestOp {
            home_c_idx: usize,
            person_idx: usize,
            interest: f64,
            foreign_c_idx: usize,
            recall: bool,
        }
        let mut interest_ops: Vec<InterestOp> = Vec::new();

        for (home_c_idx, country) in self.countries.iter().enumerate() {
            let params = country.sim_params.banking;
            for (p_idx, person) in country.people.iter().enumerate() {
                let Some(fd) = &person.foreign_deposit else {
                    continue;
                };
                let Some(foreign_c_idx) = infos.iter().position(|i| i.id == fd.country_id) else {
                    continue;
                };
                let annual_rate = infos[foreign_c_idx].annual_deposit_rate;
                let monthly_rate = (1.0_f64 + annual_rate).powf(1.0 / 12.0) - 1.0;
                let interest = round_cents((fd.balance * monthly_rate).max(0.0));

                let recall = person.cash_balance < params.cross_country_deposit_distress_threshold
                    && fd.balance > 0.0;

                interest_ops.push(InterestOp {
                    home_c_idx,
                    person_idx: p_idx,
                    interest,
                    foreign_c_idx,
                    recall,
                });
            }
        }

        // Apply interest and recalls.
        for op in interest_ops {
            // ---- credit interest from foreign bank ----
            if op.interest > 0.0 {
                // Debit foreign bank reserves.
                {
                    let foreign = &mut self.countries[op.foreign_c_idx];
                    let bid = foreign
                        .people
                        .get(0) // we only need the country's bank; use bank directly
                        .and_then(|_| None::<BankId>); // placeholder — handled below
                    let _ = bid;
                    // Debit from the public bank's reserves (best-effort; floor at 0).
                    foreign.bank.reserve_balance =
                        round_cents((foreign.bank.reserve_balance - op.interest).max(0.0));
                    foreign.bank.foreign_deposit_liabilities = round_cents(
                        (foreign.bank.foreign_deposit_liabilities - op.interest).max(0.0),
                    );
                }
                // Credit to person's local cash.
                if let Some(person) = self.countries[op.home_c_idx].people.get_mut(op.person_idx) {
                    person.cash_balance = round_cents(person.cash_balance + op.interest);
                    if let Some(fd) = &mut person.foreign_deposit {
                        // Interest stays in the bank; balance unchanged (not reinvested).
                        let _ = fd;
                    }
                }
            }

            // ---- recall on distress ----
            if op.recall {
                let (returned, foreign_country_name, person_id, day) = {
                    let person = match self.countries[op.home_c_idx].people.get(op.person_idx) {
                        Some(p) => p,
                        None => continue,
                    };
                    let fd = match &person.foreign_deposit {
                        Some(fd) => fd,
                        None => continue,
                    };
                    let returned = fd.balance;
                    let fc_name = infos[op.foreign_c_idx].name.clone();
                    (returned, fc_name, person.id, self.countries[op.home_c_idx].day)
                };

                // Debit from foreign bank.
                {
                    let foreign = &mut self.countries[op.foreign_c_idx];
                    foreign.bank.reserve_balance =
                        round_cents((foreign.bank.reserve_balance - returned).max(0.0));
                    foreign.bank.foreign_deposit_liabilities = round_cents(
                        (foreign.bank.foreign_deposit_liabilities - returned).max(0.0),
                    );
                }
                // Return to person + clear deposit.
                {
                    let home = &mut self.countries[op.home_c_idx];
                    if let Some(person) = home.people.get_mut(op.person_idx) {
                        person.cash_balance = round_cents(person.cash_balance + returned);
                        person.foreign_deposit = None;
                    }
                    home.pending_events.push(SimEvent::ForeignDepositClosed {
                        home_country_name: home.name.clone(),
                        foreign_country_name,
                        person_id,
                        returned_amount: returned,
                        day,
                    });
                }
            }
        }

        // ---------------------------------------------------------------
        // Step 3: Open new foreign deposits.
        // ---------------------------------------------------------------
        struct NewDepositOp {
            home_c_idx: usize,
            person_idx: usize,
            foreign_c_idx: usize,
            foreign_bank_id: BankId,
            amount: f64,
            annual_rate: f64,
        }
        let mut new_ops: Vec<NewDepositOp> = Vec::new();

        for (home_c_idx, country) in self.countries.iter().enumerate() {
            let params = country.sim_params.banking;
            let local_rate = infos[home_c_idx].annual_deposit_rate;

            // Best foreign option for this home country.
            let best_foreign = infos
                .iter()
                .enumerate()
                .filter(|(fi, info)| {
                    *fi != home_c_idx
                        && info.best_bank_id.is_some()
                        && info.annual_deposit_rate
                            > local_rate + params.cross_country_deposit_rate_premium
                })
                .max_by(|(_, a), (_, b)| {
                    a.annual_deposit_rate
                        .partial_cmp(&b.annual_deposit_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            let Some((foreign_c_idx, foreign_info)) = best_foreign else {
                continue;
            };
            let foreign_bank_id = match foreign_info.best_bank_id {
                Some(id) => id,
                None => continue,
            };
            let annual_rate = foreign_info.annual_deposit_rate;

            for (p_idx, person) in country.people.iter().enumerate() {
                // Skip if already has a foreign deposit.
                if person.foreign_deposit.is_some() {
                    continue;
                }
                if person.cash_balance < params.cross_country_deposit_min_balance {
                    continue;
                }
                let surplus = round_cents(
                    person.cash_balance - params.cross_country_deposit_min_balance,
                );
                let amount = round_cents(surplus * params.cross_country_deposit_fraction);
                if amount <= 0.0 {
                    continue;
                }
                new_ops.push(NewDepositOp {
                    home_c_idx,
                    person_idx: p_idx,
                    foreign_c_idx,
                    foreign_bank_id,
                    amount,
                    annual_rate,
                });
            }
        }

        // Apply new deposits.
        for op in new_ops {
            // Debit person's local cash.
            let (person_id, home_country_name, day, _foreign_c_id) = {
                let home = &mut self.countries[op.home_c_idx];
                let Some(person) = home.people.get_mut(op.person_idx) else {
                    continue;
                };
                if person.cash_balance < op.amount {
                    continue;
                }
                person.cash_balance = round_cents(person.cash_balance - op.amount);
                person.foreign_deposit = Some(ForeignDeposit {
                    country_id: infos[op.foreign_c_idx].id,
                    bank_id: op.foreign_bank_id,
                    balance: op.amount,
                });
                (person.id, home.name.clone(), home.day, infos[op.foreign_c_idx].id)
            };

            // Credit foreign bank reserves.
            {
                let foreign = &mut self.countries[op.foreign_c_idx];
                foreign.bank.reserve_balance =
                    round_cents(foreign.bank.reserve_balance + op.amount);
                foreign.bank.foreign_deposit_liabilities =
                    round_cents(foreign.bank.foreign_deposit_liabilities + op.amount);
            }

            // Emit event (attached to home country).
            {
                let foreign_name = infos[op.foreign_c_idx].name.clone();
                let home = &mut self.countries[op.home_c_idx];
                home.pending_events.push(SimEvent::ForeignDepositOpened {
                    home_country_name,
                    foreign_country_name: foreign_name,
                    person_id,
                    bank_id: op.foreign_bank_id,
                    amount: op.amount,
                    annual_rate: op.annual_rate,
                    day,
                });
            }
        }
    }
}
