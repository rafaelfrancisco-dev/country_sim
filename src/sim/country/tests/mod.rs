use super::*;
use crate::sim::{
    bank::LoanAccount,
    company::build_default_companies,
    event::SimEvent,
    money::round_cents,
    params::{
        CompanyGrowthParams, FiscalParams, GovernmentBackfillParams, LaborMobilityParams,
        SimulationParams,
    },
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
};
use std::collections::HashSet;

mod helpers_and_invariants;
use helpers_and_invariants::*;
mod company_dynamics;
mod government_and_labor;
