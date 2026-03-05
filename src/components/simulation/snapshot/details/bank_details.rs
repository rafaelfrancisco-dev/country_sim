use super::*;

pub(in crate::components::simulation) fn bank_details_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };
    let Some(bank) = ui.selected_bank_on_page(world) else {
        return vec!["No bank available for the current selection.".to_string()];
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

    let kind = match bank.kind {
        crate::sim::bank::BankKind::Public => "Public",
        crate::sim::bank::BankKind::Private => "Private",
    };
    let computed_equity =
        bank.reserve_balance + bank.created_money_outstanding - bank.deposit_liabilities;
    let total_country_deposits = country.total_bank_deposit_liabilities().max(0.0);
    let deposit_share = if total_country_deposits <= 0.0 {
        0.0
    } else {
        (bank.deposit_liabilities / total_country_deposits).clamp(0.0, 1.0)
    };

    let mut lines = vec![
        format!("Country: {}", country.name),
        format!("Bank: {}", bank.name),
        format!("Bank ID: {}", bank.id),
        format!("Type: {kind}"),
        String::new(),
        "Balance Sheet:".to_string(),
        format!(
            "Reserve Balance: {}",
            money(bank.reserve_balance, currency_symbol)
        ),
        format!(
            "Deposit Liabilities: {}",
            money(bank.deposit_liabilities, currency_symbol)
        ),
        format!(
            "Created Money Outstanding: {}",
            money(bank.created_money_outstanding, currency_symbol)
        ),
        format!(
            "Computed Equity: {}",
            money(computed_equity, currency_symbol)
        ),
        format!(
            "Seed Equity Capital: {}",
            money(bank.equity_capital, currency_symbol)
        ),
        String::new(),
        "Flow Metrics:".to_string(),
        format!(
            "Total Loans Originated: {}",
            money(bank.total_originated, currency_symbol)
        ),
        format!(
            "Total Principal Repaid: {}",
            money(bank.total_principal_repaid, currency_symbol)
        ),
        format!("Country Deposit Share: {:.2}%", deposit_share * 100.0),
        String::new(),
        "Stability:".to_string(),
        format!("Liquidity Ratio: {:.2}", bank.liquidity_ratio()),
        format!("Capital Ratio: {:.2}%", bank.capital_ratio() * 100.0),
        format!(
            "Health Score: {:.1}%",
            country.bank_health_score(bank.id) * 100.0
        ),
        format!("Run Intensity: {:.1}%", bank.run_intensity * 100.0),
        format!(
            "Resolution State: {}",
            if bank.is_in_resolution {
                "In Resolution"
            } else {
                "Operating"
            }
        ),
        format!("Months In Resolution: {}", bank.months_in_resolution),
        String::new(),
        "Employment:".to_string(),
        format!("Employees: {}", bank.employee_count),
        format!("Open Positions: {}", bank.open_position_count()),
        format!(
            "Monthly Payroll Paid: {}",
            money(bank.last_month_payroll_paid, currency_symbol)
        ),
    ];
    if bank.kind == crate::sim::bank::BankKind::Public {
        lines.push(String::new());
        lines.push("Monetary Operations:".to_string());
        lines.push(format!(
            "  MTD Minted to Reserves:  {}",
            money(country.monthly_money_minted, currency_symbol)
        ));
        lines.push(format!(
            "  Cumulative Minted:        {}",
            money(country.cumulative_money_minted, currency_symbol)
        ));
    }

    lines
}
