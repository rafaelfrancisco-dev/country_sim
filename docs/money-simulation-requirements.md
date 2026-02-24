# Money Simulation Requirements (Phase 2)

## Goal
Implement a monthly money simulation with realistic circulation between people, companies, country treasury, and a bank.

## Time Model
- One simulation tick remains one in-game day.
- Money processing runs once every 30 days.

## Currency Model
- Use floating-point money (`f64`) because cents are required.
- Round persisted monetary values to 2 decimal places after transaction batches.

## Initial Money Supply
- Initial supply is fixed at generation time.
- Formula: `initial_supply = population * money_per_citizen`.
- Initial split:
  - 50% distributed to people.
  - 50% distributed to companies.
  - 0% to treasury.

## Employment and Jobs
- Every person starts employed.
- Use exactly 10 common real-world job types:
  1. Retail Salesperson
  2. Cashier
  3. Office Clerk
  4. Driver
  5. Construction Worker
  6. Manufacturing Operator
  7. Food Service Worker
  8. Registered Nurse
  9. Teacher
  10. Software Developer
- Jobs pay monthly salaries.
- Companies are multi-role employers:
  - Each company offers multiple job types.
  - Companies employ many people across those roles.

## Country Revenue
- Income tax is collected monthly and transferred to country treasury.
- Treasury can only grow through income tax for now.
- Progressive tax brackets:
  - 10% for first `3000.00`
  - 20% for next `4000.00` (`3000.01` to `7000.00`)
  - 30% for income above `7000.00`

## Consumption and Sales
- Product catalog is currently one item: `generic product`.
- Each month, people spend a random 5% to 15% of available cash.
- Seller company is chosen uniformly at random among companies in the same country.

## Bank and Loan Rules
- Bank is a dedicated country entity.
- Borrowers: people and companies.
- Loan issuance creates new money.
- Principal repayment destroys previously created money.
- Interest is transferred to bank reserves.
- Affordability constraint:
  - Monthly loan payment must be `<= 35%` of borrower monthly income.
- Income basis:
  - Person: monthly salary.
  - Company: rolling 3-month average of monthly sales revenue.
- Loan profile:
  - Fixed APR.
  - Fixed term.
- Missed payment handling:
  - Missed amount remains in debt balance (capitalized delinquency).

## Monthly Processing Order
1. Company liquidity check and pre-payroll loan request.
2. Payroll from companies to people.
3. Income tax withholding and transfer to treasury.
4. Loan repayment pass (people, then companies).
5. Household spending on `generic product`.
6. End-of-month loan requests for negative cash balances.
7. Company monthly close and rolling revenue update.
8. Aggregate recomputation and money-supply invariant check.

## Invariants
- Treasury inflows come only from income tax.
- Net new money enters the system only through loan issuance.
- Repaying created principal reduces created-money outstanding.
- Tracked money supply should stay close to:
  - `initial_supply + created_money_outstanding`

## Scope Notes
- No unemployment model yet.
- No government spending yet.
- No cross-country trade yet.
