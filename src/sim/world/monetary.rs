use super::World;
use crate::sim::{event::SimEvent, person::EmploymentStatus};

impl World {
    /// Apply the Taylor Rule to update `self.policy_rate` and propagate a
    /// `RatePolicyChange` event to each country if the move is large enough.
    pub(super) fn update_policy_rate(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        let params = self.countries[0].sim_params.monetary;
        let count = self.countries.len() as f64;

        // Average annualised month-over-month inflation across countries
        let avg_inflation = self
            .countries
            .iter()
            .map(|c| c.monthly_inflation_rate)
            .sum::<f64>()
            / count;

        // Average unemployment rate across countries
        let avg_unemp_rate = self
            .countries
            .iter()
            .map(|c| {
                let labor_force = c
                    .people
                    .iter()
                    .filter(|p| p.is_labor_force_participant())
                    .count();
                let unemployed = c
                    .people
                    .iter()
                    .filter(|p| {
                        p.is_labor_force_participant()
                            && matches!(p.employment, EmploymentStatus::Unemployed)
                    })
                    .count();
                if labor_force > 0 {
                    unemployed as f64 / labor_force as f64
                } else {
                    params.natural_unemployment_rate
                }
            })
            .sum::<f64>()
            / count;

        let old_rate = self.policy_rate;

        // Taylor Rule: r = r* + α*(π - π*) + β*(u* - u)
        let new_rate = (params.neutral_policy_rate
            + params.inflation_coefficient * (avg_inflation - params.target_inflation_rate)
            + params.unemployment_coefficient
                * (params.natural_unemployment_rate - avg_unemp_rate))
            .clamp(params.rate_floor, params.rate_ceiling);

        self.policy_rate = new_rate;

        if (new_rate - old_rate).abs() >= params.rate_change_event_threshold {
            let day = self.day;
            for country in &mut self.countries {
                country.pending_events.push(SimEvent::RatePolicyChange {
                    country_name: country.name.clone(),
                    new_rate,
                    old_rate,
                    inflation_rate: avg_inflation,
                    unemployment_rate: avg_unemp_rate,
                    day,
                });
            }
        }
    }
}
