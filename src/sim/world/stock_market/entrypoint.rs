use super::*;

impl World {
    /// Run the daily stock market for all countries after the goods market.
    pub(in crate::sim::world) fn run_daily_stock_market(&mut self) {
        for country in &mut self.countries {
            if !country.sim_params.stock_market.enabled {
                continue;
            }
            if country.stock_market.listed.is_empty() {
                continue;
            }
            run_country_stock_market(country, self.day);
        }
        // Export must run before day-close reset so turnover/volume/day% reflect
        // today's actual trading activity.
        if self.export_queue_active {
            self.push_daily_stock_snapshot();
        }
        // Close every active market day after exports are captured.
        for country in &mut self.countries {
            if !country.sim_params.stock_market.enabled {
                continue;
            }
            if country.stock_market.listed.is_empty() {
                continue;
            }
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


