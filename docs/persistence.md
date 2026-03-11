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

Each `.db` file is a self-contained SQLite database with two tables:

| Table | Content |
|-------|---------|
| `save_metadata` | Save name, schema version, timestamps, seed, day, country count |
| `world_json` | Full world state serialized as a single JSON blob |

The JSON blob is a snapshot of all simulation state at the moment `Shift+S` was pressed:

- `World`: day, seed, paused flag, policy rate, tariff matrix
- `Country[]`: all fiscal, monetary, FX, housing, bond, and tracker fields
- `Person[]` per country: employment, cash, loan, housing, stock, education, migration history
- `Company[]` per country: financials, history ring-buffers, open positions, founder stakes
- `Bank[]` per country: reserve, deposits, equity, run/resolution state, stock positions
- `Household[]` per country: membership and shelter state
- `StockMarket` per country: portfolios, listings, order-book history
- `world_history` / `country_history`: rolling 120-month metric snapshots
- `bank_history`: per-bank reserve/deposit/equity time series
- `country_fx_source_history`: per-source FX reserve history
- `event_log`: recent simulation events

---

## Existing Saves List

The setup screen scans the executable directory for `.db` files and lists them sorted by most-recently-updated first. Each entry shows:

```
▶ my-save (day 1234, seed 0x5eedc0de)
```

The `▶` marker highlights the currently focused row. Up to five saves are shown; the list scrolls if there are more.

---

## Schema Version

The schema version is stored in `save_metadata.schema_version` (currently `1`). Loading a file with a mismatched schema version returns an error and does not attempt deserialization. This prevents silent corruption from version mismatches.

---

## Technical Notes

- **DTO layer**: domain structs are not given `Serialize`/`Deserialize` derives. A separate mirror layer in `src/sim/persistence/dto.rs` maps domain types to plain data structs for serialization. This keeps simulation internals free of serde coupling.
- **Reconstruction**: on load, `Country::rebuild_person_runtime()` re-indexes all employment and portfolio caches from the raw person data. Ephemeral runtime state (`CountryRuntime`, `GoodsMarketDayCache`) is rebuilt from scratch.
- **Fixed-size arrays**: Rust arrays (`[f32; N]`, `[u16; N]`) are serialized as `Vec` in the DTO and reconstructed with a bounds-checked copy on load.
- **Numeric map keys**: `HashMap<u32, …>` and `HashMap<u16, …>` keys are serialized as `String` (JSON requires string keys) and parsed back with `key.parse::<T>().ok()` on load.
- **Events**: `SimEvent` is serialized as `{ kind: "TypeName", payload: { … } }`. Known variants are reconstructed from the payload on load; unrecognized kinds fall back to a neutral `PopBirth { count: 0 }` placeholder so future schema additions do not break loading older saves.
- **Source modules**: `src/sim/persistence/` — `mod.rs`, `error.rs`, `repository.rs`, `dto.rs`, `sqlite/mod.rs`, `sqlite/to.rs`, `sqlite/to_rest.rs`, `sqlite/from.rs`, `sqlite/from_rest.rs`.
