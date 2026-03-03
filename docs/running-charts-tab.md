# Running UI Tabs

## Overview
The running simulation UI has five tabs:
- `Main`: country/people navigation and details screens.
- `Goods`: company and goods market view.
- `Stocks`: listed equities and stock-market charts.
- `Charts`: trend charts for core economy metrics.
- `Events`: scrollable log of simulation events.

Tab switching uses `Tab` and `Shift+Tab`. The active tab is shown in the top header.

## Global Running Controls
- `Tab` / `Shift+Tab`: switch tabs
- `Space`: pause/resume
- `n`: step one day while paused
- `m`: toggle simulation mode (`realtime` / `max-speed`)
- app-level quit/suspend shortcuts remain available

## Navigation Standard
The running UI uses one navigation convention across tabs:
- `w` / `s`: move selection up/down (or scroll in Events)
- `a` / `d`: move focus left/right between panes
- `PageUp` / `PageDown`: previous/next page in paged tables
- `Enter`: open selected details
- `Esc`: close details or close active search editing
- `,` / `.`: previous/next country context where applicable

## Tab-Specific Controls
- `Main`: list navigation and person/country details; in person details, `c` jumps to employer company in Goods.
- `Goods`: country/company navigation, company details panel.
- `Main`/`Goods` details view: `PageUp` / `PageDown` navigate detail pages.
- `Stocks`: country/table focus, stock-row navigation, stock charts.
- `Charts`: `t` toggles scope between selected country and world aggregate; `PageUp` / `PageDown` switch chart pages.
- `Events`: `/` toggles search editing, `o` toggles sort order, `f` cycles event category, `Ctrl+u` clears search query while editing.
  - Category cycle order: `All` -> `Stock Market` -> `Births/Deaths` -> `Bankruptcies & Distress` -> `Loans & Debt` -> `Companies & Labor` -> `Government & Policy` -> `All`
  - When both a search query and category are set, results must match both filters.

UI navigation commands are applied on the UI side from the latest engine snapshot and are not queued as simulation mutations.

## Charts and Metrics
The `Charts` tab renders eight charts per page in a 4×2 grid. Use `PageUp` / `PageDown` to switch pages.

### Page 1 — Liquidity & Balance Sheet
| Row | Left | Right |
|---|---|---|
| 1 | Population | Total Money Supply |
| 2 | Household Cash Total | Company Cash Total |
| 3 | Created Money Outstanding | Bank Reserve |
| 4 | Expected Money Supply | Money Supply Gap |

### Page 2 — Fiscal & External Sector
| Row | Left | Right |
|---|---|---|
| 1 | Treasury Balance | Budget Balance |
| 2 | Monthly Tax Collected | Monthly Gov Spending |
| 3 | Government Debt | Bonds Outstanding |
| 4 | Monthly Goods Import Spending | Monthly Goods Export Revenue |

### Page 3 — Prices, Stress & Market Risk
| Row | Left | Right |
|---|---|---|
| 1 | CPI Index | Inflation Rate % |
| 2 | Policy Rate % | Real Policy Rate % |
| 3 | Monthly Unmet Need Events | Monthly Essential Debt Spending |
| 4 | Stock Market Cap | Bank Equity Exposure |

Each chart is a line chart with dynamic axis bounds and month-based x-axis labels.
