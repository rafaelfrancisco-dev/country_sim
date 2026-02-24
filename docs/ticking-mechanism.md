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
- Runtime `Action::Tick` maps to `GameCommand::Tick` and advances `World::tick_day`.
- Manual step (`n`) maps to `GameCommand::StepOneDay` and also advances one day when paused.

## Monthly Boundary Model
- Month length: `DAYS_PER_MONTH = 30`.
- Completed boundaries: `world.day / 30`.
- Processed months are tracked per country as `country.month`.

## Execution Model
Simulation commands are processed by a dedicated worker thread that owns the authoritative `GameEngine`.

Implications:
- Daily and monthly writes remain totally ordered (single worker applies commands sequentially).
- Worker publishes a lightweight UI snapshot after each command.
- UI thread enqueues commands and renders from the latest snapshot, so rendering and input stay responsive even while a tick is running.
- No merge/reconciliation step is required between daily and monthly state.

## History Capture
- Initial snapshot is captured at world creation.
- Additional snapshots are captured at each monthly boundary after country monthly processing.
- History retention remains capped (`HISTORY_LIMIT = 120`).

## Rationale
- Daily demographics and labor are stateful and mutate the same entities as monthly economics.
- Synchronous month processing avoids overwrite/drift risks from async snapshot replacement.
