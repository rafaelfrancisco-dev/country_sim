use std::sync::Arc;

use super::World;
use crate::sim::{bank::BankId, event::SimEvent, money::round_cents, person::ForeignDeposit};

/// Per-country snapshot used during cross-country deposit processing.
struct CountryDepositInfo {
    id: u16,
    name: Arc<str>,
    best_bank_id: Option<BankId>,
    annual_deposit_rate: f64,
    /// Pre-computed monthly compound rates per bank to avoid `powf` in the per-person loop.
    bank_monthly_rates: Vec<(BankId, f64)>,
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

        let infos: Vec<CountryDepositInfo> = self
            .countries
            .iter()
            .map(|country| {
                let best_bank_id = country.best_deposit_bank_id();
                let annual_deposit_rate = best_bank_id
                    .map(|bank_id| country.deposit_rate_for(bank_id))
                    .unwrap_or(0.0);
                let bank_monthly_rates = country
                    .banks
                    .iter()
                    .map(|bank| {
                        let annual = country.deposit_rate_for(bank.id);
                        let monthly = (1.0_f64 + annual).powf(1.0 / 12.0) - 1.0;
                        (bank.id, monthly)
                    })
                    .collect();
                CountryDepositInfo {
                    id: country.id,
                    name: country.name.clone(),
                    best_bank_id,
                    annual_deposit_rate,
                    bank_monthly_rates,
                }
            })
            .collect();

        struct InterestOp {
            home_c_idx: usize,
            person_idx: usize,
            interest: f64,
            foreign_c_idx: usize,
            foreign_bank_id: BankId,
            recall: bool,
        }
        let mut interest_ops = Vec::new();

        for (home_c_idx, country) in self.countries.iter().enumerate() {
            let params = country.sim_params.banking;
            for (person_idx, person) in country.people.iter().enumerate() {
                let Some(fd) = &person.foreign_deposit else {
                    continue;
                };
                let Some(foreign_c_idx) = infos.iter().position(|info| info.id == fd.country_id)
                else {
                    continue;
                };
                let Some(foreign_country) = self.countries.get(foreign_c_idx) else {
                    continue;
                };
                if foreign_country.bank_by_id(fd.bank_id).is_none() {
                    continue;
                }

                let monthly_rate = infos[foreign_c_idx]
                    .bank_monthly_rates
                    .iter()
                    .find(|(bid, _)| *bid == fd.bank_id)
                    .map(|(_, r)| *r)
                    .unwrap_or(0.0);
                let interest = round_cents((fd.balance * monthly_rate).max(0.0));
                let recall = person.cash_balance < params.cross_country_deposit_distress_threshold
                    && fd.balance > 0.0;

                interest_ops.push(InterestOp {
                    home_c_idx,
                    person_idx,
                    interest,
                    foreign_c_idx,
                    foreign_bank_id: fd.bank_id,
                    recall,
                });
            }
        }

        for op in interest_ops {
            if op.interest > 0.0 {
                {
                    let foreign = &mut self.countries[op.foreign_c_idx];
                    let Some(bank) = foreign.bank_by_id_mut(op.foreign_bank_id) else {
                        continue;
                    };
                    bank.reserve_balance =
                        round_cents((bank.reserve_balance - op.interest).max(0.0));
                    bank.foreign_deposit_liabilities =
                        round_cents((bank.foreign_deposit_liabilities - op.interest).max(0.0));
                }

                if let Some(person) = self.countries[op.home_c_idx].people.get_mut(op.person_idx) {
                    person.cash_balance = round_cents(person.cash_balance + op.interest);
                }
            }

            if op.recall {
                let (returned, foreign_country_name, person_id, day) = {
                    let person = match self.countries[op.home_c_idx].people.get(op.person_idx) {
                        Some(person) => person,
                        None => continue,
                    };
                    let fd = match &person.foreign_deposit {
                        Some(fd) => fd,
                        None => continue,
                    };
                    (
                        fd.balance,
                        infos[op.foreign_c_idx].name.clone(),
                        person.id,
                        self.countries[op.home_c_idx].day,
                    )
                };

                {
                    let foreign = &mut self.countries[op.foreign_c_idx];
                    let Some(bank) = foreign.bank_by_id_mut(op.foreign_bank_id) else {
                        continue;
                    };
                    bank.reserve_balance = round_cents((bank.reserve_balance - returned).max(0.0));
                    bank.foreign_deposit_liabilities =
                        round_cents((bank.foreign_deposit_liabilities - returned).max(0.0));
                }

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

        struct NewDepositOp {
            home_c_idx: usize,
            person_idx: usize,
            foreign_c_idx: usize,
            foreign_bank_id: BankId,
            amount: f64,
            annual_rate: f64,
        }
        let mut new_ops = Vec::new();

        for (home_c_idx, country) in self.countries.iter().enumerate() {
            let params = country.sim_params.banking;
            let local_rate = infos[home_c_idx].annual_deposit_rate;

            let best_foreign = infos
                .iter()
                .enumerate()
                .filter(|(foreign_c_idx, info)| {
                    *foreign_c_idx != home_c_idx
                        && info.best_bank_id.is_some()
                        && info.annual_deposit_rate
                            > local_rate + params.cross_country_deposit_rate_premium
                })
                .max_by(|(_, left), (_, right)| {
                    left.annual_deposit_rate
                        .partial_cmp(&right.annual_deposit_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            let Some((foreign_c_idx, foreign_info)) = best_foreign else {
                continue;
            };
            let Some(foreign_bank_id) = foreign_info.best_bank_id else {
                continue;
            };

            for (person_idx, person) in country.people.iter().enumerate() {
                if person.foreign_deposit.is_some() {
                    continue;
                }
                if person.cash_balance < params.cross_country_deposit_min_balance {
                    continue;
                }

                let surplus =
                    round_cents(person.cash_balance - params.cross_country_deposit_min_balance);
                let amount = round_cents(surplus * params.cross_country_deposit_fraction);
                if amount <= 0.0 {
                    continue;
                }

                new_ops.push(NewDepositOp {
                    home_c_idx,
                    person_idx,
                    foreign_c_idx,
                    foreign_bank_id,
                    amount,
                    annual_rate: foreign_info.annual_deposit_rate,
                });
            }
        }

        for op in new_ops {
            if self.countries[op.foreign_c_idx]
                .bank_by_id(op.foreign_bank_id)
                .is_none()
            {
                continue;
            }

            let (person_id, home_country_name, day) = {
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
                (person.id, home.name.clone(), home.day)
            };

            {
                let foreign = &mut self.countries[op.foreign_c_idx];
                let Some(bank) = foreign.bank_by_id_mut(op.foreign_bank_id) else {
                    continue;
                };
                bank.reserve_balance = round_cents(bank.reserve_balance + op.amount);
                bank.foreign_deposit_liabilities =
                    round_cents(bank.foreign_deposit_liabilities + op.amount);
            }

            let foreign_country_name = infos[op.foreign_c_idx].name.clone();
            let home = &mut self.countries[op.home_c_idx];
            home.pending_events.push(SimEvent::ForeignDepositOpened {
                home_country_name,
                foreign_country_name,
                person_id,
                bank_id: op.foreign_bank_id,
                amount: op.amount,
                annual_rate: op.annual_rate,
                day,
            });
        }
    }
}
