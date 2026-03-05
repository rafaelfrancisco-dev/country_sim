use super::{
    bank::BankKind,
    country::Country,
    money::round_cents,
};

/// Pre-seed private banks at world generation time, bypassing the formation ratio
/// check (appropriate at gen time). Pulls seed capital from company cash, assigns
/// a fraction of people and companies directly (no reserve transfer needed at bootstrap).
pub(super) fn seed_initial_private_banks(country: &mut Country) {
    let params = country.sim_params.banking;
    if params.initial_private_banks == 0 {
        return;
    }

    let target_people = params.target_people_per_private_bank.max(1);
    let capacity = (country.people.len() + target_people - 1) / target_people;
    let target_banks = (params.initial_private_banks as usize).min(capacity);
    if target_banks == 0 {
        return;
    }

    let seed_capital_needed = round_cents(params.min_private_bank_seed_capital.max(0.0));
    let people_count = country.people.len();
    let company_count = country.companies.len();

    for bank_idx in 0..target_banks {
        // Pull seed capital proportionally from company cash (same as fund_initial_bank_reserve).
        let total_company_cash: f64 = country.companies.iter().map(|c| c.cash_balance).sum();
        let pull_amount = round_cents(seed_capital_needed.min(total_company_cash.max(0.0)));
        if pull_amount <= 0.0 {
            break;
        }
        let company_share = pull_amount / (company_count as f64).max(1.0);
        let mut remaining = pull_amount;
        for (idx, company) in country.companies.iter_mut().enumerate() {
            if remaining <= 0.0 {
                break;
            }
            let planned = if idx + 1 == company_count { remaining } else { round_cents(company_share) };
            let pulled = round_cents(planned.min(company.cash_balance.max(0.0)));
            company.cash_balance = round_cents(company.cash_balance - pulled);
            remaining = round_cents((remaining - pulled).max(0.0));
        }
        let actual_seed = round_cents(pull_amount - remaining);
        if actual_seed <= 0.0 {
            break;
        }

        let Some(bank_id) = country.create_bank(BankKind::Private, actual_seed) else {
            break;
        };

        // Directly assign deposit_bank_ids (gen-time bootstrap; no reserve transfer needed).
        let person_slice = people_count / 3;
        let person_start = bank_idx * person_slice;
        for idx in person_start..(person_start + person_slice).min(people_count) {
            if let Some(person) = country.people.get_mut(idx) {
                person.deposit_bank_id = Some(bank_id);
            }
        }
        let company_slice = (company_count / 4).max(1);
        let company_start = bank_idx * company_slice;
        for idx in company_start..(company_start + company_slice).min(company_count) {
            if let Some(company) = country.companies.get_mut(idx) {
                company.deposit_bank_id = Some(bank_id);
            }
        }

        // Seed job openings for the new private bank.
        let job_types = params.bank_employee_job_types;
        let multiplier = params.bank_wage_multiplier.max(1.0);
        if let Some(new_bank) = country.bank_by_id_mut(bank_id) {
            let target = new_bank.target_employee_count(
                new_bank.deposit_liabilities,
                params.bank_min_employees,
                params.bank_max_employees,
                params.bank_employees_per_deposit_unit,
            );
            for i in 0..target {
                use super::job::salary_for_job;
                let job_id = job_types[i as usize % job_types.len()];
                let salary = round_cents(salary_for_job(job_id) * multiplier);
                new_bank.add_opening_with_salary(job_id, 1, salary);
            }
        }
    }

    country.recompute_bank_deposit_liabilities();
    country.recompute_aggregates();
}
