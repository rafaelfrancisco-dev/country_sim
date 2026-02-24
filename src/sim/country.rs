use super::{
    bank::{Bank, DEFAULT_MAX_DEBT_TO_INCOME},
    company::{Company, DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies},
    money::round_cents,
    person::Person,
    tickable::Tickable,
};

pub const DAYS_PER_MONTH: u64 = 30;
pub const TAX_BRACKET_1_MAX: f64 = 3_000.0;
pub const TAX_BRACKET_2_MAX: f64 = 7_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Country {
    pub id: u16,
    pub name: String,
    pub people: Vec<Person>,
    pub companies: Vec<Company>,
    pub day: u64,
    pub month: u64,
    pub treasury_balance: f64,
    pub monthly_tax_collected: f64,
    pub monthly_household_spending: f64,
    pub initial_money_supply: f64,
    pub household_cash_total: f64,
    pub company_cash_total: f64,
    pub total_money_supply: f64,
    pub expected_money_supply: f64,
    pub money_supply_gap: f64,
    pub bank: Bank,
}

impl Country {
    pub fn new(id: u16, name: String, people: Vec<Person>) -> Self {
        let companies = build_default_companies(&name, DEFAULT_COMPANIES_PER_COUNTRY);
        Self::with_economy(id, name, people, companies, 0.0)
    }

    pub fn with_economy(
        id: u16,
        name: String,
        people: Vec<Person>,
        companies: Vec<Company>,
        initial_money_supply: f64,
    ) -> Self {
        let mut country = Self {
            id,
            name,
            people,
            companies,
            day: 0,
            month: 0,
            treasury_balance: 0.0,
            monthly_tax_collected: 0.0,
            monthly_household_spending: 0.0,
            initial_money_supply: round_cents(initial_money_supply),
            household_cash_total: 0.0,
            company_cash_total: 0.0,
            total_money_supply: 0.0,
            expected_money_supply: 0.0,
            money_supply_gap: 0.0,
            bank: Bank::new(),
        };
        country.recompute_company_headcounts();
        country.recompute_aggregates();
        country
    }

    pub fn population(&self) -> usize {
        self.people.len()
    }

    pub fn company_name(&self, company_id: u16) -> Option<&str> {
        self.companies
            .get(company_id as usize)
            .map(|company| company.name.as_str())
    }

    pub fn process_month(&mut self) {
        self.monthly_tax_collected = 0.0;
        self.monthly_household_spending = 0.0;
        self.recompute_company_headcounts();

        for company in &mut self.companies {
            company.open_month();
        }

        self.ensure_company_payroll_liquidity();
        self.process_payroll_and_taxes();
        self.collect_monthly_loan_repayments();
        self.process_household_spending();
        self.process_end_of_month_loan_requests();
        self.close_company_months();

        self.month = self.month.saturating_add(1);
        self.recompute_aggregates();
    }

    fn ensure_company_payroll_liquidity(&mut self) {
        let mut payroll_requirements = vec![0.0; self.companies.len()];
        for person in &self.people {
            let company_idx = person.employer_company_id as usize;
            if let Some(total) = payroll_requirements.get_mut(company_idx) {
                *total = round_cents(*total + person.monthly_salary());
            }
        }

        for (company_idx, company) in self.companies.iter_mut().enumerate() {
            let required_payroll = payroll_requirements
                .get(company_idx)
                .copied()
                .unwrap_or(0.0);
            company.monthly_payroll_budget = required_payroll;

            if company.cash_balance >= required_payroll {
                continue;
            }

            let shortfall = round_cents(required_payroll - company.cash_balance);
            let monthly_income = company.average_monthly_revenue();
            let issued = self.bank.originate_loan(
                shortfall,
                monthly_income,
                DEFAULT_MAX_DEBT_TO_INCOME,
                &mut company.loan,
            );
            if issued > 0.0 {
                company.cash_balance = round_cents(company.cash_balance + issued);
                company.last_month_new_loan = round_cents(company.last_month_new_loan + issued);
            }
        }
    }

    fn process_payroll_and_taxes(&mut self) {
        for person in &mut self.people {
            person.reset_monthly_trackers();

            let company_idx = person.employer_company_id as usize;
            let salary = person.monthly_salary();
            let gross_pay = if let Some(company) = self.companies.get_mut(company_idx) {
                let payout = round_cents(salary.min(company.cash_balance.max(0.0)));
                company.cash_balance = round_cents(company.cash_balance - payout);
                company.last_month_payroll_paid =
                    round_cents(company.last_month_payroll_paid + payout);
                payout
            } else {
                0.0
            };

            let tax = monthly_income_tax(gross_pay).min(gross_pay);
            let net_pay = round_cents(gross_pay - tax);

            person.cash_balance = round_cents(person.cash_balance + net_pay);
            person.last_month_gross_pay = gross_pay;
            person.last_month_tax_paid = tax;
            person.last_month_net_pay = net_pay;

            self.treasury_balance = round_cents(self.treasury_balance + tax);
            self.monthly_tax_collected = round_cents(self.monthly_tax_collected + tax);
        }
    }

    fn collect_monthly_loan_repayments(&mut self) {
        for person in &mut self.people {
            let payment = self
                .bank
                .collect_monthly_payment(&mut person.cash_balance, &mut person.loan);
            person.last_month_loan_payment = payment.amount_paid;
        }

        for company in &mut self.companies {
            let payment = self
                .bank
                .collect_monthly_payment(&mut company.cash_balance, &mut company.loan);
            company.last_month_loan_payment = payment.amount_paid;
        }
    }

    fn process_household_spending(&mut self) {
        if self.companies.is_empty() {
            return;
        }

        let month_seed = self.month.saturating_add(1);
        let company_count = self.companies.len();

        for person in &mut self.people {
            if person.cash_balance <= 0.0 {
                continue;
            }

            let seed = hash64(
                (u64::from(self.id) << 48)
                    ^ (month_seed << 24)
                    ^ u64::from(person.id)
                    ^ u64::from(person.name_seed),
            );
            let spend_percent = 5.0 + (seed % 11) as f64;
            let spend = round_cents(person.cash_balance * spend_percent / 100.0);
            if spend <= 0.0 {
                continue;
            }

            let seller_idx = (hash64(seed ^ 0x9e37_79b9_7f4a_7c15) as usize) % company_count;
            person.cash_balance = round_cents(person.cash_balance - spend);
            person.last_month_spending = spend;

            if let Some(company) = self.companies.get_mut(seller_idx) {
                company.cash_balance = round_cents(company.cash_balance + spend);
                company.last_month_sales = round_cents(company.last_month_sales + spend);
            }

            self.monthly_household_spending = round_cents(self.monthly_household_spending + spend);
        }
    }

    fn process_end_of_month_loan_requests(&mut self) {
        for person in &mut self.people {
            if person.cash_balance >= 0.0 {
                continue;
            }

            let shortfall = round_cents(-person.cash_balance);
            let monthly_income = person.monthly_salary();
            let issued = self.bank.originate_loan(
                shortfall,
                monthly_income,
                DEFAULT_MAX_DEBT_TO_INCOME,
                &mut person.loan,
            );

            if issued > 0.0 {
                person.cash_balance = round_cents(person.cash_balance + issued);
                person.last_month_new_loan = round_cents(person.last_month_new_loan + issued);
            }
        }

        for company in &mut self.companies {
            if company.cash_balance >= 0.0 {
                continue;
            }

            let shortfall = round_cents(-company.cash_balance);
            let monthly_income = company.average_monthly_revenue();
            let issued = self.bank.originate_loan(
                shortfall,
                monthly_income,
                DEFAULT_MAX_DEBT_TO_INCOME,
                &mut company.loan,
            );

            if issued > 0.0 {
                company.cash_balance = round_cents(company.cash_balance + issued);
                company.last_month_new_loan = round_cents(company.last_month_new_loan + issued);
            }
        }
    }

    fn close_company_months(&mut self) {
        for company in &mut self.companies {
            company.close_month();
        }
    }

    fn recompute_company_headcounts(&mut self) {
        for company in &mut self.companies {
            company.employee_count = 0;
        }

        for person in &self.people {
            if let Some(company) = self.companies.get_mut(person.employer_company_id as usize) {
                company.employee_count = company.employee_count.saturating_add(1);
            }
        }
    }

    pub fn recompute_aggregates(&mut self) {
        self.household_cash_total =
            round_cents(self.people.iter().map(|person| person.cash_balance).sum());
        self.company_cash_total = round_cents(
            self.companies
                .iter()
                .map(|company| company.cash_balance)
                .sum(),
        );
        self.total_money_supply = round_cents(
            self.household_cash_total
                + self.company_cash_total
                + self.treasury_balance
                + self.bank.reserve_balance,
        );
        self.expected_money_supply =
            round_cents(self.initial_money_supply + self.bank.created_money_outstanding);
        self.money_supply_gap = round_cents(self.total_money_supply - self.expected_money_supply);
    }
}

impl Tickable for Country {
    fn tick_day(&mut self) {
        self.day = self.day.saturating_add(1);
        if self.day % DAYS_PER_MONTH == 0 {
            self.process_month();
        }
    }
}

pub fn monthly_income_tax(gross_pay: f64) -> f64 {
    let mut remaining = gross_pay.max(0.0);
    let mut tax = 0.0;

    let first_bracket = remaining.min(TAX_BRACKET_1_MAX);
    tax += first_bracket * 0.10;
    remaining -= first_bracket;

    let second_span = TAX_BRACKET_2_MAX - TAX_BRACKET_1_MAX;
    let second_bracket = remaining.min(second_span);
    tax += second_bracket * 0.20;
    remaining -= second_bracket;

    if remaining > 0.0 {
        tax += remaining * 0.30;
    }

    round_cents(tax)
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{company::build_default_companies, person::Person};

    fn build_people(count: usize) -> Vec<Person> {
        (0..count)
            .map(|idx| Person::new(idx as u32, (idx % 10) as u8, (idx % 4) as u16, idx as u32))
            .collect()
    }

    #[test]
    fn tax_brackets_follow_progressive_curve() {
        assert!((monthly_income_tax(2_000.0) - 200.0).abs() < f64::EPSILON);
        assert!((monthly_income_tax(5_000.0) - 700.0).abs() < f64::EPSILON);
        assert!((monthly_income_tax(9_000.0) - 1_700.0).abs() < f64::EPSILON);
    }

    #[test]
    fn month_processing_runs_every_thirty_days() {
        let companies = build_default_companies("A", 4);
        let mut country =
            Country::with_economy(1, "A".to_string(), build_people(10), companies, 0.0);

        for _ in 0..29 {
            country.tick_day();
        }
        assert_eq!(country.month, 0);

        country.tick_day();
        assert_eq!(country.month, 1);
    }

    #[test]
    fn treasury_only_grows_from_taxation() {
        let companies = build_default_companies("A", 4);
        let mut country =
            Country::with_economy(1, "A".to_string(), build_people(20), companies, 0.0);
        let treasury_before = country.treasury_balance;

        for _ in 0..30 {
            country.tick_day();
        }

        assert!(country.treasury_balance >= treasury_before);
        assert!(
            (country.monthly_tax_collected - (country.treasury_balance - treasury_before)).abs()
                < 0.01
        );
    }

    #[test]
    fn money_is_created_by_loan_issuance_only() {
        let companies = build_default_companies("A", 2);
        let mut people = build_people(2);
        people[0].cash_balance = -500.0;
        let mut country = Country::with_economy(1, "A".to_string(), people, companies, 100_000.0);

        country.process_end_of_month_loan_requests();
        country.recompute_aggregates();

        assert!(country.bank.total_originated >= 0.0);
        let expected =
            round_cents(country.initial_money_supply + country.bank.created_money_outstanding);
        assert!((country.expected_money_supply - expected).abs() < f64::EPSILON);
    }
}
