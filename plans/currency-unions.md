# Currency Unions / Shared Currencies Implementation Plan

## Problem Statement

When countries have the same currency symbol and name (e.g., USD with "$"), the FX system treats them as separate currencies with independent exchange rates. In reality, countries sharing a currency (like Eurozone countries) should:

1. Have a 1:1 exchange rate between them
2. Have one "issuer" country that acts as the central bank
3. Only the issuer can mint new money for that currency
4. The issuer controls the FX rate for the shared currency

## Design Decisions

- **Currency Model**: Currency as a first-class entity with an explicit issuer
- **Monetary Policy**: Non-issuer countries skip minting entirely
- **FX Reserves**: Per-country holdings (each country tracks its own reserves)

---

## Implementation Plan

### Phase 1: Data Model Changes

#### 1.1 Update `CurrencySpec` (`src/sim/currency.rs`)

- Add `Hash` implementation to `CurrencySpec` for use as HashMap key
- Create new `Currency` struct:
  ```rust
  pub struct Currency {
      pub spec: CurrencySpec,
      pub issuer_country_idx: usize,
  }
  ```

#### 1.2 Modify `World` (`src/sim/world/mod.rs`)

- Add `pub currencies: Vec<Currency>` - the canonical list of currencies
- Add `currency_index_by_spec: HashMap<CurrencySpec, usize>` for O(1) lookup

#### 1.3 Modify `Country` (`src/sim/country/mod.rs`)

- Replace `currency: CurrencySpec` with `currency_idx: usize`
- Add `is_currency_issuer: bool` flag
- Keep `fx_rate_local_per_base` and `fx_reserves_base` per-country for holdings tracking

---

### Phase 2: Generation Logic

#### 2.1 Update `generator.rs`

After all countries are created:

1. Iterate through countries and group by their `CurrencySpec`
2. For each unique `CurrencySpec`, create a `Currency` entry
   - First country with that spec becomes the issuer (`issuer_country_idx`)
3. Assign `currency_idx` to all countries sharing that spec
4. Set `is_currency_issuer = true` only for the issuer country

---

### Phase 3: FX System Updates

#### 3.1 Update `fx.rs`

- `convert_between_countries()`: 
  - If both countries share the same `currency_idx`, return 1:1 rate immediately
  - No FX conversion needed for same-currency transactions
  
- `settle_cross_border_cash()`:
  - Skip FX reserve adjustments when same currency
  
- `update_fx_rates()`:
  - Only the issuer country's pressure affects the currency's rate
  - Non-issuer countries inherit the issuer's rate
  
- `initialize_fx_state()`:
  - Non-issuers start with `fx_rate = 1.0` relative to their issuer

---

### Phase 4: Money Minting

#### 4.1 Update `monthly_money_minting.rs`

Add guard clause at the start:

```rust
if !self.is_currency_issuer {
    return;
}
```

Non-issuer countries cannot mint money - money supply changes only from:
- Cross-border flows
- The issuer's minting decisions

---

### Phase 5: Accessor Updates

#### 5.1 Update `accessors.rs` and `accessors_fx.rs`

- Replace direct `self.currency` access with world lookup
- Add `currency(&self, world: &World) -> &Currency` helper
- Add `currency_spec(&self, world: &World) -> &CurrencySpec` helper
- Update `currency_code()`, `currency_name()`, `currency_symbol()` to use world reference

---

### Phase 6: UI Updates

#### 6.1 Update `snapshot/fx.rs`

- FX leaderboard: Group countries by currency
- Show "(issuer)" badge next to the controlling country
- Cross-rate calculation respects shared currency (1:1)

#### 6.2 Update `snapshot/models.rs`

- Add `is_issuer: bool` to FX row models

#### 6.3 Update `render/fx.rs`

- Display issuer indicator in UI

---

## Files to Modify

| File | Change |
|------|--------|
| `src/sim/currency.rs` | Add `Hash` for `CurrencySpec`, add `Currency` struct |
| `src/sim/world/mod.rs` | Add `currencies` vec, `currency_index_by_spec` map |
| `src/sim/country/mod.rs` | Replace `currency` field, add `is_currency_issuer` |
| `src/sim/generator.rs` | Build currency pool after country generation |
| `src/sim/world/fx.rs` | Shared currency detection, 1:1 rate, issuer-only rate updates |
| `src/sim/country/monthly/monthly_money_minting.rs` | Skip for non-issuers |
| `src/sim/country/accessors.rs` | Update currency accessor methods |
| `src/sim/country/accessors_fx.rs` | Update FX accessors if needed |
| `src/components/simulation/snapshot/fx.rs` | UI grouping & issuer display |
| `src/components/simulation/snapshot/models.rs` | Add issuer flag to models |
| `src/components/simulation/render/fx.rs` | Render issuer badge |

---

## Testing Considerations

1. Unit test: Two countries with same `CurrencySpec` share one `Currency` entry
2. Unit test: Non-issuer country cannot mint money
3. Unit test: FX conversion between same-currency countries is 1:1
4. Integration test: Full simulation with currency unions works correctly
5. Visual test: UI shows issuer badge correctly
