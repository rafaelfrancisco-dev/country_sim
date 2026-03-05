# Short-Run Banking Diagnostics (2026-03-05)

## Scope
One-year headless simulation diagnostics after banking-behavior fixes.

## Commands
```bash
cargo test -q --test src_line_limit
cargo test -q bank_runs_and_deposits

EXPORT_CSV_TEST_DIR=/tmp/country_sim_diag1_fix \
STOCK_EXPORT_COUNTRIES=1 \
STOCK_EXPORT_THOUSANDS=100 \
STOCK_EXPORT_DAYS=365 \
cargo bench -q --features manual-harness --bench manual_export_csv

EXPORT_CSV_TEST_DIR=/tmp/country_sim_diag2_fix \
STOCK_EXPORT_COUNTRIES=2 \
STOCK_EXPORT_THOUSANDS=100 \
STOCK_EXPORT_DAYS=365 \
cargo bench -q --features manual-harness --bench manual_export_csv
```

## CSV Outputs
- `/tmp/country_sim_diag1_fix/country_metrics_000000005eedc0de.csv`
- `/tmp/country_sim_diag1_fix/events_000000005eedc0de.csv`
- `/tmp/country_sim_diag2_fix/country_metrics_000000005eedc0de.csv`
- `/tmp/country_sim_diag2_fix/events_000000005eedc0de.csv`

## KPI Results
### 1 country (100k)
- `bank_month_below50_ratio`: `0.3871`
- `private_bank_month_below50_ratio`: `0.0000`
- `avg_public_health_pct`: `42.50`
- `avg_private_health_pct`: `74.60`
- `avg_private_deposit_share_pct`: `23.66`
- `max_private_banks`: `2`
- `events_banks_founded`: `1`
- `events_bank_runs_started`: `0`

### 2 countries (100k each)
- `bank_month_below50_ratio`: `0.3871`
- `private_bank_month_below50_ratio`: `0.0000`
- `avg_public_health_pct`: `42.50`
- `avg_private_health_pct`: `68.41`
- `avg_private_deposit_share_pct`: `23.39`
- `max_private_banks`: `2`
- `events_banks_founded`: `2`
- `events_bank_runs_started`: `0`

## Observed Delta vs Prior Baseline
Previous baseline (before this fix pass) had:
- `bank_month_below50_ratio ~= 1.00`
- `avg_private_health_pct ~= 0`
- `avg_private_deposit_share_pct ~= 0`
- `max_private_banks = 1`
- recurring bank-run starts each month

After fixes:
- not all banks are pinned below 50
- private banks remain healthy and attract a meaningful deposit share (~23%)
- private-bank count grows above 1 when capacity allows
- run dynamics are no longer permanently triggered in this scenario
