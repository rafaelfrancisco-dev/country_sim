use super::*;
use crate::sim::{
    company::Company,
    country::Country,
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
};

mod helpers_and_bench;
use helpers_and_bench::*;
mod core;
mod market_behaviors;
