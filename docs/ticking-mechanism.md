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
- `World` goods market uses two-phase execution:
  - Phase A (parallel): build purchase intents from snapshot budgets/cash and quote templates.
  - Phase B (single-writer): apply intents in deterministic key order `(round, person_idx, country_rr_order)`.
- Money aggregates and payroll accumulation use cent-based deterministic reductions, then convert back to `f64`.
- World generation remains deterministic for a fixed seed.
- Runtime entropy is intentionally targeted to:
  - daily deaths
  - daily births (Poisson draw)
  - discretionary superficial-good purchases when stock is empty and cash allows

### Explicit Sequential Exceptions
- Bank-dependent loan origination/repayment commits remain sequential where bank mutation ordering is business-significant.
- Snapshot/history writes remain sequential after all country ticks complete.

## History Capture
- Initial snapshot is captured at world creation.
- Additional snapshots are captured at each monthly boundary after country monthly processing.
- History retention remains capped (`HISTORY_LIMIT = 120`).

## Rationale
- Daily demographics and labor are stateful and mutate the same entities as monthly economics.
- Synchronous month processing avoids overwrite/drift risks from async snapshot replacement.
- Benchmarking protocol for deterministic performance comparisons: [docs/benchmarking.md](benchmarking.md)
