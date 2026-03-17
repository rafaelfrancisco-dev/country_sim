# Company Input Purchasing and Supply Chains

## Overview
The goods system now has a first-pass business-to-business layer.

- Household demand still drives the existing non-durable round-robin goods market.
- Companies now also buy required inputs before they produce output.
- Production is no longer free once labor exists; downstream firms need both labor effort and recipe inventory.
- Housing remains special: company facility demand is handled outside the household round-robin because housing is durable stock, not a person-style consumable.

This pass is intentionally static and deterministic:

- recipes are fixed tables in code
- industry job catalogs are fixed tables in code
- company input purchasing happens before daily production
- company facility housing is settled in the monthly housing pipeline

## Goods Added
Three upstream goods were added at the end of the goods catalog so earlier IDs remain stable:

- `15`: `Raw Metals`
- `16`: `Timber`
- `17`: `Fuel`

These goods have low household demand factors and primarily exist as upstream company inputs.

## Static Recipes
Each produced good maps to a `CompanyInputRecipe` that describes required input units per one unit of output.

### No-input sectors
- `Raw Metals`
- `Timber`
- `Fuel`
- `Water`
- `Food`
- `Electricity`
- `Medicine`
- `Transport`
- `Internet`
- `Education Services`
- `Entertainment`

### Recipe-driven sectors
- `Housing`: `Timber x1`, `Raw Metals x1`
- `Clothing`: `Timber x1`
- `Hygiene Products`: `Fuel x1`
- `Household Goods`: `Timber x1`, `Fuel x1`
- `Electronics`: `Raw Metals x1`, `Fuel x1`
- `Appliances`: `Raw Metals x2`, `Fuel x1`
- `Cars`: `Raw Metals x3`, `Fuel x2`

Recipes are stored centrally in the goods domain so generators, startups, production, and tests all share the same source of truth.

## Industry Job Catalogs
Company hiring is now industry-specific instead of drawing from the full global job list.

### Sector mappings
- Mine-like sectors (`Raw Metals`, `Fuel`):
  - `Construction Worker`
  - `Driver`
  - `Manufacturing Operator`
- Timber and basic production (`Timber`, `Water`, `Food`, `Electricity`):
  - `Driver`
  - `Manufacturing Operator`
  - `Office Clerk`
- Housing:
  - `Construction Worker`
  - `Driver`
  - `Office Clerk`
- Consumer and industrial manufacturing (`Clothing`, `Hygiene Products`, `Household Goods`, `Electronics`, `Appliances`, `Cars`):
  - `Manufacturing Operator`
  - `Driver`
  - `Office Clerk`
- Medicine:
  - `Registered Nurse`
  - `Office Clerk`
  - `Driver`
- Education Services:
  - `Teacher`
  - `Office Clerk`
- Internet:
  - `Software Developer`
  - `Office Clerk`
- Transport:
  - `Driver`
  - `Office Clerk`
- Entertainment:
  - `Office Clerk`
  - `Driver`

Practical consequence:

- generator-created companies only advertise jobs that fit their sector
- generator-created companies that start without staff now seed openings with one production-role slot first, plus at most one support-role slot
- startup formation is anchored on the selected good's designated production role rather than an arbitrary founder profession
- public backfill companies also inherit sector-appropriate catalogs

### Production roles
The simulation now distinguishes a sector's full job catalog from the single role that actually contributes output effort:

- Water, Food, Electricity, upstream extraction, and consumer/industrial manufacturing use `Manufacturing Operator`
- Housing uses `Construction Worker`
- Medicine uses `Registered Nurse`
- Education Services uses `Teacher`
- Internet uses `Software Developer`
- Transport uses `Driver`
- Entertainment uses `Office Clerk`

The persisted field name is still `manufacturing_employee_count` for compatibility, but at runtime it now means "productive employees for this company's designated production role."

## Company State
Companies now track additional state beyond output inventory:

- `input_inventory_units[GoodId]`: owned material inventory used for recipes
- `facility_housing_units`: housing units currently allocated to company facilities
- `last_month_input_spending`
- `last_completed_month_input_spending`
- `last_month_facility_spending`
- `last_completed_month_facility_spending`

These feed profitability so input purchases and facility costs affect expansion and distress behavior.

## Daily Goods Pipeline
The daily goods sequence is now:

1. prepare goods-market runtime and quote caches
2. company input purchasing
3. daily company production
4. household superficial-stock consumption
5. household essential round-robin market
6. discretionary superficial purchases

### Company purchasing
Before production, each company estimates same-day potential output from:

- production-role headcount
- accumulated production effort
- per-good `effort_per_unit`

It then buys missing recipe inputs through the same price-led quote logic used by household buyers:

- landed price includes export and import tariffs
- purchases can be domestic or cross-border
- buyer cash comes from company cash, not person cash
- seller sales and inventory depletion are credited exactly like household purchases
- cross-border purchases still update import/export metrics and tariff collection
- supplier choice rotates across a bounded set of near-cheapest in-stock quotes instead of concentrating all demand on the single cheapest seller

The goods market now models buyers as either a person or a company so company demand does not overload the person-only path.

## Production Gating
After input purchasing, production runs.

A company only produces when:

- it has enough labor effort to cover at least one unit
- it has enough recipe inventory for that unit

For each unit produced:

- required input inventory is decremented
- output `inventory_units` is incremented
- monthly produced-unit counters update normally

If recipe inventory runs short, remaining potential output is left unmet rather than producing free units.

Sell-through and excess-inventory throttling still apply, so a firm can be blocked by:

- too much finished-goods inventory
- missing recipe inputs
- insufficient production labor

## Company Facility Housing
Company facility demand is modeled outside the person housing round-robin.

Why:

- household housing uses durable stock accounting
- household occupancy counters are meaningful invariants
- reusing the person-style buyer path would mix company facilities with renter/owner occupancy

### Monthly company facility step
After housing construction drains new housing-company output into stock:

- each company computes a target facility requirement from employee count
- missing facility units are acquired from vacant housing stock
- excess facility units are released back to vacancy
- occupied facility units are charged monthly using the housing rent level

This affects:

- company cash
- company facility spending
- treasury balance
- a separate `housing_stock_company_facility` counter

It does not affect:

- `housing_stock_occupied_owned`
- `housing_stock_occupied_rented`
- renter/owner person status

The housing stock invariant is now:

```text
housing_stock_total ==
    housing_stock_occupied_owned +
    housing_stock_occupied_rented +
    housing_stock_company_facility +
    housing_stock_vacant
```

## Startup and Backfill Heuristics
Startup targeting no longer relies only on household demand proxies.

Expected revenue for a candidate good now blends:

- entrant-share observed sales by current producers
- household demand proxy
- company input-demand proxy derived from recipe usage by downstream sectors

Startup pricing is also clamped to a labor-aware minimum viable unit price so new firms are not admitted at obviously loss-making price levels.

This makes upstream goods like `Raw Metals`, `Timber`, and `Fuel` startable even when direct household demand is weak.

Generator seeding also ensures the initial economy includes producers for the new upstream goods so downstream sectors do not deadlock immediately.

## Minimum Viable Unit Price

Every good has a floor price below which no company should operate: `minimum_viable_unit_price(good_id, worker_effort_per_month, min_margin_rate)` in `src/sim/company.rs`.

### Formula

```
input_recipe_cost = Σ minimum_viable_unit_price(input_good_id) × qty_required
baseline_cost     = max(good.base_unit_cost, input_recipe_cost)
                  + direct_labor_cost × DIRECT_LABOR_OVERHEAD_MULTIPLIER (2.5×)
floor_price       = baseline_cost × (1 + min_margin_rate)
```

### Why `max(base_unit_cost, input_recipe_cost)` not `+`

`base_unit_cost` was designed as a catch-all material cost for goods that lacked explicit recipes, and it already captures full input cost for goods like Housing ($520 ≥ Timber+RawMetal at $344). Adding recipe cost on top would double-count. Using `max` means:

- Recipe-free goods (Water, Food, Electricity, Raw Metals, Timber, Fuel, …): recipe cost is 0, so `base_unit_cost` wins.
- Goods where `base_unit_cost` understates actual recipe cost: recipe cost wins. This was the root cause of the structural pricing bug below.
- Goods where `base_unit_cost` is already generous (Housing, Cars): `base_unit_cost` wins, no double-count.

No infinite recursion: all input goods (Raw Metals, Timber, Fuel) have `MATERIAL_FREE_RECIPE`, so the recursive call bottoms out immediately.

### Structural pricing bug — root cause and fix (2026-03-17)

For five recipe-driven sectors, `base_unit_cost` badly understated the actual cost of the required inputs at market prices:

| Good            | base_unit_cost | actual recipe cost at min-viable inputs | old floor | new floor |
|-----------------|---------------:|-----------------------------------------:|----------:|----------:|
| Clothing        |            $38 |                          $159 (×1 Timber) |      $193 |      ~$333 |
| Hygiene Products|            $22 |                          $176 (×1 Fuel)   |      $149 |      ~$326 |
| Household Goods |            $58 |              $335 (×1 Timber + ×1 Fuel)  |      ~$220 |      ~$565 |
| Electronics     |           $180 |             $361 (×1 RawMetal + ×1 Fuel) |      $562 |      ~$771 |
| Appliances      |           $240 |            $546 (×2 RawMetal + ×1 Fuel)  |      $687 |     ~$1,040 |

Before the fix, companies in these sectors set their initial price at the (too-low) floor, then immediately operated below their actual cost of materials. Dynamic loss-recovery pricing (`close_month`) can eventually self-correct (up to 25%/month), but many companies exhausted their starting cash before reaching a viable price.

The fix makes `minimum_viable_unit_price()` recursively compute actual recipe input cost, so the floor already reflects true material cost at model launch. Observed market prices after a 2-year run confirm the new floors hold:

| Good            | Observed avg price (2yr) |
|-----------------|-------------------------:|
| Clothing        |                  $709–$883 |
| Hygiene Products|                  $594–$836 |
| Household Goods |                       $565 |
| Appliances      |               $1,039–$1,269 |

The same corrected floor is used in `estimate_startup_monthly_input_cost()` so startup viability checks reflect true input costs even when market prices have not yet propagated.

## Limits of This Pass
This is deliberately a first pass, not a full industrial IO model.

Not modeled yet:

- capital goods and capex expansion
- recipe substitution
- negotiated contracts between firms
- safety stock policies beyond simple pre-production replenishment
- persistent facility ownership markets distinct from rent-like company housing cost

The current system is meant to add:

- upstream bottlenecks
- realistic sector hiring
- cross-border material imports
- materially different behavior between basic and complex sectors
