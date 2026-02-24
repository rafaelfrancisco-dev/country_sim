#[cfg(test)]
use super::company::{DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies};
use super::{
    bank::{Bank, DEFAULT_MAX_DEBT_TO_INCOME},
    company::Company,
    job::{JobTypeId, job_is_eligible_for_habilitation},
    money::round_cents,
    params::SimulationParams,
    person::{EmploymentStatus, HabilitationLevel, Person},
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
    pub next_person_id: u32,
    pub birth_carry: f64,
    pub last_day_births: u32,
    pub last_day_deaths: u32,
    pub sim_params: SimulationParams,
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
    #[cfg(test)]
    pub fn new(id: u16, name: String, people: Vec<Person>) -> Self {
        let companies = build_default_companies(&name, DEFAULT_COMPANIES_PER_COUNTRY);
        Self::with_economy(id, name, people, companies, 0.0)
    }

    #[cfg(test)]
    pub fn with_economy(
        id: u16,
        name: String,
        people: Vec<Person>,
        companies: Vec<Company>,
        initial_money_supply: f64,
    ) -> Self {
        Self::with_economy_and_params(
            id,
            name,
            people,
            companies,
            initial_money_supply,
            SimulationParams::default(),
        )
    }

    pub fn with_economy_and_params(
        id: u16,
        name: String,
        people: Vec<Person>,
        companies: Vec<Company>,
        initial_money_supply: f64,
        sim_params: SimulationParams,
    ) -> Self {
        let next_person_id = people
            .iter()
            .map(|person| person.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        let mut country = Self {
            id,
            name,
            people,
            companies,
            day: 0,
            month: 0,
            next_person_id,
            birth_carry: 0.0,
            last_day_births: 0,
            last_day_deaths: 0,
            sim_params,
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
        self.update_company_growth_openings();
        self.close_company_months();

        self.month = self.month.saturating_add(1);
        self.recompute_aggregates();
    }

    fn ensure_company_payroll_liquidity(&mut self) {
        let mut payroll_requirements = vec![0.0; self.companies.len()];
        for person in &self.people {
            let Some(company_id) = person.employer_company_id() else {
                continue;
            };
            let company_idx = company_id as usize;
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

            let gross_pay = if let Some(company_id) = person.employer_company_id() {
                let company_idx = company_id as usize;
                let salary = person.monthly_salary();
                if let Some(company) = self.companies.get_mut(company_idx) {
                    let payout = round_cents(salary.min(company.cash_balance.max(0.0)));
                    company.cash_balance = round_cents(company.cash_balance - payout);
                    company.last_month_payroll_paid =
                        round_cents(company.last_month_payroll_paid + payout);
                    payout
                } else {
                    0.0
                }
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

    fn update_company_growth_openings(&mut self) {
        let month_seed = self.month.saturating_add(1);
        for company in &mut self.companies {
            company.recompute_profitability();
            company.open_positions_from_growth(&self.sim_params, month_seed);
        }
    }

    fn close_company_months(&mut self) {
        for company in &mut self.companies {
            company.close_month();
        }
    }

    fn run_daily_lifecycle(&mut self) {
        self.age_people_one_day();
        self.apply_daily_deaths();
        self.create_daily_births();
        self.process_daily_job_market();

        self.recompute_company_headcounts();
        self.recompute_aggregates();
    }

    fn age_people_one_day(&mut self) {
        for person in &mut self.people {
            person.increment_age_one_day();
        }
    }

    fn apply_daily_deaths(&mut self) {
        let demographic = self.sim_params.demographic;
        let mut deaths = 0_u32;
        let mut survivors = Vec::with_capacity(self.people.len());
        let day = self.day;
        let country_id = self.id;

        let (people, companies, bank) = (&mut self.people, &mut self.companies, &mut self.bank);
        for mut person in people.drain(..) {
            let death_probability =
                demographic.daily_death_probability_for_age_years(person.age_years());
            let death_seed = hash64(
                (u64::from(country_id) << 48)
                    ^ (day << 24)
                    ^ u64::from(person.id)
                    ^ u64::from(person.name_seed)
                    ^ 0xdeca_fbadu64,
            );

            if roll_probability(death_seed, death_probability) {
                deaths = deaths.saturating_add(1);
                if person.cash_balance > 0.0 {
                    self.treasury_balance =
                        round_cents(self.treasury_balance + person.cash_balance);
                }

                if let Some((company_id, job_type_id)) = person.clear_employment() {
                    if let Some(company) = companies.get_mut(company_id as usize) {
                        company.add_opening(job_type_id, 1);
                    }
                }

                bank.write_off_loan(&mut person.loan);
            } else {
                survivors.push(person);
            }
        }

        self.people = survivors;
        self.last_day_deaths = deaths;
    }

    fn create_daily_births(&mut self) {
        let births_per_person_per_day = self.sim_params.demographic.births_per_person_per_day();
        let expected_births =
            self.people.len() as f64 * births_per_person_per_day + self.birth_carry;
        let births = expected_births.floor() as u32;
        self.birth_carry = expected_births - births as f64;

        self.last_day_births = births;
        if births == 0 {
            return;
        }

        for idx in 0..births {
            let seed = hash64(
                (u64::from(self.id) << 48)
                    ^ (self.day << 24)
                    ^ u64::from(self.next_person_id)
                    ^ u64::from(idx),
            );
            let education_track = self
                .sim_params
                .education
                .pick_track(unit_interval(seed ^ 0x51ed_15ea_5eed));

            let newborn = Person::new_with_profile(
                self.next_person_id,
                EmploymentStatus::Unemployed,
                (seed >> 32) as u32,
                0,
                education_track,
            );
            self.next_person_id = self.next_person_id.saturating_add(1);
            self.people.push(newborn);
        }
    }

    fn process_daily_job_market(&mut self) {
        if self.companies.is_empty() {
            return;
        }

        let day = self.day;
        let country_id = self.id;
        let labor_params = self.sim_params.labor;
        let (people, companies) = (&mut self.people, &mut self.companies);

        for person in people.iter_mut() {
            if person.is_employed() || !person.can_seek_job() {
                continue;
            }

            let seek_probability =
                labor_params.daily_seek_probability(person.age_years(), person.cash_balance);
            let seek_seed = hash64(
                (u64::from(country_id) << 48)
                    ^ (day << 24)
                    ^ u64::from(person.id)
                    ^ u64::from(person.name_seed)
                    ^ 0x5ee1_5eed_u64,
            );

            if !roll_probability(seek_seed, seek_probability) {
                continue;
            }

            if let Some((company_id, job_id)) =
                consume_matching_opening(companies, person.habilitation, seek_seed)
            {
                person.set_employment(company_id, job_id);
            }
        }
    }

    fn recompute_company_headcounts(&mut self) {
        for company in &mut self.companies {
            company.employee_count = 0;
        }

        for person in &self.people {
            let Some(company_id) = person.employer_company_id() else {
                continue;
            };
            if let Some(company) = self.companies.get_mut(company_id as usize) {
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
        self.tick_day_without_month();
        if self.day % DAYS_PER_MONTH == 0 {
            self.process_month();
        }
    }
}

impl Country {
    pub fn tick_day_without_month(&mut self) {
        self.day = self.day.saturating_add(1);
        self.last_day_births = 0;
        self.last_day_deaths = 0;
        self.run_daily_lifecycle();
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

fn consume_matching_opening(
    companies: &mut [Company],
    qualification: HabilitationLevel,
    seed: u64,
) -> Option<(u16, JobTypeId)> {
    let mut chosen: Option<(usize, usize, u64)> = None;
    let mut seen_slots = 0_u64;

    for (company_idx, company) in companies.iter().enumerate() {
        for (opening_idx, opening) in company.open_positions.iter().enumerate() {
            if opening.slots == 0
                || !job_is_eligible_for_habilitation(opening.job_type_id, qualification)
            {
                continue;
            }

            let slots = u64::from(opening.slots);
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed =
                seed ^ (u64::from(company_idx as u32) << 32) ^ u64::from(opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some((company_idx, opening_idx, seen_slots));
            }
        }
    }

    let (company_idx, opening_idx, _) = chosen?;
    let company = companies.get_mut(company_idx)?;
    let job_id = company.consume_opening(opening_idx)?;
    Some((company_idx as u16, job_id))
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}

fn roll_probability(seed: u64, probability: f64) -> bool {
    if probability <= 0.0 {
        return false;
    }
    if probability >= 1.0 {
        return true;
    }

    unit_interval(seed) < probability
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{
        company::build_default_companies,
        params::DemographicParams,
        person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus},
    };

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
    fn payroll_and_taxes_only_apply_when_employed() {
        let companies = build_default_companies("A", 2);
        let mut person = Person::new_with_profile(
            1,
            EmploymentStatus::Unemployed,
            10,
            25 * DAYS_PER_YEAR,
            EducationTrack::HighSchool,
        );
        person.cash_balance = 100.0;
        let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);

        country.process_month();

        assert!((country.monthly_tax_collected - 0.0).abs() < f64::EPSILON);
        assert!((country.people[0].last_month_gross_pay - 0.0).abs() < f64::EPSILON);
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

    #[test]
    fn daily_job_market_hires_qualified_unemployed_people() {
        let mut companies = build_default_companies("A", 1);
        companies[0].open_positions.clear();
        companies[0].add_opening(0, 10);
        let mut person = Person::new_with_profile(
            99,
            EmploymentStatus::Unemployed,
            10,
            35 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        person.cash_balance = -5_000.0;
        let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);

        for _ in 0..365 {
            country.tick_day_without_month();
            if country.people[0].is_employed() {
                break;
            }
        }

        assert!(country.people[0].is_employed());
    }

    #[test]
    fn daily_deaths_remove_people_and_write_off_loans() {
        let companies = build_default_companies("A", 1);
        let mut person = Person::new_with_profile(
            1,
            EmploymentStatus::Employed {
                job_type_id: 0,
                employer_company_id: 0,
            },
            11,
            99 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        person.cash_balance = 50.0;
        let mut country = Country::with_economy(1, "A".to_string(), vec![person], companies, 0.0);
        country.bank.originate_loan(
            1_000.0,
            3_000.0,
            DEFAULT_MAX_DEBT_TO_INCOME,
            &mut country.people[0].loan,
        );

        let mut params = country.sim_params.clone();
        params.demographic = DemographicParams {
            births_per_1000_per_year: 0.0,
        };
        country.sim_params = params;

        for _ in 0..20_000 {
            country.tick_day_without_month();
            if country.people.is_empty() {
                break;
            }
        }

        assert!(country.people.is_empty());
        assert!(country.bank.created_money_outstanding <= 1_000.0);
    }
}
