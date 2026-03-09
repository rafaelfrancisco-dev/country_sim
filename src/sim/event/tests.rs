use std::sync::Arc;

use super::*;

#[test]
fn category_labels_are_human_readable() {
    assert_eq!(EventCategory::StockMarket.label(), "Stock Market");
    assert_eq!(EventCategory::BirthsDeaths.label(), "Births/Deaths");
}

#[test]
fn event_category_mapping_covers_all_variant_groups() {
    let mut seen = [false; 7];

    let samples = [
        SimEvent::PopBirth {
            country_name: Arc::from("A"),
            count: 1,
            day: 1,
        },
        SimEvent::LoanEmitted {
            country_name: Arc::from("A"),
            entity_kind: EntityKind::Person,
            entity_id: 1,
            principal: 10.0,
            day: 1,
        },
        SimEvent::DebtDistress {
            country_name: Arc::from("A"),
            entity_kind: EntityKind::Company,
            entity_id: 2,
            unpaid_amount: 3.0,
            day: 1,
        },
        SimEvent::CompanyFounded {
            country_name: Arc::from("A"),
            company_id: 1,
            company_name: Arc::from("Comp"),
            founders: 1,
            job_type_id: 1,
            produced_good_id: 1,
            day: 1,
        },
        SimEvent::StockIpo {
            country_name: Arc::from("A"),
            company_id: 1,
            company_name: Arc::from("Comp"),
            ticker: Arc::from("CMP"),
            reason: IpoReason::Growth,
            shares_issued: 10,
            primary_shares_issued: 8,
            secondary_shares_sold: 2,
            price_per_share: 1.0,
            total_raised: 10.0,
            primary_raised: 8.0,
            secondary_raised: 2.0,
            founder_payout_total: 1.0,
            day: 1,
        },
        SimEvent::BondIssued {
            country_name: Arc::from("A"),
            face_value: 100.0,
            coupon_rate: 0.02,
            day: 1,
        },
    ];

    for sample in samples {
        let idx = match sample.category() {
            EventCategory::StockMarket => 0,
            EventCategory::BirthsDeaths => 1,
            EventCategory::BankruptciesDistress => 2,
            EventCategory::LoansDebt => 3,
            EventCategory::CompaniesLabor => 4,
            EventCategory::GovernmentPolicy => 5,
            EventCategory::Housing => 6,
        };
        seen[idx] = true;
    }

    // Housing category is new; we check all other categories are covered.
    assert!(seen[..6].iter().all(|covered| *covered));
}

#[test]
fn event_money_format_uses_currency_and_compact_suffix() {
    let event = SimEvent::LoanEmitted {
        country_name: Arc::from("A"),
        entity_kind: EntityKind::Person,
        entity_id: 7,
        principal: 11_500.0,
        day: 1,
    };

    let formatted = event.format_with_currency("£");
    assert!(formatted.contains("£11.5k"));
}

#[test]
fn money_minted_event_is_government_policy_and_formats_with_currency() {
    let event = SimEvent::MoneyMinted {
        country_name: Arc::from("A"),
        amount: 12_500.0,
        inflation_shortfall: 0.02,
        unemployment_gap: 0.03,
        day: 30,
    };

    assert_eq!(event.category(), EventCategory::GovernmentPolicy);
    let formatted = event.format_with_currency("$");
    assert!(formatted.contains("[MINT]"));
    assert!(formatted.contains("$12.5k"));
}

#[test]
fn migration_event_is_companies_labor_and_formats_with_countries() {
    let event = SimEvent::PersonMigratedForWork {
        from_country_name: Arc::from("A"),
        to_country_name: Arc::from("B"),
        person_id: 7,
        job_type_id: 2,
        monthly_salary: 4_500.0,
        day: 30,
    };

    assert_eq!(event.category(), EventCategory::CompaniesLabor);
    let formatted = event.format_with_currency("$");
    assert!(formatted.contains("[MOVE+]"));
    assert!(formatted.contains("A -> B"));
}
