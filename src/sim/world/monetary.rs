use super::World;

impl World {
    pub(super) fn population_weighted_policy_rate(&self) -> f64 {
        let mut total_population = 0.0_f64;
        let mut weighted_policy_sum = 0.0_f64;

        for country in &self.countries {
            let population = country.population() as f64;
            total_population += population;
            weighted_policy_sum += country.policy_rate() * population;
        }

        if total_population > 0.0 {
            weighted_policy_sum / total_population
        } else {
            0.0
        }
    }

    pub(super) fn refresh_policy_rate_aggregate(&mut self) {
        self.policy_rate = self.population_weighted_policy_rate();
    }
}
