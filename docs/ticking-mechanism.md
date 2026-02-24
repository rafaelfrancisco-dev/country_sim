# Ticking Mechanism and `Tickable` Trait

## Purpose
This document describes how simulation time advances, how the `Tickable` trait is used, and how monthly processing is executed asynchronously without overlapping runs.

## Core Trait
`src/sim/tickable.rs` defines:

```rust
pub trait Tickable {
    fn tick_day(&mut self);
}
```

The trait represents one in-game day of progression for a simulation entity.

## Where `Tickable` is Implemented
- `Person` (`src/sim/person.rs`):
  - Implements `tick_day`, currently with no daily mutations.
- `Country` (`src/sim/country.rs`):
  - `tick_day` increments day and runs monthly processing when day is divisible by 30.
  - `tick_day_without_month` increments day only (no monthly processing).
- `World` (`src/sim/world.rs`):
  - `tick_day` increments world day and calls `country.tick_day_without_month()` for each country.
  - Monthly processing is intentionally not executed inside `World::tick_day`.

## Tick Entry Points
Runtime tick events come from the TUI loop and are converted into `Action::Tick`.

Main flow:
1. `App` receives `Event::Tick` and forwards `Action::Tick`.
2. `Simulation::update` handles `Action::Tick`.
3. `Simulation` applies `GameCommand::Tick` to `GameEngine`.
4. `GameEngine` advances `World::tick_day()` when not paused.

Manual stepping path:
- `n` maps to `GameCommand::StepOneDay` (only applies when paused).
- This also advances `World::tick_day()` via `GameEngine`.

## Monthly Boundary Model
- Month length is defined by `DAYS_PER_MONTH = 30`.
- `World::completed_month_boundaries()` returns `world.day / 30`.
- Countries track processed months with `country.month`.

This separates:
- elapsed month boundaries (from day count)
- processed month count (from completed monthly jobs)

## Asynchronous Monthly Processing
Monthly work runs in `Simulation` as a background blocking task (`tokio::task::spawn_blocking`).

State fields used:
- `monthly_tick_inflight: Option<oneshot::Receiver<Vec<Country>>>`
- `pending_monthly_ticks: u64`
- `observed_month_boundaries: u64`

### Scheduling Rules
1. Reconcile current state on key points (`Tick`, `Render`, and running key commands):
   - poll completed monthly result
   - refresh backlog from month boundaries
   - start next monthly task if idle
2. Only one monthly task can run at a time.
3. If a boundary is reached while a monthly task is running, backlog is queued in `pending_monthly_ticks`.
4. When a task completes, `World::apply_processed_month` replaces country state and captures a monthly history snapshot.

## No-Overlap Guarantee
Overlap is prevented by checking `monthly_tick_inflight.is_some()` before spawning a new monthly task.

Result: monthly processing is serialized; the next monthly run starts only after the previous one completes.

## Daily Tick Pause for Catch-up
Daily progression is paused only when the simulation is more than one month behind:

- Compute:
  - `required_months = world.completed_month_boundaries()`
  - `processed_months = min(country.month)`
- Block daily advancement only when:
  - `required_months - processed_months > 1`

Implication:
- At day 30, monthly work can run in the background while daily ticks continue.
- If day 60 is reached before month 1 processing completes, daily ticks pause until catch-up reduces lag.

## History Capture
Monthly charts are updated when processed monthly data is applied:
- `World::apply_processed_month` updates countries and calls `capture_snapshot`.
- History retention remains capped (`HISTORY_LIMIT = 120`).

## Design Rationale
- Keep UI responsive during expensive monthly logic.
- Preserve deterministic, serialized monthly state transitions.
- Prevent unbounded monthly backlog while still allowing short-term overlap between daily progression and one in-flight monthly job.
