# App Review - 2026-03-05

## Scope
This review focuses on the current app state and the most important next steps based on code inspection plus local validation runs.

## Findings

### 1. High: Cross-country deposits are modeled against the wrong bank
- Evidence: [src/sim/world/cross_country_banking.rs](../src/sim/world/cross_country_banking.rs), [src/sim/country/mod.rs](../src/sim/country/mod.rs)
- Problem:
  - The foreign-deposit selector uses `public_bank_id_or_first` instead of scanning for the actual best bank.
  - Existing deposits accrue using a country-level snapshot rather than the depositor's stored `bank_id`.
  - Reserve and liability postings hardcode `foreign.bank` instead of the selected foreign bank.
- Why it matters:
  - Private-bank reserve balances and foreign-deposit liabilities can drift away from the events and person state.
  - Rate competition is effectively mis-modeled, so the feature can look active while producing the wrong balance-sheet outcomes.
- Concrete hotspots:
  - `process_cross_country_deposits` snapshot/build logic
  - interest posting path
  - recall path
  - new foreign-deposit open path

### 2. High: The TUI lifecycle is still crash-prone
- Evidence: [src/tui.rs](../src/tui.rs), [src/cli.rs](../src/cli.rs)
- Problem:
  - TUI intervals divide by user-supplied tick/frame rates.
  - CLI parsing currently accepts any `f64` for those rates, including zero or negative values.
  - The event loop still uses `expect` on init send.
  - `Drop` still uses `self.exit().unwrap()`.
- Why it matters:
  - Invalid CLI input can cause a hard crash instead of a user-facing validation error.
  - Normal shutdown or teardown failures can still panic the app while leaving terminal state in a bad place.

### 3. Medium: The repo is not on a green quality baseline
- Evidence: [tests/src_line_limit.rs](../tests/src_line_limit.rs), [src/sim/country/tests/government_and_labor.rs](../src/sim/country/tests/government_and_labor.rs)
- Problem:
  - The line-budget test currently fails because `src/sim/country/tests/government_and_labor.rs` is 780 lines, above the 550-line limit.
  - `cargo clippy --all-targets --all-features --locked -- -D warnings` currently fails with 17 diagnostics.
- Why it matters:
  - The repo already has explicit quality gates, but the main branch state is not satisfying them.
  - That reduces confidence in new changes and makes future review signal noisier.

### 4. Medium: Banking integration coverage is missing the assertion that would catch the real regression
- Evidence: [tests/banking_integration.rs](../tests/banking_integration.rs)
- Problem:
  - The cross-country deposit test only prints event counts and does not assert any banking-state outcome.
  - The same file runs several 3-year simulations, which makes it expensive while still missing a material bug.
- Why it matters:
  - A real accounting bug in cross-country deposits can ship while the test suite stays green.
  - Slow tests without precise assertions cost CI time without protecting the most important behavior.

### 5. Medium: Core simulation state is still too exposed
- Evidence: [src/sim/country/mod.rs](../src/sim/country/mod.rs), [src/sim/world/mod.rs](../src/sim/world/mod.rs)
- Problem:
  - `Country` and `World` expose a large amount of mutable `pub` state.
  - The `bank` plus `banks` split makes incorrect call sites easy to write.
- Why it matters:
  - Cross-module mutations can bypass intended invariants.
  - The simulator is now large enough that structural encapsulation would prevent whole classes of bookkeeping bugs.

## Next Steps

1. Fix cross-country deposit accounting first.
   - Select the actual best bank instead of defaulting to `public_bank_id_or_first`.
   - Accrue interest using the depositor's recorded `fd.bank_id`.
   - Post reserve and `foreign_deposit_liabilities` changes through `bank_by_id_mut`.

2. Add a targeted regression test for foreign deposits.
   - Open a foreign deposit into a private bank.
   - Assert that the exact selected bank changes reserves and liabilities on open, interest accrual, and recall.

3. Harden the TUI boundary.
   - Validate CLI tick/frame/UI-hz inputs as positive values.
   - Replace `expect` and `unwrap` in TUI startup/shutdown with best-effort handling.
   - Add one test that verifies invalid rate input is rejected cleanly.

4. Restore the quality baseline.
   - Split `government_and_labor.rs` so the line-limit test passes again.
   - Clear the current clippy failures and keep `-D warnings` green.

5. Reshape the slow banking integration suite.
   - Keep one short smoke test in the default CI path.
   - Move long 3-year simulations to ignored/perf validation jobs unless they are absolutely required for every run.

## Validation Notes

### Commands run
```bash
cargo test -q
cargo test -q --test src_line_limit
cargo clippy --all-targets --all-features --locked -- -D warnings
```

### Observed results
- `cargo test -q`:
  - Unit test binaries were green locally before the long banking integration tests dominated runtime.
  - Observed result for the main unit pass: `239 passed; 0 failed; 2 ignored`.
- `cargo test -q --test src_line_limit`:
  - Failed because `src/sim/country/tests/government_and_labor.rs` exceeds the configured line limit.
- `cargo clippy --all-targets --all-features --locked -- -D warnings`:
  - Failed with 17 diagnostics, including issues in cross-country banking and several newer monthly/render paths.

## Summary
The highest-value next step is to fix cross-country foreign-deposit accounting and add a regression test that validates the exact bank balance-sheet effects. After that, the team should restore the repo's baseline quality gates and harden the TUI input/shutdown path before taking on more feature work.
