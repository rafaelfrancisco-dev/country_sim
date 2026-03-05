use super::*;
use crate::sim::DisasterRequest;

impl UiState {
    pub(super) fn adjust_disaster_intensity(&mut self, delta: isize) {
        let Some(entry) = self
            .disaster_intensity_index
            .get_mut(self.disaster_selected_row)
        else {
            return;
        };
        let max = DISASTER_INTENSITY_COUNT.saturating_sub(1);
        if delta < 0 {
            *entry = entry.saturating_sub(delta.unsigned_abs());
        } else {
            *entry = entry.saturating_add(delta as usize).min(max);
        }
    }

    pub(super) fn build_disaster_requests(&self) -> Vec<DisasterRequest> {
        crate::sim::ALL_DISASTER_KINDS
            .iter()
            .enumerate()
            .filter_map(|(idx, kind)| {
                if !self.disaster_enabled.get(idx).copied().unwrap_or(false) {
                    return None;
                }
                let intensity_idx = self.disaster_intensity_index.get(idx).copied().unwrap_or(0);
                let intensity_pct = crate::sim::DISASTER_INTENSITY_LEVELS
                    .get(intensity_idx)
                    .copied()
                    .unwrap_or(crate::sim::DISASTER_INTENSITY_LEVELS[0]);
                Some(DisasterRequest {
                    kind: *kind,
                    intensity_pct,
                })
            })
            .collect()
    }
}
