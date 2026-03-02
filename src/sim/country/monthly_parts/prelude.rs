use super::*;
use crate::sim::{
    bank::{DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS, max_principal_for_payment},
    good::{GOODS_COUNT, good_by_id},
    job::{common_job_types, salary_for_job},
    money::round_cents,
    params::{GovernmentBackfillParams, SwitchCadence},
    stock_market::{ListedEquity, StockMarket},
};
#[cfg(feature = "bench-trace")]
use crate::time_op;
use std::cmp::Ordering;
