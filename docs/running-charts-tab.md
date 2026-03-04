# Running UI Tabs

## Overview
The running simulation UI has six tabs:
- `Main`: country/people navigation and details screens.
- `Goods`: company and goods market view.
- `Banks`: bank list and bank details view.
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
  - Person details use fixed pages:
    - page 1: person overview details
    - page 2: related events for that person
  - Person page 2 reuses Events controls (`w/s`, `/`, `o`, `f`, `Ctrl+u`) and the same shared filter/search/sort state as the Events tab.
- In country details, `o` opens Country Policy Overrides.
  - Country Policy Overrides form mode:
    - `w` / `s`: previous/next editable field
    - `Enter`: start editing focused field, then save
    - `r`: reset all editable policy values to country-managed defaults
    - `Esc`: cancel field edit; when not editing, return to country details
- `Goods`: country/company navigation, company details panel.
  - Company details use fixed pages:
    - page 1: company overview details
    - page 2: company history charts
    - page 3: related events for that company
    - page 4: current employees table (`Name`, `Job`, `Tenure (mo)`, `Pay/mo`)
  - Company page 3 reuses Events controls (`w/s`, `/`, `o`, `f`, `Ctrl+u`) and the same shared filter/search/sort state as the Events tab.
  - Company page 4 uses `w/s` to scroll employees.
- `Banks`: country/bank navigation, bank details panel.
  - Bank details use fixed pages:
    - page 1: bank overview details
    - page 2: bank history charts (`Reserve`, `Deposits`, `Created Money`, `Equity`, `Run Intensity`)
    - page 3: related events for that bank
    - page 4: current depositors table (`Type`, `Entity`, `Balance`, `Share`)
  - Bank page 3 reuses Events controls (`w/s`, `/`, `o`, `f`, `Ctrl+u`) and the same shared filter/search/sort state as the Events tab.
  - Bank page 4 uses `w/s` to scroll depositors.
- `Main`/`Goods`/`Banks` details view: `PageUp` / `PageDown` navigate detail pages.
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

### Page 4 — Interest Rates & Credit
| Row | Left | Right |
|---|---|---|
| 1 | Deposit Rate % | Loan Rate % |
| 2 | Rate Spread % | Real Deposit Rate % |
| 3 | Monthly Savings Interest Paid | Real Loan Rate % |
| 4 | Credit-to-Money % | Reserve Coverage % |

This page focuses on bank transmission and credit quality without repeating charts from page 3. Deposit and loan rates show policy passthrough; spread is the gross intermediation margin; real deposit and real loan rates adjust for inflation; monthly savings interest paid shows cash transfer to depositors; credit-to-money tracks leverage in the money stock; reserve coverage tracks liquid backing against the loan book.

Each chart is a line chart with dynamic axis bounds and month-based x-axis labels.
