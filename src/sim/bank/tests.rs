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

    let issued = bank.originate_loan(
        1_000.0,
        4_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        DEFAULT_LOAN_APR,
        &mut loan,
    );

    assert!(issued > 0.0);
    assert!(bank.created_money_outstanding >= issued);
    assert!(loan.is_some());
}

#[test]
fn principal_repayment_reduces_created_money() {
    let mut bank = Bank::new();
    let mut loan = None;
    bank.originate_loan(
        1_000.0,
        6_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        DEFAULT_LOAN_APR,
        &mut loan,
    );
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
    bank.originate_loan(
        2_000.0,
        5_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        DEFAULT_LOAN_APR,
        &mut loan,
    );

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
    bank.originate_loan(
        2_000.0,
        5_000.0,
        DEFAULT_MAX_DEBT_TO_INCOME,
        DEFAULT_LOAN_APR,
        &mut loan,
    );
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
