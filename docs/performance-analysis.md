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

*(Issues #11–#16 have been fixed — see commit history.)*

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

**D1. Goods market runtime redesign** (`goods_market/day_cache.rs`, `goods_market/round_robin.rs`)
The old flat demand buffer and per-good scratch rebuild path have been replaced by a persistent world-owned runtime cache, sparse active-buyer queues for essentials, and deterministic weekly cohorts for discretionary superficial goods.

**D2. Event log unbounded growth**
At high event rates with millions of agents, the log can spike. A fixed-size ring buffer is safer than trim-on-overflow.

**D3. Export/history aggregate rescans**
Month-close export and history now use cached country aggregates plus cached bank-derived metrics instead of rescanning people and recomputing bank exposure during serialization.

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
| 9 | Sequential goods apply phase (High #9) | 2–4% | Deferred |
| 10 | `refresh_habilitation_from_age` gating (Medium #14) | 2–5% | Open |
