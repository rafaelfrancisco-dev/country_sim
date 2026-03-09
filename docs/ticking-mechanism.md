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
  - `tick_day`: runs the shared world daily/monthly pipeline through non-traced hooks.
  - `tick_day_with_trace` (`bench-trace` only): runs the same pipeline through traced hooks and emits `DayTraceRow`.

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

Worker internals are split by responsibility:
- `command_processor`: applies `GameCommand`/`UiCommand` and emits warning events.
- `export_pipeline`: drains export queues and writes CSV rows.
- `snapshot_publisher`: handles publish cadence and snapshot build/publish.
- `scheduler`: drives realtime/max-speed tick scheduling.
- `health`: owns worker health state (`Healthy` / `Fatal`) and event queueing.

Implications:
- Daily and monthly writes remain totally ordered at command level (single worker applies commands sequentially).
- Worker publishes an engine snapshot (`GamePhase` + runtime stats).
- UI state (tabs, selection, details view) is owned by the UI component and applied locally from `UiCommand`.
- Render cadence can be throttled (for max-speed mode) without changing simulation progression speed.
- Simulation mode (`realtime` / `max-speed`) is shared through an atomic mode flag read by the clock thread.

## Worker Health and Error Signaling
- Worker exposes explicit health state and event queue to the simulation component.
- Fatal internal failures (for example, poisoned snapshot lock) transition worker health to `Fatal`, emit a fatal worker event, and stop accepting new commands.
- CSV/export failures are non-fatal: they are emitted as export-error events while simulation progression continues.
- `Simulation` drains worker events during reconcile/update and forwards them through the existing `Action::Error` reporting path.

Inside a single `World::tick_day`, country updates now run in parallel by deterministic country chunks via `rayon`.

## Parallelism and Determinism
- `World` ticks countries in parallel using fixed chunk partitioning.
- `Country` daily/monthly internals use deterministic staged pipelines:
  - Parallel intent/decision phases over chunked entities.
  - Sequential commit phases in stable index order when side effects touch shared state.
- `World` goods market (`run_daily_goods_economy`) runs several parallel regions after country ticks complete:
  - `produce_goods_daily`: outer `par_iter_mut` over countries; sequential inner company loop. Countries are fully independent.
  - `consume_superficial_stock_daily`: outer `par_iter_mut` over countries; sequential inner person loop. Static per-slot daily usage is hoisted out of the loop. No cross-person state.
  - `GoodsMarketDayCache` is now a persistent world-owned runtime cache rather than a fresh per-day build. Supplier topology and quote templates are invalidated separately, while daily work only resets cursors and sparse active-demand state.
  - Stockable essentials maintain persistent per-person deficit/target state. The daily round-robin path activates only buyers with non-zero deficit for the current good instead of sweeping the full population for every essential.
  - `process_good_round_robin` (×9 goods): prepares sparse active buyers for that good, plans intents in parallel, then applies them in deterministic order. Fairness remains round-robin by seller, but the hot path now scales with active buyers rather than total population.
  - `adjust_dynamic_country_tariffs`: outer `par_iter_mut` over countries; sequential work within each country.
  - `recompute_aggregates`: runs `par_iter_mut` over all countries after the goods market completes.
  - `process_discretionary_superficial_purchases` now uses deterministic weekly cohorts:
    - each person is assigned to one of 7 stable purchase cohorts by hash
    - each superficial good keeps a candidate set of zero-stock people
    - the daily pass filters only the active cohort inside that candidate set, then applies purchases sequentially against live inventory
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
At each 30-day boundary, the shared world tick pipeline runs the following steps in order:

1. `Country::process_month_end` (parallel) — payroll, taxes, stockable-essential month-end scoring, gov spending, housing settlement, chronic unmet scoring, loan repayments, company decisions; pushes events to `country.pending_events`
2. `process_cross_market_stock_dividends` + stale-position cleanup — pays dividends to local and foreign holders using the listing country's cash/tax rules and removes holdings in delisted securities
3. `process_cross_country_deposits`
4. `adjust_dynamic_country_tariffs` — world-level tariff adjustment:
   - tariff steps up when a good is exported while domestic stock is depleted and unmet need exists
   - otherwise tariff mean-reverts toward the baseline tariff rate
5. `update_policy_rate` + propagate to all countries — updates world policy rate and assigns it to each country before history capture
6. `refresh_monthly_export_caches` — updates country-level monthly/export aggregates and cached bank-derived metrics while month-close data is still live
7. `capture_snapshot` — records `MetricPoint` into the rolling history window
8. `push_export_snapshot` *(only when `--export-csv` was passed)* — serializes `CountryExportData` from the cached monthly aggregates while `pending_events` still holds the day-30 events; appends to `world.export_queue`
9. `Country::reset_month_trackers` (parallel) — zeroes all `monthly_*` accumulators
10. `drain_country_events` — moves `pending_events` from each country into `world.event_log`

Step 8 must precede step 9 so that monthly accumulators are readable, and precede step 10 so that month-end events are still in `pending_events` rather than the global log (which would require filtering by day to isolate the current month).

### Essential-Need Scoring
- Stockable essentials (water, food, electricity, medicine) no longer mark monthly unmet directly from a failed daily purchase.
- Daily goods-market passes buy toward a monthly requirement plus a bounded buffer.
- Month-end scoring classifies:
  - broad unmet (`any unmet`)
  - severe unmet (large remaining shortfall)
  - chronic unmet (severe unmet sustained across consecutive months)
- Housing is scored separately at month-end from household shelter status plus rent/mortgage outcomes, not from the round-robin goods market.

## Trace/Non-Trace Drift Prevention
- `World` now has a single internal tick pipeline (`tick_day_with_hooks`) and a hook trait (`WorldTickHooks`).
- Non-traced runs use `StandardWorldTickHooks`.
- `bench-trace` runs use `TraceWorldTickHooks` to collect timings while preserving identical execution order.
- This removes duplicated control flow between traced and non-traced tick paths.

## Rationale
- Daily demographics and labor are stateful and mutate the same entities as monthly economics.
- Synchronous month processing avoids overwrite/drift risks from async snapshot replacement.
- Benchmarking protocol for deterministic performance comparisons: [docs/benchmarking.md](benchmarking.md)
