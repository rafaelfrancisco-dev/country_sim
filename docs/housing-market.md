# Housing Market

Housing (Good ID 2) is a **durable asset**, not a monthly consumable. It is managed entirely outside the round-robin goods market and has its own stock, ownership, rental, mortgage, and company-facility pipeline.

## Housing Stock Model

The country maintains four occupancy buckets that always satisfy the invariant:

```
housing_stock_total == occupied_owned + occupied_rented + company_facility + vacant
```

| Field | Meaning |
|---|---|
| `housing_stock_total` | All units ever built |
| `housing_stock_occupied_owned` | Units owned outright or under mortgage |
| `housing_stock_occupied_rented` | Units rented from a landlord company |
| `housing_stock_company_facility` | Units allocated to company facilities |
| `housing_stock_vacant` | Unoccupied units (price pressure falls when high) |

Housing companies accumulate `inventory_units` from daily production. Each month, `process_housing_construction()` drains that inventory into the housing stock and assigns the new units as `rental_units_owned` on the producing company.

## Company Facility Housing

Housing is also used by companies as facility capacity, but this is tracked separately from household occupancy.

- Facility demand is **not** purchased through the daily household goods market.
- Each month, after new housing construction is drained into stock, companies compute a target facility requirement from employee count.
- Missing facility units are drawn from `housing_stock_vacant`.
- Surplus facility units are released back into vacancy.
- Occupied facility units are charged monthly using `monthly_rent`.

The company-side effect is:

- `cash_balance` decreases
- `last_month_facility_spending` increases
- `facility_housing_units` updates

The country-side effect is:

- `housing_stock_company_facility` tracks occupied company units
- household occupancy counters stay unchanged

This separation keeps renter/owner invariants meaningful while still making housing an upstream company constraint.

## Person Housing Status

Every person holds a `HousingStatus` enum:

| Status | Description |
|---|---|
| `Unhoused` | No housing (children, very poor, or recently foreclosed) |
| `Renter { landlord_company_id, landlord_country_id }` | Pays monthly rent to a landlord company |
| `Owner` | Owns home outright (mortgage fully paid off) |
| `Mortgaged { mortgage: MortgageAccount }` | Owns home subject to an active mortgage |

At world generation, adults are assigned ~60% Owner, ~35% Renter, ~5% Unhoused. Children are always Unhoused.

## Property Price Index

`housing_price_index` is a scalar (initial value ~$80,000) representing the market price of a standard housing unit. It updates monthly based on vacancy deviation from target:

```
vacancy_rate  = housing_stock_vacant / housing_stock_total
deviation     = vacancy_rate - target_vacancy_rate      (default 5%)
change        = -deviation * 2.0
change        = clamp(change, -max_monthly_price_change, +max_monthly_price_change)
new_index     = old_index * (1 + change)
```

When vacancy is **below** target (shortage), prices rise. When vacancy is **above** target (surplus), prices fall. A `HousingPriceChanged` event is emitted when `|change| > 0.5%`.

`monthly_rent = housing_price_index * rent_as_fraction_of_price` (default 0.7% per month, ~8.4% annual yield).

## Rent Collection

Each month, every `Renter` person pays `monthly_rent` to their landlord company:

- Person `cash_balance` decreases; company `cash_balance` and `last_month_rent_revenue` increase.
- If the person cannot afford the full rent, a partial payment is made and a `RentUnpaid` event is emitted.
- `monthly_rent_collected` on the country accumulates total rent received.

## Mortgages

### Origination

When a renter transitions to ownership, the bank originates a mortgage:

- **Down payment**: `price_index * mortgage_down_payment_rate` (default 20%) paid from person cash to the landlord company.
- **Principal**: remaining 80% created by the bank (`bank.created_money_outstanding += principal`).
- **Rate**: `policy_rate + mortgage_spread_over_policy` (default +2 pp), clamped to `[rate_floor, rate_ceiling]`.
- **Term**: default 240 months (20 years).
- **Monthly payment**: computed via standard amortization (`payment_for(principal, rate, term)`).

Eligibility requires:
- `cash_balance >= down_payment`
- `monthly_payment / monthly_income <= mortgage_max_dti` (default 28%)
- Deterministic probability check: `hash64(month, person_id) % 10000 < renter_purchase_probability * 10000` (default 1%/month)

### Monthly Payments

Each month, `collect_mortgage_payments()` processes every mortgaged person:

- Interest accrues on `principal_balance * annual_rate / 12`.
- Payment = `min(scheduled, available cash)`.
- Interest portion: credited to `bank.reserve_balance` and `bank.last_month_interest_income`.
- Principal portion: reduces `bank.created_money_outstanding` (money destruction).
- `mortgage.collateral_value` is updated to the current `housing_price_index`.

### Missed Payments and Foreclosure

If a payment is missed, `mortgage.months_missed` increments. At `foreclosure_missed_months` (default 3) consecutive misses:

- The remaining `principal_balance` is written off as a credit loss (`bank.last_month_credit_losses`).
- `bank.created_money_outstanding` is reduced by the written-off balance.
- The person transitions to `HousingStatus::Unhoused`.
- A `Foreclosure` event is emitted.
- The housing stock counter shifts: `occupied_owned -= 1`, `vacant += 1`.

### Payoff

When `principal_balance` reaches zero, the person transitions to `HousingStatus::Owner` and a `MortgagePaidOff` event is emitted.

## Renter-to-Owner Transitions

`process_renter_to_owner_transitions()` runs monthly after mortgage collection. It uses a collect-then-apply pattern (four separate loops) to avoid Rust split-borrow issues when updating people, companies, and banks simultaneously.

The housing stock counter shifts on purchase: `occupied_rented -= 1`, `occupied_owned += 1`. The landlord company loses one `rental_units_owned`.

## CPI Basket

Because housing is durable, its contribution to the CPI basket uses `monthly_rent` instead of a goods-market unit price:

```
housing_cpi_price = monthly_rent  (fallback: base_unit_cost if monthly_rent == 0)
```

This prevents the housing company's arbitrary `unit_price` from distorting inflation measurement.

## Events

| Event | Trigger |
|---|---|
| `HomePurchased` | Renter successfully purchases via mortgage |
| `Foreclosure` | Missed payments reach threshold |
| `MortgagePaidOff` | Mortgage balance reaches zero |
| `HousingPriceChanged` | Monthly price index move > 0.5% |
| `RentUnpaid` | Renter cannot pay full rent |

All housing events belong to `EventCategory::Housing` and can be filtered in the Events tab.

## Parameters (`HousingMarketParams`)

| Parameter | Default | Description |
|---|---|---|
| `mortgage_term_months` | 240 | Amortization term |
| `mortgage_down_payment_rate` | 0.20 | Down payment as fraction of price |
| `mortgage_max_dti` | 0.28 | Max monthly payment / monthly income |
| `mortgage_spread_over_policy` | 0.02 | Mortgage rate = policy rate + spread |
| `foreclosure_missed_months` | 3 | Consecutive missed payments before foreclosure |
| `rent_as_fraction_of_price` | 0.007 | Monthly rent = price_index × this |
| `target_vacancy_rate` | 0.05 | Vacancy rate that keeps prices stable |
| `max_monthly_price_change` | 0.02 | Maximum monthly price index move (±2%) |
| `renter_purchase_probability` | 0.01 | Monthly probability an eligible renter buys |
| `initial_owner_rate` | 0.60 | Share of adults who start as owners |
| `initial_renter_rate` | 0.35 | Share of adults who start as renters |
| `initial_price_index` | 80_000.0 | Starting housing price index |

## UI

- **Charts tab page 4**: Housing Price Index, Homeownership Rate, Monthly Rent, Vacancy Rate, Monthly Home Purchases, Monthly Foreclosures, Mortgage Rate, Rent/Price Ratio.
- **Country details**: Housing Market section showing price index, rent, stock breakdown, vacancy rate, homeownership rate, last-month purchase/foreclosure totals, last-month mortgage and rent totals, and current MTD mortgage payments.
- **Person details**: Housing status, mortgage balance/payment/rate/months remaining, missed payments (if any), landlord name for renters, and last-month rent and mortgage payment amounts.

## Money Invariants

The housing pipeline participates in the bank money creation/destruction invariant:

```
bank.created_money_outstanding += principal        on mortgage origination
bank.created_money_outstanding -= principal_paid   on each monthly payment
bank.created_money_outstanding -= remaining_balance on foreclosure write-off
```
