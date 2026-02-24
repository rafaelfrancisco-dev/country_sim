# Country Sim: Game Context and Phase 1 Requirements

## Vision and Phase 1 Scope
Country Sim is a terminal-based simulation game where each engine tick maps to one in-game day.

> Money simulation Phase 2 requirements are documented in [money-simulation-requirements.md](money-simulation-requirements.md).
>
> Running analytics tab behavior is documented in [running-charts-tab.md](running-charts-tab.md).
>
> Tick architecture and async monthly scheduling are documented in [ticking-mechanism.md](ticking-mechanism.md).

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
- Fields:
  - core navigation: `day`, `countries`, `selected_country`, `paused`, `people_page`, `selected_person_row`, `details_view`, `selection_focus`
  - running tabs: `active_tab` (`Main`/`Charts`)
  - chart scope: `chart_scope` (`SelectedCountry`/`WorldAggregate`)
  - chart history: `world_history`, `country_history` (monthly snapshots, max 120 points)
- Tick behavior: increments `day`, ticks all countries, and captures history at month boundaries (`day % 30 == 0`)
- Country navigation is bounded (clamped)
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

Running tabs:
- `Main`: countries + people lists and detail pages.
- `Charts`: 2x2 chart dashboard for:
  - population
  - total money supply
  - created money outstanding
  - bank reserve
- Scope on `Charts`: toggle between `SelectedCountry` and `WorldAggregate`.
- History source for `Charts`: monthly snapshots, rolling 120 points.

Controls:
- `Tab` / `Shift+Tab`: switch `Main`/`Charts` tab
- `Space`: pause/resume
- `n`: step one day (paused only)
- Main tab:
  - `Left`/`Right`: switch selection focus between countries and people
  - `Up`/`Down`: move selected row inside the focused list
  - `j`/`k`: page previous/next inside the focused list
  - `Enter`: open details for the focused selection (`Country` or `Person`)
  - `Esc`: close active details and return to list view
- Charts tab:
  - `t`: toggle scope (`SelectedCountry`/`WorldAggregate`)
  - `<` / `>`: previous/next selected country context
- existing quit/suspend shortcuts remain active

### Person Details View
- `Enter` from people list opens person details for the selected row.
- Country and people selection are mutually exclusive via `selection_focus`; only one is actively selected at a time.
- The details view is full-screen within the running layout.
- Details include: `Citizen ID`, `Name`, `Income`, `Expense`, `Daily Net`, `Balance`, `Country`, and `Day`.
- `Esc` closes details and returns to list view.
- If no person is available for selection, details view is not opened.

### Country Details View
- `Enter` from country list opens country details for the selected country row.
- The details view is full-screen within the running layout.
- Details include: `Country ID`, `Name`, `Day`, `Population`, `Total Balance`, `Avg Net/Day`, and active people page.
- `Esc` closes details and returns to list view.

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
- `World` day progression, country and people navigation bounds
- chart history lifecycle (initial point, monthly capture, rolling retention cap)
- tab/scope command behavior (`Main`/`Charts`, scope toggle, chart country cycle)
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
- richer drill-down analytics views
