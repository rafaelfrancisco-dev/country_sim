use super::*;
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

                if let Some((company_id, job_type_id)) = person.clear_employment()
                    && let Some(company) = self.companies.get_mut(company_id as usize)
                {
                    company.add_opening(job_type_id, 1);
                }

                let written_off = self.bank.write_off_loan(&mut person.loan);
                if written_off > 0.0 {
                    self.pending_events.push(SimEvent::LoanWrittenOff {
                        country_name: self.name.clone(),
                        entity_id: person.id,
                        principal_lost: written_off,
                        day: self.day,
                    });
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
        if self.companies.is_empty() {
            return;
        }

        let day = self.day;
        let country_id = self.id;
        let labor_params = self.sim_params.labor;
        let company_openings_snapshot = self
            .companies
            .iter()
            .map(|company| company.open_positions.clone())
            .collect::<Vec<_>>();

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
                    intents.push(JobSeekIntent {
                        person_idx: base_idx + offset,
                        seek_seed,
                        qualification: person.habilitation,
                        preferred,
                    });
                }
                intents
            })
            .collect::<Vec<_>>();

        let mut hires_total = 0_u64;
        let (people, companies) = (&mut self.people, &mut self.companies);
        for chunk_intents in intents_by_chunk {
            for intent in chunk_intents {
                let Some(person) = people.get_mut(intent.person_idx) else {
                    continue;
                };
                if person.is_employed() || !person.can_seek_job() {
                    continue;
                }

                if let Some((company_idx, job_type_id)) = intent.preferred
                    && let Some(company) = companies.get_mut(company_idx)
                    && let Some((job_id, offered_salary)) =
                        consume_opening_by_job_type(company, job_type_id)
                {
                    person.set_employment_with_salary(company_idx as u16, job_id, offered_salary);
                    hires_total = hires_total.saturating_add(1);
                    continue;
                }

                if let Some((company_id, job_id, offered_salary)) =
                    consume_matching_opening(companies, intent.qualification, intent.seek_seed)
                {
                    person.set_employment_with_salary(company_id, job_id, offered_salary);
                    hires_total = hires_total.saturating_add(1);
                }
            }
        }
        self.monthly_hires_total = self.monthly_hires_total.saturating_add(hires_total);
    }

    #[cfg(feature = "bench-trace")]
    fn process_daily_job_market_traced(&mut self) -> (f64, f64, f64) {
        if self.companies.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let day = self.day;
        let country_id = self.id;
        let labor_params = self.sim_params.labor;

        let started = Instant::now();
        let company_openings_snapshot = self
            .companies
            .iter()
            .map(|company| company.open_positions.clone())
            .collect::<Vec<_>>();
        let snapshot_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
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
                    intents.push(JobSeekIntent {
                        person_idx: base_idx + offset,
                        seek_seed,
                        qualification: person.habilitation,
                        preferred,
                    });
                }
                intents
            })
            .collect::<Vec<_>>();
        let plan_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        let mut hires_total = 0_u64;
        let (people, companies) = (&mut self.people, &mut self.companies);
        for chunk_intents in intents_by_chunk {
            for intent in chunk_intents {
                let Some(person) = people.get_mut(intent.person_idx) else {
                    continue;
                };
                if person.is_employed() || !person.can_seek_job() {
                    continue;
                }

                if let Some((company_idx, job_type_id)) = intent.preferred
                    && let Some(company) = companies.get_mut(company_idx)
                    && let Some((job_id, offered_salary)) =
                        consume_opening_by_job_type(company, job_type_id)
                {
                    person.set_employment_with_salary(company_idx as u16, job_id, offered_salary);
                    hires_total = hires_total.saturating_add(1);
                    continue;
                }

                if let Some((company_id, job_id, offered_salary)) =
                    consume_matching_opening(companies, intent.qualification, intent.seek_seed)
                {
                    person.set_employment_with_salary(company_id, job_id, offered_salary);
                    hires_total = hires_total.saturating_add(1);
                }
            }
        }
        self.monthly_hires_total = self.monthly_hires_total.saturating_add(hires_total);
        let apply_ms = crate::sim::trace::duration_ms(started.elapsed());

        (snapshot_ms, plan_ms, apply_ms)
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
