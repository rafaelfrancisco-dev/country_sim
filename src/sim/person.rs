use super::{
    bank::LoanAccount,
    job::{JobTypeId, salary_for_job},
    tickable::Tickable,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub id: u32,
    pub job_type_id: JobTypeId,
    pub employer_company_id: u16,
    pub cash_balance: f64,
    pub name_seed: u32,
    pub loan: Option<LoanAccount>,
    pub last_month_gross_pay: f64,
    pub last_month_tax_paid: f64,
    pub last_month_net_pay: f64,
    pub last_month_spending: f64,
    pub last_month_loan_payment: f64,
    pub last_month_new_loan: f64,
}

impl Person {
    pub fn new(id: u32, job_type_id: JobTypeId, employer_company_id: u16, name_seed: u32) -> Self {
        Self {
            id,
            job_type_id,
            employer_company_id,
            cash_balance: 0.0,
            name_seed,
            loan: None,
            last_month_gross_pay: 0.0,
            last_month_tax_paid: 0.0,
            last_month_net_pay: 0.0,
            last_month_spending: 0.0,
            last_month_loan_payment: 0.0,
            last_month_new_loan: 0.0,
        }
    }

    pub fn monthly_salary(&self) -> f64 {
        salary_for_job(self.job_type_id)
    }

    pub fn reset_monthly_trackers(&mut self) {
        self.last_month_gross_pay = 0.0;
        self.last_month_tax_paid = 0.0;
        self.last_month_net_pay = 0.0;
        self.last_month_spending = 0.0;
        self.last_month_loan_payment = 0.0;
        self.last_month_new_loan = 0.0;
    }
}

impl Tickable for Person {
    fn tick_day(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monthly_salary_comes_from_job_catalog() {
        let person = Person::new(1, 9, 0, 42);

        assert!((person.monthly_salary() - 8_500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn monthly_trackers_reset_without_touching_cash() {
        let mut person = Person::new(1, 0, 0, 42);
        person.cash_balance = 50.0;
        person.last_month_gross_pay = 10.0;
        person.last_month_tax_paid = 2.0;

        person.reset_monthly_trackers();

        assert!((person.cash_balance - 50.0).abs() < f64::EPSILON);
        assert!((person.last_month_gross_pay - 0.0).abs() < f64::EPSILON);
        assert!((person.last_month_tax_paid - 0.0).abs() < f64::EPSILON);
    }
}
