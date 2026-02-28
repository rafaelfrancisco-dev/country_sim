# Money Simulation Requirements

## Goal
Model monthly money circulation while supporting daily demographic and labor dynamics.

## Time Model
- One simulation tick = one in-game day.
- Daily updates include aging, deaths, births, and job seeking.
- Monthly money processing runs every 30 days.

## Currency Model
- Monetary values use `f64` and are rounded to cents after transaction batches.

## Initial Money Supply
- Fixed at generation: `population * money_per_citizen`.
- Initial split:
  - 50% to people
  - 50% to companies
  - 0% to treasury

## Employment and Jobs
- Jobs remain fixed to the existing 10 job types.
- Every job has a required habilitation:
  - Basic School: Retail Salesperson, Cashier, Driver, Construction Worker, Manufacturing Operator, Food Service Worker
  - High School: Office Clerk
  - Higher Learning: Registered Nurse, Teacher, Software Developer
- People can be unemployed.
- Job-seeking eligibility by education track:
  - BasicOnly / HighSchool tracks: age 18+
  - Bachelor track: age 21+
  - MedicalDoctor track: age 23+
- Hiring is qualification-gated and interview-free.
- Job contracts are salary-aware:
  - each opening carries an `offered_monthly_salary`
  - employed people keep an `agreed_monthly_salary` on contract
  - payroll and loan affordability use contract salary (not catalog base)

## Demographics
- Birth model:
  - 17 births per 1000 population per year
  - applied daily with carry to avoid rounding drift
- Death model:
  - age-banded annual mortality probabilities converted to daily probability
- Initial age distribution:
  - 0-14: 25%
  - 15-24: 16%
  - 25-54: 41%
  - 55-64: 10%
  - 65-74: 5%
  - 75+: 3%

## Company Hiring Growth
- Companies can create vacancies only when both are true:
  - profitable streak >= 3 months
  - cash reserve >= `max(100_000, monthly_payroll_budget * 1.5)`
- Opening count per month:
  - `min(8, 1 + floor(last_month_profit / 50_000))`
- Openings are distributed among existing offered job types with weight by assignment weight.

## Company Formation, Distress, and Bankruptcy
- **Startup formation (monthly)**:
  - founders are currently employed workers in the same profession
  - minimum founders per startup: `5`
  - each founder keeps a personal reserve of `3 months` of current salary
  - startup must fund `6 months` of planned opening payroll
  - opening plan: `10` total (`7` in founder profession + `3` mixed weighted openings)
  - startup good must be high-margin in-country:
    - margin per good = `total_profit / total_sales`, where `total_sales > 0`
    - required margin `>= 0.25`
  - startup price starts at `5%` below country average for that good, floored by margin floor
- **Distress escalation after missed company loan payments**:
  - miss #1: wage cut (`10%`) on employee contracts and openings, with floor `0.70 × catalog salary`
  - miss #2: layoffs (`20%`, ceil, min 1 when workforce > 0), highest salary first, tie by lower person ID
  - miss #3+: bankruptcy only when both are true:
    - `cash_balance <= 0`
    - no additional loan capacity under bank affordability math
  - on non-miss months, consecutive miss counter resets to `0`
- **Bankruptcy handling**:
  - all workers are laid off, openings cleared, loan principal written off, company cash forced to zero
  - bankrupt companies are physically removed from the vector
  - surviving company IDs are renumbered to contiguous indices
  - person employment references are remapped to new IDs; references to removed companies become unemployed

## Country Revenue
- Monthly payroll is paid only to employed people.
- Income tax is withheld monthly and transferred to treasury.
- Treasury inflows come from tax and deceased-person cash transfer.

## Government Spending
- After taxes are collected each month, the treasury distributes benefits to eligible people.
- **Unemployment benefit**: each unemployed working-age adult (18–65) receives `unemployment_benefit_rate × median_income_reference` per month, funded from treasury.
- **Basic needs safety net**: if a person has any unmet essential need (goods 0–4: water, food, housing, electricity, medicine) AND cash below `basic_needs_cash_threshold`, the treasury tops up their cash to that threshold.
- Both transfers are capped by the available treasury balance (no negative treasury).
- Tracked fields per country: `monthly_gov_spending`, `monthly_budget_balance` (tax collected − gov spending), `gov_debt` (cumulative shortfalls).
- A `GovTransfer` event is emitted each month when any benefit is paid.

## Government Essential Backfill
- After private startup formation each month, a country checks essential goods (`0..=4`) for local producer coverage.
- For every missing essential good, the government creates one public-utility company for that good.
- New public companies:
  - start with `is_public_utility=true` and `is_essential_backfill=true`
  - are seeded with openings (default `3`, with manufacturing-heavy split)
  - use high-attraction wage offers:
    - `offer = min(cap, max(catalog*base_multiplier, market_best*(1 + switch_raise_threshold + incentive_buffer)))`
    - default cap multiplier: `1.8×` catalog salary
  - apply vacancy wage escalation monthly before switch matching:
    - `opening_salary *= (1 + vacancy_escalation_rate)` with cap clamp
- Funding for each public company:
  - required capital = `monthly_opening_payroll × runway_months`
  - treasury pays first
  - any shortfall is deficit-financed via bank-created money
  - deficit amount is added to `gov_debt`
  - total funded amount is added to `monthly_gov_spending` and reflected in `monthly_budget_balance`
- A `GovCompanyFunded` event is emitted for each public company created.
- Configurable via `FiscalParams` in `SimulationParams`:
  - `unemployment_benefit_rate` (default: 0.5)
  - `median_income_reference` (default: $2,000 → $1,000/month benefit)
  - `basic_needs_cash_threshold` (default: $500)

## Consumption and Sales
- Monthly household spending remains 5% to 15% of available cash.
- Seller company is chosen within country companies.

## Bank and Loans
- Borrowers: people and companies.
- Loan issuance creates money.
- Principal repayment destroys created money.
- Interest flows to bank reserve.
- Affordability cap: monthly payment <= 35% of monthly income.
- Unemployed people have zero monthly income for affordability checks.
- On death, personal loans are written off and created principal outstanding is removed from bank-created money.

## Monthly Processing Order
1. Recompute company headcounts.
2. Company payroll-liquidity check and pre-payroll loan request.
3. Payroll from companies to employed people + income tax transfer to treasury.
4. Government spending (unemployment benefits + basic needs top-up).
5. Loan repayment pass (people, then companies).
6. Company distress escalation.
7. End-of-month loan requests.
8. Growth-driven opening creation.
9. Startup formation pass.
10. Monthly job-switch pass (same profession, minimum 10% raise).
11. Recompute company headcounts again.
12. Company month close (pricing history/profitability history side effects).
13. Month increment and aggregate recomputation.

## Invariants
- Treasury inflows are from taxation and deceased-person cash transfer.
- Treasury outflows are government benefit transfers to households.
- Government transfers are balance-neutral: treasury decreases by the same amount household cash increases, leaving total money supply unchanged.
- Net new money enters via loan issuance only.
- Repaying or writing off created principal decreases created-money outstanding.
- Tracked money supply remains close to:
  - `initial_supply + created_money_outstanding`

## Stock Market Microstructure

### Daily Market Mechanics (Limit-Order Engine)
The stock market runs once per day after the goods market. Each country's market uses a deterministic limit-order matching engine:

1. **Retail order generation**: A sample of up to `daily_participant_cap` (default 200) traders is selected deterministically via hashed slot indices.
2. **Signal computation**: Each sampled person × listed stock produces two signals:
   - *Trend*: `(last_price − MA20) / MA20` — positive means above recent average
   - *Value gap*: `(fundamental_anchor − last_price) / last_price` — positive means undervalued
3. **Order placement rules**:
   - Sell intent: has position AND (`value_gap < −2%` OR `trend > 5%` OR noise trade)
   - Buy intent: `value_gap > 2%` OR `trend < −2%` OR no position AND below trend OR noise trade
   - One order type per stock per slot; order size bounded by `max_order_fraction_of_float`
4. **Bank maker orders**: For each listed symbol the bank quotes:
   - Bid at `last_price × (1 − maker_spread_bps / 20000)` — limited by equity headroom
   - Ask at `last_price × (1 + maker_spread_bps / 20000)` — limited by bank inventory
5. **Price-time priority matching**: Buys sorted descending by limit, sells ascending. Fill at midpoint clamped to `[prev_close × (1 − max_daily_price_move_pct), prev_close × (1 + max_daily_price_move_pct)]`.
6. **Settlement (strict, zero-synthetic)**:
   - Person buy: `cash_balance -= cost`, shares increase
   - Person sell: shares decrease, `cash_balance += proceeds`
   - Bank buy: `reserve_balance -= cost`, bank shares increase
   - Bank sell: bank shares decrease, `reserve_balance += proceeds`
7. **One `record_day_close` per country** is called after all symbols settle.

### Monthly Stock Events
- **Margin calls** (step 5 of month-end): if a borrower cannot cover their loan payment, pledged shares are force-liquidated at `last_price × (1 − margin_liquidation_discount)`. Bank pays (reserve decreases) and receives shares. Person receives proceeds in cash.
- **Dividends** (step 17): profitable companies with sufficient runway pay `dividend_payout_ratio` of profit. Household recipients pay 15% withholding; bank receives gross dividend to reserve.

### Share Conservation Invariant
For every listed company: `Σ person_holdings + bank_holdings == free_float_shares`. This is checked by `Country::validate_money_invariants` every tick via `ApplyOutcome::warnings`.

### Key Parameters (`StockMarketParams`)
| Field | Default | Description |
|---|---|---|
| `daily_participant_cap` | 200 | Max traders sampled per day |
| `maker_spread_bps` | 50 | Bank bid-ask spread (basis points) |
| `max_daily_price_move_pct` | 0.08 | Hard ±8% daily price cap |
| `retail_noise_trade_prob` | 0.01 | Noise trade probability per slot |
| `max_order_fraction_of_float` | 0.01 | Max order size as fraction of float |
| `investor_cash_reserve_ratio` | 0.30 | Cash kept liquid before buying |
| `bank_max_equity_reserve_fraction` | 0.10 | Bank equity cap (fraction of reserve) |
