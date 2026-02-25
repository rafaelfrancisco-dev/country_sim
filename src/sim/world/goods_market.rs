use super::World;
use crate::sim::{
    good::{GOODS_COUNT, GoodId, common_goods},
    money::{from_cents, round_cents, to_cents},
};

const ESSENTIAL_GOODS_COUNT: u8 = 4;
const DYNAMIC_TARIFF_INCREASE_STEP: f64 = 0.05;
const MAX_DYNAMIC_TARIFF_RATE: f64 = 0.75;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SellerQuote {
    seller_country_idx: usize,
    seller_company_idx: usize,
    base_price_cents: i64,
    tariff_cents: i64,
    landed_price_cents: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PurchaseAttempt {
    Bought,
    Unmet,
}

impl World {
    pub(super) fn run_daily_goods_economy(&mut self) {
        self.produce_goods_daily();
        self.process_global_goods_market();
        for country in &mut self.countries {
            country.recompute_aggregates();
        }
    }

    fn produce_goods_daily(&mut self) {
        for country in &mut self.countries {
            let market = country.sim_params.goods_market;
            let daily_effort_per_worker = market.worker_effort_per_month / 30.0;

            for company in &mut country.companies {
                let Some(good) = common_goods().get(company.produced_good_id as usize) else {
                    continue;
                };
                if good.effort_per_unit <= 0.0 {
                    continue;
                }

                company.production_effort_carry +=
                    company.manufacturing_employee_count as f64 * daily_effort_per_worker;
                let produced = (company.production_effort_carry / good.effort_per_unit)
                    .floor()
                    .max(0.0) as u32;
                if produced == 0 {
                    continue;
                }

                company.production_effort_carry -= produced as f64 * good.effort_per_unit;
                company.inventory_units = company.inventory_units.saturating_add(produced);
                company.last_month_units_produced =
                    company.last_month_units_produced.saturating_add(produced);
            }
        }
    }

    pub(super) fn process_global_goods_market(&mut self) {
        if self.countries.is_empty() {
            return;
        }

        let country_count = self.countries.len();
        let cycle_seed = self
            .countries
            .first()
            .map_or(0, |country| country.month.saturating_add(1));
        let start_country_idx = (cycle_seed as usize) % country_count;

        let mut discretionary_budgets = self.initial_discretionary_budgets();

        for good in common_goods() {
            self.process_good_round_robin(
                good.id,
                good.necessity_rank,
                start_country_idx,
                &mut discretionary_budgets,
            );
        }
    }

    pub(super) fn adjust_dynamic_country_tariffs(&mut self) {
        for country in &mut self.countries {
            let mut exported_units_by_good = [0_u32; GOODS_COUNT];
            let mut inventory_by_good = [0_u32; GOODS_COUNT];
            let mut unmet_need_by_good = [false; GOODS_COUNT];

            for company in &country.companies {
                let good_idx = company.produced_good_id as usize;
                if good_idx >= GOODS_COUNT {
                    continue;
                }

                exported_units_by_good[good_idx] = exported_units_by_good[good_idx]
                    .saturating_add(company.last_month_export_units_sold);
                inventory_by_good[good_idx] =
                    inventory_by_good[good_idx].saturating_add(company.inventory_units);
            }

            for person in &country.people {
                for good_idx in 0..GOODS_COUNT {
                    let bit = 1_u16 << (good_idx as u8);
                    if person.last_month_unmet_needs_mask & bit != 0 {
                        unmet_need_by_good[good_idx] = true;
                    }
                }
            }

            for good_idx in 0..GOODS_COUNT {
                let exported = exported_units_by_good[good_idx] > 0;
                let depleted_domestic_stock = inventory_by_good[good_idx] == 0;
                let unmet_domestic_need = unmet_need_by_good[good_idx];

                if exported && depleted_domestic_stock && unmet_domestic_need {
                    country.export_tariffs_by_good[good_idx] =
                        (country.export_tariffs_by_good[good_idx] + DYNAMIC_TARIFF_INCREASE_STEP)
                            .min(MAX_DYNAMIC_TARIFF_RATE);
                }
            }
        }
    }

    fn initial_discretionary_budgets(&self) -> Vec<Vec<i64>> {
        let mut budgets = Vec::with_capacity(self.countries.len());

        for country in &self.countries {
            let month = country.month.saturating_add(1);
            let country_id = country.id;
            let mut country_budgets = Vec::with_capacity(country.people.len());

            for person in &country.people {
                let budget_seed = hash64(
                    (u64::from(country_id) << 48)
                        ^ (month << 24)
                        ^ u64::from(person.id)
                        ^ u64::from(person.name_seed),
                );
                let spend_percent = 5.0 + (budget_seed % 11) as f64;
                country_budgets.push(to_cents(
                    person.cash_balance.max(0.0) * spend_percent / 100.0 / 30.0,
                ));
            }

            budgets.push(country_budgets);
        }

        budgets
    }

    fn process_good_round_robin(
        &mut self,
        good_id: GoodId,
        necessity_rank: u8,
        start_country_idx: usize,
        discretionary_budgets: &mut [Vec<i64>],
    ) {
        let country_count = self.countries.len();
        let mut pending_demands = Vec::with_capacity(country_count);
        let mut needs_met = Vec::with_capacity(country_count);
        let mut needs_unmet = Vec::with_capacity(country_count);
        let mut max_people = 0_usize;

        for country in &self.countries {
            max_people = max_people.max(country.people.len());

            let country_id = country.id;

            let mut country_pending = Vec::with_capacity(country.people.len());
            let mut country_met = Vec::with_capacity(country.people.len());
            let mut country_unmet = Vec::with_capacity(country.people.len());

            for person in &country.people {
                country_pending.push(demand_units_for_person_good(
                    self.day,
                    country_id,
                    person.id,
                    person.name_seed,
                    good_id,
                    common_goods()[good_id as usize].monthly_demand_factor,
                ));
                country_met.push(false);
                country_unmet.push(false);
            }

            pending_demands.push(country_pending);
            needs_met.push(country_met);
            needs_unmet.push(country_unmet);
        }

        let essential = necessity_rank <= ESSENTIAL_GOODS_COUNT;

        loop {
            let mut any_pending = false;
            let mut progress = false;

            for person_idx in 0..max_people {
                for offset in 0..country_count {
                    let buyer_country_idx = (start_country_idx + offset) % country_count;
                    let Some(country_pending) = pending_demands.get_mut(buyer_country_idx) else {
                        continue;
                    };
                    if person_idx >= country_pending.len() {
                        continue;
                    }

                    let pending_units = country_pending[person_idx];
                    if pending_units == 0 {
                        continue;
                    }

                    any_pending = true;
                    let Some(budget) = discretionary_budgets
                        .get_mut(buyer_country_idx)
                        .and_then(|country_budgets| country_budgets.get_mut(person_idx))
                    else {
                        continue;
                    };

                    match self.try_purchase_one_unit(
                        buyer_country_idx,
                        person_idx,
                        good_id,
                        essential,
                        budget,
                    ) {
                        PurchaseAttempt::Bought => {
                            country_pending[person_idx] =
                                country_pending[person_idx].saturating_sub(1);
                            if let Some(row) = needs_met.get_mut(buyer_country_idx)
                                && let Some(flag) = row.get_mut(person_idx)
                            {
                                *flag = true;
                            }
                            progress = true;
                        }
                        PurchaseAttempt::Unmet => {
                            if let Some(row) = needs_unmet.get_mut(buyer_country_idx)
                                && let Some(flag) = row.get_mut(person_idx)
                            {
                                *flag = true;
                            }
                            self.bump_unmet_need_events(buyer_country_idx, pending_units);
                            country_pending[person_idx] = 0;
                        }
                    }
                }
            }

            if !any_pending || !progress {
                break;
            }
        }

        for country_idx in 0..country_count {
            let person_count = pending_demands[country_idx].len();
            for person_idx in 0..person_count {
                let need_met = needs_met[country_idx][person_idx];
                let need_unmet = needs_unmet[country_idx][person_idx];
                self.mark_person_need_result(
                    country_idx,
                    person_idx,
                    good_id,
                    need_met,
                    need_unmet,
                );
            }
        }
    }

    fn try_purchase_one_unit(
        &mut self,
        buyer_country_idx: usize,
        person_idx: usize,
        good_id: GoodId,
        essential: bool,
        discretionary_budget_cents: &mut i64,
    ) -> PurchaseAttempt {
        let Some(quote) = self.find_cheapest_supplier(good_id, buyer_country_idx) else {
            return PurchaseAttempt::Unmet;
        };

        let buyer_cash_cents = self.person_cash_cents(buyer_country_idx, person_idx);
        if !essential {
            if *discretionary_budget_cents < quote.landed_price_cents
                || buyer_cash_cents < quote.landed_price_cents
            {
                return PurchaseAttempt::Unmet;
            }
            *discretionary_budget_cents =
                discretionary_budget_cents.saturating_sub(quote.landed_price_cents);
        } else if buyer_cash_cents < quote.landed_price_cents {
            let covered_by_cash = buyer_cash_cents.max(0);
            let debt_cents = quote.landed_price_cents.saturating_sub(covered_by_cash);
            self.bump_essential_debt_spending(buyer_country_idx, debt_cents);
        }

        self.execute_goods_purchase(buyer_country_idx, person_idx, quote);
        PurchaseAttempt::Bought
    }

    fn find_cheapest_supplier(
        &self,
        good_id: GoodId,
        buyer_country_idx: usize,
    ) -> Option<SellerQuote> {
        let mut best_quote: Option<SellerQuote> = None;

        for (seller_country_idx, country) in self.countries.iter().enumerate() {
            for (seller_company_idx, company) in country.companies.iter().enumerate() {
                if company.produced_good_id != good_id || company.inventory_units == 0 {
                    continue;
                }

                let base_price_cents = to_cents(company.unit_price);
                if base_price_cents <= 0 {
                    continue;
                }

                let tariff_cents = if seller_country_idx == buyer_country_idx {
                    0
                } else {
                    let good_idx = good_id as usize;
                    let tariff_rate = country
                        .export_tariffs_by_good
                        .get(good_idx)
                        .copied()
                        .unwrap_or(0.0);
                    to_cents(from_cents(base_price_cents) * tariff_rate)
                };
                let landed_price_cents = base_price_cents.saturating_add(tariff_cents);

                let candidate = SellerQuote {
                    seller_country_idx,
                    seller_company_idx,
                    base_price_cents,
                    tariff_cents,
                    landed_price_cents,
                };

                let is_better = match best_quote {
                    None => true,
                    Some(current) => {
                        (
                            candidate.landed_price_cents,
                            candidate.base_price_cents,
                            candidate.seller_country_idx,
                            candidate.seller_company_idx,
                        ) < (
                            current.landed_price_cents,
                            current.base_price_cents,
                            current.seller_country_idx,
                            current.seller_company_idx,
                        )
                    }
                };

                if is_better {
                    best_quote = Some(candidate);
                }
            }
        }

        best_quote
    }

    fn execute_goods_purchase(
        &mut self,
        buyer_country_idx: usize,
        person_idx: usize,
        quote: SellerQuote,
    ) {
        if buyer_country_idx == quote.seller_country_idx {
            let Some(country) = self.countries.get_mut(buyer_country_idx) else {
                return;
            };
            Self::apply_purchase_to_buyer(country, person_idx, quote.landed_price_cents, false);
            Self::apply_sale_to_seller(
                country,
                quote.seller_company_idx,
                quote.base_price_cents,
                true,
            );
            return;
        }

        if buyer_country_idx < quote.seller_country_idx {
            let (left, right) = self.countries.split_at_mut(quote.seller_country_idx);
            let buyer_country = &mut left[buyer_country_idx];
            let seller_country = &mut right[0];
            Self::apply_purchase_to_buyer(
                buyer_country,
                person_idx,
                quote.landed_price_cents,
                true,
            );
            Self::apply_sale_to_seller(
                seller_country,
                quote.seller_company_idx,
                quote.base_price_cents,
                false,
            );
            Self::apply_tariff_to_exporter(seller_country, quote.tariff_cents);
        } else {
            let (left, right) = self.countries.split_at_mut(buyer_country_idx);
            let seller_country = &mut left[quote.seller_country_idx];
            let buyer_country = &mut right[0];
            Self::apply_purchase_to_buyer(
                buyer_country,
                person_idx,
                quote.landed_price_cents,
                true,
            );
            Self::apply_sale_to_seller(
                seller_country,
                quote.seller_company_idx,
                quote.base_price_cents,
                false,
            );
            Self::apply_tariff_to_exporter(seller_country, quote.tariff_cents);
        }
    }

    fn apply_purchase_to_buyer(
        country: &mut crate::sim::country::Country,
        person_idx: usize,
        landed_price_cents: i64,
        imported: bool,
    ) {
        let amount = from_cents(landed_price_cents);

        if let Some(person) = country.people.get_mut(person_idx) {
            person.cash_balance = round_cents(person.cash_balance - amount);
            person.last_month_spending = round_cents(person.last_month_spending + amount);
            person.last_month_goods_spending =
                round_cents(person.last_month_goods_spending + amount);
            if imported {
                person.last_month_imported_goods_spending =
                    round_cents(person.last_month_imported_goods_spending + amount);
            }
            person.last_month_goods_units_bought =
                person.last_month_goods_units_bought.saturating_add(1);
        }

        country.monthly_household_spending =
            round_cents(country.monthly_household_spending + amount);
        if imported {
            country.monthly_goods_import_spending =
                round_cents(country.monthly_goods_import_spending + amount);
        }
    }

    fn apply_sale_to_seller(
        country: &mut crate::sim::country::Country,
        company_idx: usize,
        base_price_cents: i64,
        domestic: bool,
    ) {
        let amount = from_cents(base_price_cents);

        if let Some(company) = country.companies.get_mut(company_idx) {
            company.inventory_units = company.inventory_units.saturating_sub(1);
            company.cash_balance = round_cents(company.cash_balance + amount);
            company.last_month_sales = round_cents(company.last_month_sales + amount);
            company.last_month_units_sold = company.last_month_units_sold.saturating_add(1);
            if domestic {
                company.last_month_domestic_units_sold =
                    company.last_month_domestic_units_sold.saturating_add(1);
            } else {
                company.last_month_export_units_sold =
                    company.last_month_export_units_sold.saturating_add(1);
            }
        }

        if !domestic {
            country.monthly_goods_export_revenue =
                round_cents(country.monthly_goods_export_revenue + amount);
        }
    }

    fn apply_tariff_to_exporter(country: &mut crate::sim::country::Country, tariff_cents: i64) {
        if tariff_cents <= 0 {
            return;
        }

        let amount = from_cents(tariff_cents);
        country.monthly_tariff_collected = round_cents(country.monthly_tariff_collected + amount);
        country.treasury_balance = round_cents(country.treasury_balance + amount);
    }

    fn person_cash_cents(&self, buyer_country_idx: usize, person_idx: usize) -> i64 {
        self.countries
            .get(buyer_country_idx)
            .and_then(|country| country.people.get(person_idx))
            .map_or(0, |person| to_cents(person.cash_balance))
    }

    fn mark_person_need_result(
        &mut self,
        buyer_country_idx: usize,
        person_idx: usize,
        good_id: GoodId,
        need_met: bool,
        need_unmet: bool,
    ) {
        let bit = 1_u16 << good_id;

        let Some(country) = self.countries.get_mut(buyer_country_idx) else {
            return;
        };
        let Some(person) = country.people.get_mut(person_idx) else {
            return;
        };

        if need_met {
            person.last_month_needs_met_mask |= bit;
        }
        if need_unmet {
            person.last_month_unmet_needs_mask |= bit;
        }
    }

    fn bump_unmet_need_events(&mut self, buyer_country_idx: usize, count: u32) {
        if let Some(country) = self.countries.get_mut(buyer_country_idx) {
            country.monthly_unmet_need_events = country
                .monthly_unmet_need_events
                .saturating_add(u64::from(count));
        }
    }

    fn bump_essential_debt_spending(&mut self, buyer_country_idx: usize, debt_cents: i64) {
        if debt_cents <= 0 {
            return;
        }

        if let Some(country) = self.countries.get_mut(buyer_country_idx) {
            country.monthly_essential_debt_spending =
                round_cents(country.monthly_essential_debt_spending + from_cents(debt_cents));
        }
    }
}

fn demand_units_for_person_good(
    day: u64,
    country_id: u16,
    person_id: u32,
    name_seed: u32,
    good_id: GoodId,
    monthly_demand_factor: f64,
) -> u32 {
    let daily_demand_factor = (monthly_demand_factor / 30.0).max(0.0);
    let integer_part = daily_demand_factor.floor().max(0.0) as u32;
    let fractional_part = (daily_demand_factor - f64::from(integer_part)).clamp(0.0, 1.0);

    if fractional_part <= 0.0 {
        return integer_part;
    }

    let seed = hash64(
        (day << 40)
            ^ (u64::from(country_id) << 24)
            ^ (u64::from(person_id) << 8)
            ^ u64::from(good_id)
            ^ u64::from(name_seed),
    );

    integer_part + u32::from(unit_interval(seed) < fractional_part)
}

fn hash64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_interval(seed: u64) -> f64 {
    hash64(seed) as f64 / u64::MAX as f64
}
