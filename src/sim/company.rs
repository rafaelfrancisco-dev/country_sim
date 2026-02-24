use super::{
    bank::LoanAccount,
    job::{JobTypeId, common_job_types},
    money::round_cents,
};

pub const DEFAULT_COMPANIES_PER_COUNTRY: u16 = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub id: u16,
    pub name: String,
    pub offered_job_types: Vec<JobTypeId>,
    pub employee_count: u32,
    pub cash_balance: f64,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub last_month_sales: f64,
    pub last_month_loan_payment: f64,
    pub last_month_new_loan: f64,
    pub sales_history: Vec<f64>,
    pub loan: Option<LoanAccount>,
}

impl Company {
    pub fn new(id: u16, name: String, offered_job_types: Vec<JobTypeId>) -> Self {
        Self {
            id,
            name,
            offered_job_types,
            employee_count: 0,
            cash_balance: 0.0,
            monthly_payroll_budget: 0.0,
            last_month_payroll_paid: 0.0,
            last_month_sales: 0.0,
            last_month_loan_payment: 0.0,
            last_month_new_loan: 0.0,
            sales_history: Vec::with_capacity(3),
            loan: None,
        }
    }

    pub fn supports_job(&self, job_id: JobTypeId) -> bool {
        self.offered_job_types.contains(&job_id)
    }

    pub fn average_monthly_revenue(&self) -> f64 {
        if self.sales_history.is_empty() {
            return 0.0;
        }

        let total: f64 = self.sales_history.iter().sum();
        round_cents(total / self.sales_history.len() as f64)
    }

    pub fn open_month(&mut self) {
        self.monthly_payroll_budget = 0.0;
        self.last_month_payroll_paid = 0.0;
        self.last_month_sales = 0.0;
        self.last_month_loan_payment = 0.0;
        self.last_month_new_loan = 0.0;
    }

    pub fn close_month(&mut self) {
        if self.sales_history.len() == 3 {
            self.sales_history.remove(0);
        }
        self.sales_history.push(round_cents(self.last_month_sales));
    }
}

pub fn build_default_companies(country_name: &str, count: u16) -> Vec<Company> {
    let available_job_ids = common_job_types()
        .iter()
        .map(|job| job.id)
        .collect::<Vec<_>>();

    (0..count)
        .map(|idx| {
            Company::new(
                idx,
                format!("{country_name} Co {}", idx + 1),
                available_job_ids.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_monthly_revenue_uses_available_history() {
        let mut company = Company::new(1, "Test".to_string(), vec![0, 1, 2]);
        company.sales_history = vec![10.0, 20.0, 40.0];

        assert!((company.average_monthly_revenue() - 23.33).abs() < 0.01);
    }

    #[test]
    fn default_companies_offer_multiple_job_types() {
        let companies = build_default_companies("A", 2);

        assert_eq!(companies.len(), 2);
        assert!(
            companies
                .iter()
                .all(|company| company.offered_job_types.len() > 1)
        );
    }
}
