# Essential Needs Model

## Overview
The simulation now treats essential deprivation as a month-end accounting problem instead of a daily purchase-failure counter.

- Shared essentials:
  - housing is satisfied at the household level
  - electricity demand includes a household base load plus a smaller per-person increment
- Individual essentials:
  - water
  - food
  - medicine

This keeps the old broad unmet headline for compatibility while adding severity levels that are closer to how real deprivation behaves over time.

## Household Model
- People are assigned to persistent households.
- Household membership is rebuilt from the current population and housing state.
- Adults with housing anchor household formation; remaining members are assigned deterministically.
- Household size is used to scale shared utility demand.

## Stockable Essentials
Stockable essentials no longer rely on a sampled same-day purchase to decide whether the whole month failed.

- During the month, buyers purchase toward:
  - their monthly requirement
  - plus a bounded safety buffer
- At month-end, stock is compared against the month’s required amount.
- Remaining stock carries into the next month.

## Unmet Classifications
- `any unmet`: at least one essential obligation was not fully covered by month-end.
- `severe unmet`: the remaining shortfall for at least one essential crossed the severe threshold.
- `chronic unmet`: severe unmet persisted across consecutive months.

The compatibility fields remain:
- person unmet/met masks
- `monthly_unmet_need_events`
- `count_with_unmet_needs`

Additive fields expose the refined signals:
- `monthly_severe_unmet_need_events`
- `count_with_severe_unmet_needs`
- `count_with_chronic_unmet_needs`
- `monthly_chronic_unmet_need_count`

## Housing Semantics
- Housing is not purchased through the daily round-robin goods market.
- Month-end housing status uses household shelter plus rent/mortgage outcomes.
- A household without adequate shelter marks housing unmet for its members.
