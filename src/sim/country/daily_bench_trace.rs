use super::*;
#[cfg(feature = "bench-trace")]
use std::time::Instant;

impl Country {
    #[cfg(feature = "bench-trace")]
    pub(super) fn process_daily_job_market_traced(&mut self) -> (f64, f64, f64) {
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
                        preferred_bank: None,
                    });
                }
                intents
            })
            .collect::<Vec<_>>();
        let plan_ms = crate::sim::trace::duration_ms(started.elapsed());

        let started = Instant::now();
        let mut hires_total = 0_u64;
        let day_val = self.day;
        {
            let (people, companies) = (&mut self.people, &mut self.companies);
            for chunk_intents in &intents_by_chunk {
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
                        person.set_employment_with_salary(
                            company_idx as u16,
                            job_id,
                            offered_salary,
                            day_val,
                        );
                        hires_total = hires_total.saturating_add(1);
                        continue;
                    }

                    if let Some((company_id, job_id, offered_salary)) =
                        consume_matching_opening(companies, intent.qualification, intent.seek_seed)
                    {
                        person.set_employment_with_salary(
                            company_id,
                            job_id,
                            offered_salary,
                            day_val,
                        );
                        hires_total = hires_total.saturating_add(1);
                    }
                }
            }
        }
        self.monthly_hires_total = self.monthly_hires_total.saturating_add(hires_total);
        let apply_ms = crate::sim::trace::duration_ms(started.elapsed());

        (snapshot_ms, plan_ms, apply_ms)
    }
}
