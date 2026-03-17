use std::{collections::VecDeque, sync::Arc};

use super::{
    bank::{BankId, LoanAccount},
    good::{
        CompanyInputRecipe, GOODS_COUNT, GoodId, good_by_id, industry_job_types_for_good,
        production_job_for_good, recipe_for_good,
    },
    job::{JobTypeId, job_by_id, salary_for_job},
    money::round_cents,
    params::GoodsMarketParams,
};

pub const DEFAULT_COMPANIES_PER_COUNTRY: u16 = 18;
const COMPANY_HISTORY_WINDOW: usize = 12;
const PROFIT_HISTORY_WINDOW: usize = 12;
const DIRECT_LABOR_OVERHEAD_MULTIPLIER: f64 = 2.5;

pub fn minimum_viable_unit_price(
    good_id: GoodId,
    worker_effort_per_month: f64,
    min_margin_rate: f64,
) -> f64 {
    let Some(good) = good_by_id(good_id) else {
        return 0.0;
    };

    let direct_labor_cost = production_job_for_good(good_id)
        .filter(|_| worker_effort_per_month > 0.0 && good.effort_per_unit > 0.0)
        .map(|job_id| salary_for_job(job_id) * (good.effort_per_unit / worker_effort_per_month))
        .unwrap_or(0.0);

    let input_recipe_cost: f64 = recipe_for_good(good_id)
        .inputs()
        .map(|(input_id, qty)| {
            minimum_viable_unit_price(input_id, worker_effort_per_month, min_margin_rate)
                * f64::from(qty)
        })
        .sum();

    let baseline_cost = good.base_unit_cost.max(input_recipe_cost)
        + direct_labor_cost.max(0.0) * DIRECT_LABOR_OVERHEAD_MULTIPLIER;

    round_cents(baseline_cost * (1.0 + min_margin_rate.max(0.0)))
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct JobOpening {
    pub job_type_id: JobTypeId,
    pub slots: u16,
    pub offered_monthly_salary: f64,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FounderStake {
    pub person_id: u32,
    pub shares_owned: u64,
    pub cash_contributed: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub id: u16,
    pub historical_ids: Vec<u16>,
    pub name: Arc<str>,
    pub is_public_utility: bool,
    pub is_essential_backfill: bool,
    pub offered_job_types: Vec<JobTypeId>,
    pub produced_good_id: GoodId,
    pub open_positions: Vec<JobOpening>,
    pub founder_stakes: Vec<FounderStake>,
    pub employee_count: u32,
    pub manufacturing_employee_count: u32,
    pub inventory_units: u32,
    pub input_inventory_units: [u32; GOODS_COUNT],
    pub facility_housing_units: u32,
    pub unit_price: f64,
    pub wage_offer_multiplier: f64,
    pub consecutive_missed_loan_payments: u8,
    pub missed_loan_payment_this_month: bool,
    pub production_effort_carry: f64,
    pub cash_balance: f64,
    pub deposit_bank_id: Option<BankId>,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub last_completed_month_payroll_paid: f64,
    pub last_month_sales: f64,
    pub last_month_input_spending: f64,
    pub last_completed_month_input_spending: f64,
    pub last_month_facility_spending: f64,
    pub last_completed_month_facility_spending: f64,
    pub last_month_profit: f64,
    pub last_completed_month_profit: f64,
    pub consecutive_profitable_months: u8,
    pub consecutive_loss_months: u8,
    pub last_month_loan_payment: f64,
    pub last_completed_month_loan_payment: f64,
    pub last_month_new_loan: f64,
    pub last_completed_month_new_loan: f64,
    pub last_month_savings_interest: f64,
    pub last_completed_month_savings_interest: f64,
    pub last_month_units_produced: u32,
    pub last_month_units_sold: u32,
    pub last_month_export_units_sold: u32,
    pub last_month_domestic_units_sold: u32,
    pub last_month_sell_through: f64,
    pub last_completed_month_sales: f64,
    pub last_completed_month_units_produced: u32,
    pub last_completed_month_units_sold: u32,
    pub last_completed_month_export_units_sold: u32,
    pub last_completed_month_domestic_units_sold: u32,
    pub last_completed_month_sell_through: f64,
    pub sales_history: VecDeque<f64>,
    pub profit_history: VecDeque<f64>,
    pub sold_units_history: VecDeque<u32>,
    pub employee_count_history: VecDeque<u32>,
    pub price_history: VecDeque<f64>,
    pub loan_payment_history: VecDeque<f64>,
    pub cash_balance_history: VecDeque<f64>,
    pub age_months: u16,
    pub ipo_cooldown_months: u8,
    pub failed_ipo_attempts: u8,
    pub loan: Option<LoanAccount>,
    /// Number of housing units this company owns and rents out (housing companies only).
    pub rental_units_owned: u32,
    pub last_month_rent_revenue: f64,
}

impl Company {
    pub fn new(
        id: u16,
        name: String,
        offered_job_types: Vec<JobTypeId>,
        produced_good_id: GoodId,
    ) -> Self {
        let market = GoodsMarketParams::default();
        let default_price =
            minimum_viable_unit_price(produced_good_id, market.worker_effort_per_month, 0.20);
        let offered_job_types = if offered_job_types.is_empty() {
            industry_job_types_for_good(produced_good_id).to_vec()
        } else {
            offered_job_types
        };

        Self {
            id,
            historical_ids: Vec::new(),
            name: name.into(),
            is_public_utility: false,
            is_essential_backfill: false,
            offered_job_types,
            produced_good_id,
            open_positions: Vec::new(),
            founder_stakes: Vec::new(),
            employee_count: 0,
            manufacturing_employee_count: 0,
            inventory_units: 0,
            input_inventory_units: [0; GOODS_COUNT],
            facility_housing_units: 0,
            unit_price: default_price,
            wage_offer_multiplier: 1.0,
            consecutive_missed_loan_payments: 0,
            missed_loan_payment_this_month: false,
            production_effort_carry: 0.0,
            cash_balance: 0.0,
            deposit_bank_id: None,
            monthly_payroll_budget: 0.0,
            last_month_payroll_paid: 0.0,
            last_completed_month_payroll_paid: 0.0,
            last_month_sales: 0.0,
            last_month_input_spending: 0.0,
            last_completed_month_input_spending: 0.0,
            last_month_facility_spending: 0.0,
            last_completed_month_facility_spending: 0.0,
            last_month_profit: 0.0,
            last_completed_month_profit: 0.0,
            consecutive_profitable_months: 0,
            consecutive_loss_months: 0,
            last_month_loan_payment: 0.0,
            last_completed_month_loan_payment: 0.0,
            last_month_new_loan: 0.0,
            last_completed_month_new_loan: 0.0,
            last_month_savings_interest: 0.0,
            last_completed_month_savings_interest: 0.0,
            last_month_units_produced: 0,
            last_month_units_sold: 0,
            last_month_export_units_sold: 0,
            last_month_domestic_units_sold: 0,
            last_month_sell_through: 0.0,
            last_completed_month_sales: 0.0,
            last_completed_month_units_produced: 0,
            last_completed_month_units_sold: 0,
            last_completed_month_export_units_sold: 0,
            last_completed_month_domestic_units_sold: 0,
            last_completed_month_sell_through: 0.0,
            sales_history: VecDeque::with_capacity(3),
            profit_history: VecDeque::with_capacity(PROFIT_HISTORY_WINDOW),
            sold_units_history: VecDeque::with_capacity(COMPANY_HISTORY_WINDOW),
            employee_count_history: VecDeque::with_capacity(COMPANY_HISTORY_WINDOW),
            price_history: VecDeque::with_capacity(COMPANY_HISTORY_WINDOW),
            loan_payment_history: VecDeque::with_capacity(COMPANY_HISTORY_WINDOW),
            cash_balance_history: VecDeque::with_capacity(COMPANY_HISTORY_WINDOW),
            age_months: 0,
            ipo_cooldown_months: 0,
            failed_ipo_attempts: 0,
            loan: None,
            rental_units_owned: 0,
            last_month_rent_revenue: 0.0,
        }
    }

    pub fn supports_job(&self, job_id: JobTypeId) -> bool {
        self.offered_job_types.contains(&job_id)
    }

    pub fn industry_job_types(&self) -> &'static [JobTypeId] {
        industry_job_types_for_good(self.produced_good_id)
    }

    pub fn recipe_for_output_good(&self) -> &'static CompanyInputRecipe {
        recipe_for_good(self.produced_good_id)
    }

    pub fn input_inventory_units(&self, good_id: GoodId) -> u32 {
        self.input_inventory_units
            .get(good_id as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn add_input_inventory(&mut self, good_id: GoodId, units: u32) {
        if let Some(inventory) = self.input_inventory_units.get_mut(good_id as usize) {
            *inventory = inventory.saturating_add(units);
        }
    }

    pub fn can_produce_one_unit(&self) -> bool {
        self.recipe_for_output_good()
            .inputs()
            .all(|(good_id, required)| self.input_inventory_units(good_id) >= u32::from(required))
    }

    pub fn consume_inputs_for_unit(&mut self) -> bool {
        if !self.can_produce_one_unit() {
            return false;
        }

        for (good_id, required) in self.recipe_for_output_good().inputs() {
            if let Some(inventory) = self.input_inventory_units.get_mut(good_id as usize) {
                *inventory = inventory.saturating_sub(u32::from(required));
            }
        }
        true
    }

    pub fn target_facility_housing_units(&self) -> u32 {
        if self.employee_count == 0 {
            0
        } else {
            self.employee_count.div_ceil(12).max(1)
        }
    }

    pub fn remember_historical_id(&mut self, historical_id: u16) {
        if historical_id == self.id || self.historical_ids.contains(&historical_id) {
            return;
        }
        self.historical_ids.push(historical_id);
    }

    pub fn matches_event_company_id(&self, company_id: u16) -> bool {
        self.id == company_id || self.historical_ids.contains(&company_id)
    }

    pub fn add_opening(&mut self, job_type_id: JobTypeId, slots: u16) {
        let salary = round_cents(salary_for_job(job_type_id) * self.wage_offer_multiplier.max(0.0));
        self.add_opening_with_salary(job_type_id, slots, salary);
    }

    pub fn add_opening_with_salary(
        &mut self,
        job_type_id: JobTypeId,
        slots: u16,
        offered_salary: f64,
    ) {
        if slots == 0 {
            return;
        }
        if !self.supports_job(job_type_id) {
            self.offered_job_types.push(job_type_id);
        }

        let offered_salary = round_cents(offered_salary.max(0.0));
        if let Some(opening) = self.open_positions.iter_mut().find(|opening| {
            opening.job_type_id == job_type_id
                && (opening.offered_monthly_salary - offered_salary).abs() < 0.01
        }) {
            opening.slots = opening.slots.saturating_add(slots);
            return;
        }

        self.open_positions.push(JobOpening {
            job_type_id,
            slots,
            offered_monthly_salary: offered_salary,
        });
    }

    pub fn consume_opening(&mut self, opening_index: usize) -> Option<(JobTypeId, f64)> {
        let opening = self.open_positions.get_mut(opening_index)?;
        if opening.slots == 0 {
            return None;
        }

        opening.slots -= 1;
        let job_type_id = opening.job_type_id;
        let offered_salary = opening.offered_monthly_salary;
        if opening.slots == 0 {
            self.open_positions.remove(opening_index);
        }

        Some((job_type_id, offered_salary))
    }

    pub fn average_monthly_revenue(&self) -> f64 {
        let average_sales = if self.sales_history.is_empty() {
            0.0
        } else {
            let total: f64 = self.sales_history.iter().sum();
            round_cents(total / self.sales_history.len() as f64)
        };

        if self.last_month_rent_revenue > 0.0 {
            round_cents(average_sales + self.last_month_rent_revenue)
        } else {
            average_sales
        }
    }

    pub fn last_month_total_revenue(&self) -> f64 {
        round_cents(self.last_month_sales + self.last_month_rent_revenue)
    }

    pub fn last_month_operating_surplus(&self) -> f64 {
        round_cents(
            self.last_month_total_revenue()
                - self.last_month_payroll_paid
                - self.last_month_input_spending
                - self.last_month_facility_spending,
        )
    }

    pub fn open_month(&mut self) {
        self.monthly_payroll_budget = 0.0;
        self.last_completed_month_payroll_paid = self.last_month_payroll_paid;
        self.last_month_payroll_paid = 0.0;
        self.last_completed_month_sales = self.last_month_sales;
        self.last_month_sales = 0.0;
        self.last_completed_month_input_spending = self.last_month_input_spending;
        self.last_month_input_spending = 0.0;
        self.last_completed_month_facility_spending = self.last_month_facility_spending;
        self.last_month_facility_spending = 0.0;
        self.last_completed_month_profit = self.last_month_profit;
        self.last_completed_month_loan_payment = self.last_month_loan_payment;
        self.last_completed_month_new_loan = self.last_month_new_loan;
        self.last_completed_month_savings_interest = self.last_month_savings_interest;
        self.last_completed_month_units_produced = self.last_month_units_produced;
        self.last_completed_month_units_sold = self.last_month_units_sold;
        self.last_completed_month_export_units_sold = self.last_month_export_units_sold;
        self.last_completed_month_domestic_units_sold = self.last_month_domestic_units_sold;
        self.last_completed_month_sell_through = self.last_month_sell_through;
        self.last_month_loan_payment = 0.0;
        self.last_month_new_loan = 0.0;
        self.last_month_savings_interest = 0.0;
        self.missed_loan_payment_this_month = false;
        self.last_month_units_produced = 0;
        self.last_month_units_sold = 0;
        self.last_month_export_units_sold = 0;
        self.last_month_domestic_units_sold = 0;
        self.last_month_sell_through = 0.0;
        self.last_month_rent_revenue = 0.0;
    }

    pub fn close_month(&mut self, market: &GoodsMarketParams) {
        self.age_months = self.age_months.saturating_add(1);
        if self.ipo_cooldown_months > 0 {
            self.ipo_cooldown_months -= 1;
        }

        let available_units = self
            .inventory_units
            .saturating_add(self.last_month_units_sold)
            .max(1);
        self.last_month_sell_through =
            self.last_month_units_sold as f64 / f64::from(available_units);

        if good_by_id(self.produced_good_id).is_some() {
            let mut next_price = self.unit_price;
            let selling_at_loss = self.last_month_sales > 0.0 && self.last_month_profit <= 0.0;
            let chronic_loss_seller =
                self.last_month_sales > 0.0 && self.average_profit_over_months(3) < 0.0;
            let loss_recovery_rate = if selling_at_loss && self.last_month_sales > 0.0 {
                let recovery = (-self.last_month_profit / self.last_month_sales).clamp(0.0, 0.15);
                if chronic_loss_seller {
                    (recovery * 1.5).max(market.price_raise_rate).clamp(0.0, 0.25)
                } else {
                    recovery
                }
            } else {
                0.0
            };
            if self.last_month_sell_through >= market.sell_through_raise_threshold {
                next_price *= 1.0 + market.price_raise_rate;
            } else if selling_at_loss {
                next_price *= 1.0 + loss_recovery_rate.max(market.price_raise_rate * 0.5);
            } else if self.last_month_sell_through <= market.sell_through_cut_threshold {
                next_price *= 1.0 - market.price_cut_rate;
            }

            let months_of_inventory = if self.last_month_units_sold > 0 {
                self.inventory_units as f64 / self.last_month_units_sold as f64
            } else if self.inventory_units > 0 {
                f64::INFINITY
            } else {
                0.0
            };
            if months_of_inventory > market.excess_inventory_months && !selling_at_loss {
                next_price *= 1.0 - market.excess_inventory_cut_rate;
            }

            let min_price = minimum_viable_unit_price(
                self.produced_good_id,
                market.worker_effort_per_month,
                market.min_margin_rate,
            );
            self.unit_price = round_cents(next_price.max(min_price));
        }

        if self.sales_history.len() >= 3 {
            self.sales_history.pop_front();
        }
        self.sales_history
            .push_back(round_cents(self.last_month_sales));
        Self::push_history_f64(
            &mut self.profit_history,
            round_cents(self.last_month_profit),
            PROFIT_HISTORY_WINDOW,
        );
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
        Self::push_history_f64(
            &mut self.cash_balance_history,
            round_cents(self.cash_balance),
            COMPANY_HISTORY_WINDOW,
        );
    }

    pub fn recompute_profitability(&mut self) {
        self.last_month_profit = round_cents(
            self.last_month_sales + self.last_month_rent_revenue
                - self.last_month_payroll_paid
                - self.last_month_input_spending
                - self.last_month_facility_spending
                - self.last_month_loan_payment,
        );
        if self.last_month_profit > 0.0 {
            self.consecutive_profitable_months =
                self.consecutive_profitable_months.saturating_add(1);
            self.consecutive_loss_months = 0;
        } else {
            self.consecutive_profitable_months = 0;
            self.consecutive_loss_months = self.consecutive_loss_months.saturating_add(1);
        }
    }

    fn push_history_f64(history: &mut VecDeque<f64>, value: f64, max_len: usize) {
        if history.len() >= max_len {
            history.pop_front();
        }
        history.push_back(value);
    }

    fn push_history_u32(history: &mut VecDeque<u32>, value: u32, max_len: usize) {
        if history.len() >= max_len {
            history.pop_front();
        }
        history.push_back(value);
    }
}
mod hiring;
mod helpers;
pub use helpers::build_default_companies;
use helpers::pick_weighted_job;

#[cfg(test)]
mod tests;
