use tokio::sync::oneshot;

use super::Simulation;
use crate::sim::country::Country;

impl Simulation {
    pub(super) fn reconcile_monthly_processing(&mut self) {
        self.poll_monthly_result();
        self.refresh_monthly_queue();
        self.start_monthly_tick_if_idle();
    }

    pub(super) fn should_pause_daily_ticks_for_monthly_catchup(&self) -> bool {
        let Some(world) = self.engine.world() else {
            return false;
        };

        let required_months = world.completed_month_boundaries();
        let processed_months = world
            .countries
            .iter()
            .map(|country| country.month)
            .min()
            .unwrap_or(0);

        required_months.saturating_sub(processed_months) > 1
    }

    fn poll_monthly_result(&mut self) {
        let mut completed: Option<Vec<Country>> = None;
        let mut clear_inflight = false;

        if let Some(receiver) = &mut self.monthly_tick_inflight {
            match receiver.try_recv() {
                Ok(countries) => {
                    completed = Some(countries);
                    clear_inflight = true;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(oneshot::error::TryRecvError::Closed) => {
                    clear_inflight = true;
                }
            }
        }

        if clear_inflight {
            self.monthly_tick_inflight = None;
        }

        if let Some(countries) = completed {
            if let Some(world) = self.engine.world_mut() {
                world.apply_processed_month(countries);
            }
        }
    }

    fn refresh_monthly_queue(&mut self) {
        let Some(world) = self.engine.world() else {
            self.monthly_tick_inflight = None;
            self.pending_monthly_ticks = 0;
            self.observed_month_boundaries = 0;
            return;
        };

        let completed_boundaries = world.completed_month_boundaries();
        if completed_boundaries < self.observed_month_boundaries {
            self.pending_monthly_ticks = 0;
        } else if completed_boundaries > self.observed_month_boundaries {
            self.pending_monthly_ticks += completed_boundaries - self.observed_month_boundaries;
        }
        self.observed_month_boundaries = completed_boundaries;
    }

    fn start_monthly_tick_if_idle(&mut self) {
        if self.monthly_tick_inflight.is_some() || self.pending_monthly_ticks == 0 {
            return;
        }

        let Some(world) = self.engine.world() else {
            return;
        };
        let countries = world.countries.clone();

        let (tx, rx) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let mut processed = countries;
            for country in &mut processed {
                country.process_month();
            }

            let _ = tx.send(processed);
        });

        self.pending_monthly_ticks = self.pending_monthly_ticks.saturating_sub(1);
        self.monthly_tick_inflight = Some(rx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::GameCommand;

    fn setup_running_world(simulation: &mut Simulation) {
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupNextField);
        simulation.engine.apply(GameCommand::SetupEditDigit(1));
        simulation.engine.apply(GameCommand::SetupSubmit);
    }

    #[test]
    fn monthly_catchup_gate_blocks_only_when_more_than_one_month_behind() {
        let mut simulation = Simulation::new();
        setup_running_world(&mut simulation);

        {
            let world = simulation
                .engine
                .world_mut()
                .expect("running world should exist");
            world.day = 30;
            for country in &mut world.countries {
                country.day = 30;
                country.month = 0;
            }
        }
        assert!(!simulation.should_pause_daily_ticks_for_monthly_catchup());

        {
            let world = simulation
                .engine
                .world_mut()
                .expect("running world should exist");
            world.day = 60;
            for country in &mut world.countries {
                country.day = 60;
                country.month = 0;
            }
        }
        assert!(simulation.should_pause_daily_ticks_for_monthly_catchup());

        {
            let world = simulation
                .engine
                .world_mut()
                .expect("running world should exist");
            for country in &mut world.countries {
                country.month = 1;
            }
        }
        assert!(!simulation.should_pause_daily_ticks_for_monthly_catchup());
    }
}
