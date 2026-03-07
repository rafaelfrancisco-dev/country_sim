# Benchmarking Protocol

## Goal
Provide one canonical benchmark path for before/after performance comparisons.

## Standard Scenario
- `seed=0x5eedc0de`
- `countries=5`
- `population_per_country=1_000_000`
- `days=180`
- `threads=8`
- `sim_mode=max-speed`

## Canonical Command
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
  --thread-sweep \
  --flamegraph target/bench/flamegraph.svg
```

## Optional Trace Mode (Feature-Gated)
- Enable per-day phase tracing for benchmark diagnosis:
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
- `--trace-enabled` compiles the harness with `--features bench-trace`.
- If a 100-day run is inconclusive, the script auto-runs a 300-day fallback with `_300d` suffixed artifacts.
- Trace code is not included in default release builds unless `bench-trace` is explicitly enabled.

## Result Files
- JSON (`--out-json`) and CSV (`--out-csv`) contain:
  - `ticks_per_sec`
  - `avg_tick_ms`
  - `p95_tick_ms`
  - `total_wall_clock_ms`
  - `scenario`
  - `thread_count`
  - `world_state_hash`
  - `timestamp`
- `--thread-sweep` appends deterministic runs for `1,2,4,8,max` threads into the same JSON/CSV output so hash mismatches and weak scaling show up in one artifact.
- With `--trace-enabled`, the JSON also includes a compact per-phase report for country daily, goods market, stock market, month-boundary work, and event draining.
- Flamegraph (`--flamegraph`) is generated from the same ignored benchmark harness.

## Baseline Workflow
1. Run the canonical command on the current baseline.
2. Save outputs as:
   - `target/bench/baseline.json`
   - `target/bench/baseline.csv`
   - `target/bench/baseline-flamegraph.svg`
3. Implement one change set.
4. Re-run the exact same command and compare against baseline artifacts.

## Deterministic Market Ordering Contract
The goods market apply phase uses a fixed total order key:
- `(round, person_idx, country_rr_order)`

Where:
- `round` is the round-robin pass number for the current good/day.
- `person_idx` is the buyer index inside a country.
- `country_rr_order` is the round-robin country offset from the cycle start.

Planning runs in parallel, but apply is single-writer and always uses this deterministic key order.

## Compiler Profile
The `[profile.release]` section uses `opt-level = 3` (not `"s"`). The `"s"` setting optimises for binary size and suppresses autovectorisation and aggressive inlining on arithmetic-heavy hot loops. With `lto = true` and `codegen-units = 1` already set, `opt-level = 3` lets LLVM autovectorise inner hash loops, `f64` arithmetic chains, and `round_cents` paths.

## Rollout Gates
- Correctness gate:
  - invariants and accounting tests pass.
  - generation is deterministic for the same seed at day 0.
- Performance gate:
  - higher `ticks_per_sec` and lower goods-market flamegraph share versus baseline.
- Regression gate:
  - full `cargo test` passes.
  - monthly accounting and invariant tests continue passing.
