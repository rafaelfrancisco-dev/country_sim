use super::{
    CountryMigrationSnapshot, CountryPlanSnapshot, DestinationOpeningRef, JobMarketSnapshot,
    opening_is_better, opening_salary,
};
use crate::sim::{
    country::Country,
    money::round_cents,
    person::{ClearedEmployment, EmploymentStatus, HabilitationLevel, HousingStatus, Person},
};

fn labor_force_unemployment(country: &Country) -> f64 {
    let mut labor_force = 0_usize;
    let mut unemployed = 0_usize;
    for person in &country.people {
        if !person.is_labor_force_participant() {
            continue;
        }
        labor_force += 1;
        if !person.is_employed() {
            unemployed += 1;
        }
    }
    if labor_force == 0 {
        0.0
    } else {
        unemployed as f64 / labor_force as f64
    }
}

pub(super) fn summarize_country_migration_market(country: &Country) -> CountryMigrationSnapshot {
    let mut work_by_job_type = std::collections::HashMap::new();
    let mut distress_by_habilitation = [CountryPlanSnapshot::default(); 4];

    for company in &country.companies {
        for opening in &company.open_positions {
            if opening.slots == 0 {
                continue;
            }
            let candidate = DestinationOpeningRef::Company {
                company_id: company.id,
                job_type_id: opening.job_type_id,
                offered_salary: opening.offered_monthly_salary,
            };
            add_opening_to_market(
                &mut work_by_job_type,
                &mut distress_by_habilitation,
                opening.job_type_id,
                opening.slots,
                candidate,
            );
        }
    }

    for bank in &country.banks {
        for opening in &bank.open_positions {
            if opening.slots == 0 {
                continue;
            }
            let candidate = DestinationOpeningRef::Bank {
                bank_id: bank.id,
                job_type_id: opening.job_type_id,
                offered_salary: opening.offered_monthly_salary,
            };
            add_opening_to_market(
                &mut work_by_job_type,
                &mut distress_by_habilitation,
                opening.job_type_id,
                opening.slots,
                candidate,
            );
        }
    }

    CountryMigrationSnapshot {
        unemployment_rate: labor_force_unemployment(country),
        distress_by_habilitation,
        work_by_job_type,
    }
}

fn add_opening_to_market(
    work_by_job_type: &mut std::collections::HashMap<u8, JobMarketSnapshot>,
    distress_by_habilitation: &mut [CountryPlanSnapshot; 4],
    job_type_id: u8,
    slots: u16,
    candidate: DestinationOpeningRef,
) {
    let slots = u32::from(slots);
    let job_market = work_by_job_type.entry(job_type_id).or_default();
    job_market.open_slots = job_market.open_slots.saturating_add(slots);
    if opening_is_better(job_market.best_opening, Some(candidate)) {
        job_market.best_opening = Some(candidate);
    }

    for qualification in qualifications() {
        if !crate::sim::job::job_is_eligible_for_habilitation(job_type_id, qualification) {
            continue;
        }
        let snapshot = &mut distress_by_habilitation[qualification_bucket(qualification)];
        snapshot.compatible_opening_slots = snapshot.compatible_opening_slots.saturating_add(slots);
        if opening_is_better(snapshot.best_compatible_opening, Some(candidate)) {
            snapshot.best_compatible_opening = Some(candidate);
        }
    }
}

fn qualification_bucket(qualification: HabilitationLevel) -> usize {
    match qualification {
        HabilitationLevel::NoSchoolYet => 0,
        HabilitationLevel::BasicSchool => 1,
        HabilitationLevel::HighSchool => 2,
        HabilitationLevel::HigherLearning => 3,
    }
}

fn qualifications() -> [HabilitationLevel; 4] {
    [
        HabilitationLevel::NoSchoolYet,
        HabilitationLevel::BasicSchool,
        HabilitationLevel::HighSchool,
        HabilitationLevel::HigherLearning,
    ]
}

pub(super) fn unwind_housing_departure(country: &mut Country, person: &Person) {
    match person.housing {
        HousingStatus::Owner | HousingStatus::Mortgaged { .. } => {
            country.housing_stock_occupied_owned =
                country.housing_stock_occupied_owned.saturating_sub(1);
            country.housing_stock_vacant = country.housing_stock_vacant.saturating_add(1);
        }
        HousingStatus::Renter { .. } => {
            country.housing_stock_occupied_rented =
                country.housing_stock_occupied_rented.saturating_sub(1);
            country.housing_stock_vacant = country.housing_stock_vacant.saturating_add(1);
        }
        HousingStatus::Unhoused => {}
    }
}

pub(super) fn transfer_person_loan_out(country: &mut Country, person: &mut Person) -> Option<f64> {
    let transfer_amount = person
        .loan
        .as_ref()
        .map(|loan| round_cents(loan.created_principal_outstanding.max(0.0)))?;
    if let Some(loan) = &person.loan
        && let Some(bank) = country.bank_by_id_mut(loan.lender_bank_id)
    {
        bank.created_money_outstanding =
            round_cents((bank.created_money_outstanding - transfer_amount).max(0.0));
    }
    Some(transfer_amount)
}

pub(super) fn transfer_person_loan_in(
    country: &mut Country,
    person: &mut Person,
    transfer_amount: Option<f64>,
) {
    let Some(loan) = &mut person.loan else {
        return;
    };
    let lender_bank_id = country
        .preferred_lender_bank_id()
        .or(country.public_bank_id_or_first());
    let Some(lender_bank_id) = lender_bank_id else {
        return;
    };
    let transfer_amount =
        transfer_amount.unwrap_or_else(|| round_cents(loan.created_principal_outstanding.max(0.0)));
    loan.lender_bank_id = lender_bank_id;
    if let Some(bank) = country.bank_by_id_mut(lender_bank_id) {
        bank.created_money_outstanding =
            round_cents(bank.created_money_outstanding + transfer_amount);
    }
}

pub(super) fn backfill_departed_opening(
    country: &mut Country,
    previous: ClearedEmployment,
    offered_salary: f64,
) {
    match previous {
        ClearedEmployment::Company {
            company_id,
            job_type_id,
        } => {
            if let Some(company) = country
                .companies
                .iter_mut()
                .find(|company| company.id == company_id)
            {
                company.add_opening_with_salary(job_type_id, 1, offered_salary);
            }
        }
        ClearedEmployment::Bank {
            bank_id,
            job_type_id,
        } => {
            if let Some(bank) = country.bank_by_id_mut(bank_id) {
                bank.add_opening_with_salary(job_type_id, 1, offered_salary);
            }
        }
    }
}

pub(super) fn reserve_opening(
    country: &mut Country,
    opening: DestinationOpeningRef,
) -> Option<DestinationOpeningRef> {
    match opening {
        DestinationOpeningRef::Company {
            company_id,
            job_type_id,
            offered_salary,
        } => {
            let company = country
                .companies
                .iter_mut()
                .find(|company| company.id == company_id)?;
            let opening_idx = company.open_positions.iter().position(|opening| {
                opening.job_type_id == job_type_id
                    && opening.slots > 0
                    && (opening.offered_monthly_salary - offered_salary).abs() < 0.01
            })?;
            company.consume_opening(opening_idx)?;
            Some(opening)
        }
        DestinationOpeningRef::Bank {
            bank_id,
            job_type_id,
            offered_salary,
        } => {
            let bank = country.bank_by_id_mut(bank_id)?;
            let opening_idx = bank.open_positions.iter().position(|opening| {
                opening.job_type_id == job_type_id
                    && opening.slots > 0
                    && (opening.offered_monthly_salary - offered_salary).abs() < 0.01
            })?;
            bank.consume_opening(opening_idx)?;
            Some(opening)
        }
    }
}

pub(super) fn apply_assignment_to_person(
    person: &mut Person,
    assignment: DestinationOpeningRef,
    current_day: u64,
) {
    match assignment {
        DestinationOpeningRef::Company {
            company_id,
            job_type_id,
            offered_salary,
        } => {
            person.set_employment_with_salary(company_id, job_type_id, offered_salary, current_day)
        }
        DestinationOpeningRef::Bank {
            bank_id,
            job_type_id,
            offered_salary,
        } => person.set_bank_employment(bank_id, job_type_id, offered_salary, current_day),
    }
}

pub(super) fn assignment_job_type_id(assignment: Option<DestinationOpeningRef>) -> u8 {
    match assignment {
        Some(DestinationOpeningRef::Company { job_type_id, .. })
        | Some(DestinationOpeningRef::Bank { job_type_id, .. }) => job_type_id,
        None => 0,
    }
}

pub(super) fn assignment_salary(assignment: Option<DestinationOpeningRef>) -> f64 {
    assignment.map(opening_salary).unwrap_or(0.0)
}

pub(super) fn refresh_country_after_migration(country: &mut Country) {
    for company in &mut country.companies {
        company.employee_count = 0;
        company.manufacturing_employee_count = 0;
    }
    for bank in &mut country.banks {
        bank.employee_count = 0;
    }
    let employment_rows = country
        .people
        .iter()
        .map(|person| person.employment)
        .collect::<Vec<_>>();
    for employment in employment_rows {
        match employment {
            EmploymentStatus::Employed {
                employer_company_id,
                job_type_id,
            } => {
                if let Some(company) = country
                    .companies
                    .iter_mut()
                    .find(|company| company.id == employer_company_id)
                {
                    company.employee_count = company.employee_count.saturating_add(1);
                    if job_type_id == 5 {
                        company.manufacturing_employee_count =
                            company.manufacturing_employee_count.saturating_add(1);
                    }
                }
            }
            EmploymentStatus::EmployedAtBank { bank_id, .. } => {
                if let Some(bank) = country.bank_by_id_mut(bank_id) {
                    bank.employee_count = bank.employee_count.saturating_add(1);
                }
            }
            EmploymentStatus::Unemployed => {}
        }
    }
    country.rebuild_households();
    country.recompute_aggregates();
    country.rebuild_person_runtime();
}
