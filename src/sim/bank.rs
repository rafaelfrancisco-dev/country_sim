use super::money::round_cents;

pub const DEFAULT_LOAN_APR: f64 = 0.12;
pub const DEFAULT_LOAN_TERM_MONTHS: u16 = 12;
pub const DEFAULT_MAX_DEBT_TO_INCOME: f64 = 0.35;

#[derive(Debug, Clone, PartialEq)]
pub struct LoanAccount {
    pub debt_balance: f64,
    pub created_principal_outstanding: f64,
    pub monthly_payment: f64,
    pub annual_rate: f64,
    pub term_months: u16,
    pub months_remaining: u16,
}

impl LoanAccount {
    pub fn new(principal: f64, annual_rate: f64, term_months: u16) -> Self {
        let principal = round_cents(principal.max(0.0));
        Self {
            debt_balance: principal,
            created_principal_outstanding: principal,
            monthly_payment: payment_for(principal, annual_rate, term_months),
            annual_rate,
            term_months,
            months_remaining: term_months,
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
    pub reserve_balance: f64,
    pub created_money_outstanding: f64,
    pub total_originated: f64,
    pub total_principal_repaid: f64,
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

impl Bank {
    pub fn new() -> Self {
        Self {
            reserve_balance: 0.0,
            created_money_outstanding: 0.0,
            total_originated: 0.0,
            total_principal_repaid: 0.0,
        }
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
            }
            None => {
                *loan = Some(LoanAccount::new(
                    approved_principal,
                    annual_rate,
                    DEFAULT_LOAN_TERM_MONTHS,
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
        written_off
    }
}

pub fn payment_for(principal: f64, annual_rate: f64, term_months: u16) -> f64 {
    if principal <= 0.0 || term_months == 0 {
        return 0.0;
    }

    let monthly_rate = monthly_interest_rate(annual_rate);
    if monthly_rate <= f64::EPSILON {
        return round_cents(principal / f64::from(term_months));
    }

    let n = f64::from(term_months);
    let factor = (1.0 - (1.0 + monthly_rate).powf(-n)) / monthly_rate;
    round_cents(principal / factor)
}

pub fn max_principal_for_payment(payment: f64, annual_rate: f64, term_months: u16) -> f64 {
    if payment <= 0.0 || term_months == 0 {
        return 0.0;
    }

    let monthly_rate = monthly_interest_rate(annual_rate);
    if monthly_rate <= f64::EPSILON {
        return round_cents(payment * f64::from(term_months));
    }

    let n = f64::from(term_months);
    let factor = (1.0 - (1.0 + monthly_rate).powf(-n)) / monthly_rate;
    round_cents(payment * factor)
}

fn monthly_interest_rate(annual_rate: f64) -> f64 {
    annual_rate.max(0.0) / 12.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affordability_cap_limits_principal() {
        let cap_payment = 1_000.0 * DEFAULT_MAX_DEBT_TO_INCOME;
        let principal =
            max_principal_for_payment(cap_payment, DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS);
        let payment = payment_for(principal, DEFAULT_LOAN_APR, DEFAULT_LOAN_TERM_MONTHS);

        assert!(payment <= cap_payment + 0.01);
    }

    #[test]
    fn loan_issuance_increases_created_money() {
        let mut bank = Bank::new();
        let mut loan = None;

        let issued = bank.originate_loan(1_000.0, 4_000.0, DEFAULT_MAX_DEBT_TO_INCOME, DEFAULT_LOAN_APR, &mut loan);

        assert!(issued > 0.0);
        assert!(bank.created_money_outstanding >= issued);
        assert!(loan.is_some());
    }

    #[test]
    fn principal_repayment_reduces_created_money() {
        let mut bank = Bank::new();
        let mut loan = None;
        bank.originate_loan(1_000.0, 6_000.0, DEFAULT_MAX_DEBT_TO_INCOME, DEFAULT_LOAN_APR, &mut loan);
        let created_before = bank.created_money_outstanding;
        let mut cash = 5_000.0;

        let payment = bank.collect_monthly_payment(&mut cash, &mut loan);

        assert!(payment.principal_paid > 0.0);
        assert!(bank.created_money_outstanding < created_before);
    }

    #[test]
    fn missed_payment_is_capitalized_in_debt_balance() {
        let mut bank = Bank::new();
        let mut loan = None;
        bank.originate_loan(2_000.0, 5_000.0, DEFAULT_MAX_DEBT_TO_INCOME, DEFAULT_LOAN_APR, &mut loan);

        let opening_debt = loan.as_ref().map_or(0.0, |loan| loan.debt_balance);
        let mut cash = 0.0;
        let payment = bank.collect_monthly_payment(&mut cash, &mut loan);
        let closing_debt = loan.as_ref().map_or(0.0, |loan| loan.debt_balance);

        assert!(payment.unpaid_amount > 0.0);
        assert!(closing_debt > opening_debt);
    }

    #[test]
    fn write_off_reduces_created_money() {
        let mut bank = Bank::new();
        let mut loan = None;
        bank.originate_loan(2_000.0, 5_000.0, DEFAULT_MAX_DEBT_TO_INCOME, DEFAULT_LOAN_APR, &mut loan);
        let before = bank.created_money_outstanding;

        let written_off = bank.write_off_loan(&mut loan);

        assert!(written_off > 0.0);
        assert!(loan.is_none());
        assert!(bank.created_money_outstanding < before);
    }

    #[test]
    fn government_deficit_funding_increases_created_money() {
        let mut bank = Bank::new();

        let funded = bank.fund_government_deficit(2_500.0);

        assert!((funded - 2_500.0).abs() < 0.01);
        assert!((bank.created_money_outstanding - 2_500.0).abs() < 0.01);
        assert!((bank.total_originated - 2_500.0).abs() < 0.01);
    }
}
