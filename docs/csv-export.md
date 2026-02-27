# Monthly CSV Export

## Overview

Pass `--export-csv <DIR>` at startup to write one row per country per completed simulation month to four CSV files. The flag is fully opt-in: omitting it incurs zero runtime cost (the snapshot aggregation pass is skipped entirely).

```bash
cargo run --release -- --export-csv /tmp/sim_out
```

Files are created (or truncated) when the first month boundary is crossed. They are keyed by the world seed in lowercase hex so re-runs with different seeds never overwrite each other.

```
/tmp/sim_out/
  country_metrics_005eedc0de123456.csv
  people_agg_005eedc0de123456.csv
  company_agg_005eedc0de123456.csv
  events_005eedc0de123456.csv
```

---

## File Schemas

### `country_metrics_<seed>.csv`

Fiscal and monetary state per country per month.

| Column | Type | Description |
|--------|------|-------------|
| `month` | integer | Completed month number (1-based) |
| `country_id` | integer | Country ID |
| `country_name` | string | Country name |
| `population` | integer | Total living persons |
| `treasury_balance` | float | Government treasury cash balance |
| `monthly_tax_collected` | float | Income + tariff tax collected this month |
| `monthly_tariff_collected` | float | Import tariff revenue this month |
| `monthly_gov_spending` | float | Unemployment benefits + basic-needs transfers paid this month |
| `monthly_budget_balance` | float | `monthly_tax_collected − monthly_gov_spending` |
| `gov_debt` | float | Cumulative deficit-funded government spending |
| `monthly_household_spending` | float | Total household goods spending this month |
| `monthly_goods_import_spending` | float | Household spending on imported goods this month |
| `monthly_goods_export_revenue` | float | Revenue from goods exported to other countries this month |
| `monthly_unmet_need_events` | integer | Count of unmet-essential-need events this month |
| `monthly_essential_debt_spending` | float | Essential goods paid via negative cash (debt) this month |
| `total_money_supply` | float | `household_cash + company_cash + bank_reserve + treasury` |
| `household_cash_total` | float | Sum of all person cash balances |
| `company_cash_total` | float | Sum of all company cash balances |
| `bank_reserve_balance` | float | Bank reserve cash |
| `bank_created_money_outstanding` | float | Principal of outstanding bank-created loans |
| `expected_money_supply` | float | Initial money supply adjusted for created/destroyed money |
| `money_supply_gap` | float | `total_money_supply − expected_money_supply` |

**Invariant check:** `total_money_supply ≈ household_cash_total + company_cash_total + bank_reserve_balance + treasury_balance` (small floating-point drift expected).

---

### `people_agg_<seed>.csv`

Population and labour aggregates per country per month.

| Column | Type | Description |
|--------|------|-------------|
| `month` | integer | Completed month number |
| `country_id` | integer | Country ID |
| `country_name` | string | Country name |
| `population` | integer | Total living persons |
| `employed_count` | integer | Persons with an active employer |
| `unemployed_count` | integer | Persons without an employer |
| `unemployment_rate_pct` | float | `unemployed / (employed + unemployed) × 100` |
| `avg_age_years` | float | Mean age of the living population in years |
| `avg_cash_balance` | float | Mean person cash balance |
| `total_gross_pay` | float | Sum of last-month gross wages across all persons |
| `total_tax_paid` | float | Sum of last-month income tax paid across all persons |
| `total_goods_spending` | float | Sum of last-month goods spending across all persons |
| `count_with_active_loans` | integer | Persons currently carrying a loan |
| `avg_loan_balance` | float | Mean outstanding debt balance among persons with loans |
| `count_with_unmet_needs` | integer | Persons with at least one unmet essential good last month |

---

### `company_agg_<seed>.csv`

Company-level production and profit aggregates per country per month.

| Column | Type | Description |
|--------|------|-------------|
| `month` | integer | Completed month number |
| `country_id` | integer | Country ID |
| `country_name` | string | Country name |
| `total_companies` | integer | Number of active companies |
| `total_employees` | integer | Sum of `employee_count` across all companies |
| `total_open_positions` | integer | Sum of open job slots across all companies |
| `total_company_cash` | float | Sum of company cash balances |
| `total_revenue` | float | Sum of last-month sales revenue across all companies |
| `total_payroll` | float | Sum of last-month payroll paid across all companies |
| `total_profit` | float | Sum of last-month profit across all companies |
| `count_profitable` | integer | Companies with `last_month_profit > 0` |
| `count_unprofitable` | integer | Companies with `last_month_profit ≤ 0` |
| `total_units_produced` | integer | Sum of units produced last month |
| `total_units_sold` | integer | Sum of units sold last month (domestic + export) |
| `total_units_exported` | integer | Sum of units sold to other countries last month |
| `count_public_utility` | integer | Government-owned public utility companies |
| `count_backfill` | integer | Government essential-good backfill companies |
| `count_companies_with_loans` | integer | Companies carrying an active bank loan |

---

### `events_<seed>.csv`

Discrete simulation event counts per country per month. Each column is the count of that event type that occurred during the month (including month-end processing events).

| Column | Type | Description |
|--------|------|-------------|
| `month` | integer | Completed month number |
| `country_id` | integer | Country ID |
| `country_name` | string | Country name |
| `births` | integer | Total births (sum of `PopBirth.count`) |
| `deaths` | integer | Total deaths (sum of `PopDeath.count`) |
| `job_switches` | integer | Workers who changed employer |
| `layoffs_total` | integer | Total workers laid off (sum of `CompanyLayoffs.laid_off_count`) |
| `companies_founded` | integer | New startups created |
| `bankruptcies` | integer | Companies that went bankrupt |
| `wage_cuts` | integer | Companies that issued a wage cut |
| `loans_issued` | integer | New loan accounts opened |
| `loans_ended` | integer | Loans fully repaid |
| `loans_defaulted` | integer | Loans written off (person default) |
| `debt_distress_events` | integer | Missed loan payment events |
| `gov_transfers` | integer | Government benefit transfer batches paid |
| `stock_ipos` | integer | Companies that completed an IPO |
| `stock_delistings` | integer | Companies that were delisted |
| `stock_margin_calls` | integer | Person margin call liquidations |

---

## Implementation Notes

- Snapshot timing: the export pass runs after `capture_snapshot()` and **before** `Country::reset_month_trackers()`, so all `monthly_*` fields are still populated at collection time. Month-end events (from `process_month_end`) are still in `country.pending_events` at that point and are counted directly; events from days 1–29 are filtered from `world.event_log` by day range.
- The `CsvExporter` is opened lazily on the first month boundary, so the files are only created if the simulation actually runs past day 30.
- Each run opens files with `File::create` (truncate), so re-running with the same seed and output directory will overwrite the previous files.
- Relevant source: `src/csv_export.rs`, `src/sim/world/export.rs`, `src/sim/engine.rs` (`drain_export_queue`, `enable_export_queue`).
