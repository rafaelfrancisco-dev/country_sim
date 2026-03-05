use super::*;

impl Country {
    pub fn ensure_entity_deposit_assignments(&mut self) {
        let fallback_bank = self.public_bank_id_or_first();
        for person in &mut self.people {
            if person.deposit_bank_id.is_none() {
                person.deposit_bank_id = fallback_bank;
            }
        }
        for company in &mut self.companies {
            if company.deposit_bank_id.is_none() {
                company.deposit_bank_id = fallback_bank;
            }
        }
        if self.treasury_bank_id.is_none() {
            self.treasury_bank_id = fallback_bank;
        }
        self.recompute_bank_deposit_liabilities();
    }

    pub(crate) fn transfer_reserve_between_banks(
        &mut self,
        from_bank_id: BankId,
        to_bank_id: BankId,
        amount: f64,
    ) -> f64 {
        if from_bank_id == to_bank_id {
            return 0.0;
        }
        if self.bank_by_id(from_bank_id).is_none() || self.bank_by_id(to_bank_id).is_none() {
            return 0.0;
        }
        let amount = round_cents(amount.max(0.0));
        if amount <= 0.0 {
            return 0.0;
        }
        let source_is_public = self
            .bank_by_id(from_bank_id)
            .map(|bank| bank.kind == BankKind::Public)
            .unwrap_or(false);
        let available = self
            .bank_by_id(from_bank_id)
            .map(|bank| bank.reserve_balance.max(0.0))
            .unwrap_or(0.0);
        if source_is_public && available + 0.01 < amount {
            let support = round_cents((amount - available).max(0.0));
            if let Some(source_bank) = self.bank_by_id_mut(from_bank_id) {
                source_bank.reserve_balance = round_cents(source_bank.reserve_balance + support);
                source_bank.created_money_outstanding =
                    round_cents(source_bank.created_money_outstanding + support);
                source_bank.total_originated = round_cents(source_bank.total_originated + support);
            }
        }
        let available = self
            .bank_by_id(from_bank_id)
            .map(|bank| bank.reserve_balance.max(0.0))
            .unwrap_or(0.0);
        let moved = round_cents(amount.min(available));
        if moved <= 0.0 {
            return 0.0;
        }
        if let Some(from_bank) = self.bank_by_id_mut(from_bank_id) {
            from_bank.reserve_balance = round_cents((from_bank.reserve_balance - moved).max(0.0));
        }
        if let Some(to_bank) = self.bank_by_id_mut(to_bank_id) {
            to_bank.reserve_balance = round_cents(to_bank.reserve_balance + moved);
        }
        moved
    }

    pub(crate) fn move_person_deposit_bank(
        &mut self,
        person_index: usize,
        target_bank_id: BankId,
    ) -> bool {
        let Some(person) = self.people.get(person_index) else {
            return false;
        };
        let source_bank_id = person.deposit_bank_id;
        let cash_balance = person.cash_balance.max(0.0);
        if source_bank_id == Some(target_bank_id) {
            return true;
        }
        if let Some(source_bank_id) = source_bank_id {
            let moved =
                self.transfer_reserve_between_banks(source_bank_id, target_bank_id, cash_balance);
            if moved + 0.01 < cash_balance {
                return false;
            }
        }
        if let Some(person) = self.people.get_mut(person_index) {
            person.deposit_bank_id = Some(target_bank_id);
        }
        true
    }

    pub(crate) fn move_company_deposit_bank(
        &mut self,
        company_index: usize,
        target_bank_id: BankId,
    ) -> bool {
        let Some(company) = self.companies.get(company_index) else {
            return false;
        };
        let source_bank_id = company.deposit_bank_id;
        let cash_balance = company.cash_balance.max(0.0);
        if source_bank_id == Some(target_bank_id) {
            return true;
        }
        if let Some(source_bank_id) = source_bank_id {
            let moved =
                self.transfer_reserve_between_banks(source_bank_id, target_bank_id, cash_balance);
            if moved + 0.01 < cash_balance {
                return false;
            }
        }
        if let Some(company) = self.companies.get_mut(company_index) {
            company.deposit_bank_id = Some(target_bank_id);
        }
        true
    }

    pub(crate) fn move_treasury_deposit_bank(&mut self, target_bank_id: BankId) -> bool {
        if self.treasury_bank_id == Some(target_bank_id) {
            return true;
        }
        if let Some(source_bank_id) = self.treasury_bank_id {
            let moved = self.transfer_reserve_between_banks(
                source_bank_id,
                target_bank_id,
                self.treasury_balance,
            );
            if moved + 0.01 < self.treasury_balance.max(0.0) {
                return false;
            }
        }
        self.treasury_bank_id = Some(target_bank_id);
        true
    }

    pub(crate) fn healthiest_acquirer_bank_id(&self, failed_bank_id: BankId) -> Option<BankId> {
        std::iter::once(&self.bank)
            .chain(self.banks.iter())
            .filter(|bank| bank.id != failed_bank_id && !bank.is_in_resolution)
            .max_by(|a, b| {
                self.bank_health_score(a.id)
                    .partial_cmp(&self.bank_health_score(b.id))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bank| bank.id)
            .or_else(|| {
                if self.bank.id != failed_bank_id {
                    Some(self.bank.id)
                } else {
                    None
                }
            })
    }

    pub(crate) fn close_private_bank_terminally(&mut self, failed_bank_id: BankId) -> bool {
        let failed_idx = match self
            .banks
            .iter()
            .position(|bank| bank.id == failed_bank_id && bank.kind == BankKind::Private)
        {
            Some(idx) => idx,
            None => return false,
        };
        let Some(acquirer_bank_id) = self.healthiest_acquirer_bank_id(failed_bank_id) else {
            return false;
        };

        let failed_bank_name = self.banks[failed_idx].name.clone();
        let migrated_depositors = self
            .people
            .iter()
            .filter(|person| person.deposit_bank_id == Some(failed_bank_id))
            .count() as u32;
        let migrated_companies = self
            .companies
            .iter()
            .filter(|company| company.deposit_bank_id == Some(failed_bank_id))
            .count() as u32;

        let person_indexes: Vec<usize> = self
            .people
            .iter()
            .enumerate()
            .filter_map(|(idx, person)| {
                (person.deposit_bank_id == Some(failed_bank_id)).then_some(idx)
            })
            .collect();
        for idx in person_indexes {
            if let Some(person) = self.people.get_mut(idx) {
                person.deposit_bank_id = Some(acquirer_bank_id);
            }
        }

        let company_indexes: Vec<usize> = self
            .companies
            .iter()
            .enumerate()
            .filter_map(|(idx, company)| {
                (company.deposit_bank_id == Some(failed_bank_id)).then_some(idx)
            })
            .collect();
        for idx in company_indexes {
            if let Some(company) = self.companies.get_mut(idx) {
                company.deposit_bank_id = Some(acquirer_bank_id);
            }
        }

        if self.treasury_bank_id == Some(failed_bank_id) {
            self.treasury_bank_id = Some(acquirer_bank_id);
        }

        let mut migrated_loans = 0_u32;
        for person in &mut self.people {
            if let Some(loan) = person.loan.as_mut()
                && loan.lender_bank_id == failed_bank_id
            {
                loan.lender_bank_id = acquirer_bank_id;
                migrated_loans = migrated_loans.saturating_add(1);
            }
        }
        for company in &mut self.companies {
            if let Some(loan) = company.loan.as_mut()
                && loan.lender_bank_id == failed_bank_id
            {
                loan.lender_bank_id = acquirer_bank_id;
                migrated_loans = migrated_loans.saturating_add(1);
            }
        }

        let failed_created_money = self.banks[failed_idx].created_money_outstanding;
        let failed_originated = self.banks[failed_idx].total_originated;
        let failed_principal_repaid = self.banks[failed_idx].total_principal_repaid;
        let failed_equity_capital = self.banks[failed_idx].equity_capital;
        let failed_foreign_deposits = self.banks[failed_idx].foreign_deposit_liabilities;
        let failed_reserve_before_transfer = self.banks[failed_idx].reserve_balance.max(0.0);
        let transferred_reserves = self.transfer_reserve_between_banks(
            failed_bank_id,
            acquirer_bank_id,
            failed_reserve_before_transfer,
        );

        if let Some(acquirer_bank) = self.bank_by_id_mut(acquirer_bank_id) {
            acquirer_bank.created_money_outstanding = round_cents(
                acquirer_bank.created_money_outstanding + failed_created_money.max(0.0),
            );
            acquirer_bank.total_originated =
                round_cents(acquirer_bank.total_originated + failed_originated.max(0.0));
            acquirer_bank.total_principal_repaid = round_cents(
                acquirer_bank.total_principal_repaid + failed_principal_repaid.max(0.0),
            );
            acquirer_bank.equity_capital =
                round_cents(acquirer_bank.equity_capital + failed_equity_capital);
            acquirer_bank.foreign_deposit_liabilities = round_cents(
                acquirer_bank.foreign_deposit_liabilities + failed_foreign_deposits.max(0.0),
            );
        }

        for person in &mut self.people {
            if person.employer_bank_id() == Some(failed_bank_id) {
                person.clear_employment();
            }
        }

        self.banks.retain(|bank| bank.id != failed_bank_id);
        self.recompute_bank_deposit_liabilities();
        self.recompute_bank_headcounts();
        self.monthly_private_banks_closed = self.monthly_private_banks_closed.saturating_add(1);
        self.cumulative_private_banks_closed =
            self.cumulative_private_banks_closed.saturating_add(1);

        let acquired_by_bank_name = self
            .bank_by_id(acquirer_bank_id)
            .map(|bank| bank.name.clone())
            .unwrap_or_else(|| format!("Bank #{acquirer_bank_id}"));
        self.pending_events.push(SimEvent::BankClosed {
            country_name: self.name.clone(),
            bank_id: failed_bank_id,
            bank_name: failed_bank_name,
            acquired_by_bank_id: acquirer_bank_id,
            acquired_by_bank_name,
            migrated_depositors,
            migrated_companies,
            migrated_loans,
            transferred_reserves,
            day: self.day,
        });
        true
    }

    pub fn recompute_bank_deposit_liabilities(&mut self) {
        let mut liabilities_by_bank: std::collections::HashMap<BankId, f64> =
            std::collections::HashMap::new();
        for person in &self.people {
            let Some(bank_id) = person.deposit_bank_id else {
                continue;
            };
            let entry = liabilities_by_bank.entry(bank_id).or_insert(0.0);
            *entry = round_cents(*entry + person.cash_balance.max(0.0));
        }
        for company in &self.companies {
            let Some(bank_id) = company.deposit_bank_id else {
                continue;
            };
            let entry = liabilities_by_bank.entry(bank_id).or_insert(0.0);
            *entry = round_cents(*entry + company.cash_balance.max(0.0));
        }
        if let Some(bank_id) = self.treasury_bank_id {
            let entry = liabilities_by_bank.entry(bank_id).or_insert(0.0);
            *entry = round_cents(*entry + self.treasury_balance.max(0.0));
        }

        self.bank.set_deposit_liabilities(
            liabilities_by_bank
                .remove(&self.bank.id)
                .unwrap_or_default()
                .max(0.0),
        );
        for bank in &mut self.banks {
            let amount = liabilities_by_bank.remove(&bank.id).unwrap_or_default();
            bank.set_deposit_liabilities(amount.max(0.0));
        }
    }

    pub fn create_bank(&mut self, kind: BankKind, seed_capital: f64) -> Option<BankId> {
        let mut bank_id = self.next_bank_id.max(1);
        loop {
            let occupied =
                bank_id == self.bank.id || self.banks.iter().any(|bank| bank.id == bank_id);
            if !occupied {
                break;
            }
            bank_id = bank_id.checked_add(1)?;
        }
        self.next_bank_id = bank_id.saturating_add(1);
        let serial = self.next_bank_serial;
        self.next_bank_serial = self.next_bank_serial.saturating_add(1);
        let kind_label = match kind {
            BankKind::Public => "Public",
            BankKind::Private => "Private",
        };
        let mut bank = Bank::with_identity(
            bank_id,
            format!("{} {} Bank {}", self.name, kind_label, serial),
            kind,
        );
        bank.reserve_balance = round_cents(seed_capital.max(0.0));
        bank.equity_capital = round_cents(seed_capital.max(0.0));
        self.banks.push(bank);
        if kind == BankKind::Public {
            self.public_bank_id = Some(bank_id);
        } else {
            self.monthly_private_banks_founded =
                self.monthly_private_banks_founded.saturating_add(1);
            self.cumulative_private_banks_founded =
                self.cumulative_private_banks_founded.saturating_add(1);
        }
        Some(bank_id)
    }
}
