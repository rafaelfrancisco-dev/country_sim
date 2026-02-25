# country_sim

[![CI](https://github.com//country_sim/workflows/CI/badge.svg)](https://github.com//country_sim/actions)

A toy app...

## Recent UI Additions
- Running views (`Main` and `Charts`) switched via `Tab` / `Shift+Tab` (no dedicated tabs box)
- Monthly trend charts for:
  - population
  - total money supply
  - created money outstanding
  - bank reserve

## Documentation
- [Game context](docs/game-context.md)
- [Running charts tab](docs/running-charts-tab.md)
- [Money simulation requirements](docs/money-simulation-requirements.md)
- [Demographics and labor model](docs/demographics-labor-model.md)
- [Benchmarking protocol](docs/benchmarking.md)

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
  - `COUNTRY_SIM_SEED` env var is used when `--seed` is not provided
  - `m` toggles simulation mode at runtime from the running UI
- The script enables debug symbols by default for useful function names (`profile=flamegraph`).
- Disable symbols explicitly when needed: `FLAMEGRAPH_DEBUG_SYMBOLS=0 ./scripts/run-flamegraph.sh`
- On macOS, default mode is interactive attach so the TUI/game opens.
- Force classic `cargo flamegraph` mode with: `MODE=cargo ./scripts/run-flamegraph.sh`
- The script no longer auto-opens `flamegraph.svg` when profiling ends.
