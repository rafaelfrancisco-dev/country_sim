# Demographics and Labor Model

## Scope
This document defines the daily population and labor simulation added on top of the monthly money system.

## Daily Processing Order
Each `Country` day tick executes in this order:
1. Increase age by one day for all people.
2. Refresh habilitation based on age and education track.
3. Apply age-based mortality (probabilistic draw).
4. Apply births using a daily Poisson draw.
5. Run daily job seeking and qualification-gated hiring.
6. Recompute headcounts and money aggregates.

## Person Model
People carry:
- `age_days`
- `education_track`: `BasicOnly`, `HighSchool`, `Bachelor`, `Doctor`
- `habilitation`: `NoSchoolYet`, `BasicSchool`, `HighSchool`, `HigherLearning`
- `employment`: `Unemployed` or `Employed { job_type_id, employer_company_id }`
- `agreed_monthly_salary` (employment contract salary; 0 for unemployed)
- `months_unemployed`
- `months_on_benefits`
- `birth_country_id`, `current_country_id`
- `lifetime_migrations`, `last_migration_day`

### Habilitation Progression
- age < 12: `NoSchoolYet`
- age 12..17: `BasicSchool`
- age >= 18:
  - `BasicOnly`: `BasicSchool`
  - `HighSchool`: `HighSchool`
  - `Bachelor`: `HigherLearning` from age 21, else `HighSchool`
  - `Doctor`: `HigherLearning` from age 23, else `HighSchool`

### Job Seeking Gate
`can_seek_job()` minimum age by track:
- `BasicOnly` / `HighSchool`: 18
- `Bachelor`: 21
- `Doctor`: 23

The `Doctor` track represents doctoral or PhD-level academic study, not medical doctors.

## Demographic Rates
### Births
- Annual crude rate: `17.0 births / 1000 population`
- Daily expected rate:
  - `lambda = population * (17 / 1000 / 365)`
- Daily births sampled from `Poisson(lambda)`.

### Deaths
Annual mortality by age (years):
- 0: 0.5%
- 1-14: 0.02%
- 15-24: 0.07%
- 25-34: 0.13%
- 35-44: 0.20%
- 45-54: 0.45%
- 55-64: 0.95%
- 65-74: 2.2%
- 75-84: 5.7%
- 85-94: 14.0%
- 95+: 30.0%

Converted to daily with:
- `p_day = 1 - (1 - p_year)^(1/365)`
- Each person rolls against `p_day` using runtime entropy each day.

## Labor Market
### Job Requirement Mapping
- Basic School jobs: Retail Salesperson, Cashier, Driver, Construction Worker, Manufacturing Operator, Food Service Worker
- High School jobs: Office Clerk
- Higher Learning jobs: Registered Nurse, Teacher, Software Developer

Qualification is only one side of the match. Companies also expose sector-specific catalogs:

- mine-like sectors: `Construction Worker`, `Driver`, `Manufacturing Operator`
- timber/basic production: `Driver`, `Manufacturing Operator`, `Office Clerk`
- housing: `Construction Worker`, `Driver`, `Office Clerk`
- consumer and industrial manufacturing: `Manufacturing Operator`, `Driver`, `Office Clerk`
- medicine: `Registered Nurse`, `Office Clerk`, `Driver`
- education: `Teacher`, `Office Clerk`
- internet/software: `Software Developer`, `Office Clerk`

### Daily Seek Probability
For unemployed, eligible people:
- `base = 0.010 + max(age - 18, 0) * 0.0003`
- `cash_pressure = clamp((3000 - cash_balance)/3000, 0, 1) * 0.05`
- `seek_prob = min(0.15, base + cash_pressure)`

### Hiring Behavior
- If a seeker passes seek probability, they attempt to match any open slot.
- Slot must be offered by the target company's sector catalog.
- Slot must satisfy habilitation requirement.
- No interview failure model: a valid slot is accepted immediately.
- On hire, the person contract salary is set from the opening offer.

### Monthly Job Mobility
- Evaluated once per month for each employed worker.
- A switch is allowed only when all conditions hold:
  - same profession (`job_type_id`) as current contract
  - different target company
  - offered salary `>= current_salary * 1.10`
- Best target is chosen by highest offered salary (tie: lower company ID).
- On switch:
  - worker moves to target and adopts offered salary
  - prior employer receives a replacement opening at the worker's previous salary

### Monthly Cross-Country Migration
- Evaluated once per month at `World` scope after all countries finish month-end processing.
- Distress migration targets long-term unemployed workers with prolonged benefits or chronic severe unmet needs.
- Work migration targets already-employed workers only when a foreign same-job opening exceeds the current contract by the configured irresistible premium.
- Destination countries must pass an unemployment guard and offer compatible openings for the person's habilitation.
- Migration physically moves the `Person` record between countries.
- Distress migrants may arrive unemployed; work migrants consume a destination opening and arrive already employed.
- Housing resets to `Unhoused`; deposit placement and loan lender are reattached locally in the destination country.

## Monthly Labor Churn + Separations
Independent from the distress/bankruptcy path, month-end processing includes labor churn for employed workers:
- `retirement`: sampled first, only for workers age 65+
- `layoff` (non-distress): sampled second, independent from loan/distress state
- `quit`: sampled third

Behavior:
- Each separated worker becomes `Unemployed`.
- The former employer receives one replacement opening with the same `job_type_id` and the worker's prior contract salary.
- A `LaborChurnMonthly` event is emitted with monthly counts:
  - `quits`
  - `retirements`
  - `layoffs_non_distress`
  - `replacement_openings`

Notes:
- Channel ordering is deterministic and mutually exclusive per worker in a given month.
- Distress layoffs remain tracked separately via `CompanyLayoffs`.

## Company Growth Openings
Openings are created monthly, only if both hold:
- `consecutive_profitable_months >= 3`
- `cash_balance >= max(100_000, monthly_payroll_budget * 1.5)`

Opening count:
- `openings_to_add = min(8, 1 + floor(last_month_profit / 50_000))`

Openings are distributed across existing offered job types weighted by each job's assignment weight.

## Annual Market Wage Raises
- A cadence-gated raise pass runs yearly by default (`cadence_months = 12`).
- Eligibility requires:
  - positive payroll reference (`monthly_payroll_budget > 0`)
  - sufficient trailing profit history window (`min_history_months`, default 12)
  - positive average profit over that window
  - enough cash runway: `cash_balance >= monthly_payroll_budget × min_cash_runway_months` (default `2.0`)
  - enough profit margin: `trailing_average_profit / monthly_payroll_budget >= min_profit_to_payroll_ratio` (default `5%`)
- Raise size is market-following:
  - internal reference salary per job = average incumbent contract salary (fallback to average opening salary)
  - external benchmark salary per job = highest same-job salary among other companies (contracts and openings)
  - private-company benchmarking excludes public-utility wages
  - company raise rate = maximum positive `(external / internal - 1) × market_gap_convergence` across jobs
  - optional cap via `max_raise_rate` (default `6%`)
- Applied to all contracts and openings in that company (and updates `wage_offer_multiplier` accordingly).

## Startup Formation (Monthly)
- Founders can come from employed workers and qualified unemployed adults.
- Team size is bounded (`3..6` by default).
- Investable cash is reserve-adjusted and loan-aware:
  - `investable = max(0, cash_balance - reserve_cash)`
  - low-cash unemployed founders may still participate when startup lending is enabled.
- Product gate:
  - prefer essential goods with strong observed sales
  - fallback to high-margin goods (`>= 25%`)
  - founder profession must be allowed for the chosen sector
- Viability gate:
  - payroll for the opening plan must be affordable vs expected revenue for the target good
  - expected revenue blends observed sales with household demand and company-input demand proxies
- Default opening-plan envelope is `3..8` and `max_startups_per_month` defaults to `8`.
- Default affordability cap is tighter: `startup_max_payroll_to_revenue = 0.70`.
- Funding stack:
  - founder capital
  - optional startup bank loan (capped fraction of required capital)
- Founders leave prior employers, old employers receive replacement openings, founders join the startup, and founder equity stakes are recorded.

## Job Switching
- Voluntary job switching still requires beating the configured raise threshold (`switch_raise_threshold`, default `10%`).
- Workers must now satisfy a minimum-tenure gate before switching: `min_tenure_months_before_switch` defaults to `6`.

## Distress and Bankruptcy (Monthly)
- Distress is tracked by consecutive missed company loan payments.
- Escalation:
  - miss #1: 10% wage cut (employees + openings, floored to 70% catalog salary)
  - later misses/losses: deterministic layoffs are capped by `max_monthly_layoff_rate` (default `3%`)
  - loss-only correction starts after `loss_distress_month_threshold` months (default `2`)
  - default loss-trigger wage cut is `6%`
  - default loss-trigger layoff rate is `5%`
  - bankruptcy requires deep sustained distress (missed-payment and loss duration gates), non-positive cash, and no additional loan capacity
  - before bankruptcy, private firms attempt distress IPO rescue
- Countercyclical brake:
  - when labor-force unemployment is high, layoffs and bankruptcy escalation are throttled
  - government can spawn additional public employers as a stability backstop
- Bankruptcy removes companies physically and renumbers IDs; person employment references are remapped.

## Death Side Effects
When a person dies:
- positive cash transfers to treasury
- employment is cleared and a replacement opening is added back to their former employer/job
- personal loan is written off through bank accounting so `created_money_outstanding` is reduced by written-off created principal

## Runtime Model
Monthly processing is synchronous with daily ticks. This avoids async state divergence between daily demographic mutations and monthly economic mutations.
