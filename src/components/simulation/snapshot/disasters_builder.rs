use super::*;

pub(in crate::components::simulation) fn build_disasters_tab_snapshot(
    world: &World,
    ui: &UiState,
) -> DisastersTabSnapshot {
    let selected_country = ui.selected_country(world);
    let selected_country_name = selected_country
        .map(|country| country.name.clone())
        .unwrap_or_else(|| "N/A".to_string());
    let population = selected_country
        .map(|country| compact_count(country.population() as f64))
        .unwrap_or_else(|| "0".to_string());
    let companies = selected_country
        .map(|country| country.companies.len().to_string())
        .unwrap_or_else(|| "0".to_string());
    let rows = crate::sim::ALL_DISASTER_KINDS
        .iter()
        .enumerate()
        .map(|(idx, kind)| DisasterRowSnapshot {
            label: kind.label().to_string(),
            enabled: ui.disaster_enabled.get(idx).copied().unwrap_or(false),
            selected_intensity_idx: ui
                .disaster_intensity_index
                .get(idx)
                .copied()
                .unwrap_or(0)
                .min(
                    crate::sim::DISASTER_INTENSITY_LEVELS
                        .len()
                        .saturating_sub(1),
                ),
            intensity_options: crate::sim::DISASTER_INTENSITY_LEVELS,
            is_selected: idx == ui.disaster_selected_row,
        })
        .collect::<Vec<_>>();

    DisastersTabSnapshot {
        selected_country_name,
        population,
        companies,
        rows,
        last_apply_status: ui.disaster_last_apply_status.clone(),
    }
}
