use super::{Country, SimEvent, World};
use crate::sim::{country::DAYS_PER_MONTH, event::EVENT_LOG_LIMIT, tickable::Tickable};
use rayon::prelude::*;

impl World {
    fn trim_event_log(&mut self) {
        // VecDeque::drain(0..n) drops n elements from the front in O(n) without shifting.
        if self.event_log.len() > EVENT_LOG_LIMIT {
            let evict = EVENT_LOG_LIMIT / 4;
            self.event_log.drain(0..evict);
        }
    }

    pub(super) fn drain_country_events(&mut self) {
        let pending: Vec<SimEvent> = self
            .countries
            .iter_mut()
            .flat_map(Country::drain_pending_events)
            .collect();
        self.event_log.extend(pending);
        self.trim_event_log();
    }

    pub(super) fn update_policy_rate_and_propagate(&mut self) {
        self.update_policy_rate();
        let rate = self.policy_rate;
        for country in &mut self.countries {
            country.set_policy_rate(rate);
        }
    }

    pub(super) fn tick_day_with_hooks<H: WorldTickHooks>(&mut self, hooks: &mut H) {
        self.day = self.day.saturating_add(1);

        hooks.run_country_daily(self);
        hooks.run_daily_goods(self);
        hooks.run_daily_stock(self);

        if self.day.is_multiple_of(DAYS_PER_MONTH) {
            hooks.on_monthly_start();
            hooks.run_monthly_country_processing(self);
            self.process_cross_market_stock_dividends();
            self.cleanup_stale_cross_market_positions();
            self.process_cross_country_deposits();
            self.process_cross_country_bank_stock_investing();
            hooks.run_monthly_tariff_adjust(self);
            hooks.run_monthly_rate_update(self);
            hooks.run_monthly_capture_snapshot(self);
            if self.export_queue_active {
                self.push_export_snapshot();
            }
            hooks.run_monthly_reset_trackers(self);
            hooks.on_monthly_end();
        }
        hooks.run_drain_country_events(self);
    }
}

pub(super) trait WorldTickHooks {
    fn run_country_daily(&mut self, world: &mut World);
    fn run_daily_goods(&mut self, world: &mut World);
    fn run_daily_stock(&mut self, world: &mut World);
    fn on_monthly_start(&mut self) {}
    fn run_monthly_country_processing(&mut self, world: &mut World);
    fn run_monthly_tariff_adjust(&mut self, world: &mut World);
    fn run_monthly_rate_update(&mut self, world: &mut World);
    fn run_monthly_capture_snapshot(&mut self, world: &mut World);
    fn run_monthly_reset_trackers(&mut self, world: &mut World);
    fn on_monthly_end(&mut self) {}
    fn run_drain_country_events(&mut self, world: &mut World);
}

struct StandardWorldTickHooks;

impl StandardWorldTickHooks {
    fn dispatch_countries(world: &mut World, f: impl Fn(&mut Country) + Sync + Send) {
        if world.countries.len() >= 2 {
            world.countries.par_iter_mut().for_each(|c| {
                c.use_inner_parallelism = false;
                f(c);
            });
        } else {
            for c in &mut world.countries {
                c.use_inner_parallelism = true;
                f(c);
            }
        }
    }
}

impl WorldTickHooks for StandardWorldTickHooks {
    fn run_country_daily(&mut self, world: &mut World) {
        Self::dispatch_countries(world, Country::tick_day_without_month);
    }

    fn run_daily_goods(&mut self, world: &mut World) {
        world.run_daily_goods_economy();
    }

    fn run_daily_stock(&mut self, world: &mut World) {
        world.run_daily_stock_market();
    }

    fn run_monthly_country_processing(&mut self, world: &mut World) {
        Self::dispatch_countries(world, Country::process_month_end);
    }

    fn run_monthly_tariff_adjust(&mut self, world: &mut World) {
        world.adjust_dynamic_country_tariffs();
    }

    fn run_monthly_rate_update(&mut self, world: &mut World) {
        world.update_policy_rate_and_propagate();
    }

    fn run_monthly_capture_snapshot(&mut self, world: &mut World) {
        world.capture_snapshot();
    }

    fn run_monthly_reset_trackers(&mut self, world: &mut World) {
        Self::dispatch_countries(world, Country::reset_month_trackers);
    }

    fn run_drain_country_events(&mut self, world: &mut World) {
        world.drain_country_events();
    }
}

impl Tickable for World {
    fn tick_day(&mut self) {
        let mut hooks = StandardWorldTickHooks;
        self.tick_day_with_hooks(&mut hooks);
    }
}
