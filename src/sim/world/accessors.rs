use std::collections::VecDeque;

use super::{
    BankMetricPoint, Country, HISTORY_LIMIT, MetricPoint, MonthlyExportData, SimEvent, World,
};

impl World {
    #[cfg(test)]
    pub fn new(countries: Vec<Country>) -> Self {
        Self::new_with_generation_seed(countries, 0)
    }

    pub fn new_with_generation_seed(countries: Vec<Country>, generation_seed: u64) -> Self {
        let country_count = countries.len();

        let tariff_rates_by_good = Self::build_tariff_rates_from_countries(&countries);
        let mut world = Self {
            day: 0,
            generation_seed,
            countries,
            paused: false,
            policy_rate: 0.0,
            world_history: VecDeque::with_capacity(HISTORY_LIMIT),
            country_history: vec![VecDeque::with_capacity(HISTORY_LIMIT); country_count],
            bank_history: std::collections::HashMap::new(),
            event_log: VecDeque::with_capacity(crate::sim::event::EVENT_LOG_LIMIT),
            export_queue: Vec::new(),
            export_queue_active: false,
            stock_country_export_queue: Vec::new(),
            stock_equity_export_queue: Vec::new(),
            monthly_export_caches: Vec::new(),
            tariff_rates_by_good,
            goods_runtime: super::goods_market::GoodsMarketDayCache::default(),
        };
        world.initialize_fx_state();
        world.refresh_policy_rate_aggregate();
        world.refresh_monthly_export_caches();
        world.capture_snapshot();
        world
    }

    pub fn total_population(&self) -> usize {
        self.countries.iter().map(Country::population).sum()
    }

    pub fn day(&self) -> u64 {
        self.day
    }

    pub fn generation_seed(&self) -> u64 {
        self.generation_seed
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn policy_rate(&self) -> f64 {
        self.population_weighted_policy_rate()
    }

    pub fn countries(&self) -> &[Country] {
        &self.countries
    }

    pub fn country(&self, index: usize) -> Option<&Country> {
        self.countries.get(index)
    }

    pub fn country_mut(&mut self, index: usize) -> Option<&mut Country> {
        self.countries.get_mut(index)
    }

    pub fn country_count(&self) -> usize {
        self.countries.len()
    }

    pub fn world_history(&self) -> &VecDeque<MetricPoint> {
        &self.world_history
    }

    pub fn country_history(&self) -> &[VecDeque<MetricPoint>] {
        &self.country_history
    }

    pub fn bank_history(
        &self,
    ) -> &std::collections::HashMap<(usize, u16), VecDeque<BankMetricPoint>> {
        &self.bank_history
    }

    pub fn event_log(&self) -> &VecDeque<SimEvent> {
        &self.event_log
    }

    pub(crate) fn push_export_row(&mut self, row: MonthlyExportData) {
        self.export_queue.push(row);
    }

    #[cfg(test)]
    pub fn completed_month_boundaries(&self) -> u64 {
        self.day / crate::sim::country::DAYS_PER_MONTH
    }
}
