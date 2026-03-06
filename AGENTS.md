# Repository Guidelines

## Project Structure & Module Organization
`src/main.rs` starts the TUI app, and `src/lib.rs` exposes the reusable modules. Core simulation logic lives in `src/sim/` (`country/`, `world/`, `bank/`, `event/`), while the terminal UI and worker pipeline live in `src/components/simulation/`. Configuration and startup plumbing sit in `src/config/`, `src/cli.rs`, `src/app.rs`, and `src/tui.rs`. Integration tests live in `tests/`, benchmarks in `benches/`, helper scripts in `scripts/`, and deeper design notes in `docs/`.

## Build, Test, and Development Commands
- `cargo run -- --help`: inspect runtime flags before changing CLI behavior.
- `cargo run --release -- --seed 0x123 --export-csv target/out`: run the app with a fixed seed and CSV export.
- `cargo test`: run the default unit and integration suite.
- `cargo test --test banking_integration -- --ignored`: run the slow banking regression suite on demand.
- `cargo test --features runtime-invariants`: enable invariant warnings during test runs.
- `cargo fmt` and `cargo clippy --all-targets --all-features`: format and lint before opening a PR.
- `./scripts/bench-sim.sh ...` or `./scripts/run-flamegraph.sh`: use these for performance work, not ad hoc commands.

## Coding Style & Naming Conventions
Use standard Rust formatting: 4-space indentation, `snake_case` for functions/modules, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep modules narrow and split files before they become large; `tests/src_line_limit.rs` fails the build if any file under `src/` exceeds 550 lines. Prefer descriptive test names such as `valid_setup_submit_transitions_to_running`.

## Testing Guidelines
Keep fast unit tests close to the code with `#[cfg(test)]` modules or sibling `*_tests.rs` files. Put cross-module or long-horizon scenarios in `tests/`. Mark expensive scenarios with `#[ignore]` when they are not suitable for the default suite, and document the exact command needed to run them.

## Commit & Pull Request Guidelines
Recent history uses short, imperative, sentence-style subjects such as `Fix month-close income and dividend details` and `Add view-scroll and scrollbar to all list tables`. Follow that format, keep commits focused, and explain behavior changes in the PR body. For UI work, include screenshots or a short recording; for simulation or performance work, list the `cargo test` or benchmark commands you ran and note any seed or feature flags used.
