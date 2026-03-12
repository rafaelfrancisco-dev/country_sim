# Managed FX and Central-Bank Reserves

The simulation now treats each country's public bank as its central bank for foreign exchange.

## Balance-Sheet Model

There are now two distinct reserve concepts on the public bank:

- `reserve_balance`: domestic banking liquidity in the country's own currency
- `fx_reserve_assets_base`: foreign-currency reserve assets, measured in base units for cross-country accounting

These are intentionally separate. Domestic reserve cash still participates in money-supply accounting. FX reserves do not.

The public bank also carries:

- `fx_reserve_assets_by_source`: which foreign-country assets make up the reserve pool
- `central_bank_liquidity_liabilities_local`: the domestic counterpart liability created or absorbed during managed-FX settlement

## Settlement Flow

All cross-border currency conversion is routed through the world FX layer with the public bank as intermediary.

For an outflow such as an import or foreign investment:

1. the payer loses local purchasing power in their home country
2. the home public bank absorbs local liquidity
3. the home public bank spends `fx_reserve_assets_base`
4. the destination public bank receives the foreign asset inflow
5. the destination public bank issues local liquidity so the payee can be credited in local currency

For an inflow such as an export or foreign capital inflow, the same path happens in reverse.

## Hard Reserve Constraint

Managed FX is now reserve-constrained.

- If the payer-side public bank does not have enough `fx_reserve_assets_base`, the cross-border settlement does not complete.
- Callers that require full settlement, such as stock trades, pre-check reserve sufficiency and abort the transaction when the public bank cannot cover it.
- FX reserve pressure events are emitted instead of silently allowing the country to spend an unowned shadow pool.

## Country-Level Metrics

The UI, history, and persistence layers still expose country-level FX reserve metrics:

- `Country::fx_reserves_base()`
- `Country::fx_reserves_by_source()`

These are now derived compatibility views backed by the public bank's owned FX asset ledger. They are not mutable source-of-truth state.

## Initialization

Initial FX reserves are seeded onto the public bank, split evenly across foreign currencies only, excluding the country's own currency. The initial counterpart is booked to the public bank's local liquidity liability account.
