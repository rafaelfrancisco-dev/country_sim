# FX Tab

## Overview
The `FX` tab is a read-only foreign-exchange monitor in the running simulation UI.

It is world-monitor first:
- the center of gravity is the cross-country FX leaderboard
- the selected country is still used for the summary, pair board, and per-country charts

The tab sits between `Stocks` and `Disasters` in the running tab order.

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
  - `Selected FX Rate`
  - `World Avg FX Rate`
  - `Selected FX Reserves`
  - `Selected FX Pressure`

## Selected-Country Summary
The summary panel shows:
- currency name, code, and symbol
- current FX rate versus the shared base unit
- last monthly FX move
- FX reserves in base terms
- FX pressure with a qualitative label
- goods exports/imports in base terms
- financial inflows/outflows in base terms
- net FX flow in base terms

## Leaderboard Columns
The main table is sorted by strongest current FX rate first and shows:
- country
- currency code
- current FX rate
- monthly move %
- FX reserves (base)
- net FX flow (base)
- FX pressure
- export/import ratio

If a country does not have enough history for a monthly move, the table shows `N/A`.
If imports are zero, the export/import ratio also shows `N/A`.

## Pair Board
The pair board compares the selected country against a small set of top peer currencies:
- implied pair rate from the selected currency to the peer currency
- a simple relative-strength label (`stronger`, `weaker`, or `flat`)

This panel is meant to be fast to scan, not a full market-depth or order-book view.
