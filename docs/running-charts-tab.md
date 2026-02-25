# Running Charts Tab

## Overview
The running simulation UI has four tabs:
- `Main`: country/people navigation and details screens.
- `Goods`: company and goods market view.
- `Charts`: a dashboard with trend charts for core economy metrics.
- `Events`: scrollable log of simulation events (births, deaths, loans, debt distress, government transfers).

Tab switching uses `Tab` and `Shift+Tab`. The active tab is shown in the header; there is no dedicated "Running Tabs" panel.

## Goals
- Keep the current simulation workflow intact in `Main`.
- Add quick visual trend analysis in `Charts` without leaving the running simulation.
- Reuse existing simulation state; do not add persistence or external storage.

## Tab Model and Controls
Global running controls (all tabs):
- `Tab` / `Shift+Tab`: switch tabs
- `Space`: pause/resume
- `n`: step one day while paused
- `m`: toggle simulation mode (`realtime` / `max-speed`)
- existing quit/suspend shortcuts

`Main` tab controls are unchanged:
- `Left` / `Right`, `Up` / `Down`, `j` / `k`, `Enter`, `Esc`

`Charts` tab controls:
- `t`: toggle chart scope
- `<` / `>`: change selected-country context

On `Charts`, list/details navigation keys are intentionally disabled.

`Events` tab controls:
- `↑` / `↓`: scroll event list
- `/`: activate search (type to filter, `Esc` to close, `Ctrl+u` to clear)
- `s`: toggle sort order (Newest First / Oldest First)

UI navigation commands are applied on the UI side from the latest engine snapshot and are not queued as simulation mutations.

## Charts and Metrics
The `Charts` tab renders six charts in a 3×2 grid (three rows, two columns):
- Population
- Total Money Supply
- Created Money Outstanding
- Bank Reserve
- Treasury Balance
- Budget Balance

Each chart is a line chart with dynamic axis bounds and month-based x-axis labels.

## Scope Toggle
Scope is single-mode (not overlaid):
- `SelectedCountry`: charts read from the currently selected country history.
- `WorldAggregate`: charts read from the aggregate history across all countries.

`t` toggles between these scopes.

## History Model
- Snapshot cadence: monthly only (captured at `day % 30 == 0`).
- Initial snapshot: month `0` at world creation.
- Retention: rolling max of `120` points.
- Storage:
  - `world_history`: aggregate series
  - `country_history`: per-country series (aligned by country index)

## Metric Semantics
- `population`: current simulated citizen count.
- `total_money_supply`: tracked money currently in circulation.
- `created_money_outstanding`: principal-derived created money still outstanding.
- `bank_reserve`: reserve balance accumulated by collected interest.
- `treasury_balance`: cumulative unspent government revenue (taxes minus benefit payments).
- `budget_balance`: monthly net fiscal position (tax collected − government spending); negative = deficit month.

`bank_reserve` is used as the “money created via interest” proxy in charts.

## Known Limitations
- History is in-memory only and resets when the app exits.
- Charts use monthly snapshots only; no daily granularity view.
- Scope is toggled one mode at a time (no dual-series overlay).

## Possible Extensions
- Persist chart history with save/load support.
- Add selectable chart windows and zoom.
- Add invariant drift chart (`money_supply_gap`) and event annotations.
