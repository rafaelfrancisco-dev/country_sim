# Country Sim: Game Context and Phase 1 Requirements

## Vision and Phase 1 Scope
Country Sim is a terminal-based simulation game where each engine tick maps to one in-game day.

Phase 1 delivers a foundational simulation slice:
- countries and citizens (`Person`) as core entities
- daily economy updates on every tick
- setup and visualization in Ratatui
- strict separation of pure game logic from UI adapter code

Out of scope for phase 1:
- markets, prices, jobs, migration, diplomacy, wars, policies, persistence, networking

## Entity Model and Tick Semantics
### `Tickable`
- Shared trait: `tick_day(&mut self)`
- Implemented by `Person` and `Country`

### `Person`
- Fields: `id`, `income`, `expense`, `balance`, `name_seed`
- Daily update: `balance += income - expense`
- Starts with `balance = 0`

### `Country`
- Fields: `id`, `name`, `people`, `total_balance`, `avg_net_per_day`
- Daily update: tick every person, then recompute aggregates
- `total_balance`: sum of all person balances
- `avg_net_per_day`: average of `(income - expense)` over all citizens

### `World`
- Fields: `day`, `countries`, `selected_country`, `paused`, `people_page`
- Tick behavior: increments `day`, ticks all countries
- Country navigation wraps (`left/right`)
- People pagination is bounded (clamped)

## Startup Flow and Validation Rules
Startup begins inside a Ratatui setup screen.

User inputs:
- `country_count` (allowed: 1..=50)
- `millions_per_country` (allowed: 1..=20)

Rules:
- one population value applies to all countries
- values must be whole numbers
- validation errors block world generation
- memory warning is non-blocking

## Economy Model and Generation Defaults
Generation defaults:
- income range: 80..=200
- expense range: 40..=160
- initial balance: 0

Population model:
- 1 simulated `Person` = 1 citizen
- generated population per country = `millions_per_country * 1_000_000`

Name generation:
- country names: one word, deterministic from seed
- person names: two words, deterministic from seed, generated on demand for display

## UI and Controls Contract
## Setup Screen
- Edit inputs with digits/backspace
- `Tab` / `Shift+Tab` switch active field
- `Enter` submits and starts simulation when valid

## Running Screen
Header displays:
- day
- run state (`Running` / `Paused`)
- selected country
- total citizens

Main panels:
- countries table: `Country`, `Population`, `Total Balance`, `Avg Net/Day`
- people table (selected country page): `Citizen ID`, `Name`, `Income`, `Expense`, `Balance`

Controls:
- `Space`: pause/resume
- `n`: step one day (paused only)
- `Left`/`Right`: selected country
- `j`/`k`: people page next/prev
- existing quit/suspend shortcuts remain active

## Performance and Memory Policy
- per-country limit is capped by input validation (`<= 20 million`)
- no global citizen cap in phase 1
- memory estimate uses `size_of(Person) * estimated_citizens`
- if estimate exceeds 2.5 GiB, show warning but allow generation

## Testing Strategy and Logic-Complete Definition
Game logic is testable without Ratatui or Crossterm dependencies.

Coverage requirements:
- `Person` tick behavior and expected-range stability
- `Country` aggregate consistency after ticks
- `World` day progression, country wrapping, people-page bounds
- setup validation and deterministic error strings
- memory warning threshold behavior (non-blocking)
- generator count/range/determinism guarantees
- integration-style command sequence from setup to running/ticking

"Logic complete" means all simulation decisions and invariants are verified through pure unit tests; UI tests are not required to validate business rules.

## Deferred Features / Roadmap After Phase 1
- economic sectors and job systems
- production/consumption markets and dynamic pricing
- policy levers and government budgets
- migration and demographic dynamics
- save/load persistence and replay
- analytics/graphs and richer drill-down views
