use super::*;
use crate::sim::{good::is_production_job_for_good, money::to_cents, person::ClearedEmployment};
use rand::{Rng, RngCore};
use rand_distr::{Distribution, Poisson};
use std::cell::RefCell;
#[cfg(feature = "bench-trace")]
use std::time::Instant;

mod labor_market;

use labor_market::CountryDailyScratch;

thread_local! {
    static COUNTRY_DAILY_SCRATCH: RefCell<CountryDailyScratch> = RefCell::new(CountryDailyScratch::default());
}

impl Country {
    pub(super) fn run_daily_lifecycle(&mut self) {
        self.with_country_daily_scratch(Self::run_daily_lifecycle_with_scratch);
    }

    #[cfg(feature = "bench-trace")]
    pub(crate) fn run_daily_lifecycle_traced(&mut self) -> crate::sim::trace::CountryDailyTrace {
        let total_started = Instant::now();
        let mut trace = crate::sim::trace::CountryDailyTrace::default();

        self.with_country_daily_scratch(|country, scratch| {
            country.run_daily_lifecycle_with_scratch_traced(scratch, &mut trace);
        });

        trace.total_ms = crate::sim::trace::duration_ms(total_started.elapsed());
        trace
    }

    fn with_country_daily_scratch<R>(
        &mut self,
        run: impl FnOnce(&mut Country, &mut CountryDailyScratch) -> R,
    ) -> R {
        COUNTRY_DAILY_SCRATCH.with(|scratch| {
            if let Ok(mut borrowed) = scratch.try_borrow_mut() {
                run(self, &mut borrowed)
            } else {
                // Nested Rayon work can re-enter country daily logic on the same worker thread.
                // Fall back to a temporary scratch instead of panicking on the thread-local borrow.
                let mut fallback = CountryDailyScratch::default();
                run(self, &mut fallback)
            }
        })
    }

    fn run_daily_lifecycle_with_scratch(&mut self, scratch: &mut CountryDailyScratch) {
        self.age_people_one_day();
        self.apply_daily_deaths(scratch);
        self.create_daily_births();
        self.process_daily_job_market(scratch);
        self.process_daily_job_switches();
    }

    #[cfg(feature = "bench-trace")]
    fn run_daily_lifecycle_with_scratch_traced(
        &mut self,
        scratch: &mut CountryDailyScratch,
        trace: &mut crate::sim::trace::CountryDailyTrace,
    ) {
        let started = Instant::now();
        self.age_people_one_day();
        trace.age_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        self.apply_daily_deaths(scratch);
        trace.deaths_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        self.create_daily_births();
        trace.births_ms = crate::sim::trace::duration_ms(started.elapsed());

        let (prepare_ms, plan_ms, apply_ms) = self.process_daily_job_market_traced(scratch);
        trace.job_market_snapshot_ms = prepare_ms;
        trace.job_market_plan_ms = plan_ms;
        trace.job_market_apply_ms = apply_ms;

        self.process_daily_job_switches();
    }

    fn age_people_one_day(&mut self) {
        if self.use_inner_parallelism {
            self.people
                .par_chunks_mut(PEOPLE_CHUNK_SIZE)
                .for_each(|chunk| {
                    for person in chunk {
                        person.increment_age_one_day();
                    }
                });
        } else {
            for person in &mut self.people {
                person.increment_age_one_day();
            }
        }

        for person_idx in 0..self.people.len() {
            if self.people[person_idx].age_days % 365 == 0 {
                self.refresh_person_runtime_membership(person_idx);
            }
        }
    }

    fn apply_daily_deaths(&mut self, scratch: &mut CountryDailyScratch) {
        let demographic = self.sim_params.demographic;
        let cached_daily_death_probabilities: [f64; 101] = std::array::from_fn(|age_years| {
            demographic.daily_death_probability_for_age_years(age_years as u8)
        });
        scratch.prepare_death_flags(self.people.len());
        if self.use_inner_parallelism {
            scratch
                .death_flags
                .par_chunks_mut(PEOPLE_CHUNK_SIZE)
                .zip(self.people.par_chunks(PEOPLE_CHUNK_SIZE))
                .for_each(|(flags_chunk, people_chunk)| {
                    let mut rng = rand::thread_rng();
                    for (died_flag, person) in flags_chunk.iter_mut().zip(people_chunk.iter()) {
                        let death_probability = cached_daily_death_probabilities
                            [usize::from(person.age_years().min(100))];
                        *died_flag = u8::from(rng.gen_bool(death_probability.clamp(0.0, 1.0)));
                    }
                });
        } else {
            let mut rng = rand::thread_rng();
            for (died_flag, person) in scratch.death_flags.iter_mut().zip(self.people.iter()) {
                let death_probability =
                    cached_daily_death_probabilities[usize::from(person.age_years().min(100))];
                *died_flag = u8::from(rng.gen_bool(death_probability.clamp(0.0, 1.0)));
            }
        }

        let mut people = std::mem::take(&mut self.people);
        let mut deaths = 0_u32;
        let mut write_idx = 0_usize;

        for read_idx in 0..people.len() {
            if scratch.death_flags[read_idx] == 0 {
                if write_idx != read_idx {
                    people.swap(write_idx, read_idx);
                }
                write_idx += 1;
                continue;
            }

            deaths = deaths.saturating_add(1);
            let cash_cents = to_cents(people[read_idx].cash_balance);
            let person_id = people[read_idx].id;
            let cleared = people[read_idx].clear_employment();
            let mut loan = people[read_idx].loan.take();

            self.adjust_household_cash_total_cents(-cash_cents);
            if cash_cents > 0 {
                self.adjust_treasury_balance_cents(cash_cents);
            }

            match cleared {
                Some(ClearedEmployment::Company {
                    company_id,
                    job_type_id,
                }) => {
                    self.adjust_company_headcount(company_id as usize, job_type_id, -1);
                    if let Some(company) = self.companies.get_mut(company_id as usize) {
                        company.add_opening(job_type_id, 1);
                    }
                }
                Some(ClearedEmployment::Bank {
                    bank_id,
                    job_type_id,
                }) => {
                    self.adjust_bank_headcount(bank_id, -1);
                    if let Some(bank) = self.bank_by_id_mut(bank_id) {
                        bank.add_opening(job_type_id, 1);
                    }
                }
                None => {}
            }

            let written_off = self.primary_bank_mut().write_off_loan(&mut loan);
            if written_off > 0.0 {
                self.adjust_expected_money_supply_cents(-to_cents(written_off));
                self.pending_events.push(SimEvent::LoanWrittenOff {
                    country_name: self.name.clone(),
                    entity_id: person_id,
                    principal_lost: written_off,
                    day: self.day,
                });
            }

            use crate::sim::person::HousingStatus;
            match &people[read_idx].housing {
                HousingStatus::Owner => {
                    self.housing_stock_occupied_owned =
                        self.housing_stock_occupied_owned.saturating_sub(1);
                    self.housing_stock_vacant = self.housing_stock_vacant.saturating_add(1);
                }
                HousingStatus::Mortgaged { mortgage } => {
                    let bank_id = mortgage.lender_bank_id;
                    let principal = mortgage.principal_balance.max(0.0);
                    if let Some(bank) = self.bank_by_id_mut(bank_id) {
                        bank.created_money_outstanding = crate::sim::money::round_cents(
                            (bank.created_money_outstanding - principal).max(0.0),
                        );
                        bank.last_month_credit_losses = crate::sim::money::round_cents(
                            bank.last_month_credit_losses + principal,
                        );
                    }
                    self.adjust_expected_money_supply_cents(-to_cents(principal));
                    self.housing_stock_occupied_owned =
                        self.housing_stock_occupied_owned.saturating_sub(1);
                    self.housing_stock_vacant = self.housing_stock_vacant.saturating_add(1);
                }
                HousingStatus::Renter {
                    landlord_country_id,
                    ..
                } if *landlord_country_id == self.id => {
                    self.housing_stock_occupied_rented =
                        self.housing_stock_occupied_rented.saturating_sub(1);
                    self.housing_stock_vacant = self.housing_stock_vacant.saturating_add(1);
                }
                _ => {}
            }
        }

        people.truncate(write_idx);
        self.people = people;
        self.rebuild_person_runtime();
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
            let person_idx = self.people.len().saturating_sub(1);
            self.register_person_runtime(person_idx);
        }

        self.pending_events.push(SimEvent::PopBirth {
            country_name: self.name.clone(),
            count: births,
            day: self.day,
        });
    }

    fn process_daily_job_market(&mut self, scratch: &mut CountryDailyScratch) {
        let _ = self.process_daily_job_market_inner(scratch);
    }

    #[cfg(feature = "bench-trace")]
    fn process_daily_job_market_traced(
        &mut self,
        scratch: &mut CountryDailyScratch,
    ) -> (f64, f64, f64) {
        self.process_daily_job_market_inner(scratch)
    }

    fn process_daily_job_market_inner(
        &mut self,
        scratch: &mut CountryDailyScratch,
    ) -> (f64, f64, f64) {
        if self.companies.is_empty() && self.banks.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        #[cfg(feature = "bench-trace")]
        let prepare_started = Instant::now();
        scratch.prepare_labor_market(self);
        #[cfg(feature = "bench-trace")]
        let prepare_ms = crate::sim::trace::duration_ms(prepare_started.elapsed());
        #[cfg(not(feature = "bench-trace"))]
        let prepare_ms = 0.0;

        #[cfg(feature = "bench-trace")]
        let plan_started = Instant::now();
        scratch.plan_job_seekers(
            self,
            &self.people,
            self.day,
            self.sim_params.labor,
            self.use_inner_parallelism,
        );
        #[cfg(feature = "bench-trace")]
        let plan_ms = crate::sim::trace::duration_ms(plan_started.elapsed());
        #[cfg(not(feature = "bench-trace"))]
        let plan_ms = 0.0;

        #[cfg(feature = "bench-trace")]
        let apply_started = Instant::now();
        let hires_total = scratch.apply_job_market(self);
        #[cfg(feature = "bench-trace")]
        let apply_ms = crate::sim::trace::duration_ms(apply_started.elapsed());
        #[cfg(not(feature = "bench-trace"))]
        let apply_ms = 0.0;

        self.monthly_hires_total = self.monthly_hires_total.saturating_add(hires_total);
        (prepare_ms, plan_ms, apply_ms)
    }

    pub(super) fn adjust_company_headcount(
        &mut self,
        company_idx: usize,
        job_type_id: JobTypeId,
        delta: i32,
    ) {
        let Some(company) = self.companies.get_mut(company_idx) else {
            return;
        };
        if delta >= 0 {
            company.employee_count = company.employee_count.saturating_add(delta as u32);
            if is_production_job_for_good(company.produced_good_id, job_type_id) {
                company.manufacturing_employee_count = company
                    .manufacturing_employee_count
                    .saturating_add(delta as u32);
            }
            return;
        }

        let decrement = (-delta) as u32;
        company.employee_count = company.employee_count.saturating_sub(decrement);
        if is_production_job_for_good(company.produced_good_id, job_type_id) {
            company.manufacturing_employee_count = company
                .manufacturing_employee_count
                .saturating_sub(decrement);
        }
    }

    pub(super) fn adjust_bank_headcount(&mut self, bank_id: BankId, delta: i32) {
        let Some(bank) = self.bank_by_id_mut(bank_id) else {
            return;
        };
        if delta >= 0 {
            bank.employee_count = bank.employee_count.saturating_add(delta as u32);
        } else {
            bank.employee_count = bank.employee_count.saturating_sub((-delta) as u32);
        }
    }

    pub(super) fn recompute_company_headcounts(&mut self) {
        for company in &mut self.companies {
            company.employee_count = 0;
            company.manufacturing_employee_count = 0;
        }
        if self.use_inner_parallelism {
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
                            let Some(company) = self.companies.get(company_idx) else {
                                continue;
                            };
                            if person.job_type_id().is_some_and(|job_id| {
                                is_production_job_for_good(company.produced_good_id, job_id)
                            }) {
                                *manufacturing = manufacturing.saturating_add(1);
                            }
                        }
                    }
                    local
                })
                .collect::<Vec<_>>();
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
        } else {
            for person in &self.people {
                let Some(company_id) = person.employer_company_id() else {
                    continue;
                };
                let company_idx = company_id as usize;
                if let Some(company) = self.companies.get_mut(company_idx) {
                    company.employee_count = company.employee_count.saturating_add(1);
                    if person.job_type_id().is_some_and(|job_id| {
                        is_production_job_for_good(company.produced_good_id, job_id)
                    }) {
                        company.manufacturing_employee_count =
                            company.manufacturing_employee_count.saturating_add(1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::person::Person;

    #[test]
    fn daily_lifecycle_falls_back_when_thread_local_scratch_is_reentered() {
        let mut country = Country::new(1, "Nested".to_string(), vec![Person::new(1, 1, 1, 1)]);

        COUNTRY_DAILY_SCRATCH.with(|scratch| {
            let _held = scratch.borrow_mut();
            country.run_daily_lifecycle();
        });
    }
}
