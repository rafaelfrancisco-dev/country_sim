use super::*;

pub(in crate::components::simulation) fn build_banks_tab_snapshot(
    world: &World,
    ui: &UiState,
    currency_symbol: &str,
) -> RunningTabSnapshot {
    let selected_country = ui.selected_country(world);
    let disaster_days = selected_country.map_or_else(Vec::new, |country| {
        disaster_days_for_country(world, country.name.as_str())
    });
    let mut bank_rows = Vec::new();
    if selected_country.is_some() {
        let bank_page_start =
            ui.bank_page * crate::components::simulation::ui_state::BANK_PAGE_SIZE;
        let page_len = ui.selected_banks_page_len(world);
        for row_idx in 0..page_len {
            let absolute_idx = bank_page_start + row_idx;
            let Some(bank) = ui.selected_bank_by_absolute_index(world, absolute_idx) else {
                continue;
            };
            let kind = match bank.kind {
                crate::sim::bank::BankKind::Public => "Public",
                crate::sim::bank::BankKind::Private => "Private",
            }
            .to_string();
            let resolution = if bank.is_in_resolution {
                "In Resolution"
            } else {
                "Operating"
            }
            .to_string();

            bank_rows.push(BankRowSnapshot {
                bank_id: bank.id.to_string(),
                name: bank.name.clone(),
                kind,
                reserve: money(bank.reserve_balance, currency_symbol),
                deposits: money(bank.deposit_liabilities, currency_symbol),
                created_money: money(bank.created_money_outstanding, currency_symbol),
                health: format!(
                    "{:.1}%",
                    selected_country
                        .map(|country| country.bank_health_score(bank.id))
                        .unwrap_or(0.0)
                        * 100.0
                ),
                run_intensity: format!("{:.1}%", bank.run_intensity * 100.0),
                resolution,
                is_selected: absolute_idx == ui.selected_bank_absolute_index(),
                is_even: row_idx.is_multiple_of(2),
            });
        }
    }

    let (
        bank_reserve_history,
        bank_deposit_history,
        bank_created_money_history,
        bank_equity_history,
        bank_run_intensity_history,
    ) = selected_bank_histories(world, ui);

    RunningTabSnapshot::Banks(Box::new(BanksTabSnapshot {
        bank_rows,
        bank_details_lines: bank_details_lines(world, ui),
        bank_related_events: selected_bank_related_events_snapshot(world, ui),
        bank_depositor_rows: selected_bank_depositor_rows(world, ui, currency_symbol),
        bank_reserve_history: bank_reserve_history.into(),
        bank_deposit_history: bank_deposit_history.into(),
        bank_created_money_history: bank_created_money_history.into(),
        bank_equity_history: bank_equity_history.into(),
        bank_run_intensity_history: bank_run_intensity_history.into(),
        disaster_days: disaster_days.into(),
        country_banks_summary_lines: country_banks_summary_lines(world, ui),
    }))
}

fn selected_bank_related_events_snapshot(world: &World, ui: &UiState) -> EventsTabSnapshot {
    let Some(country) = ui.selected_country(world) else {
        return build_bank_events_tab_snapshot(world, ui, |_| false);
    };
    let Some(bank) = ui.selected_bank_on_page(world) else {
        return build_bank_events_tab_snapshot(world, ui, |_| false);
    };
    let country_name = country.name.as_str();
    let bank_id = bank.id;
    build_bank_events_tab_snapshot(world, ui, |event| {
        event_matches_bank(event, country_name, bank_id)
    })
}

fn build_bank_events_tab_snapshot<F>(
    world: &World,
    ui: &UiState,
    mut include_event: F,
) -> EventsTabSnapshot
where
    F: FnMut(&SimEvent) -> bool,
{
    let search_lower = ui.events_search_query.to_ascii_lowercase();
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);
    let mut event_log_lines: Vec<String> = world
        .event_log
        .iter()
        .filter(|event| include_event(event))
        .filter(|event| event_matches_category(event.category(), ui.events_category))
        .map(|event| event.format_with_currency(currency_symbol))
        .filter(|line| search_lower.is_empty() || line.to_ascii_lowercase().contains(&search_lower))
        .collect();
    if matches!(ui.events_sort, EventsSort::NewestFirst) {
        event_log_lines.reverse();
    }

    EventsTabSnapshot {
        events_total_count: event_log_lines.len(),
        event_log_lines,
        events_scroll: ui.events_scroll,
        events_sort_label: match ui.events_sort {
            EventsSort::NewestFirst => "Newest First",
            EventsSort::OldestFirst => "Oldest First",
        },
        events_category_label: ui.events_category.label(),
        events_search_query: ui.events_search_query.clone(),
    }
}

fn event_matches_category(event_category: EventCategory, filter: EventsCategoryFilter) -> bool {
    match filter {
        EventsCategoryFilter::All => true,
        EventsCategoryFilter::StockMarket => matches!(event_category, EventCategory::StockMarket),
        EventsCategoryFilter::BirthsDeaths => {
            matches!(event_category, EventCategory::BirthsDeaths)
        }
        EventsCategoryFilter::BankruptciesDistress => {
            matches!(event_category, EventCategory::BankruptciesDistress)
        }
        EventsCategoryFilter::LoansDebt => matches!(event_category, EventCategory::LoansDebt),
        EventsCategoryFilter::CompaniesLabor => {
            matches!(event_category, EventCategory::CompaniesLabor)
        }
        EventsCategoryFilter::GovernmentPolicy => {
            matches!(event_category, EventCategory::GovernmentPolicy)
        }
    }
}

fn event_matches_bank(event: &SimEvent, selected_country_name: &str, bank_id: u16) -> bool {
    if event_country_name(event) != selected_country_name {
        return false;
    }

    match event {
        SimEvent::BankFounded {
            bank_id: event_bank_id,
            ..
        }
        | SimEvent::BankRunStarted {
            bank_id: event_bank_id,
            ..
        }
        | SimEvent::BankRunWithdrawal {
            bank_id: event_bank_id,
            ..
        }
        | SimEvent::BankResolutionStarted {
            bank_id: event_bank_id,
            ..
        }
        | SimEvent::BankResolved {
            bank_id: event_bank_id,
            ..
        }
        | SimEvent::BankClosed {
            bank_id: event_bank_id,
            ..
        }
        | SimEvent::BankDepositHaircut {
            bank_id: event_bank_id,
            ..
        } => *event_bank_id == bank_id,
        _ => false,
    }
}

fn event_country_name(event: &SimEvent) -> &str {
    match event {
        SimEvent::PopBirth { country_name, .. }
        | SimEvent::PopDeath { country_name, .. }
        | SimEvent::LoanEmitted { country_name, .. }
        | SimEvent::LoanEnded { country_name, .. }
        | SimEvent::LoanWrittenOff { country_name, .. }
        | SimEvent::DebtDistress { country_name, .. }
        | SimEvent::GovTransfer { country_name, .. }
        | SimEvent::GovCompanyFunded { country_name, .. }
        | SimEvent::CompanyFounded { country_name, .. }
        | SimEvent::BankFounded { country_name, .. }
        | SimEvent::BankRunStarted { country_name, .. }
        | SimEvent::BankRunWithdrawal { country_name, .. }
        | SimEvent::BankResolutionStarted { country_name, .. }
        | SimEvent::BankResolved { country_name, .. }
        | SimEvent::BankClosed { country_name, .. }
        | SimEvent::BankDepositHaircut { country_name, .. }
        | SimEvent::CompanyWageCut { country_name, .. }
        | SimEvent::CompanyLayoffs { country_name, .. }
        | SimEvent::LaborChurnMonthly { country_name, .. }
        | SimEvent::JobSwitched { country_name, .. }
        | SimEvent::CompanyBankrupt { country_name, .. }
        | SimEvent::StockIpo { country_name, .. }
        | SimEvent::StockDividend { country_name, .. }
        | SimEvent::StockMarginCall { country_name, .. }
        | SimEvent::StockDelisted { country_name, .. }
        | SimEvent::StockMarketClose { country_name, .. }
        | SimEvent::RatePolicyChange { country_name, .. }
        | SimEvent::BondIssued { country_name, .. }
        | SimEvent::CouponPaid { country_name, .. }
        | SimEvent::BondMatured { country_name, .. }
        | SimEvent::BondDefault { country_name, .. }
        | SimEvent::MoneyMinted { country_name, .. }
        | SimEvent::StockSecondaryOffering { country_name, .. }
        | SimEvent::DisasterApplied { country_name, .. } => country_name,
        SimEvent::ForeignDepositOpened {
            home_country_name: country_name,
            ..
        }
        | SimEvent::ForeignDepositClosed {
            home_country_name: country_name,
            ..
        } => country_name,
    }
}
