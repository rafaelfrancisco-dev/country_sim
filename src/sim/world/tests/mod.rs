use super::*;
use crate::sim::{
    company::Company,
    country::Country,
    generator::{GenerationConfig, generate_countries},
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
};
use rayon::ThreadPoolBuilder;
use std::{
    env, fs,
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

mod helpers_and_bench;
use helpers_and_bench::*;
mod core;
mod market_behaviors;

