use std::sync::Arc;

use super::*;
use crate::components::simulation::snapshot::FxReserveEntry;

fn sample_fx_tab(summary_lines: usize, pair_lines: usize, breakdown_lines: usize) -> FxTabSnapshot {
    FxTabSnapshot {
        selected_country_name: "A".to_string(),
        selected_currency_code: "AAA".to_string(),
        selected_currency_name: "Alpha".to_string(),
        selected_currency_symbol: "$".to_string(),
        selected_fx_summary_lines: vec!["line".to_string(); summary_lines],
        pair_board_lines: vec!["pair".to_string(); pair_lines],
        fx_rows: Vec::new(),
        selected_reserves_breakdown: (0..breakdown_lines)
            .map(|_| FxReserveEntry {
                is_unattributed: false,
                currency_symbol: "¥".to_string(),
                currency_code: "JPY".to_string(),
                amount_local: 10.0,
            })
            .collect(),
        selected_fx_rate_history: Arc::default(),
        selected_fx_reserves_by_currency: Vec::new(),
        selected_fx_net_flow_history: Arc::default(),
        selected_fx_pressure_history: Arc::default(),
    }
}

#[test]
fn fx_top_height_grows_to_fit_reserve_breakdown_when_space_is_available() {
    let tab = sample_fx_tab(7, 5, 3);

    assert_eq!(Simulation::fx_top_height(40, &tab), 12);
}

#[test]
fn fx_top_height_stays_at_baseline_when_terminal_has_no_spare_height() {
    let tab = sample_fx_tab(7, 5, 3);

    assert_eq!(Simulation::fx_top_height(32, &tab), 10);
}
