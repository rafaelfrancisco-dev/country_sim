# Benchmarking Protocol

## Goal
Provide one deterministic, repeatable benchmark path for before/after performance comparisons.

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
  --flamegraph target/bench/flamegraph.svg
```

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

## Rollout Gates
- Correctness gate:
  - determinism/parity tests pass.
- Performance gate:
  - higher `ticks_per_sec` and lower goods-market flamegraph share versus baseline.
- Regression gate:
  - full `cargo test` passes.
  - monthly accounting and invariant tests continue passing.
