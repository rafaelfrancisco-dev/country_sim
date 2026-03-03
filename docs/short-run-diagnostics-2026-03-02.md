# Short-Run Simulation Diagnostics (2026-03-02)

## Scenarios run

Both runs used the existing headless export harness:

```bash
EXPORT_CSV_TEST_DIR=<DIR> \
STOCK_EXPORT_DAYS=90 \
STOCK_EXPORT_COUNTRIES=<2 or 3> \
STOCK_EXPORT_MILLIONS=0.1 \
cargo test export_stock_csv_for_one_year_headless -- --ignored --nocapture
```

- Scenario A: 3 countries, 0.1 million each, 90 days.
- Scenario B: 2 countries, 0.1 million each, 90 days.

## Raw outputs

- 3-country outputs: `target/sim_exports/`
- 2-country outputs: `target/sim_exports_2c/`

Each directory contains:

- `country_metrics_000000005eedc0de.csv`
- `people_agg_000000005eedc0de.csv`
- `company_agg_000000005eedc0de.csv`
- `events_000000005eedc0de.csv`
- `stock_daily_country_000000005eedc0de.csv`
- `stock_daily_equity_000000005eedc0de.csv`

## Comparison summary

### Unemployment and labor dynamics

3-country:
- Month 1 avg unemployment: 35.266%
- Month 2 avg unemployment: 35.316%
- Month 3 avg unemployment: 35.362%
- Employed count fixed at 194,326 all 3 months
- Job switches: 0, layoffs: 0, startups: 0, bankruptcies: 0
- Open positions: 0 for all month-country rows

2-country:
- Month 1 avg unemployment: 35.248%
- Month 2 avg unemployment: 35.295%
- Month 3 avg unemployment: 35.341%
- Employed count fixed at 129,588 all 3 months
- Job switches: 0, layoffs: 0, startups: 0, bankruptcies: 0
- Open positions: 0 for all month-country rows

Interpretation:
- Results are stable across 2c/3c; issue is model structure, not country-count noise.
- The headline unemployment metric includes non-job-seeking population and is likely overstating labor slack.

Quick consistency check (month 1, 3-country):
- Using baseline unemployment of 6%, implied labor-force unemployment is ~6.00%.
- Implied non-participants are ~31% of population, which explains headline ~35% unemployment when all non-employed people are counted.

### Stock-market behavior

3-country:
- Country-day rows: 270
- Avg trades/day/country: 94.55
- Avg volume/day/country: 220.56
- Zero-trade country-days: 55.6%
- Equity-day rows: 1,350
- Zero-volume equity-days: 67.0%
- Mean daily equity return: +0.0181%
- Daily return stdev: 0.1273%
- Avg turnover/free-float per equity-day: 0.0221%
- Equity-days with turnover >1% of float: 0.0%
- Bank position shares positive: 0.0% of equity-days

2-country:
- Country-day rows: 180
- Avg trades/day/country: 89.38
- Avg volume/day/country: 208.29
- Zero-trade country-days: 66.7%
- Equity-day rows: 900
- Zero-volume equity-days: 67.8%
- Mean daily equity return: +0.0178%
- Daily return stdev: 0.1221%
- Avg turnover/free-float per equity-day: 0.0208%
- Equity-days with turnover >1% of float: 0.0%
- Bank position shares positive: 0.0% of equity-days

Index drift (90-day):
- 3-country: +1.14% to +1.25%
- 2-country: +1.11% to +1.24%

Interpretation:
- Market is very illiquid and frequently inactive.
- Return distribution is narrow with persistent positive drift.
- Bank maker leg is effectively absent.

## Code-level observations tied to failures

1) Unemployment metric definition likely inflated
- Export counts all `EmploymentStatus::Unemployed` people, regardless of job-seeking eligibility:
  - `src/sim/world/export/monthly.rs` lines 43-47
  - `src/csv_export.rs` lines 174-177
- Job-seeking is age/education-gated:
  - `src/sim/person.rs` lines 37-41 and 213-215

2) Labor market freezes because openings never appear
- Companies start with no openings:
  - `src/sim/company.rs` line 76
- Growth openings require profitability streak and cash threshold:
  - `src/sim/company.rs` lines 256-263 and 265-271
- Monthly pipeline only creates openings through growth/startups/backfill:
  - `src/sim/country/monthly/monthly_growth_backfill_public_wages.rs` lines 4-13
- In these runs, all companies are unprofitable each month; growth/startup gates never trigger.

3) Distress path does not fire despite negative profits
- Distress is keyed to missed loan payments, not low/negative profit directly:
  - `src/sim/country/monthly/monthly_company_distress_bankruptcy.rs` lines 15-19
- With large initial cash buffers and no debt stress, layoffs/wage cuts are never reached.

4) Stock liquidity is mechanically constrained
- Participant cap is fixed at 1000/day:
  - `src/sim/params.rs` line 325
  - `src/sim/world/stock_market/matching.rs` lines 44 and 61
- Max order size is capped at 1% of float per order:
  - `src/sim/params.rs` line 338
  - `src/sim/world/stock_market/matching.rs` line 49
- Retail trades only when directional score crosses thresholds:
  - `src/sim/world/stock_market/matching.rs` lines 136-142

5) Bank market-maker is inactive by construction at start
- Bank bid capacity depends on bank reserve:
  - `src/sim/world/stock_market/matching.rs` lines 199-201
- Bank reserve starts at 0 and only grows from loan interest:
  - `src/sim/bank.rs` lines 56-63
- In these runs, loans are not active enough to build reserves, so bank positions remain zero.

6) Upward bias in directional signal is plausible
- Directional signal includes value-gap using valuation anchor with large cash contribution:
  - `src/sim/world/stock_market/matching.rs` lines 113-122 and 138-142
  - `src/sim/stock_market.rs` lines 444-460
- Given high company cash and low sales/profit, `anchor - last_price` may systematically skew positive and favor buys.

## Suggested fix-planning tracks

1. Metrics correctness
- Add labor-force metrics to exports (working-age eligible employed/unemployed + participation rate).
- Keep current headline for backward compatibility, but rename/clarify it.

2. Labor-market activation
- Introduce baseline replacement openings each month (or turnover openings) independent of profitability.
- Consider linking distress to sustained negative profit, not only missed debt service.

3. Stock-market realism
- Scale participant cap with population and/or shareholder base.
- Seed non-zero bank reserve or decouple maker liquidity from reserve-only funding.
- Recalibrate directional thresholds/noise and valuation anchor so early drift is not structurally positive.
- Add validation targets: zero-volume share, turnover/free-float range, return volatility bands.
