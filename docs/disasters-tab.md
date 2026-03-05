# Disasters Tab

## Overview
The `Disasters` tab provides manual shock controls for the running simulation.

- Scope: selected country only.
- Apply model: configure one or more disasters, then press `Enter` to apply all enabled shocks together.
- Intensity options: `5%`, `10%`, `25%`, `50%`.

## Controls
- `Tab` / `Shift+Tab`: switch tabs.
- `,` / `.`: switch selected country.
- `w` / `s`: move selected disaster row.
- `a` / `d`: change intensity tier for selected disaster.
- `x`: enable/disable selected disaster.
- `Enter`: apply all enabled disasters with their selected intensity.

## Disaster Types
- `Company Bankruptcy Wave`: forces a percentage of companies into bankruptcy.
- `Population Loss`: removes a percentage of citizens.
- `Stock Market Crash`: reprices listed equities downward by selected percentage.
- `Bank Reserve Drain`: removes a percentage of reserves from all banks in the selected country.
- `Loan Default Wave`: writes off a percentage of outstanding created principal across person/company loans.
- `Treasury Shock`: removes a percentage of treasury balance.
- `Demand Shock`: removes a percentage of household cash balances.

## Event Logging
Each applied disaster emits a `[DISAS]` event in the event log with:
- country
- disaster label
- intensity percent
- short effect summary

## Notes
- `0%` is not available in the UI, so each enabled row always has non-zero impact.
- Applying with no enabled disasters does not mutate the simulation and shows a status prompt in the tab.
