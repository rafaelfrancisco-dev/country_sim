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

### Page 4 — Interest Rates & Credit
| Row | Left | Right |
|---|---|---|
| 1 | Policy Rate % | Deposit Rate % |
| 2 | Loan Rate % | Rate Spread % |
| 3 | Real Deposit Rate % | Svgs Interest Paid |
| 4 | Loan Book | Inflation Rate % |

This page shows the full interest rate story: the central bank sets the policy rate in discrete steps; banks add a spread to derive loan and deposit rates; the rate spread is the bank's gross intermediation margin; the real deposit rate is the deposit rate minus inflation (negative = financial repression); savings interest paid is the monthly cash distributed to depositors; the loan book tracks total money created via lending; inflation closes the feedback loop back to the Taylor Rule.

Each chart is a line chart with dynamic axis bounds and month-based x-axis labels.
