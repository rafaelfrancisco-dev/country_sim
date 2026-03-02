use super::World;
use crate::sim::{
    good::{GOODS_COUNT, GoodId, common_goods},
    money::{from_cents, round_cents, to_cents},
    person::SUPERFICIAL_GOOD_SLOT_COUNT,
};
use rand::Rng;
use rayon::prelude::*;
use std::sync::Arc;

mod types;
mod entrypoints;
mod round_robin;
mod quotes_and_tariffs;
mod demand_sampling;

pub(in crate::sim::world) use types::*;
pub(in crate::sim::world) use demand_sampling::*;
