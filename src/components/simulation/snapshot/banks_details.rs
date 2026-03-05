use super::*;
use std::cmp::Ordering;

type BankHistorySeries = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

pub(in crate::components::simulation) fn selected_bank_histories(
    world: &World,
    ui: &UiState,
) -> BankHistorySeries {
    let Some(bank) = ui.selected_bank_on_page(world) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };

    let key = (ui.selected_country, bank.id);
    let Some(history) = world.bank_history.get(&key) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };

    (
        history.iter().map(|point| point.reserve).collect(),
        history.iter().map(|point| point.deposits).collect(),
        history.iter().map(|point| point.created_money).collect(),
        history.iter().map(|point| point.equity).collect(),
        history.iter().map(|point| point.run_intensity).collect(),
    )
}

pub(in crate::components::simulation) fn country_banks_summary_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

    let all_banks = std::iter::once(&country.bank).chain(country.banks.iter());
    let total_banks = country.banks.len().saturating_add(1);
    let public_banks = all_banks
        .clone()
        .filter(|bank| bank.kind == crate::sim::bank::BankKind::Public)
        .count();
    let private_banks = total_banks.saturating_sub(public_banks);
    let in_resolution = all_banks
        .clone()
        .filter(|bank| bank.is_in_resolution)
        .count();
    let strongest_bank = all_banks.max_by(|a, b| {
        country
            .bank_health_score(a.id)
            .partial_cmp(&country.bank_health_score(b.id))
            .unwrap_or(Ordering::Equal)
    });

    vec![
        format!("Country: {} (Month {})", country.name, country.month),
        format!(
            "Banks (Public / Private / In Resolution): {} ({} / {} / {})",
            compact_count(total_banks as f64),
            compact_count(public_banks as f64),
            compact_count(private_banks as f64),
            compact_count(in_resolution as f64)
        ),
        format!(
            "Total Reserve: {}",
            money(country.total_bank_reserves(), currency_symbol)
        ),
        format!(
            "Total Deposits: {}",
            money(country.total_bank_deposit_liabilities(), currency_symbol)
        ),
        format!(
            "Total Created Money: {}",
            money(country.total_bank_created_money(), currency_symbol)
        ),
        strongest_bank.map_or_else(
            || "Healthiest Bank: N/A".to_string(),
            |bank| {
                format!(
                    "Healthiest Bank: {} ({:.1}%)",
                    bank.name,
                    country.bank_health_score(bank.id) * 100.0
                )
            },
        ),
    ]
}

pub(in crate::components::simulation) fn selected_bank_depositor_rows(
    world: &World,
    ui: &UiState,
    currency_symbol: &str,
) -> Vec<BankDepositorRowSnapshot> {
    let Some(country) = ui.selected_country(world) else {
        return Vec::new();
    };
    let Some(bank) = ui.selected_bank_on_page(world) else {
        return Vec::new();
    };

    let mut rows = Vec::new();

    for person in &country.people {
        if person.deposit_bank_id != Some(bank.id) || person.cash_balance <= 0.0 {
            continue;
        }
        let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
        let name = person_name_from_seed(name_seed);
        rows.push((
            "Person".to_string(),
            format!("{name} (#{})", person.id),
            person.cash_balance,
        ));
    }

    for company in &country.companies {
        if company.deposit_bank_id != Some(bank.id) || company.cash_balance <= 0.0 {
            continue;
        }
        rows.push((
            "Company".to_string(),
            format!("{} (#{})", company.name, company.id),
            company.cash_balance,
        ));
    }

    if country.treasury_bank_id == Some(bank.id) && country.treasury_balance > 0.0 {
        rows.push((
            "Treasury".to_string(),
            format!("{} Treasury", country.name),
            country.treasury_balance,
        ));
    }

    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));
    let total: f64 = rows.iter().map(|row| row.2.max(0.0)).sum();

    rows.into_iter()
        .map(|(entity_kind, entity, balance)| {
            let share = if total <= 0.0 {
                0.0
            } else {
                (balance.max(0.0) / total).clamp(0.0, 1.0)
            };
            BankDepositorRowSnapshot {
                entity_kind,
                entity,
                balance: money(balance, currency_symbol),
                share: format!("{:.2}%", share * 100.0),
            }
        })
        .collect()
}
