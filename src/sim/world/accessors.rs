use std::collections::{BTreeMap, HashMap, VecDeque};

use super::{
    BankMetricPoint, Country, Currency, CurrencySpec, MetricPoint, MonthlyExportData, SimEvent,
    World, HISTORY_LIMIT,
};

impl World {
    #[cfg(test)]
    pub fn new(countries: Vec<Country>) -> Self {
        Self::new_with_generation_seed(countries, 0)
    }

    pub fn new_with_generation_seed(mut countries: Vec<Country>, generation_seed: u64) -> Self {
        Self::assign_currency_state_from_specs(&mut countries);
        let country_count = countries.len();

        let tariff_rates_by_good = Self::build_tariff_rates_from_countries(&countries);
        let import_tariff_rates_by_good =
            Self::build_import_tariff_rates_from_countries(&countries);
        let (currencies, currency_index_by_spec) =
            Self::build_currency_pool_from_countries(&countries);
        let mut world = Self {
            day: 0,
            generation_seed,
            countries,
            paused: false,
            policy_rate: 0.0,
            world_history: VecDeque::with_capacity(HISTORY_LIMIT),
            country_history: vec![VecDeque::with_capacity(HISTORY_LIMIT); country_count],
            country_fx_source_history: vec![std::collections::BTreeMap::new(); country_count],
            bank_history: std::collections::HashMap::new(),
            event_log: VecDeque::with_capacity(crate::sim::event::EVENT_LOG_LIMIT),
            export_queue: Vec::new(),
            export_queue_active: false,
            stock_country_export_queue: Vec::new(),
            stock_equity_export_queue: Vec::new(),
            monthly_export_caches: Vec::new(),
            tariff_rates_by_good,
            import_tariff_rates_by_good,
            goods_runtime: super::goods_market::GoodsMarketDayCache::default(),
            currencies,
            currency_index_by_spec,
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

    pub fn currencies(&self) -> &[Currency] {
        &self.currencies
    }

    pub fn currency_by_idx(&self, idx: usize) -> Option<&Currency> {
        self.currencies.get(idx)
    }

    pub fn currency_index_by_spec(&self) -> &HashMap<CurrencySpec, usize> {
        &self.currency_index_by_spec
    }

    /// Construct a `World` from deserialized persistence data.
    /// Sets ephemeral cache fields to defaults and calls required
    /// post-construction refresh methods.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_for_persistence(
        day: u64,
        generation_seed: u64,
        countries: Vec<Country>,
        paused: bool,
        policy_rate: f64,
        world_history: VecDeque<MetricPoint>,
        country_history: Vec<VecDeque<MetricPoint>>,
        country_fx_source_history: Vec<BTreeMap<usize, VecDeque<f64>>>,
        bank_history: HashMap<(usize, u16), VecDeque<BankMetricPoint>>,
        event_log: VecDeque<SimEvent>,
        tariff_rates_by_good: Vec<Vec<f64>>,
    ) -> Self {
        let import_tariff_rates_by_good =
            Self::build_import_tariff_rates_from_countries(&countries);
        let (currencies, currency_index_by_spec) =
            Self::build_currency_pool_from_countries(&countries);
        let mut world = Self {
            day,
            generation_seed,
            countries,
            paused,
            policy_rate,
            world_history,
            country_history,
            country_fx_source_history,
            bank_history,
            event_log,
            export_queue: Vec::new(),
            export_queue_active: false,
            stock_country_export_queue: Vec::new(),
            stock_equity_export_queue: Vec::new(),
            monthly_export_caches: Vec::new(),
            tariff_rates_by_good,
            import_tariff_rates_by_good,
            goods_runtime: super::goods_market::GoodsMarketDayCache::default(),
            currencies,
            currency_index_by_spec,
        };
        world.refresh_monthly_export_caches();
        world.refresh_policy_rate_aggregate();
        world
    }

    #[cfg(test)]
    pub fn completed_month_boundaries(&self) -> u64 {
        self.day / crate::sim::country::DAYS_PER_MONTH
    }

    /// Assign `currency_idx` and `is_currency_issuer` based on each country's `currency_spec`.
    /// The first country seen with each unique spec becomes the issuer; subsequent ones are not.
    /// Called from `new_with_generation_seed` so that manually-constructed worlds (tests, etc.)
    /// get consistent currency state without requiring a separate generator step.
    fn assign_currency_state_from_specs(countries: &mut [Country]) {
        let mut currency_index_by_spec: HashMap<CurrencySpec, usize> = HashMap::new();
        let mut issuer_by_currency_idx: HashMap<usize, usize> = HashMap::new();
        let mut currency_counter = 0usize;

        for (vec_pos, country) in countries.iter_mut().enumerate() {
            let currency_idx = *currency_index_by_spec
                .entry(country.currency_spec.clone())
                .or_insert_with(|| {
                    let idx = currency_counter;
                    currency_counter += 1;
                    idx
                });

            let issuer_pos = *issuer_by_currency_idx
                .entry(currency_idx)
                .or_insert(vec_pos);

            country.currency_idx = currency_idx;
            country.is_currency_issuer = issuer_pos == vec_pos;
        }
    }

    /// Build the currency pool from countries that already have currency_idx assigned.
    /// Returns (currencies Vec, currency_index_by_spec HashMap).
    ///
    /// The returned `currencies` Vec is indexed so that `currencies[i]` is the currency
    /// whose `currency_idx == i`, matching every country's `country.currency_idx`.
    fn build_currency_pool_from_countries(
        countries: &[Country],
    ) -> (Vec<Currency>, HashMap<CurrencySpec, usize>) {
        // Collect one entry per unique currency_idx, keyed by currency_idx.
        // Also record the Vec position (not country.id) of the issuer.
        let mut entries: std::collections::HashMap<usize, (CurrencySpec, usize)> =
            std::collections::HashMap::new();

        for (vec_pos, country) in countries.iter().enumerate() {
            entries
                .entry(country.currency_idx)
                .or_insert_with(|| (country.currency_spec.clone(), usize::MAX));

            if country.is_currency_issuer {
                if let Some(entry) = entries.get_mut(&country.currency_idx) {
                    entry.1 = vec_pos;
                }
            }
        }

        // Build the Vec ordered by currency_idx so Vec[i] == currency with idx i.
        let max_idx = entries.keys().copied().max().map_or(0, |m| m + 1);
        let currencies: Vec<Currency> = (0..max_idx)
            .map(|idx| {
                let (spec, issuer_vec_pos) = entries
                    .remove(&idx)
                    .unwrap_or_else(|| (CurrencySpec::new("???", "Unknown", "?"), 0));
                Currency {
                    spec,
                    issuer_country_idx: issuer_vec_pos,
                }
            })
            .collect();

        // currency_index_by_spec maps each spec back to its currency_idx.
        let currency_index_by_spec: HashMap<CurrencySpec, usize> = currencies
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.spec.clone(), idx))
            .collect();

        (currencies, currency_index_by_spec)
    }
}
