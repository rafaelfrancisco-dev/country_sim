use super::*;

impl World {
    /// Run the daily stock market for all countries after the goods market.
    pub(in crate::sim::world) fn run_daily_stock_market(&mut self) {
        let active_country_indices = self
            .countries
            .iter()
            .enumerate()
            .filter_map(|(idx, country)| {
                (country.sim_params().stock_market.enabled
                    && !country.stock_market().listed.is_empty())
                .then_some(idx)
            })
            .collect::<Vec<_>>();
        run_world_stock_market(self, self.day);
        // Export must run before day-close reset so turnover/volume/day% reflect
        // today's actual trading activity.
        if self.export_queue_active {
            self.push_daily_stock_snapshot();
        }
        // Close every active market day after exports are captured.
        for idx in active_country_indices {
            let bank_exposure = self.market_bank_portfolio_value(self.countries[idx].id());
            let country = &mut self.countries[idx];
            country.stock_market_mut().last_bank_equity_exposure = bank_exposure;
            country
                .stock_market_mut()
                .record_day_close(self.day, bank_exposure);
            if self.day.is_multiple_of(30) {
                country.stock_market_mut().gc_portfolios();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic hash helpers
// ---------------------------------------------------------------------------
