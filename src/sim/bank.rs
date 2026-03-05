use super::{
    job::{JobTypeId, salary_for_job},
    money::round_cents,
};
use std::collections::HashMap;

mod loan_math;
mod stock_positions;
use loan_math::monthly_interest_rate;
pub use loan_math::{max_principal_for_payment, payment_for};

pub const DEFAULT_LOAN_APR: f64 = 0.12;
pub const DEFAULT_LOAN_TERM_MONTHS: u16 = 12;
pub const DEFAULT_MAX_DEBT_TO_INCOME: f64 = 0.35;
pub const DEFAULT_BANK_NAME: &str = "National Bank";

pub type BankId = u16;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BankSecurityId {
    pub country_id: u16,
    pub company_id: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BankKind {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoanAccount {
    pub debt_balance: f64,
    pub created_principal_outstanding: f64,
    pub monthly_payment: f64,
    pub annual_rate: f64,
    pub term_months: u16,
    pub months_remaining: u16,
    pub lender_bank_id: BankId,
}

impl LoanAccount {
    pub fn new(principal: f64, annual_rate: f64, term_months: u16) -> Self {
        Self::new_with_lender(principal, annual_rate, term_months, 0)
    }

    pub fn new_with_lender(
        principal: f64,
        annual_rate: f64,
        term_months: u16,
        lender_bank_id: BankId,
    ) -> Self {
        let principal = round_cents(principal.max(0.0));
        Self {
            debt_balance: principal,
            created_principal_outstanding: principal,
            monthly_payment: payment_for(principal, annual_rate, term_months),
            annual_rate,
            term_months,
            months_remaining: term_months,
            lender_bank_id,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LoanPayment {
    pub scheduled_due: f64,
    pub amount_paid: f64,
    pub interest_paid: f64,
    pub principal_paid: f64,
    pub unpaid_amount: f64,
    pub loan_ended: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bank {
    pub id: BankId,
    pub name: String,
    pub kind: BankKind,
    pub reserve_balance: f64,
    pub created_money_outstanding: f64,
    pub total_originated: f64,
    pub total_principal_repaid: f64,
    pub deposit_liabilities: f64,
    /// Total deposits held by residents of other countries in this bank.
    pub foreign_deposit_liabilities: f64,
    pub equity_capital: f64,
    pub run_intensity: f64,
    pub is_in_resolution: bool,
    pub months_in_resolution: u16,
    pub resolution_distress_streak: u16,
    pub resolution_stable_streak: u16,
    // --- Employment ---
    pub open_positions: Vec<JobOpening>,
    pub employee_count: u32,
    pub monthly_payroll_budget: f64,
    pub last_month_payroll_paid: f64,
    pub wage_offer_multiplier: f64,
    // --- Profitability diagnostics ---
    pub last_month_interest_income: f64,
    pub last_month_deposit_interest_expense: f64,
    pub last_month_credit_losses: f64,
    pub last_month_profit: f64,
    pub stock_positions: HashMap<BankSecurityId, BankStockPosition>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct JobOpening {
    pub job_type_id: JobTypeId,
    pub slots: u16,
    pub offered_monthly_salary: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BankStockPosition {
    pub security_id: BankSecurityId,
    pub shares: u64,
    pub avg_cost: f64,
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

impl Bank {
    pub fn new() -> Self {
        Self::with_identity(0, DEFAULT_BANK_NAME.to_string(), BankKind::Public)
    }

    pub fn with_identity(id: BankId, name: String, kind: BankKind) -> Self {
        Self {
            id,
            name,
            kind,
            reserve_balance: 0.0,
            created_money_outstanding: 0.0,
            total_originated: 0.0,
            total_principal_repaid: 0.0,
            deposit_liabilities: 0.0,
            foreign_deposit_liabilities: 0.0,
            equity_capital: 0.0,
            run_intensity: 0.0,
            is_in_resolution: false,
            months_in_resolution: 0,
            resolution_distress_streak: 0,
            resolution_stable_streak: 0,
            open_positions: Vec::new(),
            employee_count: 0,
            monthly_payroll_budget: 0.0,
            last_month_payroll_paid: 0.0,
            wage_offer_multiplier: 1.15,
            last_month_interest_income: 0.0,
            last_month_deposit_interest_expense: 0.0,
            last_month_credit_losses: 0.0,
            last_month_profit: 0.0,
            stock_positions: HashMap::new(),
        }
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
        let offered_salary = round_cents(offered_salary.max(0.0));
        if let Some(opening) = self.open_positions.iter_mut().find(|o| {
            o.job_type_id == job_type_id && (o.offered_monthly_salary - offered_salary).abs() < 0.01
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

    pub fn open_position_count(&self) -> u32 {
        self.open_positions.iter().map(|o| u32::from(o.slots)).sum()
    }

    pub fn open_month(&mut self) {
        self.monthly_payroll_budget = 0.0;
        self.last_month_payroll_paid = 0.0;
        self.last_month_interest_income = 0.0;
        self.last_month_deposit_interest_expense = 0.0;
        self.last_month_credit_losses = 0.0;
        self.last_month_profit = 0.0;
    }

    pub fn close_month(&mut self) {
        self.last_month_profit = round_cents(
            self.last_month_interest_income
                - self.last_month_deposit_interest_expense
                - self.last_month_credit_losses
                - self.last_month_payroll_paid,
        );
    }

    pub fn target_employee_count(
        &self,
        deposit_liabilities: f64,
        min_employees: u32,
        max_employees: u32,
        employees_per_deposit_unit: f64,
    ) -> u32 {
        if employees_per_deposit_unit <= 0.0 {
            return min_employees;
        }
        let computed = (deposit_liabilities / employees_per_deposit_unit).ceil() as u32;
        computed.max(min_employees).min(max_employees)
    }

    pub fn originate_loan(
        &mut self,
        desired_principal: f64,
        monthly_income: f64,
        debt_to_income_cap: f64,
        annual_rate: f64,
        loan: &mut Option<LoanAccount>,
    ) -> f64 {
        let desired_principal = round_cents(desired_principal.max(0.0));
        if desired_principal <= 0.0 || monthly_income <= 0.0 {
            return 0.0;
        }

        let max_payment = round_cents(monthly_income * debt_to_income_cap.max(0.0));
        if max_payment <= 0.0 {
            return 0.0;
        }

        let existing_debt = loan.as_ref().map_or(0.0, |existing| existing.debt_balance);
        let max_total_debt =
            max_principal_for_payment(max_payment, annual_rate, DEFAULT_LOAN_TERM_MONTHS);
        let principal_capacity = round_cents((max_total_debt - existing_debt).max(0.0));
        let approved_principal = round_cents(desired_principal.min(principal_capacity));

        if approved_principal <= 0.0 {
            return 0.0;
        }

        match loan {
            Some(existing) => {
                existing.debt_balance = round_cents(existing.debt_balance + approved_principal);
                existing.created_principal_outstanding =
                    round_cents(existing.created_principal_outstanding + approved_principal);
                existing.annual_rate = annual_rate;
                existing.term_months = DEFAULT_LOAN_TERM_MONTHS;
                existing.months_remaining = DEFAULT_LOAN_TERM_MONTHS;
                existing.monthly_payment = payment_for(
                    existing.debt_balance,
                    existing.annual_rate,
                    existing.term_months,
                );
                existing.lender_bank_id = self.id;
            }
            None => {
                *loan = Some(LoanAccount::new_with_lender(
                    approved_principal,
                    annual_rate,
                    DEFAULT_LOAN_TERM_MONTHS,
                    self.id,
                ));
            }
        }

        self.created_money_outstanding =
            round_cents(self.created_money_outstanding + approved_principal);
        self.total_originated = round_cents(self.total_originated + approved_principal);
        approved_principal
    }

    pub fn fund_government_deficit(&mut self, principal: f64) -> f64 {
        let approved_principal = round_cents(principal.max(0.0));
        if approved_principal <= 0.0 {
            return 0.0;
        }

        self.created_money_outstanding =
            round_cents(self.created_money_outstanding + approved_principal);
        self.total_originated = round_cents(self.total_originated + approved_principal);
        approved_principal
    }

    pub fn collect_monthly_payment(
        &mut self,
        cash_balance: &mut f64,
        loan: &mut Option<LoanAccount>,
    ) -> LoanPayment {
        let Some(account) = loan.as_mut() else {
            return LoanPayment::default();
        };

        if account.debt_balance <= 0.0 {
            *loan = None;
            return LoanPayment::default();
        }

        let opening_debt = account.debt_balance;
        let monthly_rate = monthly_interest_rate(account.annual_rate);
        let interest_due = round_cents(opening_debt * monthly_rate);
        account.debt_balance = round_cents(account.debt_balance + interest_due);

        let scheduled_due = round_cents(account.monthly_payment.min(account.debt_balance));
        let available_cash = round_cents(cash_balance.max(0.0));
        let amount_paid = round_cents(scheduled_due.min(available_cash));
        *cash_balance = round_cents(*cash_balance - amount_paid);

        let interest_paid = round_cents(amount_paid.min(interest_due));
        let debt_component_paid = round_cents(amount_paid - interest_paid);
        account.debt_balance = round_cents((account.debt_balance - amount_paid).max(0.0));

        let principal_paid =
            round_cents(debt_component_paid.min(account.created_principal_outstanding));
        account.created_principal_outstanding =
            round_cents((account.created_principal_outstanding - principal_paid).max(0.0));

        self.reserve_balance = round_cents(self.reserve_balance + interest_paid);
        self.last_month_interest_income =
            round_cents(self.last_month_interest_income + interest_paid);
        self.created_money_outstanding =
            round_cents((self.created_money_outstanding - principal_paid).max(0.0));
        self.total_principal_repaid = round_cents(self.total_principal_repaid + principal_paid);

        if account.months_remaining > 0 {
            account.months_remaining -= 1;
        }

        let loan_ended = account.debt_balance <= 0.0;
        if loan_ended {
            *loan = None;
        }

        LoanPayment {
            scheduled_due,
            amount_paid,
            interest_paid,
            principal_paid,
            unpaid_amount: round_cents(scheduled_due - amount_paid),
            loan_ended,
        }
    }

    pub fn write_off_loan(&mut self, loan: &mut Option<LoanAccount>) -> f64 {
        let Some(account) = loan.take() else {
            return 0.0;
        };

        let written_off = round_cents(account.created_principal_outstanding.max(0.0));
        self.created_money_outstanding =
            round_cents((self.created_money_outstanding - written_off).max(0.0));
        self.last_month_credit_losses = round_cents(self.last_month_credit_losses + written_off);
        written_off
    }

    pub fn set_deposit_liabilities(&mut self, amount: f64) {
        self.deposit_liabilities = round_cents(amount.max(0.0));
    }

    pub fn apply_deposit_inflow(&mut self, amount: f64) {
        let amount = round_cents(amount.max(0.0));
        if amount <= 0.0 {
            return;
        }
        self.deposit_liabilities = round_cents(self.deposit_liabilities + amount);
        self.reserve_balance = round_cents(self.reserve_balance + amount);
    }

    pub fn apply_withdrawal(&mut self, requested: f64) -> f64 {
        let requested = round_cents(requested.max(0.0));
        if requested <= 0.0 {
            return 0.0;
        }
        let paid = round_cents(requested.min(self.reserve_balance.max(0.0)));
        if paid <= 0.0 {
            return 0.0;
        }
        self.reserve_balance = round_cents(self.reserve_balance - paid);
        self.deposit_liabilities = round_cents((self.deposit_liabilities - paid).max(0.0));
        paid
    }

    pub fn can_fund_withdrawal(&self, amount: f64) -> bool {
        round_cents(amount.max(0.0)) <= round_cents(self.reserve_balance.max(0.0)) + 0.01
    }

    pub fn liquidity_ratio(&self) -> f64 {
        if self.deposit_liabilities <= 0.0 {
            return 1.0;
        }
        (self.reserve_balance.max(0.0) / self.deposit_liabilities.max(0.01)).clamp(0.0, 10.0)
    }

    pub fn capital_ratio(&self) -> f64 {
        let assets = self.reserve_balance.max(0.0) + self.created_money_outstanding.max(0.0);
        let liabilities = self.deposit_liabilities.max(0.0);
        if assets <= 0.0 {
            return 0.0;
        }
        ((assets - liabilities) / assets).clamp(-1.0, 1.0)
    }

    pub fn health_score(&self) -> f64 {
        self.health_score_private()
    }

    pub fn health_score_private(&self) -> f64 {
        let liquidity = self.liquidity_ratio().min(1.0);
        let capital = ((self.capital_ratio() + 1.0) / 2.0).clamp(0.0, 1.0);
        let resolution_penalty = if self.is_in_resolution { 0.35 } else { 0.0 };
        let run_penalty = self.run_intensity.clamp(0.0, 1.0) * 0.50;
        (0.60 * liquidity + 0.40 * capital - resolution_penalty - run_penalty).clamp(0.0, 1.0)
    }

    pub fn health_score_public(
        &self,
        treasury_balance: f64,
        health_floor: f64,
        liquidity_weight: f64,
        capital_weight: f64,
        sovereign_support_weight: f64,
    ) -> f64 {
        let liquidity = self.liquidity_ratio().min(1.0);
        let capital = ((self.capital_ratio() + 1.0) / 2.0).clamp(0.0, 1.0);
        let support_base = self.deposit_liabilities.max(0.0) * 0.10 + 1.0;
        let sovereign_support = (treasury_balance.max(0.0) / support_base).clamp(0.0, 1.0);
        let weights_sum =
            (liquidity_weight + capital_weight + sovereign_support_weight).max(f64::EPSILON);
        let weighted = (liquidity * liquidity_weight
            + capital * capital_weight
            + sovereign_support * sovereign_support_weight)
            / weights_sum;
        let resolution_penalty = if self.is_in_resolution { 0.20 } else { 0.0 };
        let run_penalty = self.run_intensity.clamp(0.0, 1.0) * 0.25;
        let stress_adjusted = (weighted - resolution_penalty - run_penalty).clamp(0.0, 1.0);
        if self.is_in_resolution {
            stress_adjusted
        } else {
            stress_adjusted.max(health_floor.clamp(0.0, 1.0))
        }
    }

    pub fn update_run_intensity(&mut self, stress: f64) {
        self.run_intensity = round_cents(stress.clamp(0.0, 1.0));
    }

    pub fn set_resolution_state(&mut self, in_resolution: bool) {
        if in_resolution {
            if self.is_in_resolution {
                self.months_in_resolution = self.months_in_resolution.saturating_add(1);
            } else {
                self.months_in_resolution = 1;
            }
        } else {
            self.months_in_resolution = 0;
        }
        self.is_in_resolution = in_resolution;
    }

    pub fn apply_resolution_signal(
        &mut self,
        resolution_required: bool,
        entry_months: u16,
        exit_months: u16,
    ) {
        if resolution_required {
            self.resolution_distress_streak = self.resolution_distress_streak.saturating_add(1);
            self.resolution_stable_streak = 0;
        } else {
            self.resolution_stable_streak = self.resolution_stable_streak.saturating_add(1);
            self.resolution_distress_streak = 0;
        }

        let entry_months = entry_months.max(1);
        let exit_months = exit_months.max(1);
        if self.is_in_resolution {
            if resolution_required {
                self.set_resolution_state(true);
            } else if self.resolution_stable_streak >= exit_months {
                self.set_resolution_state(false);
            }
            return;
        }

        if resolution_required && self.resolution_distress_streak >= entry_months {
            self.set_resolution_state(true);
        }
    }

}

#[cfg(test)]
#[path = "bank/tests.rs"]
mod tests;
