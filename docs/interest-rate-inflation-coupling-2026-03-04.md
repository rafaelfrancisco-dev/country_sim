# Interest-Rate and Inflation Coupling (2026-03-04)

## Scope
This change couples monetary policy to real-economy behavior so rate changes affect:
- credit availability
- discretionary demand
- expansion/startup aggressiveness
- firm pricing pressure and inflation outcomes

## Transmission Design
All channels share a normalized policy stance:

```text
neutral_loan_rate = neutral_policy_rate + credit_spread
normalized_gap = clamp((bank_loan_rate - neutral_loan_rate) / rate_response_band, -1, 1)
```

Each channel then uses:

```text
multiplier = clamp(1 - normalized_gap * strength, min, max)
```

Interpretation:
- `normalized_gap < 0` (rates below neutral): multipliers rise above 1.
- `normalized_gap > 0` (rates above neutral): multipliers fall below 1.

## Implemented Channels
1. Credit:
- Loan debt-to-income caps are scaled by a rate-sensitive loan appetite multiplier.
- Distress loan-capacity checks now use current dynamic bank loan rates instead of a fixed APR.

2. Demand:
- Household discretionary budgets (used by non-essential consumption) are rate-sensitive.

3. Expansion and startup:
- Company growth-opening requirements are adjusted by rate stance.
- Startup monthly budget and startup loan fraction are adjusted by the same stance.

4. Pricing and inflation:
- Monthly repricing uses a rate-adjusted market profile:
  - raise-rate pressure is damped in high-rate regimes
  - cut-rate pressure is amplified in high-rate regimes
- Sell-through thresholds shift so high rates make hikes harder and cuts easier.
- CPI remains a measured outcome from prices (no direct CPI override term), so inflation can fall structurally.

## Defaults Added
`MonetaryPolicyParams` defaults introduced:
- `rate_response_band = 0.06`
- `loan_appetite_strength = 0.55`, min/max `0.55..1.35`
- `discretionary_spend_strength = 0.45`, min/max `0.65..1.25`
- `expansion_strength = 0.60`, min/max `0.45..1.45`
- `price_pressure_strength = 0.70`, min/max `0.35..1.20`
- `sell_through_shift_max = 0.05`
- `sell_through_shift_strength = 0.04`
- `inflation_ema_alpha = 0.35`

## Inflation Stability Calibration
Additional inflation-specific calibration in this pass:
- CPI baseline is initialized from the current basket at country creation (instead of hard-coding 1.0).
- Annualized inflation signal is smoothed with EMA before feeding policy logic.

Observed on 3-year headless export (`STOCK_EXPORT_DAYS=1095`, seed `0x5eedc0de`):
- startup month inflation spike removed:
  - before: month 1 world avg inflation `~12.1018`
  - after: month 1 world avg inflation `~0.0023`
- smoother path:
  - before world avg inflation mostly oscillated with larger jumps and outliers
  - after world avg inflation tracks in a tighter range (`~0.008` to `~0.084` over months 2–12)
- disinflation under tighter stance improved:
  - before: early avg infl (m2–m12) `0.0669`, late avg infl (m25–m36) `0.0499`
  - after: early avg infl (m2–m12) `0.0515`, late avg infl (m25–m36) `0.0444`

## Timing
Transmission keeps the existing monthly ordering:
- countries process month-end with current policy stance
- world updates policy rate after month-end
- new policy stance affects next-month behavior (one-month lag)

## Validation
Added/updated tests cover:
- higher loan issuance in low-rate vs high-rate settings
- tighter computed loan capacity in high-rate settings
- more conservative growth openings at higher rates
- lower price pressure and lower inflation under tighter rates
- lower discretionary budgets under higher rates
