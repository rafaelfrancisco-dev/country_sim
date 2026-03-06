use crate::sim::event::SimEvent;

use super::utils::format_money;

pub(super) fn format_event(event: &SimEvent, currency_symbol: &str) -> String {
    match event {
        SimEvent::HomePurchased {
            country_name,
            person_id,
            price,
            day,
        } => format!(
            "[HOME+] Day {day:>6} | {country_name}: Person #{person_id} purchased home for {}",
            format_money(*price, currency_symbol)
        ),
        SimEvent::Foreclosure {
            country_name,
            person_id,
            principal_written_off,
            day,
        } => format!(
            "[FRCLS] Day {day:>6} | {country_name}: Person #{person_id} foreclosed, {} written off",
            format_money(*principal_written_off, currency_symbol)
        ),
        SimEvent::MortgagePaidOff {
            country_name,
            person_id,
            day,
        } => {
            format!("[MORT=] Day {day:>6} | {country_name}: Person #{person_id} paid off mortgage")
        }
        SimEvent::HousingPriceChanged {
            country_name,
            new_price_index,
            change_pct,
            day,
        } => {
            let dir = if *change_pct >= 0.0 { "▲" } else { "▼" };
            format!(
                "[HPRIC] Day {day:>6} | {country_name}: Housing price index {} ({dir}{:.2}%)",
                format_money(*new_price_index, currency_symbol),
                change_pct.abs() * 100.0
            )
        }
        SimEvent::RentUnpaid {
            country_name,
            person_id,
            amount_due,
            amount_paid,
            day,
        } => format!(
            "[RENT!] Day {day:>6} | {country_name}: Person #{person_id} underpaid rent - due {}, paid {}",
            format_money(*amount_due, currency_symbol),
            format_money(*amount_paid, currency_symbol),
        ),
        _ => unreachable!("non-housing event dispatched to housing formatter"),
    }
}
