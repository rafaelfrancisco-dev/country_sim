# Country Sim: Game Context

## Vision
Country Sim is a terminal-based simulation game where one engine tick equals one in-game day.

> Money simulation requirements are documented in [money-simulation-requirements.md](money-simulation-requirements.md).
>
> Running analytics tab behavior is documented in [running-charts-tab.md](running-charts-tab.md).
>
> Tick architecture is documented in [ticking-mechanism.md](ticking-mechanism.md).

## Entity Model
### `Tickable`
- Shared trait: `tick_day(&mut self)`
- Implemented by `Person`, `Country`, and `World`

### `Person`
- Economic state: `cash_balance`, optional `loan`, monthly trackers.
- Employment state: `EmploymentStatus` (`Unemployed` or `Employed { job_type_id, employer_company_id }`).
- Demographics: `age_days`, `education_track`, `habilitation`.
- Helpers:
  - `monthly_salary()` returns job salary for employed people, `0` for unemployed.
  - `can_seek_job()` enforces education-track specific minimum working age.

### `Company`
- Offers multiple existing job types.
- Tracks open vacancies via `open_positions`.
- Tracks monthly profitability and profitable streaks.
- Can expand hiring only when profitability and reserve conditions are met.

### `Country`
- Owns `people`, `companies`, treasury, and `bank`.
- Daily lifecycle:
  1. age progression and habilitation refresh
  2. age-driven mortality
  3. births using population-rate + carry model
  4. daily job-seeking and hiring against company vacancies
  5. aggregate recomputation
- Monthly lifecycle (every 30 days): payroll, taxes, loans, spending, growth openings, and close.

### `World`
- Navigation state: `day`, `countries`, selected rows/pages, details mode.
- Running tabs: `Main` and `Charts`.
- Chart history: monthly snapshots (`world_history`, `country_history`, max 120 points).
- Daily tick increments world day and ticks every country.

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
- Input fields: `country_count` and `millions_per_country`
- Numeric validation with memory warning (non-blocking)

### Running
- `Main`: countries and people tables + details views
- `Charts`: 2x2 monthly trend dashboard
- People table/details include demographic and labor state:
  - age
  - education/habilitation
  - employment status

## Testing Focus
- Person lifecycle and qualification progression.
- Daily demographic dynamics (births/deaths).
- Qualification-gated labor matching.
- Company growth openings and hiring behavior.
- Monthly money flow invariants and tax/loan behavior.
- World tick/month boundary consistency and chart history retention.
