use super::*;
use crate::sim::{bank::JobOpening as BankJobOpening, person::ClearedEmployment};
use rand::{Rng, RngCore};
use rand_distr::{Distribution, Poisson};
#[cfg(feature = "bench-trace")]
use std::time::Instant;

impl Country {
    const MANUFACTURING_OPERATOR_JOB_ID: JobTypeId = 5;

    pub(super) fn run_daily_lifecycle(&mut self) {
        self.age_people_one_day();
        self.apply_daily_deaths();
        self.create_daily_births();
        self.process_daily_job_market();

        self.recompute_company_headcounts();
        self.recompute_bank_headcounts();
        self.recompute_aggregates();
    }

    #[cfg(feature = "bench-trace")]
    pub(crate) fn run_daily_lifecycle_traced(&mut self) -> crate::sim::trace::CountryDailyTrace {
        let total_started = Instant::now();

        let started = Instant::now();
        self.age_people_one_day();
        let age_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        self.apply_daily_deaths();
        let deaths_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        self.create_daily_births();
        let births_ms = crate::sim::trace::duration_ms(started.elapsed());

        let (job_market_snapshot_ms, job_market_plan_ms, job_market_apply_ms) =
            self.process_daily_job_market_traced();

        let started = Instant::now();
        self.recompute_company_headcounts();
        let recompute_headcounts_ms = crate::sim::trace::duration_ms(started.elapsed());

        self.recompute_bank_headcounts();

        let started = Instant::now();
        self.recompute_aggregates();
        let recompute_aggregates_ms = crate::sim::trace::duration_ms(started.elapsed());

        crate::sim::trace::CountryDailyTrace {
            total_ms: crate::sim::trace::duration_ms(total_started.elapsed()),
            age_ms,
            deaths_ms,
            births_ms,
            job_market_snapshot_ms,
            job_market_plan_ms,
            job_market_apply_ms,
            recompute_headcounts_ms,
            recompute_aggregates_ms,
        }
    }

    fn age_people_one_day(&mut self) {
        self.people
            .par_chunks_mut(PEOPLE_CHUNK_SIZE)
            .for_each(|chunk| {
                for person in chunk {
                    person.increment_age_one_day();
                }
            });
    }

    fn apply_daily_deaths(&mut self) {
        let demographic = self.sim_params.demographic;
        let cached_daily_death_probabilities: [f64; 101] = std::array::from_fn(|age_years| {
            demographic.daily_death_probability_for_age_years(age_years as u8)
        });
        let mut death_flags = vec![false; self.people.len()];
        death_flags
            .par_chunks_mut(PEOPLE_CHUNK_SIZE)
            .zip(self.people.par_chunks(PEOPLE_CHUNK_SIZE))
            .for_each(|(flags_chunk, people_chunk)| {
                let mut rng = rand::thread_rng();
                for (died_flag, person) in flags_chunk.iter_mut().zip(people_chunk.iter()) {
                    let death_probability =
                        cached_daily_death_probabilities[usize::from(person.age_years().min(100))];
                    *died_flag = rng.gen_bool(death_probability.clamp(0.0, 1.0));
                }
            });

        let people = std::mem::take(&mut self.people);
        let mut deaths = 0_u32;
        let mut survivors = Vec::with_capacity(people.len());

        for (mut person, died) in people.into_iter().zip(death_flags.into_iter()) {
            if died {
                deaths = deaths.saturating_add(1);
                if person.cash_balance > 0.0 {
                    self.treasury_balance =
                        round_cents(self.treasury_balance + person.cash_balance);
                }

                match person.clear_employment() {
                    Some(ClearedEmployment::Company {
                        company_id,
                        job_type_id,
                    }) => {
                        if let Some(company) = self.companies.get_mut(company_id as usize) {
                            company.add_opening(job_type_id, 1);
                        }
                    }
                    Some(ClearedEmployment::Bank {
                        bank_id,
                        job_type_id,
                    }) => {
                        if let Some(bank) = self.bank_by_id_mut(bank_id) {
                            bank.add_opening(job_type_id, 1);
                        }
                    }
                    None => {}
                }

                let written_off = self.primary_bank_mut().write_off_loan(&mut person.loan);
                if written_off > 0.0 {
                    self.pending_events.push(SimEvent::LoanWrittenOff {
                        country_name: self.name.clone(),
                        entity_id: person.id,
                        principal_lost: written_off,
                        day: self.day,
                    });
                }

                // Release housing on death
                use crate::sim::person::HousingStatus;
                match &person.housing {
                    HousingStatus::Owner => {
                        self.housing_stock_occupied_owned =
                            self.housing_stock_occupied_owned.saturating_sub(1);
                        self.housing_stock_vacant =
                            self.housing_stock_vacant.saturating_add(1);
                    }
                    HousingStatus::Mortgaged { mortgage } => {
                        let bank_id = mortgage.lender_bank_id;
                        let principal = mortgage.principal_balance.max(0.0);
                        if let Some(bank) = self.bank_by_id_mut(bank_id) {
                            bank.created_money_outstanding =
                                crate::sim::money::round_cents(
                                    (bank.created_money_outstanding - principal).max(0.0),
                                );
                            bank.last_month_credit_losses =
                                crate::sim::money::round_cents(
                                    bank.last_month_credit_losses + principal,
                                );
                        }
                        self.housing_stock_occupied_owned =
                            self.housing_stock_occupied_owned.saturating_sub(1);
                        self.housing_stock_vacant =
                            self.housing_stock_vacant.saturating_add(1);
                    }
                    HousingStatus::Renter {
                        landlord_country_id, ..
                    } if *landlord_country_id == self.id => {
                        // Company retains ownership of the unit; it becomes vacant.
                        self.housing_stock_occupied_rented =
                            self.housing_stock_occupied_rented.saturating_sub(1);
                        self.housing_stock_vacant =
                            self.housing_stock_vacant.saturating_add(1);
                    }
                    _ => {}
                }
            } else {
                survivors.push(person);
            }
        }

        self.people = survivors;
        self.last_day_deaths = deaths;
        if deaths > 0 {
            self.pending_events.push(SimEvent::PopDeath {
                country_name: self.name.clone(),
                count: deaths,
                day: self.day,
            });
        }
    }

    fn create_daily_births(&mut self) {
        let births_per_person_per_day = self.sim_params.demographic.births_per_person_per_day();
        let lambda = (self.people.len() as f64 * births_per_person_per_day).max(0.0);
        let births = Poisson::<f64>::new(lambda)
            .ok()
            .map(|distribution| distribution.sample(&mut rand::thread_rng()) as u32)
            .unwrap_or(0);

        self.last_day_births = births;
        if births == 0 {
            return;
        }

        let mut rng = rand::thread_rng();
        for _ in 0..births {
            let education_track = self
                .sim_params
                .education
                .pick_track(rng.gen_range(0.0..1.0));

            let newborn = Person::new_with_profile(
                self.next_person_id,
                EmploymentStatus::Unemployed,
                rng.next_u32(),
                0,
                education_track,
            );
            self.next_person_id = self.next_person_id.saturating_add(1);
            self.people.push(newborn);
        }

        self.pending_events.push(SimEvent::PopBirth {
            country_name: self.name.clone(),
            count: births,
            day: self.day,
        });
    }

    fn process_daily_job_market(&mut self) {
        let day = self.day;
        let country_id = self.id;
        let labor_params = self.sim_params.labor;
        let company_openings_snapshot = self
            .companies
            .iter()
            .map(|company| company.open_positions.clone())
            .collect::<Vec<_>>();
        // Snapshot of all bank openings: (bank_id, openings)
        let bank_openings_snapshot: Vec<(BankId, Vec<BankJobOpening>)> =
            std::iter::once(self.primary_bank())
                .chain(self.banks.iter())
                .map(|bank| (bank.id, bank.open_positions.clone()))
                .collect();
        // Flat vec of openings indexed by bank position for the selection function
        let bank_openings_flat: Vec<Vec<BankJobOpening>> = bank_openings_snapshot
            .iter()
            .map(|(_, openings)| openings.clone())
            .collect();

        let intents_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let mut intents = Vec::new();
                let base_idx = chunk_idx.saturating_mul(PEOPLE_CHUNK_SIZE);
                for (offset, person) in chunk.iter().enumerate() {
                    if person.is_employed() || !person.can_seek_job() {
                        continue;
                    }

                    let seek_probability = labor_params
                        .daily_seek_probability(person.age_years(), person.cash_balance);
                    let seek_seed = hash64(
                        (u64::from(country_id) << 48)
                            ^ (day << 24)
                            ^ u64::from(person.id)
                            ^ u64::from(person.name_seed)
                            ^ 0x5ee1_5eed_u64,
                    );

                    if !roll_probability(seek_seed, seek_probability) {
                        continue;
                    }

                    let preferred = select_matching_opening_from_snapshot(
                        &company_openings_snapshot,
                        person.habilitation,
                        seek_seed,
                    );
                    let preferred_bank = select_matching_bank_opening(
                        &bank_openings_flat,
                        person.habilitation,
                        seek_seed,
                    );
                    intents.push(JobSeekIntent {
                        person_idx: base_idx + offset,
                        seek_seed,
                        qualification: person.habilitation,
                        preferred,
                        preferred_bank,
                    });
                }
                intents
            })
            .collect::<Vec<_>>();

        let mut hires_total = 0_u64;
        let day_val = self.day;

        for chunk_intents in &intents_by_chunk {
            for intent in chunk_intents {
                if self
                    .people
                    .get(intent.person_idx)
                    .is_none_or(|p| p.is_employed() || !p.can_seek_job())
                {
                    continue;
                }

                let try_bank_first = match (intent.preferred, intent.preferred_bank) {
                    (Some(_), Some(_)) => {
                        hash64(intent.seek_seed ^ 0x4241_4e4b_u64).is_multiple_of(2)
                    }
                    (None, Some(_)) => true,
                    _ => false,
                };

                let hired = if try_bank_first {
                    self.try_hire_person_to_bank(
                        intent.person_idx,
                        intent.preferred_bank,
                        intent.qualification,
                        intent.seek_seed,
                        day_val,
                        &bank_openings_snapshot,
                    ) || self.try_hire_person_to_company(
                        intent.person_idx,
                        intent.preferred,
                        intent.qualification,
                        intent.seek_seed,
                        day_val,
                    )
                } else {
                    self.try_hire_person_to_company(
                        intent.person_idx,
                        intent.preferred,
                        intent.qualification,
                        intent.seek_seed,
                        day_val,
                    ) || self.try_hire_person_to_bank(
                        intent.person_idx,
                        intent.preferred_bank,
                        intent.qualification,
                        intent.seek_seed,
                        day_val,
                        &bank_openings_snapshot,
                    )
                };

                if hired {
                    hires_total = hires_total.saturating_add(1);
                }
            }
        }

        self.monthly_hires_total = self.monthly_hires_total.saturating_add(hires_total);
    }

    fn try_hire_person_to_company(
        &mut self,
        person_idx: usize,
        preferred: Option<(usize, JobTypeId)>,
        qualification: HabilitationLevel,
        seed: u64,
        day: u64,
    ) -> bool {
        if self
            .people
            .get(person_idx)
            .is_none_or(|p| p.is_employed() || !p.can_seek_job())
        {
            return false;
        }

        if let Some((company_idx, job_type_id)) = preferred
            && let Some(company) = self.companies.get_mut(company_idx)
            && let Some((job_id, offered_salary)) =
                consume_opening_by_job_type(company, job_type_id)
            && let Some(person) = self.people.get_mut(person_idx)
            && !person.is_employed()
        {
            person.set_employment_with_salary(company_idx as u16, job_id, offered_salary, day);
            return true;
        }

        if let Some((company_id, job_id, offered_salary)) =
            consume_matching_opening(&mut self.companies, qualification, seed ^ 0xC01A_0001_u64)
            && let Some(person) = self.people.get_mut(person_idx)
            && !person.is_employed()
        {
            person.set_employment_with_salary(company_id, job_id, offered_salary, day);
            return true;
        }
        false
    }

    fn try_hire_person_to_bank(
        &mut self,
        person_idx: usize,
        preferred_bank: Option<(usize, JobTypeId)>,
        qualification: HabilitationLevel,
        seed: u64,
        day: u64,
        bank_openings_snapshot: &[(BankId, Vec<BankJobOpening>)],
    ) -> bool {
        if self
            .people
            .get(person_idx)
            .is_none_or(|p| p.is_employed() || !p.can_seek_job())
        {
            return false;
        }

        if let Some((bank_snap_idx, job_type_id)) = preferred_bank
            && let Some((bank_id, _)) = bank_openings_snapshot.get(bank_snap_idx)
            && let Some((job_id, offered_salary)) = self.consume_bank_hire(*bank_id, job_type_id)
            && let Some(person) = self.people.get_mut(person_idx)
            && !person.is_employed()
        {
            person.set_bank_employment(*bank_id, job_id, offered_salary, day);
            return true;
        }

        let bank_openings_flat: Vec<Vec<BankJobOpening>> = std::iter::once(self.primary_bank())
            .chain(self.banks.iter())
            .map(|bank| bank.open_positions.clone())
            .collect();
        if let Some((bank_idx, job_type_id)) =
            select_matching_bank_opening(&bank_openings_flat, qualification, seed ^ 0xBA1A_0002_u64)
        {
            let current_bank_id = if bank_idx == 0 {
                self.primary_bank().id
            } else if let Some(bank) = self.banks.get(bank_idx.saturating_sub(1)) {
                bank.id
            } else {
                return false;
            };
            if let Some((job_id, offered_salary)) =
                self.consume_bank_hire(current_bank_id, job_type_id)
                && let Some(person) = self.people.get_mut(person_idx)
                && !person.is_employed()
            {
                person.set_bank_employment(current_bank_id, job_id, offered_salary, day);
                return true;
            }
        }
        false
    }

    fn consume_bank_hire(
        &mut self,
        bank_id: BankId,
        job_type_id: JobTypeId,
    ) -> Option<(JobTypeId, f64)> {
        if self.primary_bank().id == bank_id {
            return consume_bank_opening_by_job_type(self.primary_bank_mut(), job_type_id);
        }
        self.banks
            .iter_mut()
            .find(|bank| bank.id == bank_id)
            .and_then(|bank| consume_bank_opening_by_job_type(bank, job_type_id))
    }

    pub(super) fn recompute_company_headcounts(&mut self) {
        let company_count = self.companies.len();
        let counts_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                let mut local = vec![(0_u32, 0_u32); company_count];
                for person in chunk {
                    let Some(company_id) = person.employer_company_id() else {
                        continue;
                    };
                    let company_idx = company_id as usize;
                    if let Some((headcount, manufacturing)) = local.get_mut(company_idx) {
                        *headcount = headcount.saturating_add(1);
                        if person.job_type_id() == Some(Self::MANUFACTURING_OPERATOR_JOB_ID) {
                            *manufacturing = manufacturing.saturating_add(1);
                        }
                    }
                }
                local
            })
            .collect::<Vec<_>>();

        for company in &mut self.companies {
            company.employee_count = 0;
            company.manufacturing_employee_count = 0;
        }
        for local_counts in counts_by_chunk {
            for (company_idx, (count, manufacturing)) in local_counts.into_iter().enumerate() {
                if let Some(company) = self.companies.get_mut(company_idx) {
                    company.employee_count = company.employee_count.saturating_add(count);
                    company.manufacturing_employee_count = company
                        .manufacturing_employee_count
                        .saturating_add(manufacturing);
                }
            }
        }
    }
}
