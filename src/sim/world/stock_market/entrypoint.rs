use super::*;

impl World {
    /// Run the daily stock market for all countries after the goods market.
    pub(in crate::sim::world) fn run_daily_stock_market(&mut self) {
        let mut active_country_indices = Vec::with_capacity(self.countries.len());
        for (idx, country) in self.countries.iter().enumerate() {
            if country.sim_params.stock_market.enabled && !country.stock_market.listed.is_empty() {
                active_country_indices.push(idx);
            }
        }

        for &idx in &active_country_indices {
            run_country_stock_market(&mut self.countries[idx], self.day);
        }
        // Export must run before day-close reset so turnover/volume/day% reflect
        // today's actual trading activity.
        if self.export_queue_active {
            self.push_daily_stock_snapshot();
        }
        // Close every active market day after exports are captured.
        for idx in active_country_indices {
            let country = &mut self.countries[idx];
            country.stock_market.record_day_close(self.day);
            if self.day.is_multiple_of(30) {
                country.stock_market.gc_portfolios();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic hash helpers
// ---------------------------------------------------------------------------
