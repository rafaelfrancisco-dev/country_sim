use super::{
    bank::LoanAccount,
    job::{JobTypeId, common_job_types, job_by_id},
    money::round_cents,
    params::SimulationParams,
};

pub const DEFAULT_COMPANIES_PER_COUNTRY: u16 = 12;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct JobOpening {
    pub job_type_id: JobTypeId,
    pub slots: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub id: u16,
    pub name: String,
    pub offered_job_types: Vec<JobTypeId>,
    pub open_positions: Vec<JobOpening>,
    pub employee_count: u32,
    pub cash_balance: f64,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub last_month_sales: f64,
    pub last_month_profit: f64,
    pub consecutive_profitable_months: u8,
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
            open_positions: Vec::new(),
            employee_count: 0,
            cash_balance: 0.0,
            monthly_payroll_budget: 0.0,
            last_month_payroll_paid: 0.0,
            last_month_sales: 0.0,
            last_month_profit: 0.0,
            consecutive_profitable_months: 0,
            last_month_loan_payment: 0.0,
            last_month_new_loan: 0.0,
            sales_history: Vec::with_capacity(3),
            loan: None,
        }
    }

    pub fn supports_job(&self, job_id: JobTypeId) -> bool {
        self.offered_job_types.contains(&job_id)
    }

    pub fn add_opening(&mut self, job_type_id: JobTypeId, slots: u16) {
        if slots == 0 || !self.supports_job(job_type_id) {
            return;
        }

        if let Some(opening) = self
            .open_positions
            .iter_mut()
            .find(|opening| opening.job_type_id == job_type_id)
        {
            opening.slots = opening.slots.saturating_add(slots);
            return;
        }

        self.open_positions.push(JobOpening { job_type_id, slots });
    }

    pub fn consume_opening(&mut self, opening_index: usize) -> Option<JobTypeId> {
        let opening = self.open_positions.get_mut(opening_index)?;
        if opening.slots == 0 {
            return None;
        }

        opening.slots -= 1;
        let job_type_id = opening.job_type_id;
        if opening.slots == 0 {
            self.open_positions.remove(opening_index);
        }

        Some(job_type_id)
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

    pub fn recompute_profitability(&mut self) {
        self.last_month_profit = round_cents(
            self.last_month_sales - self.last_month_payroll_paid - self.last_month_loan_payment,
        );
        if self.last_month_profit > 0.0 {
            self.consecutive_profitable_months =
                self.consecutive_profitable_months.saturating_add(1);
        } else {
            self.consecutive_profitable_months = 0;
        }
    }

    pub fn can_expand_hiring(&self, params: &SimulationParams, payroll_reference: f64) -> bool {
        let required_cash = params
            .company_growth
            .min_cash_reserve
            .max(payroll_reference * params.company_growth.payroll_reserve_multiplier);
        self.consecutive_profitable_months >= params.company_growth.min_profitable_months
            && self.cash_balance >= required_cash
    }

    pub fn open_positions_from_growth(
        &mut self,
        params: &SimulationParams,
        month_seed: u64,
    ) -> u16 {
        if !self.can_expand_hiring(params, self.monthly_payroll_budget) {
            return 0;
        }

        let growth_step = params.company_growth.profit_per_opening.max(1.0);
        let openings_to_add = ((self.last_month_profit / growth_step).floor() as i64 + 1)
            .clamp(1, i64::from(params.company_growth.max_monthly_openings))
            as u16;

        let weighted_jobs = self
            .offered_job_types
            .iter()
            .filter_map(|job_id| job_by_id(*job_id).map(|job| (job.id, job.assignment_weight)))
            .collect::<Vec<_>>();

        if weighted_jobs.is_empty() {
            return 0;
        }

        for idx in 0..openings_to_add {
            let seed = month_seed ^ (u64::from(self.id) << 32) ^ u64::from(idx);
            if let Some(job_id) = pick_weighted_job(seed, &weighted_jobs) {
                self.add_opening(job_id, 1);
            }
        }

        openings_to_add
    }
}

fn pick_weighted_job(seed: u64, weighted_jobs: &[(JobTypeId, u16)]) -> Option<JobTypeId> {
    let total_weight: u64 = weighted_jobs
        .iter()
        .map(|(_, weight)| u64::from(*weight))
        .sum();
    if total_weight == 0 {
        return weighted_jobs.first().map(|(job_id, _)| *job_id);
    }

    let mut roll = hash64(seed) % total_weight;
    for (job_id, weight) in weighted_jobs {
        let weight = u64::from(*weight);
        if roll < weight {
            return Some(*job_id);
        }
        roll -= weight;
    }

    weighted_jobs.last().map(|(job_id, _)| *job_id)
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
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

    #[test]
    fn growth_openings_require_profitability_and_reserve() {
        let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2]);
        company.consecutive_profitable_months = 3;
        company.cash_balance = 200_000.0;
        company.monthly_payroll_budget = 50_000.0;
        company.last_month_profit = 180_000.0;

        let created = company.open_positions_from_growth(&SimulationParams::default(), 12);

        assert!(created > 0);
        assert!(!company.open_positions.is_empty());
    }
}
