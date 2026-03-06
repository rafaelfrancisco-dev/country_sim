# Essential Needs Validation (2026-03-06)

## Scope
This document now has two checkpoints:
- the original post-redesign baseline
- the structural calibration pass that adds essential-capacity seeding, per-good shortage diagnostics, and essential-only `people_agg` headline masking

Both runs are one-year fixed-seed headless validations.

This run checks two things:
- the simulation still completes cleanly over a full year
- the new any/severe/chronic unmet signals are exported and directionally interpretable

The newer run materially improves realism, but it still does **not** fully hit the aggressive target band.

## Structural Calibration Pass

### Commands
```bash
cargo test generator::tests
cargo test backfill
cargo test market_behaviors
cargo test collect_country_data_populates_per_good_essential_export_metrics

cargo bench -q --features manual-harness --bench manual_export_csv
```

### Outputs
- `/tmp/country_sim_year1/country_metrics_000000005eedc0de.csv`
- `/tmp/country_sim_year1/people_agg_000000005eedc0de.csv`

### Results
#### 100k, 1 country, 365 days
- completed 12 monthly exports without harness failure
- average any-unmet share: `14.58%`
- average severe-unmet share: `14.58%`
- average chronic-unmet share: `3.76%`
- final-month any-unmet share: `0.01%`
- final-month severe-unmet share: `0.01%`
- final-month chronic-unmet share: `0.00%`
- average CPI: `1.0444`
- average water unmet share: `0.00%`
- average food unmet share: `0.00%`
- average housing unmet share: `0.01%`
- average electricity unmet share: `14.57%`
- average medicine unmet share: `2.68%`

### Interpretation
- Stability: the year-long headless run still completes cleanly after the calibration changes.
- Major improvement: the essential-only headline fell from the prior `~97.9% / 95.1% / 78.4%` baseline to `14.6% / 14.6% / 3.8%` in the same 100k scenario.
- Metric fix: `people_agg.count_with_unmet_needs` now correctly masks to essential goods only, so the headline no longer includes non-essential deprivation spillover.
- Remaining gap: electricity still front-loads too much unmet in months 1–4, which keeps severe unmet above the aggressive `<5%` target even though late-year unmet is effectively zero.

## Commands
```bash
cargo test generator::tests
cargo test backfill
cargo test market_behaviors
cargo test government_spending
cargo test fiscal_policy
cargo test

EXPORT_CSV_TEST_DIR=/tmp/country_sim_essentials_10k \
STOCK_EXPORT_COUNTRIES=1 \
STOCK_EXPORT_THOUSANDS=10 \
STOCK_EXPORT_DAYS=365 \
cargo bench -q --features manual-harness --bench manual_export_csv

EXPORT_CSV_TEST_DIR=/tmp/country_sim_essentials_100k \
STOCK_EXPORT_COUNTRIES=1 \
STOCK_EXPORT_THOUSANDS=100 \
STOCK_EXPORT_DAYS=365 \
cargo bench -q --features manual-harness --bench manual_export_csv
```

`cargo test` currently fails only on `tests/src_line_limit.rs`, which reports several pre-existing oversized Rust files elsewhere in the repository. The model/runtime tests added for this change passed.

## Outputs
- `/tmp/country_sim_essentials_10k/country_metrics_000000005eedc0de.csv`
- `/tmp/country_sim_essentials_10k/people_agg_000000005eedc0de.csv`
- `/tmp/country_sim_essentials_100k/country_metrics_000000005eedc0de.csv`
- `/tmp/country_sim_essentials_100k/people_agg_000000005eedc0de.csv`

## Results
### 10k, 1 country, 365 days
- completed 12 monthly exports without harness failure
- average monthly unmet events: `34,230.25`
- average monthly severe unmet events: `34,107.75`
- average monthly chronic unmet people: `6,901.33`
- average any-unmet share: `93.81%`
- average severe-unmet share: `85.09%`
- average chronic-unmet share: `68.59%`
- final-month any-unmet share: `87.87%`
- final-month severe-unmet share: `73.65%`
- final-month chronic-unmet share: `73.39%`
- average CPI: `1.0724`
- average annualized inflation rate: `0.21%`

### 100k, 1 country, 365 days
- completed 12 monthly exports without harness failure
- average monthly unmet events: `395,536.08`
- average monthly severe unmet events: `394,406.50`
- average monthly chronic unmet people: `78,810.00`
- average any-unmet share: `97.90%`
- average severe-unmet share: `95.09%`
- average chronic-unmet share: `78.38%`
- final-month any-unmet share: `96.65%`
- final-month severe-unmet share: `91.86%`
- final-month chronic-unmet share: `91.59%`
- average CPI: `1.0733`
- average annualized inflation rate: `-0.24%`

## Interpretation
- Stability: the redesigned model holds for a one-year headless run. The export harness completed and produced the expected monthly CSVs at both 10k and 100k scale.
- Reporting quality: the new severe/chronic fields behave as intended. Severe unmet is below broad unmet, and chronic unmet trails severe rather than collapsing into the same number on month one.
- Realism: the model is still not close to real life. Even after buffering and household/shared-utility demand, the broad and severe unmet shares remain implausibly high.
- Macro side: CPI/inflation stayed comparatively tame, so the remaining gap looks more like essential-need calibration/supply structure than an economy-wide monetary blow-up.

## Next Calibration Targets
- reduce stockable essential demand and/or raise effective supply further for food, water, electricity, and medicine
- make household formation and shelter sharing more realistic than the current deterministic grouping heuristic
- revisit transfer/backfill thresholds now that normalized month-end unmet replaces the old noisy daily miss counting
- calibrate against explicit target bands instead of directional plausibility once the chronic unmet baseline is materially lower
