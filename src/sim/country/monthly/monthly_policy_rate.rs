use super::*;

impl Country {
    pub(crate) fn update_policy_rate(&mut self) {
        let monetary = self.sim_params.monetary;
        let unemployment_rate = self.policy_unemployment_rate();
        let old_rate = self.policy_rate;
        let target_rate = (monetary.neutral_policy_rate
            + monetary.inflation_coefficient
                * (self.monthly_inflation_rate - monetary.target_inflation_rate)
            + monetary.unemployment_coefficient
                * (monetary.natural_unemployment_rate - unemployment_rate))
            .clamp(monetary.rate_floor, monetary.rate_ceiling);

        let step = monetary.policy_rate_step.max(0.0001);
        let clamped_delta = (target_rate - old_rate).clamp(-step, step);
        let new_rate = ((old_rate + clamped_delta) / step).round() * step;
        let new_rate = new_rate.clamp(monetary.rate_floor, monetary.rate_ceiling);
        self.policy_rate = new_rate;

        if (new_rate - old_rate).abs() >= monetary.rate_change_event_threshold {
            self.pending_events.push(SimEvent::RatePolicyChange {
                country_name: self.name.clone(),
                new_rate,
                old_rate,
                inflation_rate: self.monthly_inflation_rate,
                unemployment_rate,
                day: self.day,
            });
        }
    }

    pub(crate) fn policy_unemployment_rate(&self) -> f64 {
        let unemployment_rate = self.labor_force_unemployment_rate();
        if unemployment_rate > 0.0 || self.people.iter().any(Person::is_labor_force_participant) {
            unemployment_rate
        } else {
            self.sim_params.monetary.natural_unemployment_rate
        }
    }
}
