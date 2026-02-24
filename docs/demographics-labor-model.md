# Demographics and Labor Model

## Scope
This document defines the daily population and labor simulation added on top of the monthly money system.

## Daily Processing Order
Each `Country` day tick executes in this order:
1. Increase age by one day for all people.
2. Refresh habilitation based on age and education track.
3. Apply age-based mortality.
4. Apply births using expected-value carry.
5. Run daily job seeking and qualification-gated hiring.
6. Recompute headcounts and money aggregates.

## Person Model
People carry:
- `age_days`
- `education_track`: `BasicOnly`, `HighSchool`, `Bachelor`, `MedicalDoctor`
- `habilitation`: `NoSchoolYet`, `BasicSchool`, `HighSchool`, `HigherLearning`
- `employment`: `Unemployed` or `Employed { job_type_id, employer_company_id }`

### Habilitation Progression
- age < 12: `NoSchoolYet`
- age 12..17: `BasicSchool`
- age >= 18:
  - `BasicOnly`: `BasicSchool`
  - `HighSchool`: `HighSchool`
  - `Bachelor`: `HigherLearning` from age 21, else `HighSchool`
  - `MedicalDoctor`: `HigherLearning` from age 23, else `HighSchool`

### Job Seeking Gate
`can_seek_job()` minimum age by track:
- `BasicOnly` / `HighSchool`: 18
- `Bachelor`: 21
- `MedicalDoctor`: 23

## Demographic Rates
### Births
- Annual crude rate: `17.0 births / 1000 population`
- Daily expected births:
  - `population * (17 / 1000 / 365) + birth_carry`
- Integer births = floor(expected), remainder kept in `birth_carry`.

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

## Labor Market
### Job Requirement Mapping
- Basic School jobs: Retail Salesperson, Cashier, Driver, Construction Worker, Manufacturing Operator, Food Service Worker
- High School jobs: Office Clerk
- Higher Learning jobs: Registered Nurse, Teacher, Software Developer

### Daily Seek Probability
For unemployed, eligible people:
- `base = 0.002 + max(age - 18, 0) * 0.0002`
- `cash_pressure = clamp((3000 - cash_balance)/3000, 0, 1) * 0.01`
- `seek_prob = min(0.03, base + cash_pressure)`

### Hiring Behavior
- If a seeker passes seek probability, they attempt to match any open slot.
- Slot must satisfy habilitation requirement.
- No interview failure model: a valid slot is accepted immediately.

## Company Growth Openings
Openings are created monthly, only if both hold:
- `consecutive_profitable_months >= 3`
- `cash_balance >= max(100_000, monthly_payroll_budget * 1.5)`

Opening count:
- `openings_to_add = min(8, 1 + floor(last_month_profit / 50_000))`

Openings are distributed across existing offered job types weighted by each job's assignment weight.

## Death Side Effects
When a person dies:
- positive cash transfers to treasury
- employment is cleared and a replacement opening is added back to their former employer/job
- personal loan is written off through bank accounting so `created_money_outstanding` is reduced by written-off created principal

## Runtime Model
Monthly processing is synchronous with daily ticks. This avoids async state divergence between daily demographic mutations and monthly economic mutations.
