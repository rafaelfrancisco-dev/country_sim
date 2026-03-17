use super::*;
use crate::sim::good::{good_by_id, recipe_for_good, GOODS_COUNT};
use crate::sim::params::CompanyFormationParams;

const STARTUP_INITIAL_STAFFING_BUFFER: u16 = 1;

impl Country {
    pub(in crate::sim::country::monthly) fn startup_target_openings(
        &self,
        founders_count: u16,
        formation: CompanyFormationParams,
    ) -> u16 {
        let labor_slack =
            (self.labor_force_unemployment_rate() - formation.labor_slack_startup_trigger).max(0.0);
        let demand_safe_cap = founders_count.saturating_add(STARTUP_INITIAL_STAFFING_BUFFER);
        formation
            .startup_openings_min
            .saturating_add((labor_slack * 20.0).round() as u16)
            .clamp(
                formation.startup_openings_min,
                formation
                    .startup_openings_max
                    .max(formation.startup_openings_min),
            )
            .min(demand_safe_cap.max(1))
    }

    pub(in crate::sim::country::monthly) fn planned_startup_staffed_headcount(
        &self,
        founders_count: u16,
        _target_openings: u16,
        founder_openings: u16,
    ) -> u32 {
        u32::from(founders_count.saturating_add(founder_openings))
    }

    pub(in crate::sim::country::monthly) fn startup_candidate_goods(
        &self,
        margin_threshold: f64,
        necessity_sales_floor: f64,
    ) -> Vec<u8> {
        let mut sales_by_good = [0.0_f64; GOODS_COUNT];
        let mut producer_count = [0_u32; GOODS_COUNT];
        let mut profit_by_good = [0.0_f64; GOODS_COUNT];
        for company in &self.companies {
            if company.is_public_utility {
                continue;
            }

            let good_idx = company.produced_good_id as usize;
            if good_idx >= GOODS_COUNT {
                continue;
            }
            sales_by_good[good_idx] =
                round_cents(sales_by_good[good_idx] + company.last_month_sales);
            profit_by_good[good_idx] =
                round_cents(profit_by_good[good_idx] + company.last_month_profit);
            producer_count[good_idx] = producer_count[good_idx].saturating_add(1);
        }

        let mut ranked = Vec::new();
        let mut best_necessity: Option<(u8, f64)> = None;
        for good_id in 0..=4_u8 {
            if production_job_for_good(good_id).is_none() {
                continue;
            }
            let sales = sales_by_good[good_id as usize];
            if sales < necessity_sales_floor {
                continue;
            }
            let producers = producer_count[good_id as usize].max(1);
            let score = sales / f64::from(producers);
            match best_necessity {
                None => best_necessity = Some((good_id, score)),
                Some((_, best_score)) if score > best_score => {
                    best_necessity = Some((good_id, score))
                }
                _ => {}
            }
        }
        if let Some((good_id, _)) = best_necessity {
            ranked.push(good_id);
        }

        let mut high_margin = Vec::new();
        for good_id in 0..GOODS_COUNT as u8 {
            if ranked.contains(&good_id) || production_job_for_good(good_id).is_none() {
                continue;
            }
            let sales = sales_by_good[good_id as usize];
            if sales <= 0.0 {
                continue;
            }
            let margin = profit_by_good[good_id as usize] / sales;
            if margin < margin_threshold {
                continue;
            }
            high_margin.push((good_id, margin));
        }
        high_margin.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked.extend(high_margin.into_iter().map(|(good_id, _)| good_id));
        ranked
    }

    pub(in crate::sim::country::monthly) fn expected_revenue_for_good(&self, good_id: u8) -> f64 {
        let mut total_sales = 0.0_f64;
        let mut total_price = 0.0_f64;
        let mut producers = 0_u32;
        for company in &self.companies {
            if company.is_public_utility || company.produced_good_id != good_id {
                continue;
            }
            total_sales = round_cents(total_sales + company.last_month_sales);
            total_price = round_cents(total_price + company.unit_price);
            producers = producers.saturating_add(1);
        }

        let Some(good) = good_by_id(good_id) else {
            return 0.0;
        };
        let observed_private_average = if producers == 0 {
            0.0
        } else {
            round_cents(total_sales / f64::from(producers))
        };
        let observed_entrant_revenue = if producers == 0 {
            0.0
        } else {
            round_cents(total_sales / (f64::from(producers) + 1.0))
        };
        let market_price = self
            .average_price_for_good(good_id)
            .max(if producers == 0 {
                0.0
            } else {
                round_cents(total_price / f64::from(producers))
            })
            .max(minimum_viable_unit_price(
                good_id,
                self.sim_params.goods_market.worker_effort_per_month,
                self.sim_params.goods_market.min_margin_rate,
            ));
        let active_population = self
            .people
            .iter()
            .filter(|person| person.can_seek_job())
            .count();
        let demand_units = active_population as f64 * good.monthly_demand_factor.max(0.0);
        let incumbent_count = producers.max(1) as f64;
        let entrant_share = (1.0 / (incumbent_count + 1.0)).clamp(0.05, 0.25);
        let consumer_demand_proxy = round_cents(demand_units * market_price * entrant_share);
        let company_demand_proxy = self.company_input_demand_proxy_value(good_id, market_price);
        let proxy_demand = round_cents(consumer_demand_proxy.max(company_demand_proxy));

        if producers == 0 {
            proxy_demand
        } else {
            round_cents(
                proxy_demand
                    .max(observed_entrant_revenue)
                    .min(observed_private_average),
            )
        }
    }

    fn estimate_startup_monthly_output_units(&self, good_id: u8, staffed_workers: u32) -> f64 {
        let Some(good) = good_by_id(good_id) else {
            return 0.0;
        };
        if staffed_workers == 0 || good.effort_per_unit <= 0.0 {
            return 0.0;
        }

        (f64::from(staffed_workers) * self.sim_params.goods_market.worker_effort_per_month
            / good.effort_per_unit)
            .max(0.0)
    }

    pub(in crate::sim::country::monthly) fn estimate_startup_monthly_input_cost(
        &self,
        good_id: u8,
        staffed_workers: u32,
        unit_price: f64,
        expected_revenue: f64,
    ) -> f64 {
        if unit_price <= 0.0 {
            return 0.0;
        }

        let sellable_units = self
            .estimate_startup_monthly_output_units(good_id, staffed_workers)
            .min(expected_revenue / unit_price);
        round_cents(
            recipe_for_good(good_id)
                .inputs()
                .map(|(input_good_id, required_per_unit)| {
                    let unit_cost = self
                        .average_price_for_good(input_good_id)
                        .max(minimum_viable_unit_price(
                            input_good_id,
                            self.sim_params.goods_market.worker_effort_per_month,
                            self.sim_params.goods_market.min_margin_rate,
                        ));
                    sellable_units * f64::from(required_per_unit) * unit_cost
                })
                .sum(),
        )
    }

    pub(in crate::sim::country::monthly) fn estimate_startup_monthly_facility_cost(
        &self,
        staffed_headcount: u32,
    ) -> f64 {
        if staffed_headcount == 0 || self.monthly_rent <= 0.0 {
            return 0.0;
        }

        round_cents(self.monthly_rent * f64::from(staffed_headcount.div_ceil(12).max(1)))
    }

    pub(in crate::sim::country::monthly) fn estimate_startup_monthly_profit(
        &self,
        good_id: u8,
        staffed_workers: u32,
        unit_price: f64,
        expected_revenue: f64,
        payroll: f64,
        input_cost: f64,
        facility_cost: f64,
        projected_loan_payment: f64,
    ) -> f64 {
        let capacity_revenue = round_cents(
            self.estimate_startup_monthly_output_units(good_id, staffed_workers) * unit_price,
        );
        let realizable_revenue = round_cents(expected_revenue.min(capacity_revenue));
        round_cents(
            realizable_revenue - payroll - input_cost - facility_cost - projected_loan_payment,
        )
    }

    fn company_input_demand_proxy_value(&self, input_good_id: u8, market_price: f64) -> f64 {
        let total_required_units = self
            .companies
            .iter()
            .filter(|company| !company.is_public_utility)
            .map(|company| {
                let required_per_unit = company
                    .recipe_for_output_good()
                    .units_required_for(input_good_id);
                if required_per_unit == 0 {
                    return 0.0;
                }

                let output_units = company
                    .last_month_units_produced
                    .max(company.last_completed_month_units_produced)
                    .max(company.last_month_units_sold)
                    .max(company.last_completed_month_units_sold);
                f64::from(output_units) * f64::from(required_per_unit)
            })
            .sum::<f64>();

        round_cents(total_required_units * market_price)
    }

    pub(in crate::sim::country::monthly) fn average_price_for_good(&self, good_id: u8) -> f64 {
        let mut total = 0.0;
        let mut count = 0_u32;

        for company in &self.companies {
            if company.produced_good_id == good_id {
                total += company.unit_price;
                count = count.saturating_add(1);
            }
        }

        if count == 0 {
            let min_price = minimum_viable_unit_price(
                good_id,
                self.sim_params.goods_market.worker_effort_per_month,
                self.sim_params.goods_market.min_margin_rate,
            );
            return round_cents(min_price);
        }

        round_cents(total / f64::from(count))
    }
}
