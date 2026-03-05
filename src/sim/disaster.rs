use std::collections::HashMap;

use super::{
    country::Country,
    event::{EVENT_LOG_LIMIT, SimEvent},
    money::round_cents,
    stock_market::ListedEquity,
    world::World,
};

pub const DISASTER_INTENSITY_LEVELS: [u8; 4] = [5, 10, 25, 50];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DisasterKind {
    CompanyBankruptcyWave,
    PopulationLoss,
    StockMarketCrash,
    BankReserveDrain,
    LoanDefaultWave,
    TreasuryShock,
    DemandShock,
}

pub const DISASTER_KIND_COUNT: usize = 7;

pub const ALL_DISASTER_KINDS: [DisasterKind; DISASTER_KIND_COUNT] = [
    DisasterKind::CompanyBankruptcyWave,
    DisasterKind::PopulationLoss,
    DisasterKind::StockMarketCrash,
    DisasterKind::BankReserveDrain,
    DisasterKind::LoanDefaultWave,
    DisasterKind::TreasuryShock,
    DisasterKind::DemandShock,
];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DisasterRequest {
    pub kind: DisasterKind,
    pub intensity_pct: u8,
}

impl DisasterKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::CompanyBankruptcyWave => "Company Bankruptcy Wave",
            Self::PopulationLoss => "Population Loss",
            Self::StockMarketCrash => "Stock Market Crash",
            Self::BankReserveDrain => "Bank Reserve Drain",
            Self::LoanDefaultWave => "Loan Default Wave",
            Self::TreasuryShock => "Treasury Shock",
            Self::DemandShock => "Demand Shock",
        }
    }
}

pub(crate) fn apply_disaster_requests(
    world: &mut World,
    country_idx: usize,
    requests: &[DisasterRequest],
) -> usize {
    let Some(country) = world.countries.get_mut(country_idx) else {
        return 0;
    };

    let mut applied = 0_usize;
    for request in requests {
        let intensity_pct = request.intensity_pct.clamp(0, 100);
        if intensity_pct == 0 {
            continue;
        }

        let summary = apply_single_disaster(country, request.kind, intensity_pct);
        if summary == "No effect." {
            continue;
        }
        applied = applied.saturating_add(1);

        world.event_log.push(SimEvent::DisasterApplied {
            country_name: country.name.clone(),
            disaster_label: request.kind.label().to_string(),
            intensity_pct,
            summary,
            day: world.day,
        });
    }

    if !requests.is_empty() {
        country.recompute_bank_deposit_liabilities();
        country.recompute_bank_headcounts();
        country.recompute_aggregates();
    }

    if world.event_log.len() > EVENT_LOG_LIMIT {
        let evict = EVENT_LOG_LIMIT / 4;
        world.event_log.drain(0..evict);
    }

    applied
}

fn apply_single_disaster(country: &mut Country, kind: DisasterKind, intensity_pct: u8) -> String {
    match kind {
        DisasterKind::CompanyBankruptcyWave => {
            let bankruptcies = apply_company_bankruptcy_wave(country, intensity_pct);
            if bankruptcies == 0 {
                return "No effect.".to_string();
            }
            format!("{bankruptcies} companies forced into bankruptcy.")
        }
        DisasterKind::PopulationLoss => {
            let removed = apply_population_loss(country, intensity_pct);
            if removed == 0 {
                return "No effect.".to_string();
            }
            format!("{removed} citizens removed from the simulation.")
        }
        DisasterKind::StockMarketCrash => {
            let impacted = apply_stock_market_crash(country, intensity_pct);
            if impacted == 0 {
                return "No effect.".to_string();
            }
            format!("{impacted} listed equities repriced lower.")
        }
        DisasterKind::BankReserveDrain => {
            let drained = apply_bank_reserve_drain(country, intensity_pct);
            if drained <= 0.0 {
                return "No effect.".to_string();
            }
            format!("{} removed from bank reserves.", money(drained))
        }
        DisasterKind::LoanDefaultWave => {
            let written_off = apply_loan_default_wave(country, intensity_pct);
            if written_off <= 0.0 {
                return "No effect.".to_string();
            }
            format!("{} of created principal written off.", money(written_off))
        }
        DisasterKind::TreasuryShock => {
            let removed = apply_treasury_shock(country, intensity_pct);
            if removed <= 0.0 {
                return "No effect.".to_string();
            }
            format!("{} removed from treasury balance.", money(removed))
        }
        DisasterKind::DemandShock => {
            let removed = apply_demand_shock(country, intensity_pct);
            if removed <= 0.0 {
                return "No effect.".to_string();
            }
            format!("{} removed from household cash balances.", money(removed))
        }
    }
}

fn scaled_count(total: usize, intensity_pct: u8) -> usize {
    if total == 0 || intensity_pct == 0 {
        return 0;
    }
    let numerator = total.saturating_mul(intensity_pct as usize);
    let rounded = (numerator + 50) / 100;
    rounded.clamp(1, total)
}

fn apply_company_bankruptcy_wave(country: &mut Country, intensity_pct: u8) -> usize {
    let count = scaled_count(country.companies.len(), intensity_pct);
    if count == 0 {
        return 0;
    }

    let start_idx = country.companies.len().saturating_sub(count);
    let indices: Vec<usize> = (start_idx..country.companies.len()).collect();
    country.resolve_company_bankruptcies(indices);
    count
}

fn apply_population_loss(country: &mut Country, intensity_pct: u8) -> usize {
    let count = scaled_count(country.people.len(), intensity_pct);
    if count == 0 {
        return 0;
    }

    let split_idx = country.people.len().saturating_sub(count);
    let removed_people = country.people.split_off(split_idx);

    for mut person in removed_people {
        if let Some(loan) = person.loan.take() {
            let lender_bank_id = loan.lender_bank_id;
            let mut loan_slot = Some(loan);
            let principal_written_off = if let Some(bank) = country.bank_by_id_mut(lender_bank_id) {
                bank.write_off_loan(&mut loan_slot)
            } else {
                country.bank.write_off_loan(&mut loan_slot)
            };
            if principal_written_off > 0.0 {
                country.pending_events.push(SimEvent::LoanWrittenOff {
                    country_name: country.name.clone(),
                    entity_id: person.id,
                    principal_lost: principal_written_off,
                    day: country.day,
                });
            }
        }
        country.stock_market.person_portfolios.remove(&person.id);
    }

    count
}

fn apply_stock_market_crash(country: &mut Country, intensity_pct: u8) -> usize {
    let crash_factor = 1.0 - f64::from(intensity_pct) / 100.0;
    let floor = 0.01;
    let mut impacted = 0_usize;
    for ListedEquity {
        last_price,
        market_cap,
        shares_outstanding,
        ..
    } in country.stock_market.listed.values_mut()
    {
        let next_price = round_cents(((*last_price * crash_factor).max(floor)).max(0.0));
        if (next_price - *last_price).abs() > f64::EPSILON {
            impacted = impacted.saturating_add(1);
        }
        *last_price = next_price;
        *market_cap = round_cents(next_price * *shares_outstanding as f64);
    }
    impacted
}

fn apply_bank_reserve_drain(country: &mut Country, intensity_pct: u8) -> f64 {
    let ratio = f64::from(intensity_pct) / 100.0;
    let mut drained_total = 0.0;

    let mut apply_drain = |reserve: &mut f64, equity: &mut f64| {
        let drained = round_cents((*reserve).max(0.0) * ratio);
        *reserve = round_cents((*reserve - drained).max(0.0));
        *equity = round_cents(*equity - drained);
        drained_total = round_cents(drained_total + drained);
    };

    apply_drain(
        &mut country.bank.reserve_balance,
        &mut country.bank.equity_capital,
    );
    for bank in &mut country.banks {
        apply_drain(&mut bank.reserve_balance, &mut bank.equity_capital);
    }

    drained_total
}

fn apply_loan_default_wave(country: &mut Country, intensity_pct: u8) -> f64 {
    let ratio = f64::from(intensity_pct) / 100.0;
    let mut writeoffs_by_bank: HashMap<u16, f64> = HashMap::new();
    let mut written_off_total = 0.0;

    let mut apply_wave_to_loan = |loan: &mut Option<crate::sim::bank::LoanAccount>| -> f64 {
        let Some(account) = loan.as_mut() else {
            return 0.0;
        };

        let lender = account.lender_bank_id;
        let principal_writeoff = round_cents(account.created_principal_outstanding * ratio);
        if principal_writeoff <= 0.0 {
            return 0.0;
        }

        account.created_principal_outstanding =
            round_cents((account.created_principal_outstanding - principal_writeoff).max(0.0));
        account.debt_balance = round_cents((account.debt_balance - principal_writeoff).max(0.0));
        if account.debt_balance <= 0.0 {
            *loan = None;
        }

        let entry = writeoffs_by_bank.entry(lender).or_insert(0.0);
        *entry = round_cents(*entry + principal_writeoff);
        written_off_total = round_cents(written_off_total + principal_writeoff);
        principal_writeoff
    };

    for person in &mut country.people {
        apply_wave_to_loan(&mut person.loan);
    }
    for company in &mut country.companies {
        apply_wave_to_loan(&mut company.loan);
    }

    for (bank_id, amount) in writeoffs_by_bank {
        if let Some(bank) = country.bank_by_id_mut(bank_id) {
            bank.created_money_outstanding =
                round_cents((bank.created_money_outstanding - amount).max(0.0));
        } else {
            country.bank.created_money_outstanding =
                round_cents((country.bank.created_money_outstanding - amount).max(0.0));
        }
    }

    written_off_total
}

fn apply_treasury_shock(country: &mut Country, intensity_pct: u8) -> f64 {
    let ratio = f64::from(intensity_pct) / 100.0;
    let removed = round_cents(country.treasury_balance.max(0.0) * ratio);
    country.treasury_balance = round_cents((country.treasury_balance - removed).max(0.0));
    removed
}

fn apply_demand_shock(country: &mut Country, intensity_pct: u8) -> f64 {
    let ratio = f64::from(intensity_pct) / 100.0;
    let mut removed_total = 0.0;
    for person in &mut country.people {
        let removed = round_cents(person.cash_balance.max(0.0) * ratio);
        person.cash_balance = round_cents((person.cash_balance - removed).max(0.0));
        removed_total = round_cents(removed_total + removed);
    }
    removed_total
}

fn money(amount: f64) -> String {
    if amount.abs() >= 10_000.0 {
        let scaled = amount / 1_000.0;
        format!("{scaled:.1}k")
    } else {
        format!("{amount:.2}")
    }
}
