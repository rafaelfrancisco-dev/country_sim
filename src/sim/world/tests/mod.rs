use super::*;
use crate::sim::{
    company::Company,
    country::{Country, DAYS_PER_MONTH},
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
    tickable::Tickable,
};

mod helpers_and_bench;
use helpers_and_bench::*;
mod core;
mod market_behaviors;
