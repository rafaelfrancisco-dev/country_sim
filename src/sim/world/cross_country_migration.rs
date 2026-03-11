use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::World;
use crate::sim::{
    bank::BankId,
    event::SimEvent,
    person::{HousingStatus, Person},
};

mod helpers;
use helpers::{
    apply_assignment_to_person, assignment_job_type_id, assignment_salary,
    backfill_departed_opening, refresh_country_after_migration, reserve_opening,
    summarize_country_migration_market, transfer_person_loan_in, transfer_person_loan_out,
    unwind_housing_departure,
};

#[derive(Debug, Copy, Clone)]
enum DestinationOpeningRef {
    Company {
        company_id: u16,
        job_type_id: u8,
        offered_salary: f64,
    },
    Bank {
        bank_id: BankId,
        job_type_id: u8,
        offered_salary: f64,
    },
}

#[derive(Debug, Copy, Clone)]
enum MigrationKind {
    Work {
        opening: DestinationOpeningRef,
    },
    Distress {
        opening: Option<DestinationOpeningRef>,
    },
}

#[derive(Debug, Copy, Clone)]
struct PlannedMigration {
    source_country_idx: usize,
    dest_country_idx: usize,
    person_id: u32,
    kind: MigrationKind,
}

#[derive(Debug, Copy, Clone, Default)]
struct CountryPlanSnapshot {
    unemployment_rate: f64,
    compatible_opening_slots: u32,
    best_compatible_opening: Option<DestinationOpeningRef>,
}

#[derive(Debug, Copy, Clone, Default)]
struct JobMarketSnapshot {
    open_slots: u32,
    best_opening: Option<DestinationOpeningRef>,
}

#[derive(Debug, Clone)]
struct CountryMigrationSnapshot {
    unemployment_rate: f64,
    distress_by_habilitation: [CountryPlanSnapshot; 4],
    work_by_job_type: HashMap<u8, JobMarketSnapshot>,
}

impl World {
    pub fn process_cross_country_migration(&mut self) {
        if self.countries.len() < 2 {
            return;
        }

        let migration_params = self
            .countries
            .first()
            .map(|country| country.sim_params().migration)
            .unwrap_or_default();
        if !migration_params.enabled {
            return;
        }

        let country_snapshots = self.build_country_migration_snapshots();
        let mut plans = Vec::new();
        for (source_country_idx, source_country) in self.countries.iter().enumerate() {
            for person in &source_country.people {
                if !person.is_labor_force_participant() {
                    continue;
                }

                let cooldown_days = u64::from(migration_params.migration_cooldown_months)
                    * crate::sim::country::DAYS_PER_MONTH;
                if person
                    .last_migration_day
                    .is_some_and(|day| source_country.day().saturating_sub(day) < cooldown_days)
                {
                    continue;
                }
                if person.cash_balance < migration_params.cash_buffer_required {
                    continue;
                }

                if let Some(plan) = self.plan_work_migration(
                    source_country_idx,
                    person,
                    &country_snapshots,
                    migration_params.irresistible_offer_salary_premium,
                    migration_params.irresistible_offer_min_open_slots,
                    migration_params.destination_unemployment_guard,
                ) {
                    plans.push(plan);
                    continue;
                }

                if let Some(plan) = self.plan_distress_migration(
                    source_country_idx,
                    person,
                    &country_snapshots,
                    migration_params,
                ) {
                    plans.push(plan);
                }
            }
        }

        let mut touched_countries = HashSet::new();
        for plan in plans {
            if let Some((source_country_idx, dest_country_idx)) = self.apply_planned_migration(plan)
            {
                touched_countries.insert(source_country_idx);
                touched_countries.insert(dest_country_idx);
            }
        }
        for country_idx in touched_countries {
            if let Some(country) = self.countries.get_mut(country_idx) {
                refresh_country_after_migration(country);
            }
        }
    }

    fn build_country_migration_snapshots(&self) -> Vec<CountryMigrationSnapshot> {
        self.countries
            .iter()
            .map(summarize_country_migration_market)
            .collect()
    }

    fn plan_work_migration(
        &self,
        source_country_idx: usize,
        person: &Person,
        country_snapshots: &[CountryMigrationSnapshot],
        salary_premium: f64,
        min_open_slots: u16,
        destination_unemployment_guard: f64,
    ) -> Option<PlannedMigration> {
        let current_salary = person.monthly_salary();
        let current_job_type_id = person.job_type_id()?;
        if current_salary <= 0.0 {
            return None;
        }

        let mut best: Option<(usize, DestinationOpeningRef)> = None;
        for (dest_country_idx, snapshot) in country_snapshots.iter().enumerate() {
            if dest_country_idx == source_country_idx {
                continue;
            }
            if snapshot.unemployment_rate > destination_unemployment_guard {
                continue;
            }

            let Some(job_market) = snapshot.work_by_job_type.get(&current_job_type_id) else {
                continue;
            };
            if job_market.open_slots < u32::from(min_open_slots) {
                continue;
            }
            let Some(opening) = job_market.best_opening else {
                continue;
            };
            if opening_salary(opening) < current_salary * (1.0 + salary_premium.max(0.0)) {
                continue;
            }
            if opening_is_better(best.map(|(_, opening)| opening), Some(opening)) {
                best = Some((dest_country_idx, opening));
            }
        }

        best.map(|(dest_country_idx, opening)| PlannedMigration {
            source_country_idx,
            dest_country_idx,
            person_id: person.id,
            kind: MigrationKind::Work { opening },
        })
    }

    fn plan_distress_migration(
        &self,
        source_country_idx: usize,
        person: &Person,
        country_snapshots: &[CountryMigrationSnapshot],
        params: crate::sim::params::MigrationParams,
    ) -> Option<PlannedMigration> {
        if person.is_employed() {
            return None;
        }

        let benefit_exhausted = self.countries[source_country_idx]
            .sim_params()
            .fiscal
            .max_benefit_months
            .saturating_add(params.benefit_exhaustion_bonus_months);
        let is_distressed = person.months_unemployed >= params.long_term_unemployment_months
            && (person.months_on_benefits >= benefit_exhausted
                || person.consecutive_severe_unmet_months >= params.chronic_unmet_needs_months);
        if !is_distressed {
            return None;
        }

        let roll = deterministic_roll(
            self.generation_seed,
            self.day,
            person.id,
            source_country_idx as u64,
        );
        if roll > params.base_monthly_migration_rate.clamp(0.0, 1.0) {
            return None;
        }

        let mut best: Option<(usize, CountryPlanSnapshot)> = None;
        for (dest_country_idx, country_snapshot) in country_snapshots.iter().enumerate() {
            if dest_country_idx == source_country_idx {
                continue;
            }
            if country_snapshot.unemployment_rate > params.destination_unemployment_guard {
                continue;
            }

            let snapshot =
                country_snapshot.distress_by_habilitation[habilitation_bucket(person.habilitation)];
            if snapshot.compatible_opening_slots == 0 {
                continue;
            }

            let replace = match best {
                None => true,
                Some((_, current)) => {
                    snapshot.compatible_opening_slots > current.compatible_opening_slots
                        || (snapshot.compatible_opening_slots == current.compatible_opening_slots
                            && snapshot.unemployment_rate < current.unemployment_rate)
                        || (snapshot.compatible_opening_slots == current.compatible_opening_slots
                            && (snapshot.unemployment_rate - current.unemployment_rate).abs()
                                < f64::EPSILON
                            && opening_is_better(
                                current.best_compatible_opening,
                                snapshot.best_compatible_opening,
                            ))
                }
            };
            if replace {
                best = Some((dest_country_idx, snapshot));
            }
        }

        best.map(|(dest_country_idx, snapshot)| PlannedMigration {
            source_country_idx,
            dest_country_idx,
            person_id: person.id,
            kind: MigrationKind::Distress {
                opening: snapshot.best_compatible_opening,
            },
        })
    }

    fn apply_planned_migration(&mut self, plan: PlannedMigration) -> Option<(usize, usize)> {
        let source_idx = plan.source_country_idx;
        let dest_idx = plan.dest_country_idx;
        if source_idx == dest_idx
            || source_idx >= self.countries.len()
            || dest_idx >= self.countries.len()
        {
            return None;
        }

        let person_idx = match self.countries[source_idx]
            .people
            .iter()
            .position(|person| person.id == plan.person_id)
        {
            Some(idx) => idx,
            None => return None,
        };

        let work_assignment = match plan.kind {
            MigrationKind::Work { opening } => {
                match reserve_opening(&mut self.countries[dest_idx], opening) {
                    Some(assignment) => Some(assignment),
                    None => return None,
                }
            }
            MigrationKind::Distress { opening } => {
                opening.and_then(|opening| reserve_opening(&mut self.countries[dest_idx], opening))
            }
        };

        let to_country_name = self.countries[dest_idx].name().to_string();
        let (from_country_name, current_day, mut person, source_bank_transfer) = {
            let source_country = &mut self.countries[source_idx];
            let from_country_name = source_country.name().to_string();
            let current_day = source_country.day();
            let mut person = source_country.people.remove(person_idx);
            let source_bank_transfer = transfer_person_loan_out(source_country, &mut person);
            unwind_housing_departure(source_country, &person);
            let previous_salary = person.monthly_salary();
            if let Some(previous) = person.clear_employment() {
                backfill_departed_opening(source_country, previous, previous_salary);
            }
            source_country.monthly_emigrants = source_country.monthly_emigrants.saturating_add(1);
            match plan.kind {
                MigrationKind::Work { .. } => {
                    source_country.monthly_migrants_for_work =
                        source_country.monthly_migrants_for_work.saturating_add(1);
                }
                MigrationKind::Distress { .. } => {
                    source_country.monthly_migrants_for_distress = source_country
                        .monthly_migrants_for_distress
                        .saturating_add(1);
                }
            }
            person.deposit_bank_id = None;
            person.foreign_deposit = None;
            person.housing = HousingStatus::Unhoused;
            (from_country_name, current_day, person, source_bank_transfer)
        };

        {
            let destination_country = &mut self.countries[dest_idx];
            transfer_person_loan_in(destination_country, &mut person, source_bank_transfer);
            let migrated_person_id = assign_destination_person_id(destination_country, &mut person);
            person.current_country_id = destination_country.id();
            person.deposit_bank_id = destination_country.public_bank_id_or_first();
            person.record_migration(destination_country.id(), current_day);
            if let Some(assignment) = work_assignment {
                apply_assignment_to_person(&mut person, assignment, current_day);
            }
            destination_country.people.push(person);
            destination_country.monthly_immigrants =
                destination_country.monthly_immigrants.saturating_add(1);
            match plan.kind {
                MigrationKind::Work { .. } => {
                    destination_country.monthly_migrants_for_work = destination_country
                        .monthly_migrants_for_work
                        .saturating_add(1);
                    destination_country.push_pending_event(SimEvent::PersonMigratedForWork {
                        from_country_name: Arc::from(from_country_name),
                        to_country_name: Arc::from(to_country_name),
                        person_id: migrated_person_id,
                        job_type_id: assignment_job_type_id(work_assignment),
                        monthly_salary: assignment_salary(work_assignment),
                        day: current_day,
                    });
                }
                MigrationKind::Distress { .. } => {
                    destination_country.monthly_migrants_for_distress = destination_country
                        .monthly_migrants_for_distress
                        .saturating_add(1);
                    destination_country.push_pending_event(SimEvent::PersonMigratedForDistress {
                        from_country_name: Arc::from(from_country_name),
                        to_country_name: Arc::from(to_country_name),
                        person_id: migrated_person_id,
                        months_unemployed: destination_country
                            .people
                            .last()
                            .map(|person| person.months_unemployed)
                            .unwrap_or_default(),
                        months_on_benefits: destination_country
                            .people
                            .last()
                            .map(|person| person.months_on_benefits)
                            .unwrap_or_default(),
                        day: current_day,
                    });
                }
            }
        }
        Some((source_idx, dest_idx))
    }
}

fn assign_destination_person_id(
    destination_country: &mut crate::sim::country::Country,
    person: &mut Person,
) -> u32 {
    let person_id_in_use = destination_country
        .people
        .iter()
        .any(|existing| existing.id == person.id);

    if person_id_in_use {
        let next_available_id = destination_country.next_person_id.max(
            destination_country
                .people
                .iter()
                .map(|existing| existing.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        person.id = next_available_id;
    }

    destination_country.next_person_id = destination_country
        .next_person_id
        .max(person.id.saturating_add(1));
    person.id
}

fn habilitation_bucket(qualification: crate::sim::person::HabilitationLevel) -> usize {
    match qualification {
        crate::sim::person::HabilitationLevel::NoSchoolYet => 0,
        crate::sim::person::HabilitationLevel::BasicSchool => 1,
        crate::sim::person::HabilitationLevel::HighSchool => 2,
        crate::sim::person::HabilitationLevel::HigherLearning => 3,
    }
}

fn opening_is_better(
    current: Option<DestinationOpeningRef>,
    candidate: Option<DestinationOpeningRef>,
) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    let candidate_salary = opening_salary(candidate);
    match current {
        None => true,
        Some(current) => {
            candidate_salary > opening_salary(current)
                || ((candidate_salary - opening_salary(current)).abs() < 0.01
                    && opening_id(candidate) < opening_id(current))
        }
    }
}

fn opening_salary(opening: DestinationOpeningRef) -> f64 {
    match opening {
        DestinationOpeningRef::Company { offered_salary, .. }
        | DestinationOpeningRef::Bank { offered_salary, .. } => offered_salary,
    }
}

fn opening_id(opening: DestinationOpeningRef) -> u16 {
    match opening {
        DestinationOpeningRef::Company { company_id, .. } => company_id,
        DestinationOpeningRef::Bank { bank_id, .. } => bank_id,
    }
}

fn deterministic_roll(seed: u64, day: u64, person_id: u32, source_country_idx: u64) -> f64 {
    let mixed = seed
        ^ day.rotate_left(7)
        ^ u64::from(person_id).wrapping_mul(0x9e37_79b9)
        ^ source_country_idx.rotate_left(13)
        ^ 0xa5a5_5a5a_1234_5678;
    (mixed % 10_000) as f64 / 10_000.0
}
