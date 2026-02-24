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

## Country Revenue
- Monthly payroll is paid only to employed people.
- Income tax is withheld monthly and transferred to treasury.
- Treasury inflows come from tax and deceased-person cash transfer.

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
1. Company liquidity check and pre-payroll loan request.
2. Payroll from companies to employed people.
3. Income tax withholding and transfer to treasury.
4. Loan repayment pass (people, then companies).
5. Household spending.
6. End-of-month loan requests.
7. Company profitability update and vacancy expansion.
8. Company close and aggregate recomputation.

## Invariants
- Treasury inflows are from taxation and deceased-person cash transfer.
- Net new money enters via loan issuance.
- Repaying or writing off created principal decreases created-money outstanding.
- Tracked money supply remains close to:
  - `initial_supply + created_money_outstanding`
