use std::collections::HashMap;

use super::Country;
use crate::sim::{job::JobTypeId, person::Person};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CountryRuntime {
    person_indices_by_id: HashMap<u32, usize>,
    eligible_unemployed: IndexedPersonSet,
    employed: IndexedPersonSet,
    monthly_switch_cache_month: u64,
    monthly_switch_openings: HashMap<(JobTypeId, i64), Option<CachedSwitchOpening>>,
}

impl Default for CountryRuntime {
    fn default() -> Self {
        Self {
            person_indices_by_id: HashMap::new(),
            eligible_unemployed: IndexedPersonSet::default(),
            employed: IndexedPersonSet::default(),
            monthly_switch_cache_month: u64::MAX,
            monthly_switch_openings: HashMap::new(),
        }
    }
}

impl CountryRuntime {
    pub(crate) fn rebuild_from_people(&mut self, people: &[Person]) {
        self.person_indices_by_id.clear();
        self.eligible_unemployed.clear();
        self.employed.clear();
        for (person_idx, person) in people.iter().enumerate() {
            self.register_person(person, person_idx);
        }
    }

    pub(crate) fn prepare_monthly_switch_cache(&mut self, month: u64) {
        if self.monthly_switch_cache_month != month {
            self.monthly_switch_cache_month = month;
            self.monthly_switch_openings.clear();
        }
    }

    pub(crate) fn clear_monthly_switch_cache(&mut self) {
        self.monthly_switch_openings.clear();
    }

    pub(crate) fn cached_switch_opening(
        &self,
        job_type_id: JobTypeId,
        threshold_bucket: i64,
    ) -> Option<Option<CachedSwitchOpening>> {
        self.monthly_switch_openings
            .get(&(job_type_id, threshold_bucket))
            .copied()
    }

    pub(crate) fn store_switch_opening(
        &mut self,
        job_type_id: JobTypeId,
        threshold_bucket: i64,
        opening: Option<CachedSwitchOpening>,
    ) {
        self.monthly_switch_openings
            .insert((job_type_id, threshold_bucket), opening);
    }

    pub(crate) fn register_person(&mut self, person: &Person, person_idx: usize) {
        self.person_indices_by_id.insert(person.id, person_idx);
        self.refresh_person_membership(person);
    }

    pub(crate) fn person_idx(&self, person_id: u32) -> Option<usize> {
        self.person_indices_by_id.get(&person_id).copied()
    }

    pub(crate) fn eligible_unemployed_ids(&self) -> &[u32] {
        self.eligible_unemployed.ids()
    }

    pub(crate) fn employed_ids(&self) -> &[u32] {
        self.employed.ids()
    }

    pub(crate) fn refresh_person_membership(&mut self, person: &Person) {
        if person.is_employed() {
            self.employed.insert(person.id);
            self.eligible_unemployed.remove(person.id);
            return;
        }

        self.employed.remove(person.id);
        if person.can_seek_job() {
            self.eligible_unemployed.insert(person.id);
        } else {
            self.eligible_unemployed.remove(person.id);
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct IndexedPersonSet {
    ids: Vec<u32>,
    positions_by_id: HashMap<u32, usize>,
}

impl IndexedPersonSet {
    fn clear(&mut self) {
        self.ids.clear();
        self.positions_by_id.clear();
    }

    fn insert(&mut self, person_id: u32) {
        if self.positions_by_id.contains_key(&person_id) {
            return;
        }
        self.positions_by_id.insert(person_id, self.ids.len());
        self.ids.push(person_id);
    }

    fn remove(&mut self, person_id: u32) {
        let Some(remove_idx) = self.positions_by_id.remove(&person_id) else {
            return;
        };
        let Some(swapped_id) = self.ids.pop() else {
            return;
        };
        if remove_idx < self.ids.len() {
            self.ids[remove_idx] = swapped_id;
            self.positions_by_id.insert(swapped_id, remove_idx);
        }
    }

    fn ids(&self) -> &[u32] {
        &self.ids
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct CachedSwitchOpening {
    pub(crate) company_idx: usize,
    pub(crate) opening_idx: usize,
    pub(crate) offered_salary: f64,
    pub(crate) company_id: u16,
}

impl Country {
    pub(crate) fn runtime_person_idx(&self, person_id: u32) -> Option<usize> {
        self.runtime.person_idx(person_id)
    }

    pub(crate) fn rebuild_person_runtime(&mut self) {
        self.runtime.rebuild_from_people(&self.people);
    }

    pub(crate) fn refresh_person_runtime_membership(&mut self, person_idx: usize) {
        if let Some(person) = self.people.get(person_idx) {
            self.runtime.refresh_person_membership(person);
        }
    }

    pub(crate) fn register_person_runtime(&mut self, person_idx: usize) {
        if let Some(person) = self.people.get(person_idx) {
            self.runtime.register_person(person, person_idx);
        }
    }

    pub(crate) fn eligible_unemployed_ids(&self) -> &[u32] {
        self.runtime.eligible_unemployed_ids()
    }

    pub(crate) fn employed_ids(&self) -> &[u32] {
        self.runtime.employed_ids()
    }
}
