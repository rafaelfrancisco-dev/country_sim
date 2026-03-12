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

It then buys missing recipe inputs through the same cheapest-supplier quote logic used by household buyers:

- landed price includes export and import tariffs
- purchases can be domestic or cross-border
- buyer cash comes from company cash, not person cash
- seller sales and inventory depletion are credited exactly like household purchases
- cross-border purchases still update import/export metrics and tariff collection

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

- observed sales by current producers
- household demand proxy
- company input-demand proxy derived from recipe usage by downstream sectors

This makes upstream goods like `Raw Metals`, `Timber`, and `Fuel` startable even when direct household demand is weak.

Generator seeding also ensures the initial economy includes producers for the new upstream goods so downstream sectors do not deadlock immediately.

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
