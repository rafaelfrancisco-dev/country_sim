use super::*;

impl World {
    pub(in crate::sim::world) fn push_export_snapshot(&mut self) {
        let month = self.day / DAYS_PER_MONTH;
        let month_start_day = self.day.saturating_sub(DAYS_PER_MONTH - 1);
        let generation_seed = self.generation_seed;

        let countries_data: Vec<CountryExportData> = {
            let event_log = &self.event_log;
            self.countries
                .iter()
                .map(|country| collect_country_data(country, event_log, month_start_day, month))
                .collect()
        };

        self.export_queue.push(MonthlyExportData {
            month,
            generation_seed,
            countries: countries_data,
        });
    }
}

fn collect_country_data(
    country: &Country,
    event_log: &[SimEvent],
    month_start_day: u64,
    month: u64,
) -> CountryExportData {
    // People aggregates
    let mut employed_count = 0usize;
    let mut unemployed_count = 0usize;
    let mut total_age_days = 0u64;
    let mut total_cash_balance = 0.0_f64;
    let mut total_gross_pay = 0.0_f64;
    let mut total_tax_paid = 0.0_f64;
    let mut total_goods_spending = 0.0_f64;
    let mut count_with_active_loans = 0usize;
    let mut total_loan_balance = 0.0_f64;
    let mut count_with_unmet_needs = 0usize;

    for person in &country.people {
        match person.employment {
            EmploymentStatus::Employed { .. } => employed_count += 1,
            EmploymentStatus::Unemployed => unemployed_count += 1,
        }
        total_age_days += u64::from(person.age_days);
        total_cash_balance += person.cash_balance;
        total_gross_pay += person.last_month_gross_pay;
        total_tax_paid += person.last_month_tax_paid;
        total_goods_spending += person.last_month_goods_spending;
        if let Some(loan) = &person.loan {
            count_with_active_loans += 1;
            total_loan_balance += loan.debt_balance;
        }
        if person.last_month_unmet_needs_mask != 0 {
            count_with_unmet_needs += 1;
        }
    }

    let population = country.people.len();
    let avg_age_years = if population > 0 {
        total_age_days as f64 / population as f64 / 365.0
    } else {
        0.0
    };
    let avg_cash_balance = if population > 0 {
        total_cash_balance / population as f64
    } else {
        0.0
    };
    let avg_loan_balance = if count_with_active_loans > 0 {
        total_loan_balance / count_with_active_loans as f64
    } else {
        0.0
    };

    // Company aggregates
    let mut total_employees = 0u32;
    let mut total_open_positions = 0usize;
    let mut total_company_cash = 0.0_f64;
    let mut total_revenue = 0.0_f64;
    let mut total_payroll = 0.0_f64;
    let mut total_profit = 0.0_f64;
    let mut count_profitable = 0usize;
    let mut count_unprofitable = 0usize;
    let mut total_units_produced = 0u32;
    let mut total_units_sold = 0u32;
    let mut total_units_exported = 0u32;
    let mut count_public_utility = 0usize;
    let mut count_backfill = 0usize;
    let mut count_companies_with_loans = 0usize;

    for company in &country.companies {
        total_employees += company.employee_count;
        total_open_positions += company
            .open_positions
            .iter()
            .map(|o| usize::from(o.slots))
            .sum::<usize>();
        total_company_cash += company.cash_balance;
        total_revenue += company.last_month_sales;
        total_payroll += company.last_month_payroll_paid;
        total_profit += company.last_month_profit;
        if company.last_month_profit > 0.0 {
            count_profitable += 1;
        } else {
            count_unprofitable += 1;
        }
        total_units_produced += company.last_month_units_produced;
        total_units_sold += company.last_month_units_sold;
        total_units_exported += company.last_month_export_units_sold;
        if company.is_public_utility {
            count_public_utility += 1;
        }
        if company.is_essential_backfill {
            count_backfill += 1;
        }
        if company.loan.is_some() {
            count_companies_with_loans += 1;
        }
    }

    // Event counting
    let mut events = EventCounts::default();

    // Events from event_log (days 1 to month_end-1, already drained from previous days)
    for event in event_log {
        let (ev_country, ev_day) = event_country_and_day(event);
        if ev_day >= month_start_day && ev_country == country.name {
            events.accumulate(event);
        }
    }

    // Events from pending_events (month-end day, not yet drained)
    for event in &country.pending_events {
        events.accumulate(event);
    }

    CountryExportData {
        month,
        id: country.id,
        name: country.name.clone(),
        population,
        treasury_balance: country.treasury_balance,
        monthly_tax_collected: country.monthly_tax_collected,
        monthly_tariff_collected: country.monthly_tariff_collected,
        monthly_gov_spending: country.monthly_gov_spending,
        monthly_budget_balance: country.monthly_budget_balance,
        gov_debt: country.gov_debt,
        monthly_household_spending: country.monthly_household_spending,
        monthly_goods_import_spending: country.monthly_goods_import_spending,
        monthly_goods_export_revenue: country.monthly_goods_export_revenue,
        monthly_unmet_need_events: country.monthly_unmet_need_events,
        monthly_essential_debt_spending: country.monthly_essential_debt_spending,
        total_money_supply: country.total_money_supply,
        household_cash_total: country.household_cash_total,
        company_cash_total: country.company_cash_total,
        bank_reserve_balance: country.bank.reserve_balance,
        bank_created_money_outstanding: country.bank.created_money_outstanding,
        expected_money_supply: country.expected_money_supply,
        money_supply_gap: country.money_supply_gap,
        employed_count,
        unemployed_count,
        avg_age_years,
        avg_cash_balance,
        total_gross_pay,
        total_tax_paid,
        total_goods_spending,
        count_with_active_loans,
        avg_loan_balance,
        count_with_unmet_needs,
        total_companies: country.companies.len(),
        total_employees,
        total_open_positions,
        total_company_cash,
        total_revenue,
        total_payroll,
        total_profit,
        count_profitable,
        count_unprofitable,
        total_units_produced,
        total_units_sold,
        total_units_exported,
        count_public_utility,
        count_backfill,
        count_companies_with_loans,
        births: events.births,
        deaths: events.deaths,
        job_switches: events.job_switches,
        layoffs_total: events.layoffs_total,
        companies_founded: events.companies_founded,
        bankruptcies: events.bankruptcies,
        wage_cuts: events.wage_cuts,
        loans_issued: events.loans_issued,
        loans_ended: events.loans_ended,
        loans_defaulted: events.loans_defaulted,
        debt_distress_events: events.debt_distress_events,
        gov_transfers: events.gov_transfers,
        stock_ipos: events.stock_ipos,
        stock_delistings: events.stock_delistings,
        stock_margin_calls: events.stock_margin_calls,
    }
}

#[derive(Default)]
struct EventCounts {
    births: u64,
    deaths: u64,
    job_switches: u64,
    layoffs_total: u64,
    companies_founded: u64,
    bankruptcies: u64,
    wage_cuts: u64,
    loans_issued: u64,
    loans_ended: u64,
    loans_defaulted: u64,
    debt_distress_events: u64,
    gov_transfers: u64,
    stock_ipos: u64,
    stock_delistings: u64,
    stock_margin_calls: u64,
}

impl EventCounts {
    fn accumulate(&mut self, event: &SimEvent) {
        match event {
            SimEvent::PopBirth { count, .. } => self.births += u64::from(*count),
            SimEvent::PopDeath { count, .. } => self.deaths += u64::from(*count),
            SimEvent::JobSwitched { .. } => self.job_switches += 1,
            SimEvent::CompanyLayoffs { laid_off_count, .. } => {
                self.layoffs_total += u64::from(*laid_off_count)
            }
            SimEvent::CompanyFounded { .. } => self.companies_founded += 1,
            SimEvent::CompanyBankrupt { .. } => self.bankruptcies += 1,
            SimEvent::CompanyWageCut { .. } => self.wage_cuts += 1,
            SimEvent::LoanEmitted { .. } => self.loans_issued += 1,
            SimEvent::LoanEnded { .. } => self.loans_ended += 1,
            SimEvent::LoanWrittenOff { .. } => self.loans_defaulted += 1,
            SimEvent::DebtDistress { .. } => self.debt_distress_events += 1,
            SimEvent::GovTransfer { .. } => self.gov_transfers += 1,
            SimEvent::StockIpo { .. } => self.stock_ipos += 1,
            SimEvent::StockDelisted { .. } => self.stock_delistings += 1,
            SimEvent::StockMarginCall { .. } => self.stock_margin_calls += 1,
            _ => {}
        }
    }
}

fn event_country_and_day(event: &SimEvent) -> (&str, u64) {
    match event {
        SimEvent::PopBirth {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::PopDeath {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LoanEmitted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LoanEnded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LoanWrittenOff {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::DebtDistress {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::GovTransfer {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::GovCompanyFunded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyFounded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyWageCut {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyLayoffs {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::JobSwitched {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyBankrupt {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockIpo {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockDividend {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockMarginCall {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockDelisted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockMarketClose {
            country_name, day, ..
        } => (country_name, *day),
    }
}


