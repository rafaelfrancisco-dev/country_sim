# Monthly CSV Export

## Overview

Pass `--export-csv <DIR>` at startup to write one row per country per completed simulation month to four CSV files. The flag is still opt-in for CSV serialization and file IO, but the engine now maintains month-close aggregate caches for history/export so export-enabled runs avoid the old full rescans rather than paying for a separate recomputation pass.

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
| `monthly_tax_collected` | float | Income tax collected this month |
| `monthly_tariff_collected` | float | Export tariff revenue collected this month (goods sold abroad; revenue stays in seller's treasury) |
| `monthly_gov_spending` | float | Unemployment benefits + basic-needs transfers paid this month |
| `monthly_budget_balance` | float | `monthly_tax_collected − monthly_gov_spending` |
| `gov_debt` | float | Cumulative deficit-funded government spending |
| `monthly_household_spending` | float | Total household goods spending this month |
| `monthly_goods_import_spending` | float | Household spending on imported goods this month |
| `monthly_goods_export_revenue` | float | Revenue from goods exported to other countries this month |
| `monthly_unmet_need_events` | integer | Broad count of unmet essential events this month (`any unmet`) |
| `monthly_severe_unmet_need_events` | integer | Unmet essential events that crossed the severe-shortfall threshold |
| `monthly_chronic_unmet_need_count` | integer | People currently in chronic severe unmet status |
| `monthly_essential_debt_spending` | float | Essential goods paid via negative cash (debt) this month |
| `water_unmet_share_pct` | float | Share of people with unmet water needs last month |
| `water_severe_unmet_share_pct` | float | Share of people with severe unmet water needs last month |
| `water_producer_count` | integer | Active water-producing companies |
| `water_public_utility_count` | integer | Public utilities producing water |
| `water_inventory_cover_months` | float | End-of-month water inventory divided by monthly requirement |
| `food_unmet_share_pct` | float | Share of people with unmet food needs last month |
| `food_severe_unmet_share_pct` | float | Share of people with severe unmet food needs last month |
| `food_producer_count` | integer | Active food-producing companies |
| `food_public_utility_count` | integer | Public utilities producing food |
| `food_inventory_cover_months` | float | End-of-month food inventory divided by monthly requirement |
| `housing_unmet_share_pct` | float | Share of people with unmet housing needs last month |
| `housing_severe_unmet_share_pct` | float | Share of people with severe unmet housing needs last month |
| `housing_producer_count` | integer | Active housing companies |
| `housing_public_utility_count` | integer | Public utilities producing housing |
| `housing_inventory_cover_months` | float | Housing cover proxy (currently `0` because housing is not stock-tracked) |
| `electricity_unmet_share_pct` | float | Share of people with unmet electricity needs last month |
| `electricity_severe_unmet_share_pct` | float | Share of people with severe unmet electricity needs last month |
| `electricity_producer_count` | integer | Active electricity-producing companies |
| `electricity_public_utility_count` | integer | Public utilities producing electricity |
| `electricity_inventory_cover_months` | float | End-of-month electricity inventory divided by monthly requirement |
| `medicine_unmet_share_pct` | float | Share of people with unmet medicine needs last month |
| `medicine_severe_unmet_share_pct` | float | Share of people with severe unmet medicine needs last month |
| `medicine_producer_count` | integer | Active medicine-producing companies |
| `medicine_public_utility_count` | integer | Public utilities producing medicine |
| `medicine_inventory_cover_months` | float | End-of-month medicine inventory divided by monthly requirement |
| `total_money_supply` | float | `household_cash + company_cash + bank_reserve + treasury` |
| `household_cash_total` | float | Sum of all person cash balances |
| `company_cash_total` | float | Sum of all company cash balances |
| `bank_reserve_balance` | float | Aggregate reserve cash across all banks in the country |
| `bank_created_money_outstanding` | float | Aggregate principal of outstanding bank-created loans across all banks |
| `expected_money_supply` | float | Initial money supply adjusted for bank-created money and cumulative minted money |
| `money_supply_gap` | float | `total_money_supply − expected_money_supply` |
| `monthly_money_minted` | float | Money minted this month into the public-bank reserve |
| `cumulative_money_minted` | float | Cumulative minted money since simulation start |
| `bank_count_total` | integer | Total active banks (`public + private`) |
| `bank_count_public` | integer | Active public-bank count (normally 1) |
| `bank_count_private` | integer | Active private-bank count |
| `bank_health_avg_pct` | float | Mean health score across all banks (0-100) |
| `bank_health_min_pct` | float | Minimum bank health score in the country (0-100) |
| `bank_health_max_pct` | float | Maximum bank health score in the country (0-100) |
| `bank_health_public_pct` | float | Public-bank health score (0-100) |
| `bank_health_private_avg_pct` | float | Mean private-bank health score (0-100, 0 if none) |
| `bank_health_private_min_pct` | float | Min private-bank health score (0-100, 0 if none) |
| `bank_health_private_max_pct` | float | Max private-bank health score (0-100, 0 if none) |
| `bank_health_below_50_count` | integer | Number of banks with health below 50 |
| `bank_health_private_below_50_count` | integer | Number of private banks with health below 50 |
| `private_bank_deposit_liabilities` | float | Total deposits held in private banks |
| `private_bank_deposit_share_pct` | float | Private-bank deposit share (`private/total * 100`) |
| `private_people_depositor_share_pct` | float | Share of people depositing in private banks |
| `private_company_depositor_share_pct` | float | Share of companies depositing in private banks |
| `monthly_private_banks_founded` | integer | Private banks founded during this month |
| `cumulative_private_banks_founded` | integer | Cumulative private banks founded |
| `monthly_private_banks_closed` | integer | Private banks closed during this month |
| `cumulative_private_banks_closed` | integer | Cumulative private banks closed |
| `bonds_outstanding` | float | Face value of sovereign bonds currently outstanding |
| `bond_coupon_rate` | float | Annual coupon rate on current bond (0 if no bond) |
| `monthly_coupon_paid` | float | Bond coupon distributed to bank + retail holders this month |
| `monthly_bonds_issued` | float | Face value of bonds issued this month (0 if none) |
| `monthly_bonds_matured` | float | Face value of bonds matured this month (0 if none) |
| `cpi_index` | float | Consumer price index (basket of goods 0–7, base = 1.0 at generation) |
| `inflation_rate` | float | Annualized month-over-month inflation: `(cpi_new/cpi_old)^12 − 1` |
| `policy_rate` | float | Country-local central bank policy rate (Taylor Rule); world charts use a derived population-weighted aggregate instead |

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
| `employed_count` | integer | Labor-force participants with an active employer |
| `unemployed_count` | integer | Labor-force participants without an employer |
| `labor_force_count` | integer | `employed_count + unemployed_count` |
| `non_participating_count` | integer | Persons outside labor force (not counted in unemployment denominator) |
| `unemployment_rate_pct` | float | Labor-force unemployment: `unemployed_count / labor_force_count × 100` |
| `participation_rate_pct` | float | `labor_force_count / population × 100` |
| `headline_non_employed_rate_pct` | float | Broad non-employment share: `(population - employed_count) / population × 100` |
| `avg_age_years` | float | Mean age of the living population in years |
| `avg_cash_balance` | float | Mean person cash balance |
| `total_gross_pay` | float | Sum of last-month gross wages across all persons |
| `total_tax_paid` | float | Sum of last-month income tax paid across all persons |
| `total_goods_spending` | float | Sum of last-month goods spending across all persons |
| `count_with_active_loans` | integer | Persons currently carrying a loan |
| `avg_loan_balance` | float | Mean outstanding debt balance among persons with loans |
| `count_with_unmet_needs` | integer | Persons with at least one unmet essential good last month (masked to goods `0..=4`) |
| `count_with_severe_unmet_needs` | integer | Persons with at least one severe unmet essential good last month |
| `count_with_chronic_unmet_needs` | integer | Persons whose severe unmet status has persisted for multiple months |
| `monthly_immigrants` | integer | People who moved into the country this month |
| `monthly_emigrants` | integer | People who moved out of the country this month |
| `monthly_migrants_for_work` | integer | Migration arrivals/departures classified as work-driven |
| `monthly_migrants_for_distress` | integer | Migration arrivals/departures classified as distress-driven |

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
| `layoffs_total` | integer | Distress-driven layoffs only (sum of `CompanyLayoffs.laid_off_count`) |
| `separations_quits` | integer | Monthly quits from `LaborChurnMonthly` |
| `separations_retirements` | integer | Monthly retirements (age-gated 65+) from `LaborChurnMonthly` |
| `separations_layoffs_non_distress` | integer | Monthly non-distress layoffs from `LaborChurnMonthly` |
| `separations_total` | integer | `separations_quits + separations_retirements + separations_layoffs_non_distress` |
| `companies_founded` | integer | New startups created |
| `banks_founded` | integer | New private banks founded |
| `banks_closed` | integer | Private banks terminally closed/acquired |
| `bank_runs_started` | integer | Bank-run start events emitted |
| `bank_resolutions_started` | integer | Bank-resolution start events emitted |
| `bankruptcies` | integer | Companies that went bankrupt |
| `wage_cuts` | integer | Companies that issued a wage cut |
| `loans_issued` | integer | New loan accounts opened |
| `loans_ended` | integer | Loans fully repaid |
| `loans_defaulted` | integer | Loans written off (person default) |
| `debt_distress_events` | integer | Missed loan payment events |
| `gov_transfers` | integer | Government benefit transfer batches paid |
| `hires_total` | integer | Total unemployed→employed transitions during the month |
| `monthly_immigrants` | integer | People who moved into the country this month |
| `monthly_emigrants` | integer | People who moved out of the country this month |
| `monthly_migrants_for_work` | integer | Migration events driven by irresistible foreign offers |
| `monthly_migrants_for_distress` | integer | Migration events driven by prolonged unemployment/benefit distress |
| `stock_ipos` | integer | Companies that completed an IPO |
| `stock_delistings` | integer | Companies that were delisted |
| `stock_margin_calls` | integer | Person margin call liquidations |

---

## Implementation Notes

- Snapshot timing: month-close now refreshes cached export aggregates first, then runs `capture_snapshot()`, then `push_export_snapshot()`, and only then `Country::reset_month_trackers()`. This keeps all `monthly_*` fields readable while also letting export/history consume cached aggregates instead of rescanning all people, companies, banks, and essential-good state.
- Event timing: month-end events (from `process_month_end`) are still in `country.pending_events` when export rows are built, so they are counted directly; events from days 1–29 are filtered from `world.event_log` by day range.
- Migration timing: the world migration pass runs after country month-close and before snapshot/export capture, so exported population, labor counts, and migration counters already reflect that month's transfers.
- Essential unmet semantics: stockable essentials now buy toward a monthly target plus buffer and are scored at month-end. `count_with_unmet_needs` is now masked to essential goods only, so the headline people aggregate no longer mixes in non-essential deprivation.
- The `CsvExporter` is opened lazily on the first month boundary, so the files are only created if the simulation actually runs past day 30.
- Each run opens files with `File::create` (truncate), so re-running with the same seed and output directory will overwrite the previous files.
- Relevant source: `src/csv_export.rs`, `src/sim/world/export/`, `src/sim/world/tick.rs`, `src/sim/engine.rs` (`drain_export_queue`, `enable_export_queue`).

---

## 5. `stock_daily_country_<seed>.csv` — Daily Stock Market Country Summary

**Cadence**: one row per country per day (only days when stock market is enabled and the country has at least one listing).

| Column | Type | Description |
|---|---|---|
| `day` | integer | Simulation day (1-based) |
| `country_id` | integer | Country identifier |
| `country_name` | string | Country name |
| `listed_count` | integer | Number of equities currently listed |
| `trades_count` | integer | Number of matched fills executed today |
| `total_volume` | integer | Total shares traded today |
| `total_turnover` | decimal | Total cash turnover today (sum across all equities) |
| `index_value` | decimal | Composite market-cap index (base 1000 at first listing) |
| `bank_equity_exposure` | decimal | Bank portfolio market value |
| `bank_reserve_balance` | decimal | Aggregate country bank reserve at end of day |
| `active_traders_count` | integer | Unique traders with at least one order in the day |
| `buy_orders_count` | integer | Total buy orders submitted during the day |
| `sell_orders_count` | integer | Total sell orders submitted during the day |

---

## 6. `stock_daily_equity_<seed>.csv` — Daily Per-Equity Data

**Cadence**: one row per listed equity per day (same days as country file).

| Column | Type | Description |
|---|---|---|
| `day` | integer | Simulation day (1-based) |
| `country_id` | integer | Country identifier |
| `country_name` | string | Country name |
| `company_id` | integer | Company (equity) identifier |
| `ticker` | string | Equity ticker symbol |
| `close_price` | decimal | Day's closing price |
| `day_return_pct` | decimal | Daily return %: `(close − prev_close) / prev_close × 100` |
| `volume` | integer | Shares traded today |
| `turnover` | decimal | Cash value traded today |
| `trades_count` | integer | Number of matched fills for this equity |
| `bank_position_shares` | integer | Bank's current holding (shares) |
| `free_float_shares` | integer | Total publicly-held shares (person + bank) |

**Usage notes**:
- `|day_return_pct| ≤ max_daily_price_move_pct × 100` (default hard cap ±8%) by engine construction.
- `free_float_shares` changes at IPO (initial float + retained founder shares), on secondary offerings (new shares issued), and at delisting; share conservation invariant: `Σ person holdings + bank_position_shares == free_float_shares`.
- Daily stock files are written each tick (not just month-end), so they can be significantly larger than the monthly CSVs for long runs.

---

## Draining Stock Export Data

Stock export data uses a separate queue from the monthly export. Use `GameEngine::drain_stock_export_queues()` (returns `(Vec<StockDailyCountryExportData>, Vec<StockDailyEquityExportData>)`) after each tick:

```rust
engine.enable_export_queue();
for _ in 0..days {
    engine.apply(GameCommand::Tick);
    // Monthly
    for batch in engine.drain_export_queue() { ... }
    // Daily stock
    let (country_rows, equity_rows) = engine.drain_stock_export_queues();
    for row in &country_rows { exporter.write_stock_country_row(row)?; }
    for row in &equity_rows  { exporter.write_stock_equity_row(row)?;  }
}
```
