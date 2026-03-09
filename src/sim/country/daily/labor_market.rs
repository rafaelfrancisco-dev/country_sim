use super::super::hash64;
use super::super::{
    BankId, Country, HabilitationLevel, JobTypeId, PEOPLE_CHUNK_SIZE, Person,
    job_is_eligible_for_habilitation,
};
use crate::sim::params::LaborParams;
use crate::sim::person::EmploymentStatus;
use rayon::prelude::*;

mod index;
use index::{apply_bank_hire, apply_company_hire};

#[derive(Debug, Copy, Clone)]
struct JobSeekIntent {
    person_idx: usize,
    seek_seed: u64,
    qualification: HabilitationLevel,
    preferred_company_entry: Option<usize>,
    preferred_bank_entry: Option<usize>,
}

#[derive(Debug, Copy, Clone)]
struct CompanyOpeningEntry {
    company_idx: usize,
    opening_idx: usize,
    job_type_id: JobTypeId,
    offered_monthly_salary: f64,
}

#[derive(Debug, Copy, Clone)]
struct BankOpeningEntry {
    bank_snapshot_idx: usize,
    bank_id: BankId,
    opening_idx: usize,
    job_type_id: JobTypeId,
    offered_monthly_salary: f64,
}

#[derive(Debug, Default)]
struct LaborMarketIndex {
    company_entries: Vec<CompanyOpeningEntry>,
    company_slots: Vec<u16>,
    company_groups: [Vec<usize>; 4],
    bank_entries: Vec<BankOpeningEntry>,
    bank_slots: Vec<u16>,
    bank_groups: [Vec<usize>; 4],
}

#[derive(Debug, Default)]
pub(super) struct CountryDailyScratch {
    pub(super) death_flags: Vec<u8>,
    labor_market: LaborMarketIndex,
    intents_by_chunk: Vec<Vec<JobSeekIntent>>,
}

impl CountryDailyScratch {
    pub(super) fn prepare_death_flags(&mut self, people_len: usize) {
        if self.death_flags.capacity() < people_len {
            self.death_flags
                .reserve(people_len.saturating_sub(self.death_flags.capacity()));
        }
        self.death_flags.resize(people_len, 0);
    }

    pub(super) fn prepare_labor_market(&mut self, country: &Country) {
        self.labor_market.rebuild(country);
        let chunk_count = country
            .eligible_unemployed_ids()
            .len()
            .div_ceil(PEOPLE_CHUNK_SIZE);
        self.intents_by_chunk.resize_with(chunk_count, Vec::new);
        for (chunk_idx, intents) in self.intents_by_chunk.iter_mut().enumerate() {
            let chunk_len = country
                .eligible_unemployed_ids()
                .len()
                .saturating_sub(chunk_idx * PEOPLE_CHUNK_SIZE)
                .min(PEOPLE_CHUNK_SIZE);
            if intents.capacity() < chunk_len {
                intents.reserve(chunk_len.saturating_sub(intents.capacity()));
            }
            intents.clear();
        }
    }

    pub(super) fn plan_job_seekers(
        &mut self,
        country: &Country,
        people: &[Person],
        day: u64,
        labor_params: LaborParams,
        use_parallelism: bool,
    ) {
        let labor_market = &self.labor_market;
        let eligible_unemployed_ids = country.eligible_unemployed_ids();
        let fill_chunk = |chunk_idx: usize, intents: &mut Vec<JobSeekIntent>| {
            intents.clear();
            let base_idx = chunk_idx.saturating_mul(PEOPLE_CHUNK_SIZE);
            let end = (base_idx + PEOPLE_CHUNK_SIZE).min(eligible_unemployed_ids.len());
            if base_idx >= end {
                return;
            }
            for &person_id in &eligible_unemployed_ids[base_idx..end] {
                let Some(person_idx) = country.runtime_person_idx(person_id) else {
                    continue;
                };
                let Some(person) = people.get(person_idx) else {
                    continue;
                };
                let seek_probability =
                    labor_params.daily_seek_probability(person.age_years(), person.cash_balance);
                let seek_seed = hash64(
                    (u64::from(country.id()) << 48)
                        ^ (day << 24)
                        ^ u64::from(person.id)
                        ^ u64::from(person.name_seed)
                        ^ 0x5ee1_5eed_u64,
                );
                if !super::super::roll_probability(seek_seed, seek_probability) {
                    continue;
                }
                intents.push(JobSeekIntent {
                    person_idx,
                    seek_seed,
                    qualification: person.habilitation,
                    preferred_company_entry: labor_market
                        .select_company_opening(person.habilitation, seek_seed),
                    preferred_bank_entry: labor_market
                        .select_bank_opening(person.habilitation, seek_seed),
                });
            }
        };

        if use_parallelism {
            self.intents_by_chunk
                .par_iter_mut()
                .enumerate()
                .for_each(|(chunk_idx, intents)| fill_chunk(chunk_idx, intents));
        } else {
            for (chunk_idx, intents) in self.intents_by_chunk.iter_mut().enumerate() {
                fill_chunk(chunk_idx, intents);
            }
        }
    }

    pub(super) fn apply_job_market(&mut self, country: &mut Country) -> u64 {
        let mut hires_total = 0_u64;
        let day = country.day;

        for chunk_idx in 0..self.intents_by_chunk.len() {
            for intent_idx in 0..self.intents_by_chunk[chunk_idx].len() {
                let intent = self.intents_by_chunk[chunk_idx][intent_idx];
                if country
                    .people
                    .get(intent.person_idx)
                    .is_none_or(|person| person.is_employed() || !person.can_seek_job())
                {
                    continue;
                }

                let try_bank_first =
                    match (intent.preferred_company_entry, intent.preferred_bank_entry) {
                        (Some(_), Some(_)) => {
                            hash64(intent.seek_seed ^ 0x4241_4e4b_u64).is_multiple_of(2)
                        }
                        (None, Some(_)) => true,
                        _ => false,
                    };

                let hired = if try_bank_first {
                    self.try_hire_person_to_bank(country, intent, day)
                        || self.try_hire_person_to_company(country, intent, day)
                } else {
                    self.try_hire_person_to_company(country, intent, day)
                        || self.try_hire_person_to_bank(country, intent, day)
                };

                if hired {
                    hires_total = hires_total.saturating_add(1);
                }
            }
        }

        self.labor_market.commit(country);
        hires_total
    }

    fn try_hire_person_to_company(
        &mut self,
        country: &mut Country,
        intent: JobSeekIntent,
        day: u64,
    ) -> bool {
        if let Some(entry_idx) = intent.preferred_company_entry
            && let Some((company_idx, job_type_id, offered_salary)) =
                self.labor_market.consume_company_entry(entry_idx)
        {
            return apply_company_hire(
                country,
                intent.person_idx,
                company_idx,
                job_type_id,
                offered_salary,
                day,
            );
        }

        if let Some((company_idx, job_type_id, offered_salary)) = self
            .labor_market
            .consume_matching_company(intent.qualification, intent.seek_seed ^ 0xC01A_0001_u64)
        {
            return apply_company_hire(
                country,
                intent.person_idx,
                company_idx,
                job_type_id,
                offered_salary,
                day,
            );
        }
        false
    }

    fn try_hire_person_to_bank(
        &mut self,
        country: &mut Country,
        intent: JobSeekIntent,
        day: u64,
    ) -> bool {
        if let Some(entry_idx) = intent.preferred_bank_entry
            && let Some((bank_id, job_type_id, offered_salary)) =
                self.labor_market.consume_bank_entry(entry_idx)
        {
            return apply_bank_hire(
                country,
                intent.person_idx,
                bank_id,
                job_type_id,
                offered_salary,
                day,
            );
        }

        if let Some((bank_id, job_type_id, offered_salary)) = self
            .labor_market
            .consume_matching_bank(intent.qualification, intent.seek_seed ^ 0xBA1A_0002_u64)
        {
            return apply_bank_hire(
                country,
                intent.person_idx,
                bank_id,
                job_type_id,
                offered_salary,
                day,
            );
        }
        false
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
