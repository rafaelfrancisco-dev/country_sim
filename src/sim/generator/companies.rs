use super::*;
use crate::sim::{
    company::minimum_viable_unit_price,
    good::{industry_job_types_for_good, production_job_for_good},
};

pub(super) fn derive_initial_company_count(
    population_per_country: u32,
    sim_params: &SimulationParams,
) -> u16 {
    let total = target_initial_company_counts_with_params(population_per_country, sim_params)
        .iter()
        .copied()
        .map(u32::from)
        .sum::<u32>();
    total
        .clamp(15, u32::from(u16::MAX))
        .try_into()
        .unwrap_or(u16::MAX)
}

#[cfg(test)]
pub(super) fn target_initial_company_counts(population_per_country: u32) -> [u16; GOODS_COUNT] {
    target_initial_company_counts_with_params(population_per_country, &SimulationParams::default())
}

fn target_initial_company_counts_with_params(
    population_per_country: u32,
    sim_params: &SimulationParams,
) -> [u16; GOODS_COUNT] {
    let pop_scale = (population_per_country as f64 / 1_000.0).max(1.0);
    let market_depth = pop_scale.sqrt();
    std::array::from_fn(|good_idx| {
        let good = common_goods()[good_idx];
        if STOCKABLE_ESSENTIAL_GOOD_IDS.contains(&good.id) {
            let target_units = estimated_country_monthly_requirement(
                &sim_params.essentials,
                good.id,
                population_per_country as usize,
            )
            .max(estimated_country_stock_target_units(
                &sim_params.essentials,
                good.id,
                population_per_country as usize,
            ));
            let per_company_capacity = production_capacity_units_for_workers(
                &sim_params.essentials,
                sim_params.goods_market.worker_effort_per_month,
                good.effort_per_unit,
                u32::from(
                    sim_params
                        .essentials
                        .target_manufacturing_workers_per_initial_company,
                ),
            );
            let firms = if per_company_capacity > 0.0 {
                (target_units / per_company_capacity).ceil() as u16
            } else {
                1
            };
            firms.clamp(1, 48)
        } else {
            let labor_load = good.monthly_demand_factor * good.effort_per_unit;
            let firms = ((market_depth * labor_load) / 12.0).ceil() as u16;
            firms.clamp(1, 8)
        }
    })
}

pub(super) fn build_initial_companies(
    country_name: &str,
    population_per_country: u32,
    company_count: u16,
    sim_params: &SimulationParams,
) -> Vec<Company> {
    let company_count = usize::from(company_count);
    if company_count == 0 {
        return Vec::new();
    }

    let target_counts =
        target_initial_company_counts_with_params(population_per_country, sim_params);
    let desired_total = target_counts
        .iter()
        .copied()
        .map(usize::from)
        .sum::<usize>();
    let company_goods = if company_count == desired_total {
        let mut goods = Vec::with_capacity(company_count);
        for (good_idx, count) in target_counts.iter().copied().enumerate() {
            for _ in 0..count {
                goods.push(common_goods()[good_idx].id);
            }
        }
        goods
    } else {
        allocate_company_goods_dhondt(company_count, &target_counts)
    };

    company_goods
        .into_iter()
        .enumerate()
        .map(|(idx, good_id)| {
            Company::new(
                idx as u16,
                format!("{country_name} Co {}", idx + 1),
                industry_job_types_for_good(good_id).to_vec(),
                good_id,
            )
        })
        .collect()
}

fn allocate_company_goods_dhondt(company_count: usize, target_counts: &[u16]) -> Vec<u8> {
    let mut allocations = vec![0_u16; target_counts.len()];
    for _ in 0..company_count {
        let mut best_idx = 0usize;
        let mut best_score = f64::NEG_INFINITY;
        for (good_idx, &target) in target_counts.iter().enumerate() {
            let score = f64::from(target) / f64::from(allocations[good_idx].saturating_add(1));
            if score > best_score {
                best_score = score;
                best_idx = good_idx;
            }
        }
        allocations[best_idx] = allocations[best_idx].saturating_add(1);
    }

    let mut goods = Vec::with_capacity(company_count);
    for (good_idx, count) in allocations.into_iter().enumerate() {
        for _ in 0..count {
            goods.push(common_goods()[good_idx].id);
        }
    }
    goods
}

pub(super) fn rebalance_essential_sector_workforce(
    people: &mut [Person],
    companies: &[Company],
    sim_params: &SimulationParams,
) {
    let mut employed_candidates = Vec::new();
    let mut unemployed_candidates = Vec::new();
    for (person_idx, person) in people.iter().enumerate() {
        if person.employer_bank_id().is_some() {
            continue;
        }

        if person.is_employed() {
            employed_candidates.push(person_idx);
        } else {
            unemployed_candidates.push(person_idx);
        }
    }
    employed_candidates.extend(unemployed_candidates);

    let mut candidate_cursor = 0usize;
    for good_id in STOCKABLE_ESSENTIAL_GOOD_IDS {
        let company_ids = companies
            .iter()
            .filter(|company| company.produced_good_id == good_id)
            .map(|company| company.id)
            .collect::<Vec<_>>();
        if company_ids.is_empty() {
            continue;
        }

        let Some(good) = good_by_id(good_id) else {
            continue;
        };
        let Some(production_job_id) = production_job_for_good(good_id) else {
            continue;
        };
        let target_units =
            estimated_country_monthly_requirement(&sim_params.essentials, good_id, people.len())
                .max(estimated_country_stock_target_units(
                    &sim_params.essentials,
                    good_id,
                    people.len(),
                ));
        let target_workers = target_manufacturing_headcount_for_units(
            &sim_params.essentials,
            sim_params.goods_market.worker_effort_per_month,
            good.effort_per_unit,
            target_units,
        ) as usize;
        if target_workers == 0 {
            continue;
        }

        for worker_offset in 0..target_workers {
            let mut selected_person_idx = None;
            while let Some(&person_idx) = employed_candidates.get(candidate_cursor) {
                candidate_cursor = candidate_cursor.saturating_add(1);
                if people.get(person_idx).is_some_and(|person| {
                    job_is_eligible_for_habilitation(production_job_id, person.habilitation)
                }) {
                    selected_person_idx = Some(person_idx);
                    break;
                }
            }
            let Some(person_idx) = selected_person_idx else {
                return;
            };
            let company_id = company_ids[worker_offset % company_ids.len()];
            people[person_idx].set_employment(company_id, production_job_id, 0);
        }
    }
}

pub(super) fn seed_initial_company_openings(country: &mut Country) {
    for company in country.companies_mut() {
        if company.employee_count > 0 || !company.open_positions.is_empty() {
            continue;
        }
        let job_types = industry_job_types_for_good(company.produced_good_id);
        if job_types.is_empty() {
            continue;
        }
        if let Some(production_job_id) = production_job_for_good(company.produced_good_id) {
            company.add_opening_with_salary(
                production_job_id,
                1,
                salary_for_job(production_job_id),
            );
        }

        if let Some(&support_job_id) = job_types
            .iter()
            .find(|&&job_id| Some(job_id) != production_job_for_good(company.produced_good_id))
        {
            company.add_opening_with_salary(support_job_id, 1, salary_for_job(support_job_id));
        }
    }
}

pub(super) fn seed_initial_essential_supply(country: &mut Country) {
    let essentials = country.sim_params().essentials;
    let worker_effort = country.sim_params().goods_market.worker_effort_per_month;

    for good_id in STOCKABLE_ESSENTIAL_GOOD_IDS {
        let company_indices = country
            .companies()
            .iter()
            .enumerate()
            .filter(|(_, company)| company.produced_good_id == good_id)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        if company_indices.is_empty() {
            continue;
        }

        let Some(good) = good_by_id(good_id) else {
            continue;
        };
        let Some(production_job_id) = production_job_for_good(good_id) else {
            continue;
        };

        let target_units = country.estimated_requirement_units_for_good(good_id).max(
            estimated_country_stock_target_units(&essentials, good_id, country.people().len()),
        );
        let target_workers = target_manufacturing_headcount_for_units(
            &essentials,
            worker_effort,
            good.effort_per_unit,
            target_units,
        ) as usize;
        let per_company_target_workers = target_workers.div_ceil(company_indices.len());
        let total_inventory_units =
            (target_units * f64::from(essentials.initial_inventory_cover_months)).ceil() as u32;
        let per_company_inventory = total_inventory_units.div_ceil(company_indices.len() as u32);

        for company_idx in company_indices {
            let current_workers =
                country.companies()[company_idx].manufacturing_employee_count as usize;
            if let Some(company) = country.companies_mut().get_mut(company_idx) {
                company.inventory_units = company
                    .inventory_units
                    .saturating_add(per_company_inventory);
                if per_company_target_workers > current_workers {
                    let missing_workers = per_company_target_workers - current_workers;
                    company.add_opening_with_salary(
                        production_job_id,
                        missing_workers.min(u16::MAX as usize) as u16,
                        salary_for_job(production_job_id),
                    );
                }
            }
        }
    }
}

pub(super) fn randomize_company_prices(
    companies: &mut [Company],
    sim_params: &SimulationParams,
    rng: &mut DeterministicRng,
) {
    for company in companies {
        if good_by_id(company.produced_good_id).is_none() {
            continue;
        }

        let min_price = minimum_viable_unit_price(
            company.produced_good_id,
            sim_params.goods_market.worker_effort_per_month,
            sim_params.goods_market.min_margin_rate,
        );
        let price_factor = 0.85 + 0.30 * rng.next_f64();
        let candidate = company.unit_price * price_factor;
        company.unit_price = round_cents(candidate.max(min_price));
    }
}

pub(super) fn fund_initial_bank_reserve(country: &mut Country) {
    let target = round_cents(
        country
            .sim_params()
            .stock_market
            .maker_bootstrap_reserve
            .max(0.0),
    );
    if target <= 0.0 || country.companies().is_empty() {
        return;
    }

    let total_company_cash: f64 = country.companies().iter().map(|c| c.cash_balance).sum();
    let transfer_total = round_cents(target.min(total_company_cash.max(0.0)));
    if transfer_total <= 0.0 {
        return;
    }

    let mut remaining = transfer_total;
    let company_count = country.companies().len() as f64;
    let companies_len = country.companies().len();
    for (idx, company) in country.companies_mut().iter_mut().enumerate() {
        if remaining <= 0.0 {
            break;
        }
        let planned = if idx + 1 == companies_len {
            remaining
        } else {
            round_cents(transfer_total / company_count)
        };
        let pulled = round_cents(planned.min(company.cash_balance.max(0.0)));
        company.cash_balance = round_cents(company.cash_balance - pulled);
        remaining = round_cents((remaining - pulled).max(0.0));
    }
    let actual_transfer = round_cents(transfer_total - remaining);
    let primary_bank = country.primary_bank_mut();
    primary_bank.reserve_balance = round_cents(primary_bank.reserve_balance + actual_transfer);
    country.recompute_aggregates();
}

pub(super) fn seed_initial_bank_openings(country: &mut Country) {
    let params = country.sim_params().banking;
    let job_types = params.bank_employee_job_types;
    let multiplier = params.bank_wage_multiplier.max(1.0);

    let primary_bank = country.primary_bank_mut();
    let target = primary_bank.target_employee_count(
        params.bank_min_employees,
        params.bank_max_employees,
        params.bank_employees_per_deposit_unit,
    );
    for i in 0..target {
        let job_id = job_types[i as usize % job_types.len()];
        let salary = round_cents(salary_for_job(job_id) * multiplier);
        primary_bank.add_opening_with_salary(job_id, 1, salary);
    }
}
