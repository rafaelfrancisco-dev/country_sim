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

## Annual Market Wage Raises
- A company-wide raise pass runs on a configurable yearly cadence (default every `12` months).
- Raise eligibility requires all of:
  - company payroll reference is positive (`monthly_payroll_budget > 0`)
  - enough profit history is available for the configured window (default 12 months)
  - trailing average profit is positive over that window
  - cash runway is sufficient: `cash_balance >= monthly_payroll_budget × min_cash_runway_months` (default `2.0`)
- Raise pressure is market-following (no fixed base raise):
  - for each job, compute the company's internal pay reference:
    - average contract salary of employed workers in that job
    - fallback: average opening offer for that job when no incumbents exist
  - external benchmark for that job is the highest salary for the same job in other companies (using both contracts and openings)
  - job-level pressure ratio: `external / internal - 1`
  - company raise rate is the maximum positive job-level pressure ratio
  - optional cap can clamp this raise rate (`max_raise_rate`; default uncapped)
- Applied effects:
  - employee contract salaries are multiplied by `(1 + raise_rate)`
  - open-position offered salaries are multiplied by `(1 + raise_rate)`
  - `wage_offer_multiplier` is multiplied by `(1 + raise_rate)`

## Company Formation, Distress, and Bankruptcy
- **Startup formation (monthly)**:
  - founder pool includes employed workers and qualified unemployed adults.
  - founder team size is bounded (`min_founders..max_founders`, default `3..6`).
  - founders keep a personal reserve (default `0.5` month of salary reference); low-cash unemployed founders can still participate when startup lending is enabled.
  - opening plan is dynamic (`startup_openings_min..startup_openings_max`, default `6..24`) and expands under labor slack.
  - viability gate uses expected demand/revenue and payroll affordability:
    - expected revenue blends observed good-level sales with a demand proxy
    - monthly opening payroll must stay under `startup_max_payroll_to_revenue` (default `1.0`)
  - funding stack:
    - founder capital first
    - optional startup loan up to `startup_bank_loan_max_fraction` (default `95%`)
  - each startup creates explicit `FounderStake` equity records (`person_id`, `shares_owned`, `cash_contributed`).
- **Distress escalation**:
  - missed-payment path:
    - first miss: wage cut (`10%`) with floor `0.70 ×` catalog salary
    - further misses: layoffs are capped by `max_monthly_layoff_rate` (default `1%`)
  - loss-only path (no missed payment): applies mild wage/layoff pressure after sustained losses.
  - countercyclical brake: when labor-force unemployment is elevated, layoffs and bankruptcy escalation are throttled.
  - bankruptcy is last resort and requires sustained distress (defaults: `>=36` missed-payment months and `>=18` loss months), non-positive cash, and no additional loan capacity.
  - before bankruptcy, private firms attempt IPO rescue.
- **Bankruptcy handling**:
  - all workers are laid off, openings cleared, loans written off, and company cash reset.
  - bankrupt companies are removed and IDs are remapped; person employment references are remapped accordingly.

## IPO Mechanics
- IPO attempts are unified under `attempt_ipo(company_idx, reason)` with reasons:
  - `Seed`
  - `Growth`
  - `Distress`
- Growth IPOs run monthly before distress escalation; distress IPOs run as rescue before bankruptcy.
- Settlement is split into:
  - **Primary issuance**: cash raised goes to `company.cash_balance`.
  - **Secondary founder sales**: cash is transferred to founders pro-rata by sold founder shares.
- IPO subscription funding uses existing household cash and bank reserves only.
- IPO-time secondary payouts are pure transfers; no new money is created by the secondary leg.
- **Founder share registration**: after IPO settlement, each founder's remaining (unsold) stake is registered in `person_portfolios` at the IPO price, and `free_float_shares` is expanded by that count. Founder shares then participate in daily trading like any other retail position.

## Country Revenue
- Monthly payroll is paid only to employed people.
- Income tax is withheld monthly and transferred to treasury.
- Treasury inflows come from tax and deceased-person cash transfer.

## Government Spending
- After taxes are collected each month, the treasury distributes benefits to eligible people.
- **Unemployment benefit**: each unemployed working-age adult (18–65) receives `unemployment_benefit_rate × median_income_reference` per month, funded from treasury.
- **Basic needs safety net**: if a person has any unmet essential need (goods 0–4: water, food, housing, electricity, medicine) AND cash below `basic_needs_cash_threshold`, the treasury tops up their cash to that threshold.
- Eligibility is cumulative: a person can receive unemployment benefit and then still receive a basic-needs top-up in the same month.
- Both transfers are capped by the available treasury balance (no negative treasury).
- Tracked fields per country: `monthly_gov_spending`, `monthly_budget_balance` (tax collected − gov spending), `gov_debt` (cumulative shortfalls).
- A `GovTransfer` event is emitted each month when any benefit is paid.

## Government Essential Backfill
- After private startup formation each month, a country checks essential goods (`0..=4`) for local producer coverage.
- For every missing essential good, the government creates one public-utility company for that good.
- When labor-force unemployment rises above the countercyclical target band, the model can also spawn additional public-utility employers (cyclical backfill) to stabilize employment.
- New public companies:
  - start with `is_public_utility=true` and `is_essential_backfill=true`
  - initial unit price is clamped to `public_utility_price_bounds_for_good(good_id)`:
    - `min_price = base_unit_cost × (1 + min_margin_rate)`
    - `dynamic_base_index = sqrt(max(1, cpi_index) × max(1, expected_money_supply / initial_money_supply))`
    - `dynamic_base_cost = base_unit_cost × dynamic_base_index`
    - `max_price = dynamic_base_cost × max(1, public_utility_price_cap_multiplier)`
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
- Public-utility price cap multiplier is configurable via `GovernmentBackfillParams.public_utility_price_cap_multiplier` (default: `1.5`, runtime floor `1.0`).

## Public Utility Price Dynamics (State-Owned Only)
- Price bounds apply only to `is_public_utility=true` companies; private companies keep existing unconstrained repricing behavior.
- During monthly close (`close_company_months`), each public utility is repriced and then adjusted toward a mid-band anchor:
  - `anchor_price = (min_price + max_price) / 2`
  - if `unit_price > anchor_price`, a downward correction is applied
  - if `unit_price < anchor_price`, only a mild upward drift is applied
- Downward correction strength increases with affordability pressure, using per-capita debt and unmet-need signals:
  - `affordability_pressure = clamp(((debt_per_capita / 80) + (unmet_events_per_capita / 14)) / 2, 0, 1)`
  - correction factor: `0.10 + affordability_pressure × 0.25`
- Final monthly public-utility price is clamped to `[min_price, max_price]`.

## Consumption and Sales
- Monthly household discretionary spending starts from a 5% to 15% base propensity of available cash, then is scaled by the monetary transmission multiplier:
  - low-rate regime => higher discretionary budget (households less conservative)
  - high-rate regime => lower discretionary budget (households more conservative)
- Seller company is chosen within country companies.

## Monetary Policy

### Central Bank Policy Rate (Taylor Rule)
The world holds a single `policy_rate` updated each month after all country month-ends complete.

**Taylor Rule formula:**
```
r = r* + α × (π − π*) + β × (u* − u)
```
Where:
- `r*` = `neutral_policy_rate` (default 4%)
- `α` = `inflation_coefficient` (default 1.5)
- `π` = current aggregate annualized inflation rate
- `π*` = `target_inflation_rate` (default 2%)
- `β` = `unemployment_coefficient` (default 0.5)
- `u*` = `natural_unemployment_rate` (default 5%)
- `u` = current aggregate unemployment rate

The Taylor Rule result is the *target* rate. The actual rate moves at most one `policy_rate_step` (default 0.0025 = 25 bps) per month toward the target, then snapped to the nearest step grid point and clamped to `[rate_floor, rate_ceiling]` (default 0%–20%). This mirrors real central bank practice of moving in discrete quarter-point increments.

The new rate is propagated to every `Country.policy_rate` before the monthly snapshot is captured.

A `RatePolicyChange` event is emitted if the rate moves by at least `rate_change_event_threshold` (default 0.5%).

### CPI Basket
Each month, after all production prices are set, `recompute_cpi()` calculates a weighted price index:
- Basket: goods 0–7 (all essential and near-essential goods)
- Weight: `base_unit_cost` of each good
- Formula: `relative_price = avg_company_unit_price / base_unit_cost`
- `cpi_index` = weighted average of relative prices across basket goods
- Initial country CPI baseline is seeded from the current basket (`cpi_basket_index`) at world creation to avoid artificial month-1 inflation spikes.
- Raw annualized month-over-month inflation is:
  - `raw = (new_cpi / old_cpi)^12 − 1`
- Reported inflation uses EMA smoothing:
  - `inflation_t = inflation_{t-1} + (raw - inflation_{t-1}) × inflation_ema_alpha`
  - default `inflation_ema_alpha = 0.35`

### Deposit Rate & Savings Interest
Each month, after loan repayments and bank-run processing, depositors earn interest on their cash balances. Interest is paid **per bank** based on each holder's `deposit_bank_id`.

```
deposit_rate = max(0, policy_rate − deposit_spread)
monthly_interest = cash_balance × (deposit_rate / 12)
```

- `deposit_spread` (default 1%) models private-bank intermediation margin between policy and deposit rates.
- Public-bank deposit rates apply a small positive premium vs private-bank base rates.
- Only balances above `savings_min_balance` (default $200) earn interest.
- Gross interest is computed per bank across people, companies, and treasury balances assigned to that bank.
- Each bank's total payout is capped by that bank's available reserve (`reserve_balance`) with proportional scaling.
- Interest is added to each holder's `cash_balance`; `last_month_savings_interest` records the per-holder amount for display.
- The country tracks `monthly_savings_interest_paid` (reset each month).

When `deposit_rate ≤ 0` (policy rate ≤ spread), no payments are made.

### Variable Loan Rate
The bank loan rate adjusts with monetary policy:
```
bank_loan_rate = clamp(policy_rate + credit_spread, rate_floor, rate_ceiling)
```
Default `credit_spread` = 3%, so the starting effective rate is 7% (4% neutral + 3% spread).

### Automatic Money Minting (Default-On)
Each country runs an automatic minting pass at month-end after CPI/deposit-liability recomputation:

```
inflation_shortfall = max(target_inflation_rate - monthly_inflation_rate, 0)
unemployment_gap = max(unemployment_rate - natural_unemployment_rate, 0)
mint_fraction_raw =
    inflation_shortfall * mint_inflation_gap_weight +
    unemployment_gap * mint_unemployment_gap_weight
mint_fraction = clamp(mint_fraction_raw, 0, mint_max_fraction_expected_supply)
mint_amount = expected_money_supply * mint_fraction
```

- Minted money is injected into the country's **public bank reserve** (`Country.bank.reserve_balance`).
- `monthly_money_minted` tracks month-to-date minting and resets each month.
- `cumulative_money_minted` tracks lifetime minting and feeds expected-supply accounting.
- Event emitted: `MoneyMinted` (`[MINT]`) with amount + macro gaps.

### Monetary Transmission to the Real Economy
Policy now propagates into economy behavior through smooth, bounded multipliers centered on the neutral loan rate:

```
neutral_loan_rate = neutral_policy_rate + credit_spread
normalized_gap = clamp((bank_loan_rate - neutral_loan_rate) / rate_response_band, -1, 1)
multiplier = clamp(1 - normalized_gap * strength, min, max)
```

Channels:
- **Credit channel**:
  - Effective debt-to-income cap for private loans is scaled by `loan_appetite_multiplier`.
  - Lower rates increase approved principal capacity; higher rates tighten it.
  - This applies to payroll-liquidity loans, end-of-month household/company refinancing, startup loans, and distress loan-capacity checks.
- **Demand channel**:
  - Discretionary household budgets are scaled by `discretionary_spend_multiplier`.
  - Higher rates reduce non-essential spending propensity; lower rates increase it.
- **Business expansion channel**:
  - Hiring-growth and startup aggressiveness are scaled by `expansion_multiplier`.
  - Higher rates raise effective expansion hurdles; lower rates relax them.
- **Price-setting channel**:
  - Monthly repricing pressure is scaled by `price_pressure_multiplier`.
  - Higher rates dampen raise rates and increase effective cut rates.
  - Sell-through thresholds shift by a bounded `sell_through_threshold_shift`:
    - higher rates make price hikes harder to trigger and price cuts easier
    - lower rates do the inverse

The tick order is unchanged: policy updates after month-end country processing, so behavior changes apply with a one-month lag.

### Calibration Notes (2026-03-04)
- Monetary transmission defaults were tightened for stronger disinflation under high rates:
  - stronger loan/demand/expansion sensitivity
  - stronger pricing-pressure sensitivity
  - smaller sell-through threshold shift range to reduce jitter
- Combined with CPI baseline seeding and inflation EMA smoothing, this yields smoother inflation cycles and removes the startup CPI spike artifact.

## Sovereign Bonds

### Issuance
Bonds are issued at month-end via `maybe_issue_bonds()` when both conditions hold:
1. `treasury_balance ≤ bond_issuance_treasury_threshold` (default 0; i.e. treasury is at or below zero)
2. `monthly_budget_balance < 0` (budget was in deficit this month)
3. No bond currently outstanding (one bond at a time)

**Face value**: `|monthly_budget_balance| + max(threshold − treasury_balance, 0)` — covers the deficit and restores treasury to the threshold.

**Coupon rate**: `clamp(policy_rate + sovereign_spread, rate_floor, rate_ceiling)`, default spread = 1%.

**Funding split**:
- Bank primary dealer: `bank_primary_dealer_fraction` (default 50%) — bank reserve decreases, treasury increases.
- Retail: top `bond_retail_top_n` (default 200) wealthiest people share the remainder; each person's cash is debited proportionally.

### Coupon & Maturity
Each month `process_bond_coupon_and_maturity()` runs before issuance:
1. Monthly coupon = `bonds_outstanding × coupon_rate / 12`
2. Bank receives its dealer fraction directly to reserve; top-N retail holders share the remainder from treasury.
3. Treasury is debited the full coupon; `monthly_coupon_paid` is updated.
4. `bond_months_to_maturity` decrements by 1 each coupon month.
5. When it reaches 0: treasury repays full face value to bank (reserve) + retail (people cash).
   - If treasury can't cover the shortfall: `fund_government_deficit(shortfall)` monetizes it → `gov_debt` increases → `BondDefault` event emitted.
   - Otherwise: `BondMatured` event emitted.
6. Bond state resets: `bonds_outstanding = 0`, `bond_coupon_rate = 0`, `bond_months_to_maturity = 0`.

### Bond Events
| Event | Tag | Description |
|---|---|---|
| `RatePolicyChange` | `[RATE]` | Policy rate changed by Taylor Rule |
| `BondIssued` | `[BOND+]` | New bond issued; shows face value and coupon rate |
| `CouponPaid` | `[COUP]` | Monthly coupon distributed |
| `BondMatured` | `[BOND=]` | Bond repaid at maturity |
| `BondDefault` | `[BDFLT]` | Treasury couldn't repay; shortfall monetized |
| `MoneyMinted` | `[MINT]` | Automatic month-end mint into public-bank reserve |

### Configuration (`MonetaryPolicyParams`)
| Field | Default | Description |
|---|---|---|
| `target_inflation_rate` | 0.02 | 2% annual CPI target |
| `neutral_policy_rate` | 0.04 | Equilibrium policy rate |
| `inflation_coefficient` | 1.5 | Taylor α |
| `unemployment_coefficient` | 0.5 | Taylor β |
| `natural_unemployment_rate` | 0.05 | NAIRU |
| `rate_floor` | 0.0 | Zero lower bound |
| `rate_ceiling` | 0.20 | 20% hard cap |
| `credit_spread` | 0.03 | Bank loan rate premium over policy rate |
| `sovereign_spread` | 0.01 | Bond coupon premium over policy rate |
| `bond_issuance_treasury_threshold` | 0.0 | Issue bonds when treasury ≤ this |
| `bank_primary_dealer_fraction` | 0.50 | Bank's share of each issuance |
| `bond_maturity_months` | 12 | 1-year term (12 monthly coupons) |
| `bond_retail_top_n` | 200 | Number of wealthiest people in retail tranche |
| `rate_change_event_threshold` | 0.005 | Min 0.5% move to emit `RatePolicyChange` |
| `policy_rate_step` | 0.0025 | Discrete step size for rate moves (25 bps per month) |
| `deposit_spread` | 0.01 | Deposit rate = `max(0, policy_rate − deposit_spread)` |
| `savings_min_balance` | 200.0 | Minimum cash balance required to earn deposit interest |
| `mint_inflation_gap_weight` | 0.20 | Weight on inflation shortfall in minting rule |
| `mint_unemployment_gap_weight` | 0.10 | Weight on unemployment gap in minting rule |
| `mint_max_fraction_expected_supply` | 0.01 | Upper cap on monthly minting as fraction of expected supply |

## Multi-Bank System
- Each country has:
  - exactly **one permanent public bank** (`Country.bank`, always ID 0, always `BankKind::Public`)
  - zero or more additional private banks (`Country.banks`)
- Banks are typed as `Public` or `Private`.
- People, companies, and treasury hold bank assignment pointers (`deposit_bank_id` / `treasury_bank_id`).
- Deposit liabilities are recomputed each month from assigned holder balances.

### Public Bank Invariant
- `Country.bank` (ID 0) is the **only** public bank and is created once at world generation.
- Public-bank health uses a sovereign-backed score:
  - liquidity + capital + treasury-support weighting
  - non-resolution floor via `public_health_floor`
  - additional compatibility uplift from `public_bank_stability_bonus`
- When the public bank's health deteriorates (`bank_health_score(public_bank_id) < public_health_floor + 0.05`)
  **and** `deposit_liabilities ≥ 10%` of recapitalize amount, it is **recapitalized in place** rather than replaced.
- Recapitalization injects `public_bank_recapitalize_amount` into the bank's reserves and equity capital.
- Funding order: treasury first, then deficit monetization (added to `gov_debt`).
- A minimum-deposit guard prevents spurious recapitalization of a freshly initialized bank with no real deposit obligations.

### Bank Formation (Monthly)
- **Private bank formation**:
  - market-constrained by population capacity (`target_people_per_private_bank`)
  - uncapped by a fixed private-bank-count parameter (no `max_private_banks`)
  - founders must collectively contribute at least `min_private_bank_seed_capital`
  - minimum founder count controlled by `min_private_bank_founders`
  - founders must retain minimum post-contribution cash (`min_founder_cash_after_contribution`)
  - seed adequacy must satisfy `seed_capital / projected_migrated_deposits >= min_private_seed_to_deposits_ratio`
  - formation throughput is capped by `max_new_banks_per_month` attempts per cycle
- **Public bank**: never created post-initialization; only recapitalized (see above).
- Events: `BankFounded` on private-bank creation.

### Bank Runs and Resolution (Monthly)
- Run processing currently applies to **private banks only**.
- A run requires:
  - private-bank health below `run_health_threshold`
  - material shortfall ratio above `run_min_uninsured_share_trigger`
- Withdrawal pressure scales by health gap and:
  - `run_withdraw_fraction`
  - `private_bank_failure_sensitivity`
- Depositor switching is bounded by `run_switch_fraction`, and switching transfers reserve assets with deposit relationships.
- Resolution path:
  - insured shortfall support up to `deposit_insurance_cap`
  - uninsured shortfall takes `uninsured_haircut_rate` loss
  - remaining uninsured gap is treasury-supported (with monetization fallback)
  - resolution is entered only if there is payout shortfall **and** post-withdraw liquidity is below `resolution_liquidity_floor`
  - if the bank later stabilizes and run intensity decays under threshold, it exits resolution (`BankResolved`)
  - if a private bank remains in resolution for `terminal_resolution_months`, it is terminally closed (`BankClosed`)
  - closure migrates deposits and loans to a surviving acquirer bank, transfers residual reserves, and removes the failed private bank from `Country.banks`
- Bank-run visibility is event-driven:
  - `BankRunStarted`, `BankRunWithdrawal`, `BankResolutionStarted`, `BankResolved`, `BankClosed`, `BankDepositHaircut`

### Cross-Country Deposits
- When a foreign bank offers a deposit rate meaningfully higher than the home country's best rate, eligible people move a portion of their savings abroad into the best eligible foreign deposit bank.
- **Eligibility**: `cash_balance >= cross_country_deposit_min_balance` and no existing foreign deposit.
- **Transfer**: fraction `cross_country_deposit_fraction` of the surplus above the minimum balance.
- **Rate threshold**: the best eligible foreign-bank rate must exceed the home-country benchmark by at least `cross_country_deposit_rate_premium` (default 50 bps).
- **Interest**: paid monthly from the foreign bank's reserves to the person's local cash (not reinvested; balance stays flat).
- **Recall on distress**: if the person's local cash falls below `cross_country_deposit_distress_threshold`, the full foreign balance is returned to local cash.
- **Accounting**: credited as `foreign_deposit_liabilities` on the receiving bank; interest and recall debit the same stored bank's reserves and liabilities.
- **Data model**: `ForeignDeposit { country_id, bank_id, balance }` on `Person`; `bank_id` identifies the exact foreign bank that holds the deposit for its full lifecycle.
- Events: `ForeignDepositOpened`, `ForeignDepositClosed`.
- Processing runs world-level monthly (after all country monthly steps, before snapshot capture).

### Banking Parameters (`BankingSystemParams`)
| Field | Default | Description |
|---|---|---|
| `deposit_insurance_cap` | 100000.0 | Per-claim insured support during bank resolution |
| `uninsured_haircut_rate` | 0.35 | Loss ratio applied to uninsured shortfall |
| `run_health_threshold` | 0.25 | Health trigger threshold for private-bank runs |
| `run_withdraw_fraction` | 0.18 | Base fraction of deposits requested during run |
| `run_switch_fraction` | 0.30 | Share of depositors that can switch banks during run |
| `public_bank_stability_bonus` | 0.20 | Compatibility uplift applied in country-level public-bank health scoring |
| `private_bank_failure_sensitivity` | 0.45 | Multiplier on private-bank withdrawal stress |
| `run_min_uninsured_share_trigger` | 0.20 | Minimum shortfall ratio needed before run stress executes |
| `resolution_liquidity_floor` | 0.05 | Resolution requires post-withdraw liquidity below this floor |
| `run_intensity_decay_monthly` | 0.75 | Monthly decay factor used in recovery logic |
| `terminal_resolution_months` | 6 | Consecutive months in resolution before terminal private-bank closure |
| `min_private_bank_founders` | 8 | Minimum founders for private-bank creation |
| `min_founder_cash_after_contribution` | 25000.0 | Required founder cash cushion after seed contribution |
| `min_private_bank_seed_capital` | 500000.0 | Minimum private-bank seed capital |
| `min_private_seed_to_deposits_ratio` | 0.05 | Minimum seed/deposit adequacy for newly formed private banks |
| `target_people_per_private_bank` | 250000 | Population-based private-bank market capacity target |
| `max_new_banks_per_month` | 2 | Max new private banks created per country per month |
| `public_bank_recapitalize_amount` | 500000.0 | Capital injected per recapitalization of the public bank |
| `public_health_floor` | 0.35 | Minimum public-bank health score when not in resolution |
| `public_health_liquidity_weight` | 0.55 | Public-bank health weight on liquidity |
| `public_health_capital_weight` | 0.15 | Public-bank health weight on capital |
| `public_health_sovereign_support_weight` | 0.30 | Public-bank health weight on treasury support |
| `cross_country_deposit_min_balance` | 10000.0 | Min local cash to open a foreign deposit |
| `cross_country_deposit_rate_premium` | 0.005 | Min rate advantage (foreign − local) to trigger migration |
| `cross_country_deposit_fraction` | 0.30 | Fraction of surplus above minimum moved to foreign bank |
| `cross_country_deposit_distress_threshold` | 500.0 | Local cash floor below which foreign deposit is recalled |

## Bank and Loans
- Borrowers: people and companies.
- Loans carry explicit lender identity (`lender_bank_id`).
- Loan issuance creates money.
- Principal repayment destroys created money.
- Interest flows to the lender bank reserve.
- Affordability cap: monthly payment <= 35% of monthly income.
- Unemployed people have zero monthly income for affordability checks.
- On death/company default, loan write-off removes created principal from the lender bank's outstanding created money.
- **Loan routing**: all new loan origination uses `preferred_lender_bank_id()`, which prefers healthy private banks over the public bank. Collection is routed to whichever bank holds the loan (`lender_bank_id`). This means private banks accumulate `created_money_outstanding` as they originate loans.
- Loan interest rate uses policy transmission with bank-kind adjustment via `bank_loan_rate_for(bank_id)`:
  - private: base rate − 50 bps (clamped to `rate_floor`) — discount attracts borrowers to private banks
  - public: base `bank_loan_rate()` with no adjustment

## Monthly Processing Order
1. Recompute company headcounts.
2. Company payroll-liquidity check and pre-payroll loan request (routed to `preferred_lender_bank_id()`).
3. Payroll from companies to employed people + income tax transfer to treasury.
4. Government spending (unemployment benefits + basic needs top-up).
5. **Bond coupon & maturity** — distribute coupons; repay or default if `bond_months_to_maturity == 0`.
6. Stock collateral seizure (margin calls).
7. Loan repayment pass (people, then companies).
7a. **Private-bank run processing** — withdrawal pressure, depositor switching, resolution/haircuts.
7b. **Savings interest** — deposit interest distributed per bank from each bank's reserves.
8. Company profitability recomputation.
9. Annual market wage raise pass (cadence-gated).
10. Growth IPO pass (non-distress listings).
11. Secondary offering pass for listed distressed companies.
12. Company distress escalation (including IPO rescue path before bankruptcy).
13. End-of-month loan requests (routed to `preferred_lender_bank_id()`).
14. Growth-driven opening creation.
15. Startup formation pass.
16. Essential + countercyclical public-employer backfill.
17. **Bank formation** — private founder-capital creation and/or stress-gated public expansion.
18. **Bond issuance** — issue new bond if treasury ≤ threshold and budget was in deficit.
19. Public-utility wage adjustment.
20. Monthly job-switch pass (same profession, minimum raise threshold).
21. Monthly labor churn pass.
22. Recompute company headcounts again.
23. Company month close (pricing/history side effects, plus state-owned utility cap + affordability-aware reversion).
24. Stock dividends.
25. **CPI recompute** — update `cpi_index` and `monthly_inflation_rate` from current company prices.
26. Recompute bank deposit liabilities.
27. **Automatic money minting** — rule-based reserve injection to the public bank.
28. Month increment and aggregate recomputation.

*World-level after all countries complete steps 1–28:*
- Taylor Rule runs → `World.policy_rate` updated → propagated to each `Country.policy_rate`.
- **Cross-country deposit processing** — interest paid, distress recalls executed, new foreign deposits opened.
- Monthly snapshot captured (picks up the new rate).
- Monthly trackers reset.

## Invariants
- Treasury inflows are from taxation and deceased-person cash transfer.
- Treasury outflows are government benefit transfers to households.
- Government transfers are balance-neutral: treasury decreases by the same amount household cash increases, leaving total money supply unchanged.
- Net new money enters via loan issuance and automatic money minting.
- Repaying or writing off created principal decreases created-money outstanding.
- Savings interest is a redistribution from bank reserves to household/company cash; it increases total measured money supply by the same amount bank reserves decrease (bank reserves are part of the money supply).
- Tracked money supply remains close to:
  - `initial_supply + Σ(bank.created_money_outstanding) + cumulative_money_minted`

## Stock Market Microstructure

### Daily Market Mechanics (Limit-Order Engine)
The stock market runs once per day after the goods market. Each country's market uses a deterministic limit-order matching engine:

1. **Retail order generation**: A sample of up to `daily_participant_cap` (default 200) traders is selected deterministically via hashed slot indices.
2. **Signal computation**: Each sampled person × listed stock produces two signals:
   - *Trend*: `(last_price − MA20) / MA20` — positive means above recent average
   - *Value gap*: `(fundamental_anchor − last_price) / last_price` — positive means undervalued
3. **Order placement rules** (evaluated in priority order):
   - *Distress sell* (highest priority): if holder's `cash_balance < retail_distress_sell_cash_threshold`, sell 50–90% of free shares at up to 8% discount — forces liquidation regardless of signal
   - *Profit-taking*: if `last_price > avg_cost × (1 + retail_take_profit_gain_threshold)`, sell 30–60% at up to 2% discount
   - *Directional signal*: sell if signal < −sell_threshold; buy if signal > buy_threshold
   - *Random noise*: if no directional intent, sell with probability `retail_random_sell_prob` (holders only) or buy with probability 0.02 (non-holders)
   - One order type per stock per slot; order size bounded by `max_order_fraction_of_float`
4. **Bank maker orders**: For each listed symbol the bank quotes:
   - Bid at `last_price × (1 − maker_spread_bps / 20000)` — limited by equity headroom
   - Ask at `last_price × (1 + maker_spread_bps / 20000)` — size scaled up by `(exposure / budget).min(5)` when bank is over its equity budget (pressure selling)
5. **Price-time priority matching**: Buys sorted descending by limit, sells ascending. Fill at midpoint clamped to `[prev_close × (1 − max_daily_price_move_pct), prev_close × (1 + max_daily_price_move_pct)]`.
6. **Settlement (strict, zero-synthetic)**:
   - Person buy: `cash_balance -= cost`, shares increase
   - Person sell: shares decrease, `cash_balance += proceeds`
   - Bank buy: `reserve_balance -= cost`, bank shares increase
   - Bank sell: bank shares decrease, `reserve_balance += proceeds`
7. **One `record_day_close` per country** is called after all symbols settle.

### Monthly Stock Events
- **Margin calls** (step 6 of month-end): if a borrower cannot cover their loan payment, pledged shares are force-liquidated at `last_price × (1 − margin_liquidation_discount)`. Bank pays (reserve decreases) and receives shares. Person receives proceeds in cash.
- **Secondary offerings** (step 11): listed companies with `consecutive_missed_loan_payments >= secondary_offering_missed_payment_trigger` may raise equity capital by issuing up to `shares_outstanding × secondary_offering_max_dilution` new shares at `last_price × (1 − secondary_offering_discount)`. Retail + bank subscribers provide cash; `shares_outstanding` and `free_float_shares` increase by shares allocated; proceeds go to company cash. Emits `StockSecondaryOffering` event (`[SEC]` log tag).
- **Dividends** (step 24): profitable companies with sufficient runway pay `dividend_payout_ratio` of profit. Household recipients pay 15% withholding; bank receives gross dividend to reserve. Each payment is recorded in a 12-slot ring buffer (`monthly_dps`, `dps_cursor`) on `ListedEquity`; the cursor advances at month end via `reset_mtd_trackers`, evicting the oldest month. The trailing 12-month sum (`trailing_annual_dps`) is capitalised at an 8% required return and added to the P/S-based fundamental anchor in the daily order-generation step, so dividend-paying stocks attract incremental buy pressure and sustain a higher equilibrium price.

### Share Conservation Invariant
For every listed company: `Σ person_holdings + bank_holdings == free_float_shares`. This is checked by `Country::validate_money_invariants` in invariant-focused tests and in runtime builds compiled with `--features runtime-invariants`.

### Key Parameters (`StockMarketParams`)
| Field | Default | Description |
|---|---|---|
| `daily_participant_cap` | 200 | Max traders sampled per day |
| `maker_spread_bps` | 50 | Bank bid-ask spread (basis points) |
| `max_daily_price_move_pct` | 0.08 | Hard ±8% daily price cap |
| `retail_noise_trade_prob` | 0.12 | Noise amplitude for directional signal |
| `retail_random_sell_prob` | 0.22 | Probability of random sell when signal is neutral and holding |
| `retail_distress_sell_cash_threshold` | 500 | Cash floor that triggers urgent position liquidation |
| `retail_take_profit_gain_threshold` | 0.40 | Unrealised gain ratio that triggers profit-taking (40%) |
| `max_order_fraction_of_float` | 0.05 | Max order size as fraction of float |
| `investor_cash_reserve_ratio` | 0.30 | Cash kept liquid before buying |
| `bank_max_equity_reserve_fraction` | 0.15 | Bank equity cap (fraction of reserve) |
| `secondary_offering_missed_payment_trigger` | 1 | Consecutive missed loan payments before secondary offering |
| `secondary_offering_discount` | 0.10 | Discount to last price for secondary offering shares |
| `secondary_offering_max_dilution` | 0.15 | Max new shares as fraction of shares outstanding |
