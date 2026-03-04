use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SeparationKind {
    Quit,
    Retirement,
    Layoff,
}

impl Country {
    pub(in crate::sim::country::monthly) fn process_monthly_labor_churn(&mut self) {
        let churn = self.sim_params.labor_churn;
        let quit_probability = churn.quit_probability();
        let retirement_probability = churn.retirement_probability();
        let layoff_probability = churn.layoff_probability();

        if quit_probability <= 0.0 && retirement_probability <= 0.0 && layoff_probability <= 0.0 {
            return;
        }

        let mut quits = 0_u32;
        let mut retirements = 0_u32;
        let mut layoffs_non_distress = 0_u32;
        let mut replacement_openings = 0_u32;

        for person_idx in 0..self.people.len() {
            let (person_id, name_seed, age_years, company_id, job_type_id, monthly_salary) = {
                let person = &self.people[person_idx];
                let Some(company_id) = person.employer_company_id() else {
                    continue;
                };
                let Some(job_type_id) = person.job_type_id() else {
                    continue;
                };
                (
                    person.id,
                    person.name_seed,
                    person.age_years(),
                    company_id,
                    job_type_id,
                    person.monthly_salary(),
                )
            };

            let separation = self.pick_monthly_separation_kind(
                person_id,
                name_seed,
                age_years,
                retirement_probability,
                layoff_probability,
                quit_probability,
            );
            let Some(separation) = separation else {
                continue;
            };

            let Some((previous_company_id, previous_job_id)) = self
                .people
                .get_mut(person_idx)
                .and_then(Person::clear_employment)
            else {
                continue;
            };
            if previous_company_id != company_id || previous_job_id != job_type_id {
                continue;
            }

            if let Some(company) = self.companies.get_mut(previous_company_id as usize) {
                company.add_opening_with_salary(previous_job_id, 1, monthly_salary);
                replacement_openings = replacement_openings.saturating_add(1);
            }

            match separation {
                SeparationKind::Quit => quits = quits.saturating_add(1),
                SeparationKind::Retirement => retirements = retirements.saturating_add(1),
                SeparationKind::Layoff => {
                    layoffs_non_distress = layoffs_non_distress.saturating_add(1)
                }
            }
        }

        let total = quits
            .saturating_add(retirements)
            .saturating_add(layoffs_non_distress);
        if total == 0 {
            return;
        }

        self.pending_events.push(SimEvent::LaborChurnMonthly {
            country_name: self.name.clone(),
            quits,
            retirements,
            layoffs_non_distress,
            replacement_openings,
            day: self.day,
        });
    }

    fn pick_monthly_separation_kind(
        &self,
        person_id: u32,
        name_seed: u32,
        age_years: u8,
        retirement_probability: f64,
        layoff_probability: f64,
        quit_probability: f64,
    ) -> Option<SeparationKind> {
        if age_years >= 65
            && self.churn_roll(
                person_id,
                name_seed,
                0xa761_0c99_u64,
                retirement_probability,
            )
        {
            return Some(SeparationKind::Retirement);
        }

        if self.churn_roll(person_id, name_seed, 0x1a7a_ffe5_u64, layoff_probability) {
            return Some(SeparationKind::Layoff);
        }

        if self.churn_roll(person_id, name_seed, 0x5eed_0001_u64, quit_probability) {
            return Some(SeparationKind::Quit);
        }

        None
    }

    fn churn_roll(&self, person_id: u32, name_seed: u32, salt: u64, probability: f64) -> bool {
        if probability <= 0.0 {
            return false;
        }
        if probability >= 1.0 {
            return true;
        }

        let seed = (u64::from(self.id) << 48)
            ^ (self.day << 24)
            ^ u64::from(person_id)
            ^ u64::from(name_seed)
            ^ salt;
        let roll = hash64(seed) as f64 / u64::MAX as f64;
        roll < probability
    }
}
