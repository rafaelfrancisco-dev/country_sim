# Simulation Logic Analysis Findings

**Date**: 2026-03-11  
**Scope**: In-depth analysis of core simulation logic for bugs and design issues

---

## Executive Summary

This document catalogs potential issues discovered during analysis of the country simulation logic. Issues are categorized by severity and include specific file locations for remediation.

---

## Resolved Issues

### R1. Recipe-Sector Minimum Price Bug *(fixed 2026-03-17)*

**Severity**: High (now resolved)
**Location**: `src/sim/company.rs` — `minimum_viable_unit_price()`

**Description**:
`minimum_viable_unit_price()` used `good.base_unit_cost` as a proxy for material input costs. For five recipe-driven sectors, `base_unit_cost` was 4–9× lower than the actual market cost of the required recipe inputs, so those companies always priced below their cost of production.

**Affected sectors**: Clothing, Hygiene Products, Household Goods, Electronics, Appliances.

**Fix**:
The function now recursively computes actual recipe input cost via `recipe_for_good()` and uses `max(base_unit_cost, input_recipe_cost)` as the material baseline. No infinite recursion — all input goods (Raw Metals, Timber, Fuel) have `MATERIAL_FREE_RECIPE`.

`estimate_startup_monthly_input_cost()` in `viability.rs` was also updated to use `minimum_viable_unit_price()` as the floor (instead of raw `base_unit_cost`) so startup viability checks are consistent.

**Verification**:
Observed market prices after a 2-year run confirm the new floors hold in-market (Clothing $709–$883, HH Goods $565, Appliances $1,039–$1,269). See `docs/company-input-supply-chains.md` for the full price table.

---

## High Priority Issues

### 1. Housing Company Profitability Bug

**Severity**: High  
**Location**: `src/sim/company.rs:334-346`

**Description**:  
Housing companies collect rent (`last_month_rent_revenue`) but it is NOT included in the profit calculation. The `recompute_profitability()` method only considers:

```rust
self.last_month_profit = round_cents(
    self.last_month_sales - self.last_month_payroll_paid - self.last_month_loan_payment,
);
```

**Impact**:  
- Housing companies appear unprofitable despite generating rental income
- This affects IPO eligibility, distress detection, and wage decisions
- Companies may incorrectly trigger bankruptcy proceedings

**Recommended Fix**:  
Include `last_month_rent_revenue` in the profit calculation:

```rust
self.last_month_profit = round_cents(
    self.last_month_sales + self.last_month_rent_revenue 
    - self.last_month_payroll_paid - self.last_month_loan_payment,
);
```

---

## Medium Priority Issues

### 2. Money Minting Logic is Counter-Cyclical

**Severity**: Medium  
**Location**: `src/sim/country/monthly/monthly_money_minting.rs:4-48`

**Description**:  
The automatic money minting logic only triggers when inflation is BELOW target:

```rust
let inflation_shortfall = (monetary.target_inflation_rate - self.monthly_inflation_rate).max(0.0);
```

Additionally, unemployment-based minting is disabled when inflation exceeds target:

```rust
let inflation_overshoot_threshold = monetary.target_inflation_rate * monetary.mint_unemployment_inflation_cap_multiplier;
let unemployment_gap = if self.monthly_inflation_rate > inflation_overshoot_threshold {
    0.0
} else {
    raw_unemployment_gap
};
```

**Impact**:  
- During stagflation (high inflation + high unemployment), no monetary support occurs
- Central banks in reality often engage in QE during crises regardless of inflation
- May cause prolonged recessions in simulation

**Recommended Review**:  
Consider whether the unemployment gap should have a non-zero floor even during moderate inflation overshoots, or if this behavior matches intended game mechanics.

---

### 3. CPI Volatility from Annualization

**Severity**: Medium  
**Location**: `src/sim/country/monthly/monthly_cpi_bonds.rs:46-51`

**Description**:  
The monthly inflation rate is computed by annualizing the month-over-month CPI change:

```rust
let raw_annualized = (new_cpi / old_cpi).powf(12.0) - 1.0;
```

**Impact**:  
- A 2% monthly CPI change becomes ~27% "annualized"
- Compound effects can produce extreme values
- EMA smoothing may not adequately dampen volatility
- Policy rate decisions based on this metric may be unstable

**Recommended Review**:  
Consider using a rolling average or dampening factor on the raw annualized rate before EMA smoothing.

---

## Low Priority Issues

### 4. IPO Rescue Creates Moral Hazard

**Severity**: Low (Design Choice)  
**Location**: `src/sim/country/monthly/monthly_company_distress_bankruptcy.rs:97-109`

**Description**:  
Companies on the verge of bankruptcy automatically receive an IPO rescue attempt before liquidation:

```rust
if company.cash_balance <= 0.0 && !has_capacity && deep_losses {
    if self.sim_params.stock_market.enabled
        && !self.stock_market.is_listed(company_id)
    {
        let rescued = self.attempt_ipo_rescue(company_idx);
        if rescued {
            self.companies[company_idx].consecutive_missed_loan_payments = 0;
            continue;
        }
    }
    bankrupt_indices.push(company_idx);
}
```

**Impact**:  
- Creates moral hazard for poorly-run companies
- Investors may bail out distressed companies at favorable prices
- Reduces consequence severity for financial mismanagement

**Recommended Review**:  
This may be intentional game design. Consider whether IPO rescue should have stricter eligibility criteria or be configurable.

---

### 5. Missing Costs for Frequent Job Switching

**Severity**: Low  
**Location**: `src/sim/country/monthly/monthly_startup_formation_pricing/labor_dynamics.rs:47-164`

**Description**:  
This finding is historical. The simulation now applies a minimum-tenure gate before voluntary job switches, using `employment_start_day` and `LaborMobilityParams.min_tenure_months_before_switch` (default 6 months).

**Impact**:  
- Frequent job-hopping is reduced relative to the prior implementation
- Remaining turnover pressure now comes primarily from wage differentials after the tenure gate is satisfied

**Recommended Enhancement**:  
Consider adding:
- Cooldown period after job switches
- Tenure-based salary negotiation bonus

---

### 6. Arbitrary Housing Price Multiplier

**Severity**: Low  
**Location**: `src/sim/country/monthly/monthly_housing.rs:45`

**Description**:  
The housing price adjustment uses an arbitrary `2.0` multiplier:

```rust
let change = (-deviation * 2.0).clamp(
    -params.max_monthly_price_change,
    params.max_monthly_price_change,
);
```

**Impact**:  
- Price sensitivity to vacancy changes may be too aggressive
- Could lead to housing bubbles or crashes
- Lacks empirical basis

**Recommended Review**:  
Consider making this multiplier a configurable parameter or documenting the design rationale.

---

### 7. Unemployment Benefit Edge Case

**Severity**: Low  
**Location**: `src/sim/country/monthly/monthly_government_spending_initial_listings.rs:101-124`

**Description**:  
If the treasury has insufficient funds, unemployment benefits aren't paid, but `months_on_benefits` tracking logic may still increment:

```rust
if !person.is_employed()
    && is_working_age
    && austerity_benefit_amount > 0.0
    && within_duration_cap
{
    let payout = round_cents(austerity_benefit_amount.min(self.treasury_balance.max(0.0)));
    if payout > 0.0 {
        // ... payment logic
        paid_unemployment_benefit = true;
    }
}

if !person.is_employed() && is_working_age {
    if paid_unemployment_benefit {
        person.months_on_benefits = person.months_on_benefits.saturating_add(1);
    } else {
        person.months_on_benefits = 0;  // Reset if no benefit paid
    }
}
```

**Impact**:  
- `months_on_benefits` correctly resets to 0 if no payment occurs
- However, benefit duration caps (`max_benefit_months`) are evaluated before checking treasury solvency
- May cause unexpected behavior during treasury insolvency

**Recommended Review**:  
The current logic appears correct but should be verified with edge case tests.

---

### 8. Bank Health Score Discrepancy

**Severity**: Low (Transparency Issue)  
**Location**: `src/sim/country/accessors.rs:370-386`

**Description**:  
Public banks receive an implicit stability bonus:

```rust
BankKind::Public => (bank.health_score_public(
    self.treasury_balance,
    params.public_health_floor,
    params.public_health_liquidity_weight,
    params.public_health_capital_weight,
    params.public_health_sovereign_support_weight,
) + params.public_bank_stability_bonus * 0.25)
    .clamp(0.0, 1.0),
```

**Impact**:  
- Public banks appear healthier than their financials warrant
- Masks potential liquidity issues
- May delay necessary interventions

**Recommended Review**:  
This is realistic (sovereign backing) but should be clearly communicated to players.

---

### 9. Cross-Country Deposits Missing Exchange Rate Risk

**Severity**: Low  
**Location**: `src/sim/params/monetary_banking.rs:175-183`

**Description**:  
The cross-country deposit system doesn't model:
- Currency conversion costs
- Exchange rate fluctuation risk
- Transfer fees

**Impact**:  
- Unrealistic arbitrage opportunities
- Missing risk factor for international banking
- Simplified economic model

**Recommended Enhancement**:  
Consider adding exchange rate risk mechanics for foreign deposits.

---

## Summary Table

| # | Issue | Severity | File | Status |
|---|-------|----------|------|--------|
| R1 | Recipe-Sector Minimum Price Bug | High | `company.rs` — `minimum_viable_unit_price()` | **Resolved 2026-03-17** |
| 1 | Housing Company Profitability Bug | High | `company.rs:334-346` | Open |
| 2 | Money Minting Counter-Cyclical Logic | Medium | `monthly_money_minting.rs:4-48` | Open |
| 3 | CPI Annualization Volatility | Medium | `monthly_cpi_bonds.rs:46-51` | Open |
| 4 | IPO Rescue Moral Hazard | Low | `monthly_company_distress_bankruptcy.rs:97-109` | Open |
| 5 | Job Switching No Costs | Low | `monthly_startup_formation_pricing/labor_dynamics.rs:47-164` | Open |
| 6 | Arbitrary Housing Price Multiplier | Low | `monthly_housing.rs:45` | Open |
| 7 | Unemployment Benefit Edge Case | Low | `monthly_government_spending_initial_listings.rs:101-124` | Open |
| 8 | Bank Health Score Discrepancy | Low | `accessors.rs:370-386` | Open |
| 9 | Cross-Country Deposits No FX Risk | Low | `params/monetary_banking.rs:175-183` | Open |

---

## Recommended Action Items

1. **Immediate**: Fix housing company profitability bug (Issue #1)
2. **Short-term**: Review money minting logic for stagflation scenarios (Issue #2)
3. **Short-term**: Evaluate CPI calculation stability (Issue #3)
4. **Medium-term**: Document design decisions for IPO rescue and public bank bonuses
5. **Future**: Consider enhancements for job switching costs and FX risk

---

## Testing Recommendations

For each issue identified, consider adding:

1. Unit tests for edge cases (e.g., housing companies with only rental income)
2. Integration tests for multi-month scenarios
3. Invariant checks during simulation runs
4. Benchmark scenarios for economic stability
