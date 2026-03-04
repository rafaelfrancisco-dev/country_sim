# App Review - 2026-03-04

## Scope
This document consolidates two review tracks:
1. Code smells and tech debt.
2. New features to add simulator depth.

## 1) Code Smells and Tech Debt

### Findings (ordered by severity)

1. **High: Panic-prone terminal shutdown path**
- Evidence: [src/tui.rs:232](/Users/rafaelfrancisco/Desktop/country_sim/src/tui.rs:232), [src/tui.rs:117](/Users/rafaelfrancisco/Desktop/country_sim/src/tui.rs:117), [src/tui.rs:149](/Users/rafaelfrancisco/Desktop/country_sim/src/tui.rs:149)
- Problem: `Drop` uses `self.exit().unwrap()` and event init uses `expect`; teardown failures can crash terminal lifecycle.
- Remediation: Make shutdown best-effort and non-panicking (`let _ = self.exit()`), replace `expect` with logged error + graceful return.

2. **High: Invariant validation in the hot runtime path**
- Evidence: [src/sim/engine.rs:166](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/engine.rs:166), [src/sim/engine.rs:177](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/engine.rs:177), [src/sim/world/mod.rs:127](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs:127), [src/sim/country/metrics.rs:72](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/country/metrics.rs:72)
- Problem: `GameEngine::apply` always runs `world.validate_invariants()`, coupling correctness checks to throughput.
- Remediation: Gate checks to debug/diagnostic cadence (e.g., monthly/every N ticks); keep strict checks in tests/manual harness.

3. **High: Simulation worker has god-object responsibilities and brittle failure behavior**
- Evidence: [src/components/simulation/worker/runtime.rs:50](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/worker/runtime.rs:50), [src/components/simulation/worker/runtime.rs:129](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/worker/runtime.rs:129), [src/components/simulation/worker/runtime.rs:160](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/worker/runtime.rs:160), [src/components/simulation/worker/runtime.rs:235](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/worker/runtime.rs:235), [src/components/simulation/worker/tests.rs:22](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/worker/tests.rs:22)
- Problem: One loop handles scheduling, command processing, snapshot construction, and duplicate CSV export paths; lock/write failures can break loop without explicit health signaling.
- Remediation: Split scheduler/command executor/exporters/snapshot publisher; dedupe exporter setup; add explicit worker error/health channel.

4. **High: Oversized snapshot model cloned frequently**
- Evidence: [src/components/simulation/snapshot/models.rs:15](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/snapshot/models.rs:15), [src/components/simulation/snapshot/models.rs:77](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/snapshot/models.rs:77), [src/components/simulation/snapshot/builder.rs:264](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/snapshot/builder.rs:264), [src/components/simulation/snapshot/builder.rs:290](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/snapshot/builder.rs:290), [src/components/simulation/mod.rs:123](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/mod.rs:123)
- Problem: Large `RunningSnapshot` is rebuilt/cloned frequently, increasing allocation and maintenance cost.
- Remediation: Box large enum payloads, split tab/section snapshots, share heavy histories via `Arc<[T]>`, avoid full snapshot clones.

5. **Medium: Weak encapsulation of core domain state**
- Evidence: [src/sim/country/mod.rs:27](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/country/mod.rs:27), [src/sim/world/mod.rs:53](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs:53)
- Problem: Many mutable `pub` fields in `Country`/`World` make invariant violations easy.
- Remediation: Make fields private by default and expose intent-based mutation methods.

6. **Medium: Duplicated traced vs non-traced tick logic**
- Status: Resolved on 2026-03-04.
- Evidence: [src/sim/world/mod.rs:182](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs:182), [src/sim/world/mod.rs:205](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs:205), [src/sim/world/mod.rs:372](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs:372)
- Problem: Duplicated flow risks drift.
- Remediation: Extracted shared tick pipeline with instrumentation hooks (`tick_day_with_hooks` + `WorldTickHooks`).

7. **Medium: Dead-code/warning suppression debt**
- Evidence: [src/tui.rs:1](/Users/rafaelfrancisco/Desktop/country_sim/src/tui.rs:1), [src/config/mod.rs:1](/Users/rafaelfrancisco/Desktop/country_sim/src/config/mod.rs:1), [src/sim/mod.rs:19](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/mod.rs:19), [src/sim/trace.rs:1](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/trace.rs:1), [src/sim/manual_harness.rs:17](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/manual_harness.rs:17), [src/sim/world/tests/helpers_and_bench.rs:16](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/tests/helpers_and_bench.rs:16)
- Problem: Blanket `allow(dead_code)` and duplicated harness/cfg paths reduce lint signal.
- Remediation: Remove blanket allows incrementally and tighten CI lint policy.

### Prioritized top-5 actions

1. Remove panic-on-drop paths in TUI lifecycle (`unwrap`/`expect`).
2. Decouple invariant validation from per-command runtime path.
3. Refactor `SimulationWorker` into smaller units with explicit failure signaling.
4. Redesign snapshot transport to reduce cloning/allocation.
5. Encapsulate `World`/`Country` mutable state behind methods/invariants.

### Validation note
- Agent report indicated `cargo test --locked` passed.
- Agent report indicated `cargo clippy --all-targets --all-features --locked` has warnings.

## 2) New Features to Add Simulator Depth

### Review snapshot
The app already has a strong depth-friendly architecture: deterministic daily/monthly loops, modular market subsystems, policy params, event logging, and CSV history export. The largest depth opportunity is endogenous regime change dynamics (labor churn, credit tightening, migration, bottlenecks).

### Feature candidates

1. **Scenario Shock Engine**
- Adds: scheduled shocks (demand, tariff, policy, commodity-cost, confidence).
- Depth impact: repeatable macro what-if experiments.
- Likely files: [src/sim/world/mod.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs), [src/sim/params.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/params.rs), [src/sim/event.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/event.rs), [src/sim/setup.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/setup.rs), [src/cli.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/cli.rs)
- Complexity/Risk: Medium
- MVP: hardcode 3 presets + `ShockApplied` events.

2. **Labor Churn + Separations**
- Adds: quits/retirements/layoffs independent of bankruptcy path.
- Depth impact: dynamic unemployment/wage pressure.
- Likely files: [src/sim/params.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/params.rs), [src/sim/country/daily.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/country/daily.rs), [src/sim/country/job_market.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/country/job_market.rs), [src/sim/world/export/monthly.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/export/monthly.rs)
- Complexity/Risk: Medium
- MVP: monthly random separations with replacement openings.

3. **Expanded Policy Overrides**
- Adds: more runtime knobs (tariff baseline, bond spread, backfill intensity, stock liquidity).
- Depth impact: better policy experimentation loop.
- Likely files: [src/sim/engine.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/engine.rs), [src/components/simulation/ui_state/types.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/ui_state/types.rs), [src/components/simulation/ui_state/commands.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/ui_state/commands.rs), [src/components/simulation/snapshot/details/country_details.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/components/simulation/snapshot/details/country_details.rs)
- Complexity/Risk: Low-Medium
- MVP: add 4 high-leverage fields + immediate apply.

4. **Household Behavior Segments**
- Adds: persistent spending/saving archetypes.
- Depth impact: richer demand cycles and policy sensitivity.
- Likely files: [src/sim/person.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/person.rs), [src/sim/generator.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/generator.rs), [src/sim/world/goods_market/entrypoints.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/goods_market/entrypoints.rs)
- Complexity/Risk: Medium
- MVP: 3 archetypes (`precautionary`, `balanced`, `spender`) affecting discretionary budgets.

5. **Bank Stress & Credit Tightening**
- Adds: stress regime that tightens credit and increases spread.
- Depth impact: financial cycle + second-order recessions/recoveries.
- Likely files: [src/sim/bank.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/bank.rs), [src/sim/country/monthly/monthly_orchestration_payroll_loans.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/country/monthly/monthly_orchestration_payroll_loans.rs), [src/sim/country/mod.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/country/mod.rs), [src/sim/event.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/event.rs)
- Complexity/Risk: Medium-High
- MVP: threshold-triggered stress mode adjusting DTI cap and spread.

6. **Cross-Country Migration**
- Adds: labor movement toward stronger outcomes.
- Depth impact: multi-country strategic interaction.
- Likely files: [src/sim/world/mod.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/mod.rs), [src/sim/event.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/event.rs), [src/sim/world/export/monthly.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/export/monthly.rs)
- Complexity/Risk: Medium
- MVP: unemployed-only migration based on wage/unemployment differentials.

7. **Input-Output Supply Chains**
- Adds: intermediate goods dependencies and bottlenecks.
- Depth impact: shortages, inflation pass-through, cascades.
- Likely files: [src/sim/good.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/good.rs), [src/sim/company.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/company.rs), [src/sim/world/goods_market/entrypoints.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/goods_market/entrypoints.rs), [src/sim/world/goods_market/round_robin.rs](/Users/rafaelfrancisco/Desktop/country_sim/src/sim/world/goods_market/round_robin.rs)
- Complexity/Risk: High
- MVP: 2-3 goods with one fixed input each.

### Prioritized roadmap

1. **Now**: Expanded Policy Overrides, Labor Churn + Separations, Scenario Shock Engine.
2. **Next**: Household Behavior Segments, Bank Stress & Credit Tightening, Cross-Country Migration.
3. **Later**: Input-Output Supply Chains.
