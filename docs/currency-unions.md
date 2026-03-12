# Currency Unions

## Overview

Currency unions allow multiple countries to share a single currency, similar to real-world arrangements like the Eurozone. Countries in a currency union share:

- A 1:1 exchange rate between them
- One designated "issuer" country that controls monetary policy
- Only the issuer can mint new money for that currency
- All other countries in the union inherit the issuer's exchange rate

## Design

### Currency Model

Currency is now a first-class entity with an explicit issuer:

```rust
pub struct Currency {
    pub spec: CurrencySpec,
    pub issuer_country_idx: usize,
}
```

### Country Changes

Each `Country` stores:
- `currency_idx: usize` - index into the world's currency pool
- `currency_spec: CurrencySpec` - the currency specification (code, name, symbol)
- `is_currency_issuer: bool` - whether this country can mint money for its currency

### World Currency Pool

The `World` maintains:
- `currencies: Vec<Currency>` - canonical list of all currencies
- `currency_index_by_spec: HashMap<CurrencySpec, usize>` - O(1) lookup from spec to index

## Generation Logic

Currency state is assigned in two places:

**During world generation** (`generate_countries` → `assign_currency_indices`):
1. Each country receives a `CurrencySpec` based on its seed
2. Countries are grouped by their `CurrencySpec`
3. For each unique `CurrencySpec`, a `Currency` entry is created
4. The first country with that spec becomes the issuer
5. All countries sharing a spec receive the same `currency_idx`
6. Only the issuer country gets `is_currency_issuer = true`

**During `World::new`** (`assign_currency_state_from_specs`):
- The same grouping logic runs on the already-constructed countries
- This ensures that manually-constructed worlds (tests, persistence load) also have consistent currency state
- For generated worlds the result is a no-op (indices were already correctly set)

## FX System Behavior

### Convert Between Countries

When converting between two countries:
- If they share the same `currency_idx`, returns 1:1 rate immediately
- No FX conversion or reserve adjustments are made
- This ensures countries in the same currency union trade at par

### Exchange Rate Updates

FX rates are controlled by currency issuers only:
- Only the issuer country's FX pressure affects the currency's rate
- Non-issuer countries inherit the issuer's exact rate
- All countries sharing a currency have identical `fx_rate_local_per_base`

### FX Reserves

Each country maintains its own reserve pool in `fx_reserves_by_source`:
- Reserves are tracked per-source-country in base units
- This allows countries in a currency union to hold each other's reserves separately
- Reserve spending draws from the payer's currency bucket first, then proportionally from others

### Initialization

At startup:
- Issuer countries receive a unique FX rate based on their index
- Non-issuer countries in the same currency union start with the issuer's exact rate
- All countries get initial FX reserves split evenly across foreign currencies

## Money Minting

Only currency issuers can mint new money:
- `process_auto_money_minting()` starts with a guard: `if !self.is_currency_issuer { return; }`
- Non-issuer countries' money supply changes only from cross-border flows and the issuer's minting
- This ensures monetary policy is centralized at the issuer level

## UI Display

The FX tab shows currency union information:
- A new "Issuer" column displays `*` next to the issuer country in the leaderboard
- The issuer country clearly controls the FX rate for all countries using its currency

## Examples

### Eurozone-style Union
```
Country 0: EUR, is_currency_issuer = true
Country 1: EUR, is_currency_issuer = false
Country 2: EUR, is_currency_issuer = false
```
- All three countries share EUR currency (idx 0)
- Country 0 controls the EUR exchange rate
- Countries 1 and 2 inherit Country 0's rate
- Only Country 0 can mint EUR

### USD Union
```
Country 3: USD, is_currency_issuer = true
Country 4: USD, is_currency_issuer = false
```
- Both countries share USD currency (idx 1)
- Country 3 controls the USD exchange rate
- Only Country 3 can mint USD

## Testing

Key scenarios covered:
1. Two countries with same currency trade at 1:1 rate
2. Non-issuer country cannot mint money
3. Issuer controls FX rate for all countries using its currency (enforced each monthly update)
4. FX reserves tracking works correctly within currency unions
5. Round-trip serialization preserves currency union state (`currency_idx`, `is_currency_issuer` persisted to and loaded from DB)

Run tests with:
```bash
cargo test
cargo test --features runtime-invariants
```
