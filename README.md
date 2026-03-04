# country_sim

[![CI](https://github.com//country_sim/workflows/CI/badge.svg)](https://github.com//country_sim/actions)

A toy app...

## CSV Export
Pass `--export-csv <DIR>` at startup to write one row per country per completed month to four CSV files in `<DIR>`:

| File | Content |
|------|---------|
| `country_metrics_<seed>.csv` | Fiscal and monetary aggregates |
| `people_agg_<seed>.csv` | Population, employment, income, loan aggregates |
| `company_agg_<seed>.csv` | Company-level production and profit aggregates |
| `events_<seed>.csv` | Event counts (births, deaths, distress layoffs, quits/retirements, IPOs, ...) |

Files are keyed by the world seed in hex so re-runs with different seeds never overwrite each other. The flag is opt-in: omitting it has zero runtime cost (the snapshot pass is completely skipped).

See [docs/csv-export.md](docs/csv-export.md) for full column schemas and usage examples.

## Recent UI Additions
- Six running tabs switched via `Tab` / `Shift+Tab`: `Main`, `Goods`, `Banks`, `Stocks`, `Charts`, `Events`
- Unified running navigation: `w/s` (move), `a/d` (focus), `PageUp/PageDown` (page), `,/.` (country cycle)
- Setup now includes editable `Currency Symbol` (default `€`) so money displays can be customized per run
- Money labels across tables, details, charts, stocks, and events now consistently include the selected currency symbol and compact large values (for example `11500` as `11.5k`)
- Country policy overrides page in Main tab country details (`o`): editable tax brackets/rates and fiscal transfer thresholds per country (`r` resets to managed defaults)
- Monthly trend charts with pagination (4x2 per page, 4 pages total):
  - page 1: liquidity and money-balance metrics (population, supply, cash totals, reserve/gap)
  - page 2: fiscal and external-sector metrics (treasury/budget, taxes/spending, debt, trade flows)
  - page 3: prices, stress, and market-risk metrics (CPI/inflation/rates, unmet needs, stock risk)
  - page 4: interest rates and credit diagnostics (deposit/loan/spread, real rates, credit ratios)
- Events log with scrollable history of births, deaths, loans, debt distress, and government transfers
  - `/` to search, `o` to toggle sort order (newest/oldest first), `f` to cycle category filter
  - category filter options: `All`, `Stock Market`, `Births/Deaths`, `Bankruptcies & Distress`, `Loans & Debt`, `Companies & Labor`, `Government & Policy`
  - search and category filters are combined (intersection)
- Person details use fixed two-page paging (`Page 1: details`, `Page 2: related events`)
  - uses the same shared events filter/search/sort state as the Events tab
- Company details in Goods tab now use fixed four-page paging:
  - page 1: detailed company overview
  - page 2: company history charts
  - page 3: related events (shared Events search/filter/sort controls)
  - page 4: current employees table (`Name`, `Job`, `Tenure (mo)`, `Pay/mo`)
- Bank details in Banks tab now use fixed four-page paging:
  - page 1: detailed bank overview (liquidity, capital, run/resolution state)
  - page 2: per-bank history charts (reserve, deposits, created money, equity, run intensity)
  - page 3: related events (shared Events search/filter/sort controls)
  - page 4: current depositors table (`Type`, `Entity`, `Balance`, `Share`)
- Government spending system:
  - Monthly unemployment benefits paid to working-age (18-65) unemployed adults
  - Basic needs safety net tops up cash for people with unmet essential needs
  - Eligible people can receive both transfers in the same month (benefit first, then essential-needs top-up)
  - Treasury balance, monthly gov spending, budget balance, and gov debt tracked per country
- Public utility pricing guardrails:
  - Only state-owned/public-utility companies are price-capped (private firms remain uncapped)
  - Cap base rises with both CPI and money-supply growth to avoid a fixed nominal anchor
  - Public-utility monthly repricing includes affordability-pressure relief (allows downward movement under stress)
- Banking system additions:
  - Countries now support multiple banks with explicit `Public` / `Private` kinds
  - People and companies are assigned to a deposit bank (`deposit_bank_id`) and savings interest is paid per bank
  - Monthly private-bank run dynamics include withdrawal pressure, depositor switching, and resolution outcomes (insured support + uninsured haircut)
  - Monthly bank formation path added (private founder-capital formation + stress-gated public expansion)
  - New banking events: `BankFounded`, `BankRunStarted`, `BankRunWithdrawal`, `BankResolutionStarted`, `BankResolved`, `BankDepositHaircut`

## Documentation
- [Game context](docs/game-context.md)
- [Running charts tab](docs/running-charts-tab.md)
- [Money simulation requirements](docs/money-simulation-requirements.md)
- [Demographics and labor model](docs/demographics-labor-model.md)
- [Benchmarking protocol](docs/benchmarking.md)
- [CSV export](docs/csv-export.md)

## Benchmarking
- Canonical command:
```bash
./scripts/bench-sim.sh \
  --seed 0x5eedc0de \
  --countries 5 \
  --population-per-country 1000000 \
  --days 180 \
  --threads 8 \
  --sim-mode max-speed \
  --out-json target/bench/latest.json \
  --out-csv target/bench/latest.csv \
  --flamegraph target/bench/flamegraph.svg
```
- Optional trace run (feature-gated, benchmark-only):
```bash
./scripts/bench-sim.sh \
  --seed 0x5eedc0de \
  --countries 4 \
  --population-per-country 500000 \
  --days 100 \
  --threads 8 \
  --sim-mode max-speed \
  --out-json target/bench/trace-metrics.json \
  --out-csv target/bench/trace-metrics.csv \
  --trace-enabled \
  --trace-out-csv target/bench/trace-days.csv \
  --trace-summary-json target/bench/trace-summary.json
```
- Run the exact command before any performance work and save:
  - `target/bench/baseline.json`
  - `target/bench/baseline.csv`
  - `target/bench/baseline-flamegraph.svg`
- Run the same command after each change and compare against baseline:
  - higher `ticks_per_sec`
  - lower `avg_tick_ms` and `p95_tick_ms`
  - lower market-loop share in flamegraph
- Benchmark outputs include:
  - `ticks_per_sec`
  - `avg_tick_ms`
  - `p95_tick_ms`
  - `total_wall_clock_ms`
  - `scenario`
  - `thread_count`
  - `world_state_hash`
  - `timestamp`
- `--sim-mode` must be `max-speed`; other values are rejected by the benchmark runner.

## Runtime Invariants
- Default runtime builds skip invariant validation in `GameEngine::apply` to keep the hot path fast.
- Enable runtime invariant warnings explicitly with:
```bash
cargo run --features runtime-invariants -- [args...]
```
- The same flag can be used in tests when validating warning emission:
```bash
cargo test --features runtime-invariants
```

## Banking Validation Commands
- Targeted banking scenarios + 10-year month-level invariant check:
```bash
cargo test sim::country::tests::company_dynamics::bank_runs_and_deposits:: -- --nocapture
```
- Existing long-horizon acceptance run used for regression validation:
```bash
cargo test sim::world::tests::core::generated_world_3x15k_730_days_meets_stability_floor -- --ignored --exact
```

## Profiling
- Run the game with flamegraph support: `./scripts/run-flamegraph.sh`
- Pass game args through the script, for example: `./scripts/run-flamegraph.sh --tick-rate 8 --frame-rate 120`
- Runtime controls:
  - `--seed <decimal|0xhex>` world generation seed (optional)
  - `--sim-mode realtime|max-speed` (default: `realtime`)
  - `--fast-ui-hz <FLOAT>` (default: `2.0`, used in `max-speed` mode)
  - `--export-csv <DIR>` write monthly CSV exports to directory (opt-in, zero cost when omitted)
  - `COUNTRY_SIM_SEED` env var is used when `--seed` is not provided
  - `m` toggles simulation mode at runtime from the running UI
- The script enables debug symbols by default for useful function names (`profile=flamegraph`).
- Disable symbols explicitly when needed: `FLAMEGRAPH_DEBUG_SYMBOLS=0 ./scripts/run-flamegraph.sh`
- On macOS, default mode is interactive attach so the TUI/game opens.
- Force classic `cargo flamegraph` mode with: `MODE=cargo ./scripts/run-flamegraph.sh`
- The script no longer auto-opens `flamegraph.svg` when profiling ends.
