# Cross-Country Migration Model

## Scope
This document defines the monthly world-level migration pass that moves people between countries.

## Processing Order
Migration runs once per completed month at world scope:
1. each country finishes its own month-end processing
2. the world evaluates cross-country migration opportunities
3. migrants are physically transferred between country populations
4. snapshots/export rows are captured from the post-migration state

This keeps migration based on finished monthly labor/fiscal conditions while ensuring history and CSV export reflect the updated populations.

## Triggers
Migration is controlled by `SimulationParams.migration`.

### Distress Migration
A person is eligible for distress migration when all of these hold:
- they are a labor-force participant
- they are unemployed
- they have been unemployed for at least `long_term_unemployment_months`
- they have either:
  - stayed on benefits long enough to clear the benefit-duration threshold plus `benefit_exhaustion_bonus_months`, or
  - accumulated `chronic_unmet_needs_months` of severe unmet-needs pressure
- they are past the migration cooldown
- they satisfy the minimum cash buffer

Distress migration is probabilistic, using `base_monthly_migration_rate`.

### Work Migration
An employed worker can migrate when a foreign opening is considered irresistible:
- same `job_type_id` as the current contract
- offered salary exceeds the current salary by at least `irresistible_offer_salary_premium`
- the destination has at least `irresistible_offer_min_open_slots` compatible openings
- the destination passes the unemployment guard
- the worker is past cooldown and satisfies the cash buffer

## Destination Selection
Destinations are rejected when labor-force unemployment is above `destination_unemployment_guard`.

Selection rules:
- work migration prefers the highest same-job foreign salary
- distress migration prefers countries with the strongest compatible opening depth, then lower unemployment, then better top opening
- only compatible openings for the person's habilitation are considered

## Transfer Rules
Migration physically removes the `Person` from one `Country.people` vector and inserts the same record into another.

Preserved state:
- cash
- age, education track, habilitation
- unmet-needs and unemployment history
- personal loan balance
- migration history

Reset or reattached state:
- housing is reset to `Unhoused`
- local and foreign deposit placement is cleared, then reattached locally in the destination
- personal loan lender is rehomed to a destination bank
- distress migrants may arrive unemployed
- work migrants consume a destination opening and arrive already employed

## Counters and Events
Each country tracks monthly migration counters:
- `monthly_immigrants`
- `monthly_emigrants`
- `monthly_migrants_for_work`
- `monthly_migrants_for_distress`

Migration emits:
- `PersonMigratedForWork`
- `PersonMigratedForDistress`

These events appear in the Events tab, country/person related-event views, and monthly CSV export counts.

## Person Metadata
People now track:
- `birth_country_id`
- `current_country_id`
- `months_on_benefits`
- `lifetime_migrations`
- `last_migration_day`

These fields are surfaced in person details and used for migration gating/cooldown.
