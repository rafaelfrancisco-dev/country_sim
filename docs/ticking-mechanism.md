# Ticking Mechanism and `Tickable` Trait

## Purpose
Describe how daily ticks and monthly processing are ordered in the current worker-backed model.

## Core Trait
`src/sim/tickable.rs` defines:

```rust
pub trait Tickable {
    fn tick_day(&mut self);
}
```

## Implementations
- `Person`:
  - Currently no direct per-day mutation through `Person::tick_day`.
  - Daily lifecycle mutations are coordinated by `Country`.
- `Country`:
  - `tick_day_without_month`: advances one day and runs daily lifecycle (age/deaths/births/job market).
  - `tick_day`: runs `tick_day_without_month`, then runs `process_month` on 30-day boundaries.
- `World`:
  - `tick_day`: increments world day, ticks all countries, captures chart snapshot on monthly boundary.

## Tick Entry Points
- Runtime progression is driven by a dedicated simulation clock thread owned by the simulation worker.
- Manual step (`n`) maps to `GameCommand::StepOneDay` and advances one day when paused.

## Monthly Boundary Model
- Month length: `DAYS_PER_MONTH = 30`.
- Completed boundaries: `world.day / 30`.
- Processed months are tracked per country as `country.month`.

## Execution Model
Simulation commands are processed by a dedicated worker thread that owns the authoritative `GameEngine`.
The worker is paired with a separate clock thread that schedules `GameCommand::Tick` independently from UI rendering.

Implications:
- Daily and monthly writes remain totally ordered at command level (single worker applies commands sequentially).
- Worker publishes an engine snapshot (`GamePhase` + runtime stats).
- UI state (tabs, selection, details view) is owned by the UI component and applied locally from `UiCommand`.
- Render cadence can be throttled (for max-speed mode) without changing simulation progression speed.
- Simulation mode (`realtime` / `max-speed`) is shared through an atomic mode flag read by the clock thread.

Inside a single `World::tick_day`, country updates now run in parallel by deterministic country chunks via `rayon`.

## Parallelism and Determinism
- `World` ticks countries in parallel using fixed chunk partitioning.
- `Country` daily/monthly internals use deterministic staged pipelines:
  - Parallel intent/decision phases over chunked entities.
  - Sequential commit phases in stable index order when side effects touch shared state.
- `World` goods market (`run_daily_goods_economy`) runs several parallel regions after country ticks complete:
  - `consume_superficial_stock_daily`: each country's people processed via `par_chunks_mut`. Static per-slot daily usage is hoisted out of the loop. No cross-person state.
  - Demand init in `process_good_round_robin`: `pending_demands` per country filled via `par_chunks_mut` zipped with `par_chunks` over people. `demand_units_for_person_good` is a pure hash-based function.
  - `recompute_aggregates`: runs `par_iter_mut` over all countries after the goods market completes.
  - `process_discretionary_superficial_purchases` uses a two-phase pattern per country:
    - Phase A (parallel planning): `par_chunks` over people, each chunk uses `thread_rng()`, produces a `Vec<person_idx>` of intending buyers. Read-only on all country/company state.
    - Phase B (sequential apply): iterates the buyer list, re-fetches the current cheapest supplier (respecting live inventory), re-checks affordability, then executes each purchase.
- Essential-goods round-robin uses two-phase execution:
  - Phase A (parallel): build purchase intents from snapshot budgets/cash and quote templates.
  - Phase B (single-writer): apply intents in deterministic key order `(round, person_idx, country_rr_order)`.
- Money aggregates and payroll accumulation use cent-based deterministic reductions, then convert back to `f64`.
- World generation remains deterministic for a fixed seed.
- Runtime entropy is intentionally targeted to:
  - daily deaths
  - daily births (Poisson draw)
  - discretionary superficial-good purchases when stock is empty and cash allows (per-chunk `thread_rng()`)

### Explicit Sequential Exceptions
- Bank-dependent loan origination/repayment commits remain sequential where bank mutation ordering is business-significant.
- Snapshot/history writes remain sequential after all country ticks complete.

## History Capture
- Initial snapshot is captured at world creation.
- Additional snapshots are captured at each monthly boundary after country monthly processing.
- History retention remains capped (`HISTORY_LIMIT = 120`).

## Monthly Boundary Execution Order
At each 30-day boundary, `World::tick_day` runs the following steps in order:

1. `Country::process_month_end` (parallel) — payroll, taxes, gov spending, loan repayments, company decisions; pushes events to `country.pending_events`
2. `adjust_dynamic_country_tariffs` — world-level tariff rebalancing
3. `capture_snapshot` — records `MetricPoint` into the rolling history window
4. `push_export_snapshot` *(only when `--export-csv` was passed)* — collects a full `CountryExportData` snapshot while monthly tracker fields are still populated and `pending_events` still holds the day-30 events; appends to `world.export_queue`
5. `Country::reset_month_trackers` (parallel) — zeroes all `monthly_*` accumulators
6. `drain_country_events` — moves `pending_events` from each country into `world.event_log`

Step 4 must precede step 5 so that monthly accumulators are readable, and precede step 6 so that month-end events are still in `pending_events` rather than the global log (which would require filtering by day to isolate the current month).

## Rationale
- Daily demographics and labor are stateful and mutate the same entities as monthly economics.
- Synchronous month processing avoids overwrite/drift risks from async snapshot replacement.
- Benchmarking protocol for deterministic performance comparisons: [docs/benchmarking.md](benchmarking.md)
