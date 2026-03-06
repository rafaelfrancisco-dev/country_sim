use super::*;

impl Country {
    pub(crate) fn recompute_bank_headcounts(&mut self) {
        let people = &self.people;
        let banks = &mut self.banks;

        for bank in banks.iter_mut() {
            bank.employee_count = 0;
        }

        for person in people {
            if let Some(bank_id) = person.employer_bank_id() {
                if let Some(bank) = banks.iter_mut().find(|bank| bank.id == bank_id) {
                    bank.employee_count = bank.employee_count.saturating_add(1);
                }
            }
        }
    }

    pub(crate) fn update_bank_openings(&mut self) {
        let params = self.sim_params.banking;
        let job_types = params.bank_employee_job_types;
        let multiplier = params.bank_wage_multiplier.max(1.0);

        let bank_ids: Vec<BankId> = self.banks.iter().map(|bank| bank.id).collect();
        for bank_id in bank_ids {
            if let Some(bank) = self.bank_by_id_mut(bank_id) {
                let target = bank.target_employee_count(
                    bank.deposit_liabilities,
                    params.bank_min_employees,
                    params.bank_max_employees,
                    params.bank_employees_per_deposit_unit,
                );
                let current = bank.employee_count + bank.open_position_count();
                let gap = target.saturating_sub(current);
                for i in 0..gap {
                    let job_id = job_types[i as usize % job_types.len()];
                    let salary = round_cents(salary_for_job(job_id) * multiplier);
                    bank.add_opening_with_salary(job_id, 1, salary);
                }
            }
        }
    }
}
