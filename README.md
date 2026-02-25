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

## Profiling
- Run the game with flamegraph support: `./scripts/run-flamegraph.sh`
- Pass game args through the script, for example: `./scripts/run-flamegraph.sh --tick-rate 8 --frame-rate 120`
- The script enables debug symbols by default for useful function names (`profile=flamegraph`).
- Disable symbols explicitly when needed: `FLAMEGRAPH_DEBUG_SYMBOLS=0 ./scripts/run-flamegraph.sh`
- On macOS, default mode is interactive attach so the TUI/game opens.
- Force classic `cargo flamegraph` mode with: `MODE=cargo ./scripts/run-flamegraph.sh`
- The script no longer auto-opens `flamegraph.svg` when profiling ends.
