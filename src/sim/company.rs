use super::{
    bank::LoanAccount,
    good::{GoodId, common_goods, good_by_id},
    job::{JobTypeId, common_job_types, job_by_id},
    money::round_cents,
    params::{GoodsMarketParams, SimulationParams},
};

pub const DEFAULT_COMPANIES_PER_COUNTRY: u16 = 18;
const COMPANY_HISTORY_WINDOW: usize = 12;

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
    pub produced_good_id: GoodId,
    pub open_positions: Vec<JobOpening>,
    pub employee_count: u32,
    pub manufacturing_employee_count: u32,
    pub inventory_units: u32,
    pub unit_price: f64,
    pub production_effort_carry: f64,
    pub cash_balance: f64,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub last_month_sales: f64,
    pub last_month_profit: f64,
    pub consecutive_profitable_months: u8,
    pub last_month_loan_payment: f64,
    pub last_month_new_loan: f64,
    pub last_month_units_produced: u32,
    pub last_month_units_sold: u32,
    pub last_month_export_units_sold: u32,
    pub last_month_domestic_units_sold: u32,
    pub last_month_sell_through: f64,
    pub sales_history: Vec<f64>,
    pub sold_units_history: Vec<u32>,
    pub employee_count_history: Vec<u32>,
    pub price_history: Vec<f64>,
    pub loan_payment_history: Vec<f64>,
    pub loan: Option<LoanAccount>,
}

impl Company {
    pub fn new(
        id: u16,
        name: String,
        offered_job_types: Vec<JobTypeId>,
        produced_good_id: GoodId,
    ) -> Self {
        let default_price = good_by_id(produced_good_id)
            .map(|good| round_cents(good.base_unit_cost * 1.2))
            .unwrap_or(0.0);

        Self {
            id,
            name,
            offered_job_types,
            produced_good_id,
            open_positions: Vec::new(),
            employee_count: 0,
            manufacturing_employee_count: 0,
            inventory_units: 0,
            unit_price: default_price,
            production_effort_carry: 0.0,
            cash_balance: 0.0,
            monthly_payroll_budget: 0.0,
            last_month_payroll_paid: 0.0,
            last_month_sales: 0.0,
            last_month_profit: 0.0,
            consecutive_profitable_months: 0,
            last_month_loan_payment: 0.0,
            last_month_new_loan: 0.0,
            last_month_units_produced: 0,
            last_month_units_sold: 0,
            last_month_export_units_sold: 0,
            last_month_domestic_units_sold: 0,
            last_month_sell_through: 0.0,
            sales_history: Vec::with_capacity(3),
            sold_units_history: Vec::with_capacity(COMPANY_HISTORY_WINDOW),
            employee_count_history: Vec::with_capacity(COMPANY_HISTORY_WINDOW),
            price_history: Vec::with_capacity(COMPANY_HISTORY_WINDOW),
            loan_payment_history: Vec::with_capacity(COMPANY_HISTORY_WINDOW),
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
        self.last_month_units_produced = 0;
        self.last_month_units_sold = 0;
        self.last_month_export_units_sold = 0;
        self.last_month_domestic_units_sold = 0;
        self.last_month_sell_through = 0.0;
    }

    pub fn close_month(&mut self, market: &GoodsMarketParams) {
        let available_units = self
            .inventory_units
            .saturating_add(self.last_month_units_sold)
            .max(1);
        self.last_month_sell_through =
            self.last_month_units_sold as f64 / f64::from(available_units);

        if let Some(good) = good_by_id(self.produced_good_id) {
            let mut next_price = self.unit_price;
            if self.last_month_sell_through >= market.sell_through_raise_threshold {
                next_price *= 1.0 + market.price_raise_rate;
            } else if self.last_month_sell_through <= market.sell_through_cut_threshold {
                next_price *= 1.0 - market.price_cut_rate;
            }

            let min_price = good.base_unit_cost * (1.0 + market.min_margin_rate);
            let max_price = good.base_unit_cost * market.max_markup_multiple;
            self.unit_price = round_cents(next_price.clamp(min_price, max_price));
        }

        if self.sales_history.len() == 3 {
            self.sales_history.remove(0);
        }
        self.sales_history.push(round_cents(self.last_month_sales));
        Self::push_history_u32(
            &mut self.sold_units_history,
            self.last_month_units_sold,
            COMPANY_HISTORY_WINDOW,
        );
        Self::push_history_u32(
            &mut self.employee_count_history,
            self.employee_count,
            COMPANY_HISTORY_WINDOW,
        );
        Self::push_history_f64(
            &mut self.price_history,
            round_cents(self.unit_price),
            COMPANY_HISTORY_WINDOW,
        );
        Self::push_history_f64(
            &mut self.loan_payment_history,
            round_cents(self.last_month_loan_payment),
            COMPANY_HISTORY_WINDOW,
        );
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

    fn push_history_f64(history: &mut Vec<f64>, value: f64, max_len: usize) {
        if history.len() == max_len {
            history.remove(0);
        }
        history.push(value);
    }

    fn push_history_u32(history: &mut Vec<u32>, value: u32, max_len: usize) {
        if history.len() == max_len {
            history.remove(0);
        }
        history.push(value);
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

fn company_good_for_index(company_index: usize) -> GoodId {
    let goods = common_goods();
    if company_index < goods.len() {
        return goods[company_index].id;
    }

    const EXTRA_GOOD_ROTATION: [GoodId; 9] = [1, 0, 3, 7, 8, 14, 5, 4, 6];
    let offset = company_index - goods.len();
    EXTRA_GOOD_ROTATION[offset % EXTRA_GOOD_ROTATION.len()]
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
                company_good_for_index(idx as usize),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_monthly_revenue_uses_available_history() {
        let mut company = Company::new(1, "Test".to_string(), vec![0, 1, 2], 1);
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
    fn default_companies_cover_goods_catalog_with_default_count() {
        let companies = build_default_companies("A", DEFAULT_COMPANIES_PER_COUNTRY);
        let mut goods = companies
            .iter()
            .map(|company| company.produced_good_id)
            .collect::<Vec<_>>();
        goods.sort_unstable();
        goods.dedup();

        assert_eq!(companies.len(), DEFAULT_COMPANIES_PER_COUNTRY as usize);
        assert_eq!(goods.len(), 15);
    }

    #[test]
    fn growth_openings_require_profitability_and_reserve() {
        let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 2);
        company.consecutive_profitable_months = 3;
        company.cash_balance = 200_000.0;
        company.monthly_payroll_budget = 50_000.0;
        company.last_month_profit = 180_000.0;

        let created = company.open_positions_from_growth(&SimulationParams::default(), 12);

        assert!(created > 0);
        assert!(!company.open_positions.is_empty());
    }

    #[test]
    fn price_cut_does_not_drop_below_min_margin_floor() {
        let market = GoodsMarketParams::default();
        let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 0);
        company.unit_price = 6.3;
        company.inventory_units = 10;
        company.last_month_units_sold = 0;

        company.close_month(&market);

        assert!((company.unit_price - 6.3).abs() < 0.01);
    }

    #[test]
    fn price_raise_is_clamped_to_max_markup() {
        let market = GoodsMarketParams::default();
        let mut company = Company::new(1, "T".to_string(), vec![0, 1, 2], 0);
        company.unit_price = 23.9;
        company.inventory_units = 0;
        company.last_month_units_sold = 100;

        company.close_month(&market);

        assert!((company.unit_price - 24.0).abs() < 0.01);
    }
}
