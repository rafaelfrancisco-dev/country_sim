use super::*;

impl LaborMarketIndex {
    pub(super) fn rebuild(&mut self, country: &Country) {
        self.company_entries.clear();
        self.company_slots.clear();
        self.bank_entries.clear();
        self.bank_slots.clear();
        for group in &mut self.company_groups {
            group.clear();
        }
        for group in &mut self.bank_groups {
            group.clear();
        }

        for (company_idx, company) in country.companies.iter().enumerate() {
            for (opening_idx, opening) in company.open_positions.iter().enumerate() {
                if opening.slots == 0 {
                    continue;
                }
                let entry_idx = self.company_entries.len();
                self.company_entries.push(CompanyOpeningEntry {
                    company_idx,
                    opening_idx,
                    job_type_id: opening.job_type_id,
                    offered_monthly_salary: opening.offered_monthly_salary,
                });
                self.company_slots.push(opening.slots);
                self.add_company_entry_to_groups(entry_idx, opening.job_type_id);
            }
        }

        for (bank_snapshot_idx, bank) in std::iter::once(country.primary_bank())
            .chain(country.banks.iter())
            .enumerate()
        {
            for (opening_idx, opening) in bank.open_positions.iter().enumerate() {
                if opening.slots == 0 {
                    continue;
                }
                let entry_idx = self.bank_entries.len();
                self.bank_entries.push(BankOpeningEntry {
                    bank_snapshot_idx,
                    bank_id: bank.id,
                    opening_idx,
                    job_type_id: opening.job_type_id,
                    offered_monthly_salary: opening.offered_monthly_salary,
                });
                self.bank_slots.push(opening.slots);
                self.add_bank_entry_to_groups(entry_idx, opening.job_type_id);
            }
        }
    }

    fn add_company_entry_to_groups(&mut self, entry_idx: usize, job_type_id: JobTypeId) {
        for qualification in qualifications() {
            if job_is_eligible_for_habilitation(job_type_id, qualification) {
                self.company_groups[qualification_bucket(qualification)].push(entry_idx);
            }
        }
    }

    fn add_bank_entry_to_groups(&mut self, entry_idx: usize, job_type_id: JobTypeId) {
        for qualification in qualifications() {
            if job_is_eligible_for_habilitation(job_type_id, qualification) {
                self.bank_groups[qualification_bucket(qualification)].push(entry_idx);
            }
        }
    }

    pub(super) fn select_company_opening(
        &self,
        qualification: HabilitationLevel,
        seed: u64,
    ) -> Option<usize> {
        let mut chosen = None;
        let mut seen_slots = 0_u64;
        for &entry_idx in &self.company_groups[qualification_bucket(qualification)] {
            let slots = u64::from(*self.company_slots.get(entry_idx)?);
            if slots == 0 {
                continue;
            }
            let entry = self.company_entries.get(entry_idx)?;
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed = seed
                ^ (u64::from(entry.company_idx as u32) << 32)
                ^ u64::from(entry.opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some(entry_idx);
            }
        }
        chosen
    }

    pub(super) fn select_bank_opening(
        &self,
        qualification: HabilitationLevel,
        seed: u64,
    ) -> Option<usize> {
        let mut chosen = None;
        let mut seen_slots = 0_u64;
        for &entry_idx in &self.bank_groups[qualification_bucket(qualification)] {
            let slots = u64::from(*self.bank_slots.get(entry_idx)?);
            if slots == 0 {
                continue;
            }
            let entry = self.bank_entries.get(entry_idx)?;
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed = seed
                ^ 0x0ba1_1000_0000_u64
                ^ (u64::from(entry.bank_snapshot_idx as u32) << 32)
                ^ u64::from(entry.opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some(entry_idx);
            }
        }
        chosen
    }

    pub(super) fn consume_company_entry(
        &mut self,
        entry_idx: usize,
    ) -> Option<(usize, JobTypeId, f64)> {
        let slots = self.company_slots.get_mut(entry_idx)?;
        if *slots == 0 {
            return None;
        }
        *slots -= 1;
        let entry = self.company_entries.get(entry_idx)?;
        Some((
            entry.company_idx,
            entry.job_type_id,
            entry.offered_monthly_salary,
        ))
    }

    pub(super) fn consume_bank_entry(
        &mut self,
        entry_idx: usize,
    ) -> Option<(BankId, JobTypeId, f64)> {
        let slots = self.bank_slots.get_mut(entry_idx)?;
        if *slots == 0 {
            return None;
        }
        *slots -= 1;
        let entry = self.bank_entries.get(entry_idx)?;
        Some((
            entry.bank_id,
            entry.job_type_id,
            entry.offered_monthly_salary,
        ))
    }

    pub(super) fn consume_matching_company(
        &mut self,
        qualification: HabilitationLevel,
        seed: u64,
    ) -> Option<(usize, JobTypeId, f64)> {
        let mut chosen = None;
        let mut seen_slots = 0_u64;
        for &entry_idx in &self.company_groups[qualification_bucket(qualification)] {
            let Some(&slots_u16) = self.company_slots.get(entry_idx) else {
                continue;
            };
            if slots_u16 == 0 {
                continue;
            }
            let Some(entry) = self.company_entries.get(entry_idx) else {
                continue;
            };
            let slots = u64::from(slots_u16);
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed = seed
                ^ (u64::from(entry.company_idx as u32) << 32)
                ^ u64::from(entry.opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some(entry_idx);
            }
        }
        self.consume_company_entry(chosen?)
    }

    pub(super) fn consume_matching_bank(
        &mut self,
        qualification: HabilitationLevel,
        seed: u64,
    ) -> Option<(BankId, JobTypeId, f64)> {
        let mut chosen = None;
        let mut seen_slots = 0_u64;
        for &entry_idx in &self.bank_groups[qualification_bucket(qualification)] {
            let Some(&slots_u16) = self.bank_slots.get(entry_idx) else {
                continue;
            };
            if slots_u16 == 0 {
                continue;
            }
            let Some(entry) = self.bank_entries.get(entry_idx) else {
                continue;
            };
            let slots = u64::from(slots_u16);
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed = seed
                ^ 0x0ba1_1000_0000_u64
                ^ (u64::from(entry.bank_snapshot_idx as u32) << 32)
                ^ u64::from(entry.opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some(entry_idx);
            }
        }
        self.consume_bank_entry(chosen?)
    }

    pub(super) fn commit(&self, country: &mut Country) {
        for (entry_idx, entry) in self.company_entries.iter().enumerate() {
            let Some(company) = country.companies.get_mut(entry.company_idx) else {
                continue;
            };
            let Some(opening) = company.open_positions.get_mut(entry.opening_idx) else {
                continue;
            };
            opening.slots = self.company_slots[entry_idx];
        }
        for company in &mut country.companies {
            company.open_positions.retain(|opening| opening.slots > 0);
        }

        for (entry_idx, entry) in self.bank_entries.iter().enumerate() {
            let Some(bank) = country.bank_by_id_mut(entry.bank_id) else {
                continue;
            };
            let Some(opening) = bank.open_positions.get_mut(entry.opening_idx) else {
                continue;
            };
            opening.slots = self.bank_slots[entry_idx];
        }
        for bank in country.all_banks_mut() {
            bank.open_positions.retain(|opening| opening.slots > 0);
        }
    }
}

pub(super) fn apply_company_hire(
    country: &mut Country,
    person_idx: usize,
    company_idx: usize,
    job_type_id: JobTypeId,
    offered_salary: f64,
    day: u64,
) -> bool {
    let Some(person) = country.people.get_mut(person_idx) else {
        return false;
    };
    if matches!(person.employment, EmploymentStatus::Unemployed) {
        person.set_employment_with_salary(company_idx as u16, job_type_id, offered_salary, day);
        country.adjust_company_headcount(company_idx, job_type_id, 1);
        country.refresh_person_runtime_membership(person_idx);
        return true;
    }
    false
}

pub(super) fn apply_bank_hire(
    country: &mut Country,
    person_idx: usize,
    bank_id: BankId,
    job_type_id: JobTypeId,
    offered_salary: f64,
    day: u64,
) -> bool {
    let Some(person) = country.people.get_mut(person_idx) else {
        return false;
    };
    if matches!(person.employment, EmploymentStatus::Unemployed) {
        person.set_bank_employment(bank_id, job_type_id, offered_salary, day);
        country.adjust_bank_headcount(bank_id, 1);
        country.refresh_person_runtime_membership(person_idx);
        return true;
    }
    false
}
