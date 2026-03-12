use super::*;
use crate::sim::params::{CompanyGrowthParams, LaborDemandParams};

const RECENT_DEMAND_SELL_THROUGH_FLOOR: f64 = 0.60;
const RECENT_DEMAND_INVENTORY_COVER_MONTHS: f64 = 2.0;
const HIRING_CASH_RUNWAY_MONTHS: f64 = 2.0;

impl Company {
    pub fn can_expand_hiring(&self, params: &CompanyGrowthParams, payroll_reference: f64) -> bool {
        let required_cash = params
            .min_cash_reserve
            .max(payroll_reference * params.payroll_reserve_multiplier);
        self.consecutive_profitable_months >= params.min_profitable_months
            && self.cash_balance >= required_cash
    }

    pub fn average_profit_over_months(&self, months: usize) -> f64 {
        if months == 0 || self.profit_history.len() < months {
            return 0.0;
        }

        let total: f64 = self.profit_history.iter().rev().take(months).sum();
        round_cents(total / months as f64)
    }

    pub fn has_recent_realized_demand(&self) -> bool {
        if self.last_month_sales > 0.0
            || self.last_completed_month_sales > 0.0
            || self.last_month_units_sold > 0
            || self.last_completed_month_units_sold > 0
            || self.last_month_rent_revenue > 0.0
            || self.sales_history.iter().rev().take(3).any(|sales| *sales > 0.0)
            || self.sold_units_history.iter().rev().take(3).any(|units| *units > 0)
        {
            return true;
        }

        let recent_units_sold = self
            .last_month_units_sold
            .max(self.last_completed_month_units_sold);
        if recent_units_sold == 0 {
            return false;
        }

        let inventory_cover = self.inventory_units as f64 / f64::from(recent_units_sold);
        self.last_month_sell_through >= RECENT_DEMAND_SELL_THROUGH_FLOOR
            || self.last_completed_month_sell_through >= RECENT_DEMAND_SELL_THROUGH_FLOOR
            || inventory_cover <= RECENT_DEMAND_INVENTORY_COVER_MONTHS
    }

    fn has_hiring_financial_support(&self) -> bool {
        let payroll_reference = self
            .monthly_payroll_budget
            .max(self.last_month_payroll_paid)
            .max(self.last_completed_month_payroll_paid);
        let has_runway =
            payroll_reference > 0.0 && self.cash_balance >= payroll_reference * HIRING_CASH_RUNWAY_MONTHS;

        self.last_month_profit > 0.0
            || self.average_profit_over_months(3) > 0.0
            || (has_runway
                && self.has_recent_realized_demand()
                && self.last_month_operating_surplus() > 0.0)
    }

    fn pick_capacity_biased_job(
        &self,
        idx: u16,
        seed: u64,
        production_job_id: Option<JobTypeId>,
        support_jobs: &[(JobTypeId, u16)],
    ) -> Option<JobTypeId> {
        if let Some(production_job_id) = production_job_id
            && (support_jobs.is_empty() || idx % 4 != 3)
        {
            return Some(production_job_id);
        }

        pick_weighted_job(seed, support_jobs).or(production_job_id)
    }

    pub fn open_positions_from_growth(
        &mut self,
        params: &CompanyGrowthParams,
        month_seed: u64,
    ) -> u16 {
        if !self.can_expand_hiring(params, self.monthly_payroll_budget)
            || !self.has_recent_realized_demand()
        {
            return 0;
        }

        let growth_step = params.profit_per_opening.max(1.0);
        let openings_to_add = (self.last_month_profit / growth_step)
            .floor()
            .clamp(0.0, f64::from(params.max_monthly_openings)) as u16;
        if openings_to_add == 0 {
            return 0;
        }

        let production_job_id = production_job_for_good(self.produced_good_id)
            .filter(|job_id| self.supports_job(*job_id));
        let support_jobs = self
            .offered_job_types
            .iter()
            .copied()
            .filter(|job_id| Some(*job_id) != production_job_id)
            .filter_map(|job_id| job_by_id(job_id).map(|job| (job.id, job.assignment_weight)))
            .collect::<Vec<_>>();
        if production_job_id.is_none() && support_jobs.is_empty() {
            return 0;
        }

        for idx in 0..openings_to_add {
            let seed = month_seed ^ (u64::from(self.id) << 32) ^ u64::from(idx);
            if let Some(job_id) =
                self.pick_capacity_biased_job(idx, seed, production_job_id, &support_jobs)
            {
                self.add_opening(job_id, 1);
            }
        }

        openings_to_add
    }

    pub fn ensure_baseline_openings(&mut self, demand: LaborDemandParams, month_seed: u64) -> u16 {
        if self.employee_count == 0
            || !self.has_hiring_financial_support()
            || !self.has_recent_realized_demand()
        {
            return 0;
        }

        let target_from_headcount = (self.employee_count as f64 * demand.replacement_opening_rate)
            .ceil()
            .max(0.0) as u16;
        let target_openings = target_from_headcount.max(demand.min_openings_per_active_company);
        if target_openings == 0 {
            return 0;
        }

        let existing_slots: u16 = self.open_positions.iter().map(|opening| opening.slots).sum();
        if existing_slots >= target_openings {
            return 0;
        }

        let slots_to_add = target_openings - existing_slots;
        let production_job_id = production_job_for_good(self.produced_good_id)
            .filter(|job_id| self.supports_job(*job_id));
        let support_jobs = self
            .offered_job_types
            .iter()
            .copied()
            .filter(|job_id| Some(*job_id) != production_job_id)
            .filter_map(|job_id| job_by_id(job_id).map(|job| (job.id, job.assignment_weight)))
            .collect::<Vec<_>>();
        if production_job_id.is_none() && support_jobs.is_empty() {
            return 0;
        }

        for idx in 0..slots_to_add {
            let seed = month_seed ^ (u64::from(self.id) << 32) ^ u64::from(idx) ^ 0x44aa_11cc;
            if let Some(job_id) =
                self.pick_capacity_biased_job(idx, seed, production_job_id, &support_jobs)
            {
                self.add_opening(job_id, 1);
            }
        }

        slots_to_add
    }
}
