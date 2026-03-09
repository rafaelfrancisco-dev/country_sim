# Performance Analysis

Analysis of performance pitfalls in the simulation codebase, organized by severity.

---

## CRITICAL

*(All five critical issues have been fixed — see commit history.)*

---

## HIGH

*(Issues #6, #7, #8, #10 have been fixed — see commit history.)*

**9. Sequential apply phase after parallel planning** (`goods_market/round_robin.rs:141–204`)
Rayon threads plan in parallel, then all purchases are applied serially per good. Fix: batch purchases by supplier and apply in parallel (complex due to shared buyer-country aggregates; deferred).

---

## MEDIUM

**11. `HousingStatus` clone on death** (`country/daily/mod.rs:146`)
`people[read_idx].housing.clone()` can clone a `MortgageAccount` variant. Fix: take ownership with `mem::take()`.

**12. Multi-pass rent collection** (`monthly_housing.rs:80–96`)
Builds an intermediate `Vec<RentEntry>` over all renters before applying. Fix: stream directly — iterate once and apply.

**13. `prepare_death_flags()` redundant `.fill(0)`** (`country/daily/mod.rs:102–126`)
`resize_with(|| 0)` already zero-fills; the subsequent `.fill(0)` is a no-op re-zeroing. Fix: remove the extra fill.

**14. `increment_age_one_day()` calls `refresh_habilitation_from_age()` unconditionally** (`person.rs:420–423`)
~730M redundant calls in a 2M-person × 365-day run when habilitation only changes on age boundaries. Fix: cache `last_habilitation_age` and skip unless a threshold was crossed.

**15. `loan_appetite_multiplier()` / `rate_response_normalized_gap()` recomputed multiple times per country per tick**
These policy helpers are called from multiple monthly functions. Fix: compute once at the start of `process_month_end()` and pass as a small struct.

**16. `monthly_rate = (1 + annual).powf(1/12) - 1` computed per person per foreign deposit** (`cross_country_banking.rs:68`)
Fix: pre-compute monthly rate factor once per bank before the person loop.

---

## LOW

**17. Rayon dispatch decision re-evaluated every tick** (`world/tick.rs:76–88`)
`dispatch_countries()` checks `countries.len()` at runtime every call. Fix: compute once on `World` creation, store as `bool`.

**18. `try_borrow_mut()` fallback branch per country per day** (`country/daily/mod.rs:41–51`)
Defensive branch with allocation fallback. This is fine as long as nested rayon doesn't fire often; document the invariant instead of silently falling back.

**19. `HashSet<u64>` for active traders when count may be small** (`stock_market/matching.rs:32`)
If trader counts stay small, `Vec + dedup` is faster. Profile to decide.

**20. `format!()` / `.to_string()` on event label strings at construction time**
Constant labels like disaster types allocated as `String` at event creation. Fix: `&'static str` or a `u8` discriminant, converting to string only in the UI.

---

## Design-Level

**D1. Demand buffer index order** (`goods_market/day_cache.rs`)
The flat buffer `[person_idx * 9 + slot]` causes strided access when iterating per-good. Reordering to `[good_slot][person_idx]` improves cache locality for the per-good consumption phase.

**D2. Event log unbounded growth**
At high event rates with millions of agents, the log can spike. A fixed-size ring buffer is safer than trim-on-overflow.

**D3. Bank health score recomputed from scratch in multiple callers**
Cache it as a `last_health_score: f64` field updated once per month, invalidated on reserve changes.

---

## Recommended Fix Order

| Priority | Issue | Est. Gain | Status |
|---|---|---|---|
| 1 | String cloning in events — `Arc<str>` for name fields (Critical #1) | 15–25% | Done |
| 2 | History snapshot single-pass (Critical #5) | 5–10% | Done |
| 3 | `collect()` in discretionary hot path (Critical #2) | 10–20% | Done |
| 4 | `pending_non_zero` incremental init (Critical #3) | 5–10% | Done |
| 5 | Retain-based round-robin active set (Critical #4) | 3–8% | Done |
| 6 | Tariff matrix as World field (High #6) | 3–5% | Done |
| 7 | HashMap → Vec for bank positions (High #7) | 1–2% | Done |
| 8 | `SimulationParams` no-clone (High #8) | 2–3% | Done |
| 9 | Sequential goods apply phase (High #9) | 2–4% | Open |
| 10 | `refresh_habilitation_from_age` gating (Medium #14) | 2–5% | Open |
