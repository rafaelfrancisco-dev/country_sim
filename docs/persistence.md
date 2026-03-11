# Save / Load Persistence

## Overview

The simulation supports saving and loading full world state to disk via SQLite. Each save is a single `.db` file stored beside the executable. Saves capture the complete world — all countries, people, companies, banks, stock markets, histories, and the event log — so a loaded game is indistinguishable from a live one at the same day.

---

## Quick Start

**Start a new run and save it while it is running:**

1. Launch the simulation (`cargo run`).
2. On the setup screen, fill in Countries / Thousands / Seed as usual.
3. Edit the **Save Name** field (Tab through the fields or focus it directly).
4. Press `Enter` to generate the world.
5. While the simulation is running, press `Shift+S` to save.

**Load a previous save:**

1. Launch the simulation.
2. Tab to the **Existing Saves** field on the setup screen.
3. Use `w` / `s` to navigate the save list.
4. Press `Enter` to load the selected save. The simulation resumes from that day.

---

## Setup Screen Fields

| Field | Keys | Description |
|-------|------|-------------|
| Countries | `0-9`, `Backspace` | Number of countries (1–50) |
| Thousands per Country | `0-9`, `Backspace` | Starting population per country in thousands (10–20000) |
| Generation Seed | `0-9`, `Backspace` | Hex or decimal world seed |
| Save Name | any printable char, `Backspace` | Name for this save file. Defaults to a random `save-<hex8>` slug. Non-alphanumeric characters (except `-` and `_`) are replaced with `_`; truncated at 64 characters. |
| Existing Saves | `w` / `s`, `Enter` | Navigate the save list; `Enter` loads. Shown only when at least one save exists. |

Field navigation: `Tab` / `Shift+Tab` cycles through all five fields in order.

---

## Running Mode

| Key | Action |
|-----|--------|
| `Shift+S` | Save the current run to the active save name. Shows a green status message on success, red on error. |

The active save name is shown in the bottom status bar while the simulation is running. It defaults to the slug chosen at setup. Re-pressing `Shift+S` overwrites the existing save file (upsert).

---

## Save Files

Save files are written to the same directory as the executable:

```
<executable_dir>/
  my-save.db
  save-a3f12c77.db
  ...
```

Each `.db` file is a self-contained SQLite database with a normalized relational schema. There is no JSON in the database.

Key tables:

| Group | Tables |
|-------|--------|
| Save metadata | `save_metadata`, `world_state`, `world_tariff_rates` |
| Countries | `countries`, `country_sim_params`, `country_export_tariffs`, `country_import_tariffs`, `country_fx_reserves_by_source` |
| People | `people`, `person_loans`, `person_mortgages`, `person_foreign_deposits`, `person_essential_stock`, `person_superficial_stock`, `person_superficial_carry` |
| Companies | `companies`, `company_open_positions`, `company_offered_job_types`, `company_founder_stakes`, `company_historical_ids`, `company_sales_history`, `company_profit_history`, `company_sold_units_history`, `company_employee_count_history`, `company_price_history`, `company_loan_payment_history`, `company_cash_balance_history`, `company_loans` |
| Banks | `banks`, `bank_open_positions`, `bank_stock_positions` |
| Households | `households` |
| Stock market | `country_stock_markets`, `stock_market_person_portfolios`, `stock_market_person_positions`, `stock_market_bank_positions`, `stock_market_listed_equities`, `stock_market_equity_price_history`, `stock_market_equity_volume_history`, `stock_market_equity_market_cap_history`, `stock_market_equity_monthly_dps`, `stock_market_daily_index_history` |
| Histories | `world_history`, `country_history`, `country_fx_source_history`, `bank_history` |
| Events | `events` |

The schema stores the same snapshot content as before, but as typed rows and columns:

- `World`: day, seed, paused flag, policy rate, export tariff matrix
- `Country[]`: fiscal, monetary, FX, housing, bond, and tracker fields
- `SimulationParams`: flattened typed row in `country_sim_params`
- `Person[]` per country: employment, cash, debt, housing, stock, education, migration history
- `Company[]` per country: financials, ring-buffer histories, open positions, founder stakes
- `Bank[]` per country: reserve, deposits, equity, run/resolution state, stock positions
- `Household[]` per country: membership and shelter state
- `StockMarket` per country: portfolios, listings, equity histories, daily index history
- `world_history` / `country_history`: rolling 120-month metric snapshots
- `bank_history`: per-bank reserve/deposit/equity time series
- `country_fx_source_history`: per-source FX reserve history
- `events`: both world event log and country pending events, stored as typed rows with nullable variant columns

---

## Existing Saves List

The setup screen scans the executable directory for `.db` files and lists them sorted by most-recently-updated first. Each entry shows:

```
▶ my-save (day 1234, seed 0x5eedc0de)
```

The `▶` marker highlights the currently focused row. Up to five saves are shown; the list scrolls if there are more.

---

## Schema Version

The schema version is stored in `save_metadata.schema_version` (currently `3`). Loading a file with a mismatched schema version returns an error and does not attempt deserialization. This prevents silent corruption from version mismatches.

Version `3` is intentionally incompatible with earlier layouts. In addition to the normalized relational tables introduced after the legacy `world_json` snapshot format, the current schema persists the UI-facing `last_completed_month_*` snapshot fields so save/load round-trips preserve last-month detail panels exactly.

---

## Technical Notes

- **DTO layer**: domain structs are not given `Serialize`/`Deserialize` derives. A separate mirror layer in `src/sim/persistence/dto.rs` maps domain types to plain data structs used by the relational reader/writer. This keeps simulation internals free of persistence coupling.
- **Reconstruction**: on load, `Country::rebuild_person_runtime()` re-indexes all employment and portfolio caches from the raw person data. Ephemeral runtime state (`CountryRuntime`, `GoodsMarketDayCache`) is rebuilt from scratch.
- **Fixed-size arrays**: Rust arrays (`[f32; N]`, `[u16; N]`) are stored as row-per-slot tables and reconstructed with a bounds-checked copy on load.
- **Numeric map keys**: runtime `HashMap<u32, …>` and `HashMap<u16, …>` DTO keys are still normalized into ordered row sets in SQLite and parsed back into numeric keys on load.
- **Events**: `SimEvent` variants are stored in one `events` table with `event_kind` plus nullable typed columns for variant-specific fields. Unknown kinds still fall back to a neutral `PopBirth { count: 0 }` placeholder during reconstruction.
- **Source modules**: `src/sim/persistence/` — `mod.rs`, `error.rs`, `repository.rs`, `dto.rs`, `sqlite/mod.rs`, `sqlite/schema.rs`, `sqlite/schema.sql`, `sqlite/write.rs`, `sqlite/read.rs`, `sqlite/params_row.rs`, `sqlite/event_row.rs`, `sqlite/metric_row.rs`.
