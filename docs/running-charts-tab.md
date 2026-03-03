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
- `Stocks`: country/table focus, stock-row navigation, stock charts.
- `Charts`: `t` toggles scope between selected country and world aggregate.
- `Events`: `/` toggles search editing, `o` toggles sort order, `Ctrl+u` clears search query while editing.

UI navigation commands are applied on the UI side from the latest engine snapshot and are not queued as simulation mutations.

## Charts and Metrics
The `Charts` tab renders eight charts in a 4×2 grid:

| Row | Left | Right |
|---|---|---|
| 1 | Population | Total Money Supply |
| 2 | Created Money Outstanding | Bank Reserve |
| 3 | Treasury Balance | Budget Balance |
| 4 | CPI Index | Policy Rate % |

Each chart is a line chart with dynamic axis bounds and month-based x-axis labels.

**CPI Index** starts at `1.0` and drifts as goods prices move. Values above `1.0` indicate accumulated price-level growth.

**Policy Rate %** shows the world-level central bank rate set by the Taylor Rule each month (displayed as percentage, e.g. `4.00` = 4%). Use `[t]` to toggle between selected country and world aggregate scope for all charts; the scope label appears in each chart title.
