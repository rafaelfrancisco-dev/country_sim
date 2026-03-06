use super::{bank::BankKind, country::Country, money::round_cents};

/// Pre-seed private banks at world generation time, bypassing the formation ratio
/// check (appropriate at gen time). Pulls seed capital from company cash, assigns
/// a bounded fraction of people/companies and moves matching reserves from public bank.
pub(super) fn seed_initial_private_banks(country: &mut Country) {
    let params = country.sim_params().banking;
    if params.initial_private_banks == 0 {
        return;
    }

    let target_people = params.target_people_per_private_bank.max(1);
    let capacity = country.people().len().div_ceil(target_people);
    let target_banks = (params.initial_private_banks as usize).min(capacity);
    if target_banks == 0 {
        return;
    }

    let seed_capital_needed = round_cents(params.min_private_bank_seed_capital.max(0.0));
    let people_count = country.people().len();
    let company_count = country.companies().len();
    let public_id = country
        .public_bank_id_or_first()
        .unwrap_or(country.primary_bank().id);
    let people_stride = (people_count / target_banks.max(1)).max(1);
    let company_stride = (company_count / target_banks.max(1)).max(1);

    for bank_idx in 0..target_banks {
        // Pull seed capital proportionally from company cash (same as fund_initial_bank_reserve).
        let total_company_cash: f64 = country.companies().iter().map(|c| c.cash_balance).sum();
        let pull_amount = round_cents(seed_capital_needed.min(total_company_cash.max(0.0)));
        if pull_amount <= 0.0 {
            break;
        }
        let company_share = pull_amount / (company_count as f64).max(1.0);
        let mut remaining = pull_amount;
        for (idx, company) in country.companies_mut().iter_mut().enumerate() {
            if remaining <= 0.0 {
                break;
            }
            let planned = if idx + 1 == company_count {
                remaining
            } else {
                round_cents(company_share)
            };
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

        // Bootstrap only up to a deposits budget implied by seed capitalization.
        // This avoids structurally insolvent private banks at t=0.
        let min_seed_ratio = params.min_private_seed_to_deposits_ratio.max(0.0);
        let max_bootstrap_deposits = if min_seed_ratio <= f64::EPSILON {
            f64::MAX
        } else {
            round_cents(actual_seed / min_seed_ratio)
        };
        let people_budget = round_cents(max_bootstrap_deposits * 0.85);
        let companies_budget = round_cents(max_bootstrap_deposits * 0.15);

        let mut people_migrated = 0.0_f64;
        let person_start = bank_idx * people_stride;
        for offset in 0..people_count {
            if people_migrated >= people_budget {
                break;
            }
            let idx = (person_start + offset) % people_count.max(1);
            let Some(person) = country.people().get(idx) else {
                continue;
            };
            if person.deposit_bank_id != Some(public_id) {
                continue;
            }
            let depositor_cash = person.cash_balance.max(0.0);
            let projected = round_cents(people_migrated + depositor_cash);
            if projected > people_budget {
                continue;
            }
            let moved = country.transfer_reserve_between_banks(public_id, bank_id, depositor_cash);
            if moved + 0.01 < depositor_cash {
                if moved > 0.0 {
                    country.transfer_reserve_between_banks(bank_id, public_id, moved);
                }
                break;
            }
            if let Some(person) = country.people_mut().get_mut(idx) {
                person.deposit_bank_id = Some(bank_id);
            }
            people_migrated = projected;
        }

        let mut companies_migrated = 0.0_f64;
        let company_start = bank_idx * company_stride;
        for offset in 0..company_count {
            if companies_migrated >= companies_budget {
                break;
            }
            let idx = (company_start + offset) % company_count.max(1);
            let Some(company) = country.companies().get(idx) else {
                continue;
            };
            if company.deposit_bank_id != Some(public_id) {
                continue;
            }
            let depositor_cash = company.cash_balance.max(0.0);
            let projected = round_cents(companies_migrated + depositor_cash);
            if projected > companies_budget {
                continue;
            }
            let moved = country.transfer_reserve_between_banks(public_id, bank_id, depositor_cash);
            if moved + 0.01 < depositor_cash {
                if moved > 0.0 {
                    country.transfer_reserve_between_banks(bank_id, public_id, moved);
                }
                break;
            }
            if let Some(company) = country.companies_mut().get_mut(idx) {
                company.deposit_bank_id = Some(bank_id);
            }
            companies_migrated = projected;
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
