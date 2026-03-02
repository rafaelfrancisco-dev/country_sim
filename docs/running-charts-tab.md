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
The `Charts` tab renders six charts in a 3x2 grid:
- Population
- Total Money Supply
- Created Money Outstanding
- Bank Reserve
- Treasury Balance
- Budget Balance

Each chart is a line chart with dynamic axis bounds and month-based x-axis labels.
