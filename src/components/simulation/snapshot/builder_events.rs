use super::*;

fn event_matches_category(event_category: EventCategory, filter: EventsCategoryFilter) -> bool {
    match filter {
        EventsCategoryFilter::All => true,
        EventsCategoryFilter::StockMarket => matches!(event_category, EventCategory::StockMarket),
        EventsCategoryFilter::BirthsDeaths => matches!(event_category, EventCategory::BirthsDeaths),
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

pub(super) fn build_events_tab_snapshot<F>(
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

pub(super) fn selected_person_related_events_snapshot(
    world: &World,
    ui: &UiState,
) -> EventsTabSnapshot {
    let Some(country) = ui.selected_country(world) else {
        return build_events_tab_snapshot(world, ui, |_| false);
    };
    let Some(person) = ui.selected_person_on_page(world) else {
        return build_events_tab_snapshot(world, ui, |_| false);
    };

    let country_name: &str = &country.name;
    let person_id = person.id;
    build_events_tab_snapshot(world, ui, |event| {
        event_matches_person(event, country_name, person_id)
    })
}

pub(super) fn selected_company_related_events_snapshot(
    world: &World,
    ui: &UiState,
) -> EventsTabSnapshot {
    let Some(country) = ui.selected_country(world) else {
        return build_events_tab_snapshot(world, ui, |_| false);
    };
    let Some(company) = ui.selected_company_on_page(world) else {
        return build_events_tab_snapshot(world, ui, |_| false);
    };

    let country_name: &str = &country.name;
    let mut company_ids = Vec::with_capacity(company.historical_ids.len() + 1);
    company_ids.push(company.id);
    company_ids.extend(company.historical_ids.iter().copied());
    build_events_tab_snapshot(world, ui, |event| {
        event_matches_company(event, country_name, &company_ids)
    })
}

fn event_matches_person(event: &SimEvent, selected_country_name: &str, person_id: u32) -> bool {
    let selected_country_matches = match event {
        SimEvent::PersonMigratedForWork {
            from_country_name,
            to_country_name,
            ..
        }
        | SimEvent::PersonMigratedForDistress {
            from_country_name,
            to_country_name,
            ..
        } => {
            from_country_name.as_ref() == selected_country_name
                || to_country_name.as_ref() == selected_country_name
        }
        _ => event_country_name(event) == selected_country_name,
    };
    if !selected_country_matches {
        return false;
    }

    match event {
        SimEvent::LoanEmitted {
            entity_kind,
            entity_id,
            ..
        }
        | SimEvent::LoanEnded {
            entity_kind,
            entity_id,
            ..
        }
        | SimEvent::DebtDistress {
            entity_kind,
            entity_id,
            ..
        } => matches!(entity_kind, EntityKind::Person) && *entity_id == person_id,
        SimEvent::LoanWrittenOff { entity_id, .. } => *entity_id == person_id,
        SimEvent::JobSwitched {
            person_id: switched_person_id,
            ..
        }
        | SimEvent::PersonMigratedForWork {
            person_id: switched_person_id,
            ..
        }
        | SimEvent::StockMarginCall {
            person_id: switched_person_id,
            ..
        } => *switched_person_id == person_id,
        SimEvent::PersonMigratedForDistress {
            person_id: migrated_person_id,
            ..
        } => *migrated_person_id == person_id,
        _ => false,
    }
}

fn event_matches_company(
    event: &SimEvent,
    selected_country_name: &str,
    company_ids: &[u16],
) -> bool {
    if event_country_name(event) != selected_country_name {
        return false;
    }

    match event {
        SimEvent::LoanEmitted {
            entity_kind,
            entity_id,
            ..
        }
        | SimEvent::LoanEnded {
            entity_kind,
            entity_id,
            ..
        }
        | SimEvent::DebtDistress {
            entity_kind,
            entity_id,
            ..
        } => {
            matches!(entity_kind, EntityKind::Company)
                && u16::try_from(*entity_id)
                    .ok()
                    .is_some_and(|event_company_id| company_ids.contains(&event_company_id))
        }
        SimEvent::GovCompanyFunded {
            company_id: event_company_id,
            ..
        }
        | SimEvent::CompanyFounded {
            company_id: event_company_id,
            ..
        }
        | SimEvent::CompanyWageCut {
            company_id: event_company_id,
            ..
        }
        | SimEvent::CompanyLayoffs {
            company_id: event_company_id,
            ..
        }
        | SimEvent::CompanyBankrupt {
            company_id: event_company_id,
            ..
        }
        | SimEvent::StockIpo {
            company_id: event_company_id,
            ..
        }
        | SimEvent::StockDividend {
            company_id: event_company_id,
            ..
        }
        | SimEvent::StockDelisted {
            company_id: event_company_id,
            ..
        }
        | SimEvent::StockSecondaryOffering {
            company_id: event_company_id,
            ..
        } => company_ids.contains(event_company_id),
        SimEvent::JobSwitched {
            from_company_id,
            to_company_id,
            ..
        } => company_ids.contains(from_company_id) || company_ids.contains(to_company_id),
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
        | SimEvent::DisasterApplied { country_name, .. }
        | SimEvent::HomePurchased { country_name, .. }
        | SimEvent::Foreclosure { country_name, .. }
        | SimEvent::MortgagePaidOff { country_name, .. }
        | SimEvent::HousingPriceChanged { country_name, .. }
        | SimEvent::RentUnpaid { country_name, .. } => country_name,
        SimEvent::PersonMigratedForWork {
            from_country_name,
            ..
        }
        | SimEvent::PersonMigratedForDistress {
            from_country_name,
            ..
        } => from_country_name,
        SimEvent::ForeignDepositOpened {
            home_country_name: country_name,
            ..
        }
        | SimEvent::ForeignDepositClosed {
            home_country_name: country_name,
            ..
        } => country_name,
    }
    .as_ref()
}
