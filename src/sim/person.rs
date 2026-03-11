use super::{
    bank::{BankId, LoanAccount, MortgageAccount},
    essentials::STOCKABLE_ESSENTIAL_GOOD_IDS,
    job::{JobTypeId, salary_for_job},
};

#[derive(Debug, Clone, PartialEq)]
pub enum HousingStatus {
    Unhoused,
    Renter {
        landlord_company_id: u16,
        landlord_country_id: u16,
    },
    Owner,
    Mortgaged {
        mortgage: MortgageAccount,
    },
}

impl HousingStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unhoused => "Unhoused",
            Self::Renter { .. } => "Renter",
            Self::Owner => "Owner",
            Self::Mortgaged { .. } => "Mortgaged",
        }
    }
}

/// A deposit held at a bank in a foreign country.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignDeposit {
    pub country_id: u16,
    pub bank_id: BankId,
    pub balance: f64,
    pub home_amount_at_open: f64,
    pub opening_fx_rate_home_per_foreign: f64,
}

pub const DAYS_PER_YEAR: u16 = 365;
pub const SUPERFICIAL_GOOD_SLOT_COUNT: usize = 5;
pub const ESSENTIAL_STOCK_SLOT_COUNT: usize = STOCKABLE_ESSENTIAL_GOOD_IDS.len();

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum HabilitationLevel {
    NoSchoolYet,
    BasicSchool,
    HighSchool,
    HigherLearning,
}

impl HabilitationLevel {
    pub fn label(self) -> &'static str {
        match self {
            HabilitationLevel::NoSchoolYet => "No School",
            HabilitationLevel::BasicSchool => "Basic School",
            HabilitationLevel::HighSchool => "High School",
            HabilitationLevel::HigherLearning => "Higher Learning",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EducationTrack {
    BasicOnly,
    HighSchool,
    Bachelor,
    Doctor,
}

impl EducationTrack {
    pub fn minimum_job_seek_age(self) -> u8 {
        match self {
            EducationTrack::BasicOnly | EducationTrack::HighSchool => 18,
            EducationTrack::Bachelor => 21,
            EducationTrack::Doctor => 23,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EducationTrack::BasicOnly => "Basic",
            EducationTrack::HighSchool => "High School",
            EducationTrack::Bachelor => "Bachelor",
            EducationTrack::Doctor => "Doctor",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EmploymentStatus {
    Unemployed,
    Employed {
        job_type_id: JobTypeId,
        employer_company_id: u16,
    },
    EmployedAtBank {
        job_type_id: JobTypeId,
        bank_id: u16,
    },
}

/// Result of clearing a person's employment, identifying who the employer was.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClearedEmployment {
    Company {
        company_id: u16,
        job_type_id: JobTypeId,
    },
    Bank {
        bank_id: u16,
        job_type_id: JobTypeId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub id: u32,
    pub birth_country_id: u16,
    pub current_country_id: u16,
    pub employment: EmploymentStatus,
    pub employment_start_day: Option<u64>,
    pub agreed_monthly_salary: f64,
    pub cash_balance: f64,
    pub deposit_bank_id: Option<BankId>,
    pub household_id: u32,
    pub name_seed: u32,
    pub age_days: u16,
    pub education_track: EducationTrack,
    pub habilitation: HabilitationLevel,
    pub loan: Option<LoanAccount>,
    pub last_month_gross_pay: f64,
    pub last_month_tax_paid: f64,
    pub last_month_net_pay: f64,
    pub last_completed_month_gross_pay: f64,
    pub last_completed_month_tax_paid: f64,
    pub last_completed_month_net_pay: f64,
    pub last_month_spending: f64,
    pub last_month_goods_spending: f64,
    pub last_month_imported_goods_spending: f64,
    pub last_month_goods_units_bought: u32,
    pub last_month_needs_met_mask: u16,
    pub last_month_unmet_needs_mask: u16,
    pub last_month_severe_unmet_needs_mask: u16,
    pub consecutive_severe_unmet_months: u8,
    pub last_month_chronic_unmet: bool,
    pub last_month_loan_payment: f64,
    pub last_month_new_loan: f64,
    pub last_month_savings_interest: f64,
    pub last_month_rent_paid: f64,
    pub last_month_mortgage_payment: f64,
    pub last_completed_month_spending: f64,
    pub last_completed_month_goods_spending: f64,
    pub last_completed_month_imported_goods_spending: f64,
    pub last_completed_month_goods_units_bought: u32,
    pub last_completed_month_loan_payment: f64,
    pub last_completed_month_new_loan: f64,
    pub last_completed_month_savings_interest: f64,
    pub last_completed_month_rent_paid: f64,
    pub last_completed_month_mortgage_payment: f64,
    pub housing: HousingStatus,
    /// Consecutive months this person has been unemployed (resets to 0 on employment).
    pub months_unemployed: u32,
    /// Consecutive months this person received unemployment benefits.
    pub months_on_benefits: u32,
    pub lifetime_migrations: u32,
    pub last_migration_day: Option<u64>,
    pub essential_stock_units: [f32; ESSENTIAL_STOCK_SLOT_COUNT],
    pub superficial_stock_units: [u16; SUPERFICIAL_GOOD_SLOT_COUNT],
    pub superficial_stock_consumption_carry: [f32; SUPERFICIAL_GOOD_SLOT_COUNT],
    /// Deposit held at a bank in another country, if any.
    pub foreign_deposit: Option<ForeignDeposit>,
}

impl Person {
    #[cfg(test)]
    pub fn new(id: u32, job_type_id: JobTypeId, employer_company_id: u16, name_seed: u32) -> Self {
        Self::new_with_profile(
            id,
            EmploymentStatus::Employed {
                job_type_id,
                employer_company_id,
            },
            name_seed,
            30 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        )
    }

    pub fn new_with_profile(
        id: u32,
        employment: EmploymentStatus,
        name_seed: u32,
        age_days: u16,
        education_track: EducationTrack,
    ) -> Self {
        let mut person = Self {
            id,
            birth_country_id: 0,
            current_country_id: 0,
            employment,
            employment_start_day: None,
            agreed_monthly_salary: 0.0,
            cash_balance: 0.0,
            deposit_bank_id: None,
            household_id: id,
            name_seed,
            age_days,
            education_track,
            habilitation: HabilitationLevel::NoSchoolYet,
            loan: None,
            last_month_gross_pay: 0.0,
            last_month_tax_paid: 0.0,
            last_month_net_pay: 0.0,
            last_completed_month_gross_pay: 0.0,
            last_completed_month_tax_paid: 0.0,
            last_completed_month_net_pay: 0.0,
            last_month_spending: 0.0,
            last_month_goods_spending: 0.0,
            last_month_imported_goods_spending: 0.0,
            last_month_goods_units_bought: 0,
            last_month_needs_met_mask: 0,
            last_month_unmet_needs_mask: 0,
            last_month_severe_unmet_needs_mask: 0,
            consecutive_severe_unmet_months: 0,
            last_month_chronic_unmet: false,
            last_month_loan_payment: 0.0,
            last_month_new_loan: 0.0,
            last_month_savings_interest: 0.0,
            last_month_rent_paid: 0.0,
            last_month_mortgage_payment: 0.0,
            last_completed_month_spending: 0.0,
            last_completed_month_goods_spending: 0.0,
            last_completed_month_imported_goods_spending: 0.0,
            last_completed_month_goods_units_bought: 0,
            last_completed_month_loan_payment: 0.0,
            last_completed_month_new_loan: 0.0,
            last_completed_month_savings_interest: 0.0,
            last_completed_month_rent_paid: 0.0,
            last_completed_month_mortgage_payment: 0.0,
            housing: HousingStatus::Unhoused,
            months_unemployed: 0,
            months_on_benefits: 0,
            lifetime_migrations: 0,
            last_migration_day: None,
            essential_stock_units: [0.0; ESSENTIAL_STOCK_SLOT_COUNT],
            superficial_stock_units: [0; SUPERFICIAL_GOOD_SLOT_COUNT],
            superficial_stock_consumption_carry: [0.0; SUPERFICIAL_GOOD_SLOT_COUNT],
            foreign_deposit: None,
        };
        match person.employment {
            EmploymentStatus::Employed { job_type_id, .. }
            | EmploymentStatus::EmployedAtBank { job_type_id, .. } => {
                person.agreed_monthly_salary = salary_for_job(job_type_id);
                person.employment_start_day = Some(0);
            }
            EmploymentStatus::Unemployed => {}
        }
        person.refresh_habilitation_from_age();
        person
    }

    pub fn age_years(&self) -> u8 {
        let years = self.age_days / DAYS_PER_YEAR;
        years as u8
    }

    pub fn is_employed(&self) -> bool {
        matches!(
            self.employment,
            EmploymentStatus::Employed { .. } | EmploymentStatus::EmployedAtBank { .. }
        )
    }

    pub fn job_type_id(&self) -> Option<JobTypeId> {
        match self.employment {
            EmploymentStatus::Unemployed => None,
            EmploymentStatus::Employed { job_type_id, .. }
            | EmploymentStatus::EmployedAtBank { job_type_id, .. } => Some(job_type_id),
        }
    }

    pub fn employer_company_id(&self) -> Option<u16> {
        match self.employment {
            EmploymentStatus::Employed {
                employer_company_id,
                ..
            } => Some(employer_company_id),
            _ => None,
        }
    }

    pub fn employer_bank_id(&self) -> Option<u16> {
        match self.employment {
            EmploymentStatus::EmployedAtBank { bank_id, .. } => Some(bank_id),
            _ => None,
        }
    }

    pub fn set_employment(
        &mut self,
        employer_company_id: u16,
        job_type_id: JobTypeId,
        current_day: u64,
    ) {
        self.set_employment_with_salary(
            employer_company_id,
            job_type_id,
            salary_for_job(job_type_id),
            current_day,
        );
    }

    pub fn set_employment_with_salary(
        &mut self,
        employer_company_id: u16,
        job_type_id: JobTypeId,
        salary: f64,
        current_day: u64,
    ) {
        let next_start_day = match self.employment {
            EmploymentStatus::Employed {
                job_type_id: current_job_id,
                employer_company_id: current_employer_id,
            } if current_job_id == job_type_id && current_employer_id == employer_company_id => {
                self.employment_start_day.or(Some(current_day))
            }
            _ => Some(current_day),
        };
        self.employment = EmploymentStatus::Employed {
            job_type_id,
            employer_company_id,
        };
        self.employment_start_day = next_start_day;
        self.agreed_monthly_salary = salary.max(0.0);
    }

    pub fn set_bank_employment(
        &mut self,
        bank_id: u16,
        job_type_id: JobTypeId,
        salary: f64,
        current_day: u64,
    ) {
        let next_start_day = match self.employment {
            EmploymentStatus::EmployedAtBank {
                job_type_id: current_job_id,
                bank_id: current_bank_id,
            } if current_job_id == job_type_id && current_bank_id == bank_id => {
                self.employment_start_day.or(Some(current_day))
            }
            _ => Some(current_day),
        };
        self.employment = EmploymentStatus::EmployedAtBank {
            job_type_id,
            bank_id,
        };
        self.employment_start_day = next_start_day;
        self.agreed_monthly_salary = salary.max(0.0);
    }

    pub fn remap_employer_company_id(&mut self, employer_company_id: u16) {
        if let EmploymentStatus::Employed {
            employer_company_id: current_employer_id,
            ..
        } = &mut self.employment
        {
            *current_employer_id = employer_company_id;
        }
    }

    /// Clears employment and returns who employed the person.
    pub fn clear_employment(&mut self) -> Option<ClearedEmployment> {
        let previous = self.employment;
        self.employment = EmploymentStatus::Unemployed;
        self.employment_start_day = None;
        self.agreed_monthly_salary = 0.0;
        match previous {
            EmploymentStatus::Unemployed => None,
            EmploymentStatus::Employed {
                job_type_id,
                employer_company_id,
            } => Some(ClearedEmployment::Company {
                company_id: employer_company_id,
                job_type_id,
            }),
            EmploymentStatus::EmployedAtBank {
                job_type_id,
                bank_id,
            } => Some(ClearedEmployment::Bank {
                bank_id,
                job_type_id,
            }),
        }
    }

    /// Convenience: clears employment and returns `(company_id, job_type_id)` only for
    /// company-employed people. Used by callers that only care about company employees.
    pub fn clear_company_employment(&mut self) -> Option<(u16, JobTypeId)> {
        match self.clear_employment()? {
            ClearedEmployment::Company {
                company_id,
                job_type_id,
            } => Some((company_id, job_type_id)),
            ClearedEmployment::Bank { .. } => None,
        }
    }

    pub fn can_seek_job(&self) -> bool {
        self.age_years() >= self.education_track.minimum_job_seek_age()
    }

    pub fn is_working_age(&self) -> bool {
        let age = self.age_years();
        (18..=65).contains(&age)
    }

    pub fn is_labor_force_participant(&self) -> bool {
        self.is_working_age() && self.can_seek_job()
    }

    pub fn refresh_habilitation_from_age(&mut self) {
        let age = self.age_years();
        self.habilitation = if age < 12 {
            HabilitationLevel::NoSchoolYet
        } else if age < 18 {
            HabilitationLevel::BasicSchool
        } else {
            match self.education_track {
                EducationTrack::BasicOnly => HabilitationLevel::BasicSchool,
                EducationTrack::HighSchool => HabilitationLevel::HighSchool,
                EducationTrack::Bachelor => {
                    if age >= 21 {
                        HabilitationLevel::HigherLearning
                    } else {
                        HabilitationLevel::HighSchool
                    }
                }
                EducationTrack::Doctor => {
                    if age >= 23 {
                        HabilitationLevel::HigherLearning
                    } else {
                        HabilitationLevel::HighSchool
                    }
                }
            }
        };
    }

    pub fn increment_age_one_day(&mut self) {
        self.age_days = self.age_days.saturating_add(1);
        if self.age_days % 365 == 0 {
            self.refresh_habilitation_from_age();
        }
    }

    pub fn monthly_salary(&self) -> f64 {
        if self.is_employed() {
            self.agreed_monthly_salary.max(0.0)
        } else {
            0.0
        }
    }

    pub fn record_migration(&mut self, destination_country_id: u16, current_day: u64) {
        self.current_country_id = destination_country_id;
        self.lifetime_migrations = self.lifetime_migrations.saturating_add(1);
        self.last_migration_day = Some(current_day);
    }

    pub fn reset_monthly_trackers(&mut self) {
        self.last_completed_month_gross_pay = self.last_month_gross_pay;
        self.last_completed_month_tax_paid = self.last_month_tax_paid;
        self.last_completed_month_net_pay = self.last_month_net_pay;
        self.last_completed_month_spending = self.last_month_spending;
        self.last_completed_month_goods_spending = self.last_month_goods_spending;
        self.last_completed_month_imported_goods_spending = self.last_month_imported_goods_spending;
        self.last_completed_month_goods_units_bought = self.last_month_goods_units_bought;
        self.last_completed_month_loan_payment = self.last_month_loan_payment;
        self.last_completed_month_new_loan = self.last_month_new_loan;
        self.last_completed_month_savings_interest = self.last_month_savings_interest;
        self.last_completed_month_rent_paid = self.last_month_rent_paid;
        self.last_completed_month_mortgage_payment = self.last_month_mortgage_payment;
        self.last_month_gross_pay = 0.0;
        self.last_month_tax_paid = 0.0;
        self.last_month_net_pay = 0.0;
        self.last_month_spending = 0.0;
        self.last_month_goods_spending = 0.0;
        self.last_month_imported_goods_spending = 0.0;
        self.last_month_goods_units_bought = 0;
        self.last_month_needs_met_mask = 0;
        self.last_month_unmet_needs_mask = 0;
        self.last_month_severe_unmet_needs_mask = 0;
        self.last_month_chronic_unmet = false;
        self.last_month_loan_payment = 0.0;
        self.last_month_new_loan = 0.0;
        self.last_month_savings_interest = 0.0;
        self.last_month_rent_paid = 0.0;
        self.last_month_mortgage_payment = 0.0;
    }
}

#[cfg(test)]
#[path = "person/tests.rs"]
mod tests;
