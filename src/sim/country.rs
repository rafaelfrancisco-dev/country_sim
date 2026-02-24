#[cfg(test)]
use super::company::{DEFAULT_COMPANIES_PER_COUNTRY, build_default_companies};
use super::{
    bank::{Bank, DEFAULT_MAX_DEBT_TO_INCOME},
    company::{Company, JobOpening},
    job::{JobTypeId, job_is_eligible_for_habilitation},
    money::{from_cents, round_cents, to_cents},
    params::SimulationParams,
    person::{EmploymentStatus, HabilitationLevel, Person},
    tickable::Tickable,
};
use rayon::prelude::*;

pub const DAYS_PER_MONTH: u64 = 30;
pub const TAX_BRACKET_1_MAX: f64 = 3_000.0;
pub const TAX_BRACKET_2_MAX: f64 = 7_000.0;
const PEOPLE_CHUNK_SIZE: usize = 2_048;
const COMPANY_CHUNK_SIZE: usize = 16;

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
        let company_count = self.companies.len();
        let payroll_by_chunk_cents = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                let mut local = vec![0_i64; company_count];
                for person in chunk {
                    let Some(company_id) = person.employer_company_id() else {
                        continue;
                    };
                    let company_idx = company_id as usize;
                    if let Some(total) = local.get_mut(company_idx) {
                        *total = total.saturating_add(to_cents(person.monthly_salary()));
                    }
                }
                local
            })
            .collect::<Vec<_>>();

        let mut payroll_requirements_cents = vec![0_i64; company_count];
        for chunk_totals in payroll_by_chunk_cents {
            for (company_idx, cents) in chunk_totals.into_iter().enumerate() {
                if let Some(total) = payroll_requirements_cents.get_mut(company_idx) {
                    *total = total.saturating_add(cents);
                }
            }
        }

        for (company_idx, company) in self.companies.iter_mut().enumerate() {
            let required_payroll = payroll_requirements_cents
                .get(company_idx)
                .copied()
                .map(from_cents)
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

        let spending_intents_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let mut intents = Vec::new();
                let base_idx = chunk_idx.saturating_mul(PEOPLE_CHUNK_SIZE);
                for (offset, person) in chunk.iter().enumerate() {
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
                    let spend_cents = to_cents(person.cash_balance * spend_percent / 100.0);
                    if spend_cents <= 0 {
                        continue;
                    }

                    let seller_idx =
                        (hash64(seed ^ 0x9e37_79b9_7f4a_7c15) as usize) % company_count;
                    intents.push((base_idx + offset, seller_idx, spend_cents));
                }
                intents
            })
            .collect::<Vec<_>>();

        let mut total_spend_cents = 0_i64;
        for chunk_intents in spending_intents_by_chunk {
            for (person_idx, seller_idx, spend_cents) in chunk_intents {
                let spend = from_cents(spend_cents);
                let Some(person) = self.people.get_mut(person_idx) else {
                    continue;
                };
                person.cash_balance = round_cents(person.cash_balance - spend);
                person.last_month_spending = spend;

                if let Some(company) = self.companies.get_mut(seller_idx) {
                    company.cash_balance = round_cents(company.cash_balance + spend);
                    company.last_month_sales = round_cents(company.last_month_sales + spend);
                }

                total_spend_cents = total_spend_cents.saturating_add(spend_cents);
            }
        }

        self.monthly_household_spending = from_cents(total_spend_cents);
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
        let sim_params = self.sim_params.clone();
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.recompute_profitability();
                    company.open_positions_from_growth(&sim_params, month_seed);
                }
            });
    }

    fn close_company_months(&mut self) {
        self.companies
            .par_chunks_mut(COMPANY_CHUNK_SIZE)
            .for_each(|chunk| {
                for company in chunk {
                    company.close_month();
                }
            });
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
        self.people
            .par_chunks_mut(PEOPLE_CHUNK_SIZE)
            .for_each(|chunk| {
                for person in chunk {
                    person.increment_age_one_day();
                }
            });
    }

    fn apply_daily_deaths(&mut self) {
        let demographic = self.sim_params.demographic;
        let day = self.day;
        let country_id = self.id;
        let death_flags_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|person| {
                        let death_probability =
                            demographic.daily_death_probability_for_age_years(person.age_years());
                        let death_seed = hash64(
                            (u64::from(country_id) << 48)
                                ^ (day << 24)
                                ^ u64::from(person.id)
                                ^ u64::from(person.name_seed)
                                ^ 0xdeca_fbadu64,
                        );
                        roll_probability(death_seed, death_probability)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let death_flags = death_flags_by_chunk
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let people = std::mem::take(&mut self.people);
        let mut deaths = 0_u32;
        let mut survivors = Vec::with_capacity(people.len());

        for (mut person, died) in people.into_iter().zip(death_flags.into_iter()) {
            if died {
                deaths = deaths.saturating_add(1);
                if person.cash_balance > 0.0 {
                    self.treasury_balance =
                        round_cents(self.treasury_balance + person.cash_balance);
                }

                if let Some((company_id, job_type_id)) = person.clear_employment() {
                    if let Some(company) = self.companies.get_mut(company_id as usize) {
                        company.add_opening(job_type_id, 1);
                    }
                }

                self.bank.write_off_loan(&mut person.loan);
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
        let company_openings_snapshot = self
            .companies
            .iter()
            .map(|company| company.open_positions.clone())
            .collect::<Vec<_>>();

        let intents_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let mut intents = Vec::new();
                let base_idx = chunk_idx.saturating_mul(PEOPLE_CHUNK_SIZE);
                for (offset, person) in chunk.iter().enumerate() {
                    if person.is_employed() || !person.can_seek_job() {
                        continue;
                    }

                    let seek_probability = labor_params
                        .daily_seek_probability(person.age_years(), person.cash_balance);
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

                    let preferred = select_matching_opening_from_snapshot(
                        &company_openings_snapshot,
                        person.habilitation,
                        seek_seed,
                    );
                    intents.push(JobSeekIntent {
                        person_idx: base_idx + offset,
                        seek_seed,
                        qualification: person.habilitation,
                        preferred,
                    });
                }
                intents
            })
            .collect::<Vec<_>>();

        let (people, companies) = (&mut self.people, &mut self.companies);
        for chunk_intents in intents_by_chunk {
            for intent in chunk_intents {
                let Some(person) = people.get_mut(intent.person_idx) else {
                    continue;
                };
                if person.is_employed() || !person.can_seek_job() {
                    continue;
                }

                if let Some((company_idx, job_type_id)) = intent.preferred
                    && let Some(company) = companies.get_mut(company_idx)
                    && let Some(job_id) = consume_opening_by_job_type(company, job_type_id)
                {
                    person.set_employment(company_idx as u16, job_id);
                    continue;
                }

                if let Some((company_id, job_id)) =
                    consume_matching_opening(companies, intent.qualification, intent.seek_seed)
                {
                    person.set_employment(company_id, job_id);
                }
            }
        }
    }

    fn recompute_company_headcounts(&mut self) {
        let company_count = self.companies.len();
        let counts_by_chunk = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                let mut local = vec![0_u32; company_count];
                for person in chunk {
                    let Some(company_id) = person.employer_company_id() else {
                        continue;
                    };
                    let company_idx = company_id as usize;
                    if let Some(count) = local.get_mut(company_idx) {
                        *count = count.saturating_add(1);
                    }
                }
                local
            })
            .collect::<Vec<_>>();

        for company in &mut self.companies {
            company.employee_count = 0;
        }
        for local_counts in counts_by_chunk {
            for (company_idx, count) in local_counts.into_iter().enumerate() {
                if let Some(company) = self.companies.get_mut(company_idx) {
                    company.employee_count = company.employee_count.saturating_add(count);
                }
            }
        }
    }

    pub fn recompute_aggregates(&mut self) {
        let household_chunks = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|person| to_cents(person.cash_balance))
                    .sum::<i64>()
            })
            .collect::<Vec<_>>();
        let household_cents = household_chunks
            .into_iter()
            .fold(0_i64, |acc, cents| acc.saturating_add(cents));

        let company_chunks = self
            .companies
            .par_chunks(COMPANY_CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|company| to_cents(company.cash_balance))
                    .sum::<i64>()
            })
            .collect::<Vec<_>>();
        let company_cents = company_chunks
            .into_iter()
            .fold(0_i64, |acc, cents| acc.saturating_add(cents));

        self.household_cash_total = from_cents(household_cents);
        self.company_cash_total = from_cents(company_cents);
        let total_money_supply_cents = household_cents
            .saturating_add(company_cents)
            .saturating_add(to_cents(self.treasury_balance))
            .saturating_add(to_cents(self.bank.reserve_balance));
        self.total_money_supply = from_cents(total_money_supply_cents);

        let expected_money_supply_cents = to_cents(self.initial_money_supply)
            .saturating_add(to_cents(self.bank.created_money_outstanding));
        self.expected_money_supply = from_cents(expected_money_supply_cents);
        self.money_supply_gap =
            from_cents(total_money_supply_cents.saturating_sub(expected_money_supply_cents));
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

#[derive(Debug, Copy, Clone)]
struct JobSeekIntent {
    person_idx: usize,
    seek_seed: u64,
    qualification: HabilitationLevel,
    preferred: Option<(usize, JobTypeId)>,
}

fn select_matching_opening_from_snapshot(
    company_openings: &[Vec<JobOpening>],
    qualification: HabilitationLevel,
    seed: u64,
) -> Option<(usize, JobTypeId)> {
    let mut chosen: Option<(usize, JobTypeId, u64)> = None;
    let mut seen_slots = 0_u64;

    for (company_idx, openings) in company_openings.iter().enumerate() {
        for (opening_idx, opening) in openings.iter().enumerate() {
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
                chosen = Some((company_idx, opening.job_type_id, seen_slots));
            }
        }
    }

    let (company_idx, job_type_id, _) = chosen?;
    Some((company_idx, job_type_id))
}

fn consume_opening_by_job_type(company: &mut Company, job_type_id: JobTypeId) -> Option<JobTypeId> {
    let opening_idx = company
        .open_positions
        .iter()
        .position(|opening| opening.slots > 0 && opening.job_type_id == job_type_id)?;
    company.consume_opening(opening_idx)
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
        params::{DemographicParams, LaborParams, SimulationParams},
        person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus},
    };
    use rayon::ThreadPoolBuilder;

    fn build_people(count: usize) -> Vec<Person> {
        (0..count)
            .map(|idx| Person::new(idx as u32, (idx % 10) as u8, (idx % 4) as u16, idx as u32))
            .collect()
    }

    fn build_deterministic_country() -> Country {
        let companies = build_default_companies("Deterministic", 4);
        let mut people = Vec::with_capacity(80);
        for idx in 0..80_u32 {
            let employment = if idx % 3 == 0 {
                EmploymentStatus::Unemployed
            } else {
                EmploymentStatus::Employed {
                    job_type_id: (idx % 10) as u8,
                    employer_company_id: (idx % 4) as u16,
                }
            };
            let mut person = Person::new_with_profile(
                idx,
                employment,
                idx.saturating_mul(13),
                30 * DAYS_PER_YEAR,
                EducationTrack::HighSchool,
            );
            person.cash_balance = if idx % 2 == 0 { 2_000.0 } else { -100.0 };
            people.push(person);
        }
        let mut country =
            Country::with_economy(1, "Deterministic".to_string(), people, companies, 100_000.0);

        let mut params = SimulationParams::default();
        params.demographic.births_per_1000_per_year = 0.0;
        country.sim_params = params;
        country
    }

    fn run_country_with_threads(days: u64, threads: usize) -> Country {
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("thread pool should build");
        pool.install(|| {
            let mut country = build_deterministic_country();
            for _ in 0..days {
                country.tick_day();
            }
            country
        })
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

    #[test]
    fn country_daily_lifecycle_deterministic_across_thread_counts() {
        let one_thread = run_country_with_threads(45, 1);
        let many_threads = run_country_with_threads(45, 8);
        assert_eq!(one_thread, many_threads);
    }

    #[test]
    fn country_monthly_processing_deterministic_across_thread_counts() {
        let one_thread = run_country_with_threads(DAYS_PER_MONTH * 4, 1);
        let many_threads = run_country_with_threads(DAYS_PER_MONTH * 4, 8);
        assert_eq!(one_thread, many_threads);
    }

    #[test]
    fn job_market_conflict_resolution_is_stable_by_person_index() {
        let mut companies = build_default_companies("A", 1);
        companies[0].open_positions.clear();
        companies[0].add_opening(0, 1);

        let first = Person::new_with_profile(
            1,
            EmploymentStatus::Unemployed,
            1,
            35 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        let second = Person::new_with_profile(
            2,
            EmploymentStatus::Unemployed,
            2,
            35 * DAYS_PER_YEAR,
            EducationTrack::BasicOnly,
        );
        let mut country =
            Country::with_economy(1, "A".to_string(), vec![first, second], companies, 0.0);

        country.sim_params = SimulationParams {
            demographic: DemographicParams {
                births_per_1000_per_year: 0.0,
            },
            labor: LaborParams {
                seek_base: 1.0,
                seek_age_slope: 0.0,
                seek_cash_pressure_scale: 0.0,
                seek_cash_reference: 1.0,
                seek_probability_cap: 1.0,
                ..LaborParams::default()
            },
            ..SimulationParams::default()
        };

        country.tick_day_without_month();

        assert!(country.people[0].is_employed());
        assert!(!country.people[1].is_employed());
    }
}
