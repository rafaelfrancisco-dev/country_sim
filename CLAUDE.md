# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run (interactive TUI)
cargo run
cargo run -- --seed 0x5eedc0de --tick-rate 4.0 --frame-rate 60.0 --sim-mode realtime

# Build optimized release
cargo build --release

# Run tests
cargo test

# Benchmark
./scripts/bench-sim.sh --seed 0x5eedc0de --countries 5 --population-per-country 1000000 --days 180 --threads 8 --sim-mode max-speed

# Flamegraph profiling
./scripts/run-flamegraph.sh
```

**CLI args:** `--seed` (hex or decimal), `--tick-rate` (default 4.0), `--frame-rate` (default 60.0), `--sim-mode realtime|max-speed`, `--fast-ui-hz` (default 2.0). Seed also readable from `COUNTRY_SIM_SEED` env var.

## Architecture

This is a terminal UI country/economic simulator built with Ratatui + Tokio. Key layers:

### App Layer (`src/app.rs`)
Central event loop. Routes keyboard/mouse events, manages frame rate throttling, and coordinates the component system via mpsc channels (`GameCommand`/`UiCommand` action types).

### Component System (`src/components/`)
Trait-based UI components. The main `simulation/` component owns all game UI state. Components implement `draw()`, action handlers, and config handlers.

### Simulation Core (`src/sim/`)
- **`engine.rs`**: `GameEngine` state machine — Setup phase (form) → Running phase (world)
- **`runtime.rs`**: `SimulationWorker` background task that drives ticks independently of UI
- **`world/`**: `World` holds countries, day counter, goods/stock markets, and history snapshots
- **`country/`**: `Country` owns people, companies, treasury, bank; runs daily/monthly cycles (jobs, tax, wages, unemployment)
- **`tickable.rs`**: `Tickable` trait implemented by `Person`, `Country`, `World` for daily progression
- **`generator.rs`**: Deterministic world generation from seed
- **`params.rs`**: Simulation constants

### Rendering (`src/components/simulation/render/`)
Multiple tabs rendered by separate modules: `running.rs` (main view), `charts.rs` (paginated 3-page 4×2 chart dashboard), `events.rs` (searchable log), `goods.rs`, `stocks.rs`, `details.rs`. Setup screen in `setup.rs`.

### TUI Layer (`src/tui.rs`)
Crossterm terminal abstraction. Streams events (keyboard, mouse, resize) plus synthetic tick/render events. Frames rendered via Ratatui.

## Key Technologies
- **Ratatui 0.30** + **Crossterm 0.28**: TUI rendering
- **Tokio 1.40**: Async runtime (simulation worker runs as background task)
- **Rayon**: Parallel iteration across agents (people/companies)
- **Clap 4**: CLI parsing
- **Serde + JSON5**: Config (`/.config/config.json5`)

## Documentation
Detailed design docs in `docs/`: `game-context.md`, `money-simulation-requirements.md`, `demographics-labor-model.md`, `ticking-mechanism.md`, `benchmarking.md`, `running-charts-tab.md`.
