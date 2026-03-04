# Country Sim: Game Context

## Vision
Country Sim is a terminal-based simulation game where one engine tick equals one in-game day.

> Money simulation requirements are documented in [money-simulation-requirements.md](money-simulation-requirements.md).
>
> Daily demographics and labor behavior are documented in [demographics-labor-model.md](demographics-labor-model.md).
>
> Running analytics tab behavior is documented in [running-charts-tab.md](running-charts-tab.md).
>
> Tick architecture is documented in [ticking-mechanism.md](ticking-mechanism.md).

## Entity Model
### `Tickable`
- Shared trait: `tick_day(&mut self)`
- Implemented by `Person`, `Country`, and `World`

### `Person`
- Economic state: `cash_balance`, optional `loan`, optional `deposit_bank_id`, monthly trackers.
- Employment state: `EmploymentStatus` (`Unemployed` or `Employed { job_type_id, employer_company_id }`).
- Employment tenure: `employment_start_day` (used to compute company-tenure months in UI worker rosters).
- Contract state: `agreed_monthly_salary` (salary set at hire/switch and later adjusted by wage policy passes such as distress cuts and annual raises).
- Demographics: `age_days`, `education_track`, `habilitation`.
- Helpers:
  - `monthly_salary()` returns contract salary for employed people, `0` for unemployed.
  - `can_seek_job()` enforces education-track specific minimum working age.

### `Company`
- Offers multiple existing job types.
- Tracks open vacancies via `open_positions` with explicit offered salary.
- Tracks monthly profitability and profitable streaks.
- Tracks trailing profit history for annual raise eligibility checks.
- Can expand hiring only when profitability and reserve conditions are met.
- Tracks distress state (`consecutive_missed_loan_payments`, `missed_loan_payment_this_month`) and wage policy (`wage_offer_multiplier`).
- Tracks optional `deposit_bank_id` for bank-side savings/run routing.

### `Country`
- Owns `people`, `companies`, treasury, a primary bank (`bank`), and additional banks (`banks`) with public/private typing.
- Maintains bank-level helpers for:
  - lender selection
  - per-bank deposit-liability recomputation
  - run health checks and resolution flow
  - dynamic bank creation
- Daily lifecycle:
  1. age progression and habilitation refresh
  2. age-driven mortality
  3. births using population-rate + carry model
  4. daily job-seeking and hiring against company vacancies
  5. aggregate recomputation
- Monthly lifecycle (every 30 days): payroll/taxes, government spending, loan repayments, **private-bank run processing**, per-bank savings interest, profitability recompute, annual market wage raises (cadence-gated), IPO/secondary/distress passes, labor flows (growth openings, startup formation, essential backfill, **bank formation**, job switching, churn), and close.
- Maintains `next_startup_serial` so startup names remain unique across company removals.

### `World`
- Engine state: `day`, `countries`, pause flag, and monthly histories.
- Running views: `Main`, `Goods`, `Stocks`, `Charts`, and `Events` (switched with `Tab` / `Shift+Tab`).
- Chart history: monthly snapshots (`world_history`, `country_history`, max 120 points).
- Daily tick increments world day and ticks every country.

### UI State
- Navigation state (`selected rows/pages`, `details mode`, active running tab) is held in `UiState` on the UI side.
- UI commands (tab switching, selection focus, paging) update `UiState` locally from the latest engine snapshot.

## Generation and Defaults
- Population per country: `millions_per_country * 1_000_000`.
- Initial money split:
  - 50% to households
  - 50% to companies
- Initial demographics:
  - age pyramid distribution (0-14, 15-24, 25-54, 55-64, 65-74, 75+)
  - education-track distribution (`BasicOnly`, `HighSchool`, `Bachelor`, `MedicalDoctor`)
  - baseline unemployment applied to work-eligible adults

## UI Contract
### Setup
- Input fields: `country_count`, `millions_per_country`, and `generation_seed`
- Numeric validation with memory warning (non-blocking)

### Running
- `Main`: countries and people tables + details views, including country policy overrides
- `Goods`: company and goods market table + company details
  - Company details pages:
    - page 1: company overview
    - page 2: company charts
    - page 3: company-related events
    - page 4: employed workers table with tenure and pay
- `Stocks`: stock table + market summary + stock charts
- `Charts`: 4x2 monthly trend dashboard
- `Events`: scrollable event log with search, sorting, and category filtering
  - category filter key: `f`
  - category options: `All`, `Stock Market`, `Births/Deaths`, `Bankruptcies & Distress`, `Loans & Debt`, `Companies & Labor`, `Government & Policy`
  - search + category are applied as an intersection
- People table/details include demographic and labor state:
  - age
  - education/habilitation
  - employment status

## Testing Focus
- Person lifecycle and qualification progression.
- Daily demographic dynamics (births/deaths).
- Qualification-gated labor matching.
- Company growth openings and hiring behavior.
- Startup formation, salary-aware job switching, distress escalation, and bankruptcy renumber/remap behavior.
- Banking behavior: private/public bank formation, bank-run haircuts, depositor switching, and lender/deposit routing.
- Monthly money flow invariants and tax/loan behavior.
- World tick/month boundary consistency and chart history retention.
