use super::person::HabilitationLevel;

pub type JobTypeId = u8;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct JobType {
    pub id: JobTypeId,
    pub name: &'static str,
    pub monthly_salary: f64,
    pub assignment_weight: u16,
    pub required_habilitation: HabilitationLevel,
}

const COMMON_JOB_TYPES: [JobType; 10] = [
    JobType {
        id: 0,
        name: "Retail Salesperson",
        monthly_salary: 3_200.0,
        assignment_weight: 14,
        required_habilitation: HabilitationLevel::BasicSchool,
    },
    JobType {
        id: 1,
        name: "Cashier",
        monthly_salary: 2_800.0,
        assignment_weight: 13,
        required_habilitation: HabilitationLevel::BasicSchool,
    },
    JobType {
        id: 2,
        name: "Office Clerk",
        monthly_salary: 3_400.0,
        assignment_weight: 11,
        required_habilitation: HabilitationLevel::HighSchool,
    },
    JobType {
        id: 3,
        name: "Driver",
        monthly_salary: 3_700.0,
        assignment_weight: 11,
        required_habilitation: HabilitationLevel::BasicSchool,
    },
    JobType {
        id: 4,
        name: "Construction Worker",
        monthly_salary: 4_200.0,
        assignment_weight: 9,
        required_habilitation: HabilitationLevel::BasicSchool,
    },
    JobType {
        id: 5,
        name: "Manufacturing Operator",
        monthly_salary: 3_900.0,
        assignment_weight: 9,
        required_habilitation: HabilitationLevel::BasicSchool,
    },
    JobType {
        id: 6,
        name: "Food Service Worker",
        monthly_salary: 2_900.0,
        assignment_weight: 10,
        required_habilitation: HabilitationLevel::BasicSchool,
    },
    JobType {
        id: 7,
        name: "Registered Nurse",
        monthly_salary: 6_800.0,
        assignment_weight: 7,
        required_habilitation: HabilitationLevel::HigherLearning,
    },
    JobType {
        id: 8,
        name: "Teacher",
        monthly_salary: 5_200.0,
        assignment_weight: 8,
        required_habilitation: HabilitationLevel::HigherLearning,
    },
    JobType {
        id: 9,
        name: "Software Developer",
        monthly_salary: 8_500.0,
        assignment_weight: 8,
        required_habilitation: HabilitationLevel::HigherLearning,
    },
];

pub fn common_job_types() -> &'static [JobType; 10] {
    &COMMON_JOB_TYPES
}

pub fn job_by_id(id: JobTypeId) -> Option<&'static JobType> {
    COMMON_JOB_TYPES.iter().find(|job| job.id == id)
}

pub fn salary_for_job(id: JobTypeId) -> f64 {
    job_by_id(id).map_or(0.0, |job| job.monthly_salary)
}

pub fn job_is_eligible_for_habilitation(id: JobTypeId, habilitation: HabilitationLevel) -> bool {
    job_by_id(id)
        .map(|job| habilitation >= job.required_habilitation)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_ten_common_jobs_exist() {
        assert_eq!(common_job_types().len(), 10);
    }

    #[test]
    fn job_ids_are_unique() {
        let mut ids = common_job_types()
            .iter()
            .map(|job| job.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn salary_lookup_returns_expected_value() {
        assert!((salary_for_job(9) - 8_500.0).abs() < f64::EPSILON);
        assert!((salary_for_job(255) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn job_eligibility_respects_habilitation_level() {
        assert!(job_is_eligible_for_habilitation(
            2,
            HabilitationLevel::HighSchool
        ));
        assert!(!job_is_eligible_for_habilitation(
            9,
            HabilitationLevel::HighSchool
        ));
    }
}
