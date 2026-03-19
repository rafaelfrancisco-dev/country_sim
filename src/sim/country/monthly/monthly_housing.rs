use super::*;
use crate::sim::{
    bank::MortgageAccount, bank::payment_for, event::SimEvent, money::round_cents, money::to_cents,
    person::HousingStatus,
};

const HOUSING_GOOD_ID: u8 = 2;

impl Country {
    fn is_local_housing_landlord(
        &self,
        landlord_company_id: u16,
        removed_companies: Option<&[bool]>,
    ) -> bool {
        let landlord_idx = landlord_company_id as usize;
        if removed_companies
            .and_then(|removed| removed.get(landlord_idx))
            .copied()
            .unwrap_or(false)
        {
            return false;
        }

        self.companies
            .get(landlord_idx)
            .is_some_and(|company| company.produced_good_id == HOUSING_GOOD_ID)
    }

    fn preferred_housing_landlord(
        &self,
        removed_companies: Option<&[bool]>,
    ) -> Option<u16> {
        let mut best_idx = None::<usize>;

        for (idx, company) in self.companies.iter().enumerate() {
            if company.produced_good_id != HOUSING_GOOD_ID
                || removed_companies
                    .and_then(|removed| removed.get(idx))
                    .copied()
                    .unwrap_or(false)
            {
                continue;
            }

            let Some(current_best_idx) = best_idx else {
                best_idx = Some(idx);
                continue;
            };
            let current_best = &self.companies[current_best_idx];

            let company_is_private = !company.is_public_utility;
            let current_is_private = !current_best.is_public_utility;
            if company_is_private != current_is_private {
                if company_is_private {
                    best_idx = Some(idx);
                }
                continue;
            }

            if company.rental_units_owned != current_best.rental_units_owned {
                if company.rental_units_owned > current_best.rental_units_owned {
                    best_idx = Some(idx);
                }
                continue;
            }

            let cash_cmp = company
                .cash_balance
                .partial_cmp(&current_best.cash_balance)
                .unwrap_or(Ordering::Equal);
            if cash_cmp == Ordering::Greater
                || (cash_cmp == Ordering::Equal && idx < current_best_idx)
            {
                best_idx = Some(idx);
            }
        }

        best_idx.map(|idx| idx as u16)
    }

    fn ensure_housing_landlord(&mut self, removed_companies: Option<&[bool]>) -> Option<u16> {
        if let Some(landlord_id) = self.preferred_housing_landlord(removed_companies) {
            return Some(landlord_id);
        }

        let created_from = self.companies.len();
        self.create_public_essential_company(HOUSING_GOOD_ID, self.sim_params.government_backfill);
        self.companies
            .iter()
            .enumerate()
            .skip(created_from)
            .find(|(_, company)| {
                company.produced_good_id == HOUSING_GOOD_ID && company.is_public_utility
            })
            .map(|(idx, _)| idx as u16)
    }

    fn evict_renters(&mut self, renter_indices: &[usize]) {
        for &person_idx in renter_indices {
            let Some(person) = self.people.get_mut(person_idx) else {
                continue;
            };
            if !matches!(person.housing, HousingStatus::Renter { .. }) {
                continue;
            }

            person.housing = HousingStatus::Unhoused;
            self.housing_stock_occupied_rented =
                self.housing_stock_occupied_rented.saturating_sub(1);
            self.housing_stock_vacant = self.housing_stock_vacant.saturating_add(1);
        }
    }

    pub(crate) fn transfer_bankrupt_landlord_tenancies(&mut self, removed_companies: &[bool]) {
        let transferred_units = self
            .companies
            .iter()
            .enumerate()
            .filter(|(idx, company)| {
                removed_companies.get(*idx).copied().unwrap_or(false) && company.rental_units_owned > 0
            })
            .map(|(_, company)| company.rental_units_owned)
            .sum::<u32>();

        let displaced_renters = self
            .people
            .iter()
            .enumerate()
            .filter_map(|(person_idx, person)| match person.housing {
                HousingStatus::Renter {
                    landlord_company_id,
                    landlord_country_id,
                } if landlord_country_id == self.id
                    && removed_companies
                        .get(landlord_company_id as usize)
                        .copied()
                        .unwrap_or(false) =>
                {
                    Some(person_idx)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        if displaced_renters.is_empty() && transferred_units == 0 {
            return;
        }

        let Some(target_landlord_id) = self.ensure_housing_landlord(Some(removed_companies)) else {
            if !displaced_renters.is_empty() {
                self.evict_renters(&displaced_renters);
            }
            return;
        };

        for &person_idx in &displaced_renters {
            if let Some(person) = self.people.get_mut(person_idx) {
                person.housing = HousingStatus::Renter {
                    landlord_company_id: target_landlord_id,
                    landlord_country_id: self.id,
                };
            }
        }

        let units_to_transfer = transferred_units.max(displaced_renters.len() as u32);
        if units_to_transfer > 0
            && let Some(company) = self.companies.get_mut(target_landlord_id as usize)
        {
            company.rental_units_owned =
                company.rental_units_owned.saturating_add(units_to_transfer);
        }
    }

    pub(crate) fn repair_renter_landlord_links(&mut self) {
        let mut assigned_renters = vec![0_u32; self.companies.len()];
        let mut invalid_renters = Vec::new();

        for (person_idx, person) in self.people.iter().enumerate() {
            let HousingStatus::Renter {
                landlord_company_id,
                landlord_country_id,
            } = person.housing
            else {
                continue;
            };
            if landlord_country_id != self.id {
                continue;
            }

            if self.is_local_housing_landlord(landlord_company_id, None) {
                if let Some(count) = assigned_renters.get_mut(landlord_company_id as usize) {
                    *count = count.saturating_add(1);
                }
            } else {
                invalid_renters.push(person_idx);
            }
        }

        for (company_idx, assigned_count) in assigned_renters.iter().copied().enumerate() {
            if assigned_count == 0 {
                continue;
            }

            if let Some(company) = self.companies.get_mut(company_idx) {
                company.rental_units_owned = company.rental_units_owned.max(assigned_count);
            }
        }

        if invalid_renters.is_empty() {
            return;
        }

        let Some(target_landlord_id) = self.ensure_housing_landlord(None) else {
            self.evict_renters(&invalid_renters);
            return;
        };

        for &person_idx in &invalid_renters {
            if let Some(person) = self.people.get_mut(person_idx) {
                person.housing = HousingStatus::Renter {
                    landlord_company_id: target_landlord_id,
                    landlord_country_id: self.id,
                };
            }
        }

        let reassigned_count = invalid_renters.len() as u32;
        if let Some(company) = self.companies.get_mut(target_landlord_id as usize) {
            company.rental_units_owned = company.rental_units_owned.saturating_add(reassigned_count);
        }
    }

    /// Drain housing company inventory into the housing stock pool.
    pub(super) fn process_housing_construction(&mut self) {
        let mut new_units: u32 = 0;
        for company in &mut self.companies {
            if company.produced_good_id != HOUSING_GOOD_ID {
                continue;
            }
            let built = company.inventory_units;
            if built == 0 {
                continue;
            }
            company.inventory_units = 0;
            company.rental_units_owned = company.rental_units_owned.saturating_add(built);
            new_units = new_units.saturating_add(built);
        }

        if new_units > 0 {
            self.housing_stock_total = self.housing_stock_total.saturating_add(new_units);
            // Newly built units are vacant until a person moves in.
            self.housing_stock_vacant = self.housing_stock_vacant.saturating_add(new_units);
            self.monthly_housing_units_built =
                self.monthly_housing_units_built.saturating_add(new_units);
        }
    }

    pub(super) fn process_company_facility_housing(&mut self) {
        for company in &mut self.companies {
            let target_units = company.target_facility_housing_units();
            if company.facility_housing_units > target_units {
                let released = company.facility_housing_units - target_units;
                company.facility_housing_units = target_units;
                self.housing_stock_company_facility =
                    self.housing_stock_company_facility.saturating_sub(released);
                self.housing_stock_vacant = self.housing_stock_vacant.saturating_add(released);
            }
        }

        for company in &mut self.companies {
            let target_units = company.target_facility_housing_units();
            let missing_units = target_units.saturating_sub(company.facility_housing_units);
            if missing_units == 0 || self.housing_stock_vacant == 0 {
                continue;
            }

            let acquired_units = missing_units.min(self.housing_stock_vacant);
            company.facility_housing_units = company
                .facility_housing_units
                .saturating_add(acquired_units);
            self.housing_stock_company_facility = self
                .housing_stock_company_facility
                .saturating_add(acquired_units);
            self.housing_stock_vacant = self.housing_stock_vacant.saturating_sub(acquired_units);
        }

        if self.monthly_rent <= 0.0 {
            return;
        }

        let mut treasury_delta_cents = 0_i64;
        let mut company_delta_cents = 0_i64;
        for company in &mut self.companies {
            if company.facility_housing_units == 0 {
                continue;
            }

            let due = round_cents(self.monthly_rent * f64::from(company.facility_housing_units));
            let paid = round_cents(due.min(company.cash_balance.max(0.0)));
            if paid <= 0.0 {
                continue;
            }

            company.cash_balance = round_cents(company.cash_balance - paid);
            company.last_month_facility_spending =
                round_cents(company.last_month_facility_spending + paid);
            company_delta_cents = company_delta_cents.saturating_sub(to_cents(paid));
            treasury_delta_cents = treasury_delta_cents.saturating_add(to_cents(paid));
        }

        self.adjust_company_cash_total_cents(company_delta_cents);
        self.adjust_treasury_balance_cents(treasury_delta_cents);
    }

    /// Adjust housing price index based on vacancy deviation from target.
    pub(super) fn update_housing_price_index(&mut self) {
        let params = self.sim_params.housing;
        if self.housing_stock_total == 0 || self.housing_price_index <= 0.0 {
            return;
        }

        let vacancy_rate = self.housing_vacancy_rate();
        let deviation = vacancy_rate - params.target_vacancy_rate;
        // If vacancy < target (shortage) → prices rise; if vacancy > target (surplus) → prices fall.
        let change = (-deviation * 2.0).clamp(
            -params.max_monthly_price_change,
            params.max_monthly_price_change,
        );

        let old_index = self.housing_price_index;
        self.housing_price_index = round_cents(self.housing_price_index * (1.0 + change));
        self.monthly_rent =
            round_cents(self.housing_price_index * params.rent_as_fraction_of_price);

        if change.abs() > 0.005 {
            self.pending_events.push(SimEvent::HousingPriceChanged {
                country_name: self.name.clone(),
                new_price_index: self.housing_price_index,
                change_pct: (self.housing_price_index - old_index) / old_index.max(1.0),
                day: self.day,
            });
        }
    }

    /// Collect monthly rent from renters and credit landlord companies.
    pub(super) fn collect_rent(&mut self) {
        self.repair_renter_landlord_links();

        let monthly_rent = self.monthly_rent;
        if monthly_rent <= 0.0 {
            return;
        }

        let country_id = self.id;
        let day = self.day;
        let country_name = self.name.clone();
        let people = &mut self.people;
        let companies = &mut self.companies;
        let pending_events = &mut self.pending_events;
        let monthly_rent_collected = &mut self.monthly_rent_collected;

        // Single-pass: split-borrow people and companies so both can be mutated in one loop.
        let mut rent_events: Vec<SimEvent> = Vec::new();

        for person in people.iter_mut() {
            let landlord_company_id = match &person.housing {
                HousingStatus::Renter {
                    landlord_company_id,
                    landlord_country_id,
                } if *landlord_country_id == country_id => *landlord_company_id,
                _ => continue,
            };
            let available = person.cash_balance.max(0.0);
            let amount_paid = round_cents(monthly_rent.min(available));

            person.cash_balance = round_cents(person.cash_balance - amount_paid);
            person.last_month_rent_paid = round_cents(person.last_month_rent_paid + amount_paid);

            if let Some(company) = companies.get_mut(landlord_company_id as usize) {
                company.cash_balance = round_cents(company.cash_balance + amount_paid);
                company.last_month_rent_revenue =
                    round_cents(company.last_month_rent_revenue + amount_paid);
            }

            *monthly_rent_collected = round_cents(*monthly_rent_collected + amount_paid);

            if amount_paid < monthly_rent {
                rent_events.push(SimEvent::RentUnpaid {
                    country_name: country_name.clone(),
                    person_id: person.id,
                    amount_due: monthly_rent,
                    amount_paid,
                    day,
                });
            }
        }

        pending_events.extend(rent_events);
    }
}
