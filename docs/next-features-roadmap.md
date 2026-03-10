# Next Features Roadmap

This document captures the next high-value feature ideas for making the simulation more complete and lifelike.

## Summary

The simulation already has broad coverage:

- daily demography and labor
- monthly fiscal, banking, and monetary flows
- housing and mortgages
- cross-country migration
- global goods trading
- stock markets
- disasters
- CSV export and TUI analysis views

The next features should focus on systems that are still structurally simplified rather than adding more surface area without deepening realism.

## Top Priorities

### 1. Per-Country Currencies and FX

Add separate currencies, exchange rates, and FX settlement for cross-border trade and finance.

Why this matters:

- cross-border activity is still effectively single-currency
- there is no exchange-rate pressure, FX reserve dynamic, or currency crisis behavior
- international trade and capital flows will feel much more lifelike once countries can diverge monetarily

Suggested scope:

- add a currency per country
- add FX rates between countries
- settle imports, exports, foreign deposits, and foreign equity flows through FX conversion
- track FX reserves and reserve depletion
- surface FX moves and pressure events in charts/events/export

Likely touch points:

- `docs/game-context.md`
- `docs/money-simulation-requirements.md`
- `src/sim/world/mod.rs`
- `src/sim/world/monetary.rs`
- `src/sim/world/cross_country_banking.rs`
- `src/sim/world/cross_market_stock.rs`

### 2. Country-Level Central Banks

Replace the single world policy rate with country-specific monetary policy.

Why this matters:

- the current shared policy regime suppresses realistic divergence between countries
- different inflation and unemployment conditions should produce different rates, spreads, and credit cycles

Suggested scope:

- give each country its own central-bank policy rate
- run a local Taylor-rule style update per country
- propagate local policy into deposit, loan, and mortgage rates
- allow policy divergence to influence migration, lending, capital flows, and FX

Likely touch points:

- `docs/money-simulation-requirements.md`
- `src/sim/world/monetary.rs`
- `src/sim/country/mod.rs`
- `src/sim/params/monetary_banking.rs`

### 3. Labor Market Frictions and Richer Labor States

Deepen the labor model beyond qualification-gated instant matching.

Why this matters:

- the current labor market is already structured, but still too frictionless
- more realistic employment dynamics will improve wages, migration, debt stress, poverty, and firm behavior

Suggested scope:

- add explicit labor-force states:
  - student
  - caregiver
  - disabled
  - retired
  - part-time
  - informal worker
- add job-search frictions:
  - vacancy duration
  - search duration
  - interviews / acceptance probability
  - reservation wage behavior
  - relocation or commuting cost
- allow firms to hold vacancies open and compete over time instead of filling immediately

Likely touch points:

- `docs/demographics-labor-model.md`
- `src/sim/person.rs`
- `src/sim/country/daily/labor_market.rs`
- `src/sim/country/monthly/monthly_labor_churn_separations.rs`

### 4. Family and Household Demography

Move beyond crude population-rate births toward household-driven demography.

Why this matters:

- births currently depend on a crude population rate, not age, partnership, or household formation
- family structure would make housing, poverty, migration, and labor participation more believable

Suggested scope:

- add sex / fertility state or an equivalent fertility model by age band
- add partnership / family-unit formation
- make births depend on household composition and adult ages
- represent child dependency explicitly in household budgets and labor participation
- allow migration to move household units, not only isolated individuals

Likely touch points:

- `docs/demographics-labor-model.md`
- `docs/essential-needs-model.md`
- `docs/migration-model.md`
- `src/sim/person.rs`
- `src/sim/essentials.rs`
- `src/sim/world/cross_country_migration.rs`

### 5. Segmented Housing Market

Extend the existing housing system into a more realistic market.

Why this matters:

- the current housing pipeline is already strong, but still treats housing too uniformly
- real housing stress often shows up as crowding, cohabitation, delayed household formation, and landlord behavior before outright loss of shelter

Suggested scope:

- add housing segments or quality tiers
- add cohabitation / doubling-up as a state between housed and unhoused
- let household formation and breakup change housing demand
- model landlord / investor ownership behavior
- make migration re-housing gradual instead of defaulting to `Unhoused`

Likely touch points:

- `docs/housing-market.md`
- `docs/essential-needs-model.md`
- `src/sim/person.rs`
- `src/sim/essentials.rs`
- `src/sim/world/cross_country_migration.rs`
- `src/sim/country/monthly/monthly_housing.rs`

### 6. Business-to-Business Production Chains

Add intermediate goods, capital goods, and business investment.

Why this matters:

- the goods market is still dominated by household demand
- there are no true supply chains, input shortages, or capex cycles yet
- firms should consume inputs and invest to expand capacity rather than only reacting through staffing and cash

Suggested scope:

- define intermediate inputs for selected goods
- require firms to buy inputs before producing outputs
- add capital expenditure and capacity expansion decisions
- allow input bottlenecks to propagate through countries
- create business-cycle behavior beyond household spending

Likely touch points:

- `docs/money-simulation-requirements.md`
- `src/sim/world/goods_market/`
- `src/sim/company.rs`
- `src/sim/good.rs`
- `src/sim/country/monthly/`

### 7. Cross-Border Financial Contagion

Finish and deepen the international finance layer.

Why this matters:

- foreign household deposits exist already
- cross-border bank equity investing is implemented (monthly private-placement at last price, target-holding sell discipline)
- this is a natural path to capital flight, contagion, and sovereign-risk dynamics

Suggested scope:

- track valuation gains/losses on foreign exposures
- add sovereign-risk and bank-country-risk pricing
- add capital controls and deposit-insurance credibility effects
- add wholesale/interbank funding channels

Likely touch points:

- `src/sim/world/cross_country_banking.rs`
- `src/sim/world/cross_country_stock_investing.rs`
- `src/sim/world/cross_market_stock.rs`
- `src/sim/bank.rs`
- `src/sim/country/banking.rs`

## Recommended Order

Recommended implementation sequence:

1. per-country currencies and FX
2. country-level central banks
3. labor-force and hiring realism
4. family / household demography
5. segmented housing market
6. B2B production chains
7. cross-border financial contagion

## Why This Order

This sequence builds from foundational macro structure outward:

- FX and country central banks create real country divergence
- labor and family systems make household behavior more believable
- housing then becomes much richer because household structure exists
- B2B production chains deepen firm behavior after macro and labor foundations are stronger
- financial contagion becomes more meaningful once currencies, rates, and cross-border exposures are richer

## Secondary Supporting Work

These are useful, but should sit behind the realism features above:

- align README, chart docs, and export docs
- add stronger export regression tests
- add chart/event drill-through
- replace the world migration placeholder with a real aggregate or comparison view
- add scenario comparison across seeds and policy/disaster runs

Likely touch points:

- `README.md`
- `docs/running-charts-tab.md`
- `docs/csv-export.md`
- `src/components/simulation/render/charts.rs`
- `src/sim/world/export/monthly.rs`
- `src/sim/world/tests/core/long_run.rs`

## Notes

This roadmap is intentionally biased toward deeper simulation behavior rather than UI breadth. The current product surface is already strong enough that the biggest quality gain now comes from improving the underlying model.
