use super::*;

#[test]
fn category_labels_are_human_readable() {
    assert_eq!(EventCategory::StockMarket.label(), "Stock Market");
    assert_eq!(EventCategory::BirthsDeaths.label(), "Births/Deaths");
}

#[test]
fn event_category_mapping_covers_all_variant_groups() {
    let mut seen = [false; 6];

    let samples = [
        SimEvent::PopBirth {
            country_name: "A".to_string(),
            count: 1,
            day: 1,
        },
        SimEvent::LoanEmitted {
            country_name: "A".to_string(),
            entity_kind: EntityKind::Person,
            entity_id: 1,
            principal: 10.0,
            day: 1,
        },
        SimEvent::DebtDistress {
            country_name: "A".to_string(),
            entity_kind: EntityKind::Company,
            entity_id: 2,
            unpaid_amount: 3.0,
            day: 1,
        },
        SimEvent::CompanyFounded {
            country_name: "A".to_string(),
            company_id: 1,
            company_name: "Comp".to_string(),
            founders: 1,
            job_type_id: 1,
            produced_good_id: 1,
            day: 1,
        },
        SimEvent::StockIpo {
            country_name: "A".to_string(),
            company_id: 1,
            company_name: "Comp".to_string(),
            ticker: "CMP".to_string(),
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
            country_name: "A".to_string(),
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
        };
        seen[idx] = true;
    }

    assert!(seen.into_iter().all(|covered| covered));
}
