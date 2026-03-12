# FX Tab

## Overview
The `FX` tab is a read-only foreign-exchange monitor in the running simulation UI.

It is world-monitor first:
- the center of gravity is a cross-country FX leaderboard
- the selected country is still used for summary, pair board, and per-country charts

The tab sits between `Stocks` and `Disasters` in the running tab order.

## Currency Unions

The FX system supports currency unions where multiple countries share a single currency (e.g., Eurozone):

### Key Features
- Countries sharing a currency have a 1:1 exchange rate between them
- One designated "issuer" country controls monetary policy for that currency
- Only the issuer can mint new money for that currency
- Non-issuer countries inherit the issuer's exchange rate
- An `*` indicator in the leaderboard marks the currency issuer

### Implementation Details
See [Currency Unions](currency-unions.md) for complete documentation on currency unions.

## Controls
- `Tab` / `Shift+Tab`: switch tabs
- `,` / `.`: previous/next selected country
- global runtime controls still apply:
  - `Space`: pause/resume
  - `n`: step one day while paused
  - `m`: toggle simulation mode

FX v1 has no row selection, no detail drill-down, and no edit actions.

## Layout
The tab uses a 3-section dashboard:

### Top Row
- left: selected-country FX summary
- right: pair board versus leading peer currencies

### Middle Row
- world FX leaderboard table for all countries

### Bottom Row
- four trend charts:
  - `FX Rate (index)` — the selected country's rate history (internal index unit)
  - `FX Net Flow` — the selected country's monthly net FX flow
  - `FX Reserves by Currency` — one line per foreign currency held, in local-currency equivalent; seeded at startup and updated by cross-border flows
  - `FX Pressure` — the selected country's FX pressure indicator

## Selected-Country Summary
The summary panel shows:
- currency name, code, and symbol
- last monthly FX move
- FX pressure with a qualitative label
- goods exports / imports in local currency equivalent
- financial inflows / outflows in local currency equivalent
- net FX flow in local currency equivalent
- FX reserves broken down by foreign currency held (e.g. `50M€ + 37M¥`); startup reserves are evenly split across all foreign currencies, excluding the country's own

## Foreign Currency Reserves
Each country tracks which foreign currencies make up its reserve pool:
- initial reserves (seeded at startup) are split evenly across every other country's currency, excluding the country's own
- reserves are attributed to the source country when cross-border payments are received (goods exports, deposit interest, stock dividends, etc.)
- when spending reserves, the simulation uses the target country's currency first, then falls back proportionally to the remaining pool

## Leaderboard Columns
The main table is sorted by the country's underlying FX rate (strongest first) and shows:
- country
- currency code
- rate vs selected (cross-rate of each peer to the selected country; `—` for the selected row itself)
- monthly move %
- FX reserves in that country's local currency equivalent
- net FX flow in that country's local currency equivalent
- FX pressure
- export/import ratio

If a country does not have enough history for a monthly move, the table shows `N/A`.
If imports are zero, the export/import ratio also shows `N/A`.

## Pair Board
The pair board compares the selected country against a small set of top peer currencies:
- cross-rate: how many units of the peer currency equal one unit of the selected currency
- a simple relative-strength label (`stronger`, `weaker`, or `flat`)

This panel is meant to be fast to scan, not a full market-depth or order-book view.
