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
| `events_<seed>.csv` | Event counts (births, deaths, layoffs, IPOs, ...) |

Files are keyed by the world seed in hex so re-runs with different seeds never overwrite each other. The flag is opt-in: omitting it has zero runtime cost (the snapshot pass is completely skipped).

See [docs/csv-export.md](docs/csv-export.md) for full column schemas and usage examples.

## Recent UI Additions
- Five running tabs switched via `Tab` / `Shift+Tab`: `Main`, `Goods`, `Stocks`, `Charts`, `Events`
- Unified running navigation: `w/s` (move), `a/d` (focus), `PageUp/PageDown` (page), `,/.` (country cycle)
- Monthly trend charts (3x2 grid):
  - population
  - total money supply
  - created money outstanding
  - bank reserve
  - treasury balance
  - budget balance (tax revenue - government spending)
- Events log with scrollable history of births, deaths, loans, debt distress, and government transfers
  - `/` to search, `o` to toggle sort order (newest/oldest first)
- Government spending system:
  - Monthly unemployment benefits paid to working-age (18-65) unemployed adults
  - Basic needs safety net tops up cash for people with unmet essential needs
  - Treasury balance, monthly gov spending, budget balance, and gov debt tracked per country

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

