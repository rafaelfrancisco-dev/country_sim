use super::*;
use builder_events::{
    selected_company_related_events_snapshot, selected_person_related_events_snapshot,
};

#[path = "builder_events.rs"]
mod builder_events;

impl UiSnapshot {
    pub(in crate::components::simulation) fn from_engine_and_ui(
        engine: &GameEngine,
        ui_state: &mut UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> Self {
        Self::from_phase_and_ui(engine.phase(), ui_state, runtime_stats)
    }

    pub(in crate::components::simulation) fn from_phase_and_ui(
        phase: &GamePhase,
        ui_state: &mut UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> Self {
        let phase = match phase {
            GamePhase::Setup(setup) => {
                SnapshotPhase::Setup(Box::new(SetupSnapshot::from_setup_form(setup, false)))
            }
            GamePhase::Running(world) => SnapshotPhase::Running(Box::new(
                Self::running_from_world(world, ui_state, runtime_stats),
            )),
        };
        Self { phase }
    }

    pub(in crate::components::simulation) fn apply_runtime_stats(
        &mut self,
        runtime_stats: RuntimeStatsSnapshot,
    ) {
        if let SnapshotPhase::Running(world) = &mut self.phase {
            world.core.sim_mode = runtime_stats.mode;
            world.core.ticks_per_second = runtime_stats.ticks_per_second;
            world.core.avg_tick_duration_ms = runtime_stats.avg_tick_duration_ms;
            world.core.queue_depth = runtime_stats.queue_depth;
            world.core.tick_queue_lag_ms = runtime_stats.tick_queue_lag_ms;
            world.core.snapshot_publish_count = runtime_stats.snapshot_publish_count;
            world.core.avg_snapshot_build_ms = runtime_stats.avg_snapshot_build_ms;
            world.core.max_snapshot_age_ms = runtime_stats.max_snapshot_age_ms;
        }
    }

    fn running_from_world(
        world: &World,
        ui: &mut UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> RunningSnapshot {
        ui.ensure_valid(world);
        if let Some(country) = ui.selected_country(world) {
            ui.currency_symbol = country.currency_symbol().to_string();
        }
        let currency_symbol_owned = normalized_currency_symbol(&ui.currency_symbol).to_string();
        let currency_symbol = currency_symbol_owned.as_str();

        let selected_country = ui.selected_country(world);
        let selected_country_name: Arc<str> = selected_country
            .map(|country| country.name.clone())
            .unwrap_or_else(|| Arc::from("N/A"));
        let selected_country_month = selected_country.map_or(0, |country| country.month);
        let selected_country_disaster_days =
            disaster_days_for_country(world, &selected_country_name);

        #[cfg(feature = "ui-diagnostics")]
        let invariant_warnings = world
            .validate_invariants()
            .into_iter()
            .map(|v| format!("{}: {}", v.code, v.message))
            .collect();
        #[cfg(not(feature = "ui-diagnostics"))]
        let invariant_warnings = Vec::new();

        let tab = match ui.active_tab {
            RunningTab::Main => {
                let country_page_start =
                    ui.country_page() * crate::components::simulation::ui_state::COUNTRY_PAGE_SIZE;
                let country_rows = ui
                    .selected_countries_page(world)
                    .iter()
                    .enumerate()
                    .map(|(page_idx, country)| {
                        let idx = country_page_start + page_idx;
                        let row_currency_symbol = country.currency_symbol();
                        CountryRowSnapshot {
                            name: country.name.to_string(),
                            population: compact_count(country.population() as f64),
                            household_cash: money(
                                country.household_cash_total,
                                row_currency_symbol,
                            ),
                            company_cash: money(country.company_cash_total, row_currency_symbol),
                            treasury: money(country.treasury_balance, row_currency_symbol),
                            loans: money(country.total_bank_created_money(), row_currency_symbol),
                            is_selected: idx == ui.selected_country,
                            is_even: idx.is_multiple_of(2),
                        }
                    })
                    .collect::<Vec<_>>();

                let mut people_rows = Vec::new();
                if let Some(country) = selected_country {
                    for (row_idx, person) in ui
                        .selected_people_page(world)
                        .unwrap_or(&[])
                        .iter()
                        .enumerate()
                    {
                        let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
                        let name = person_name_from_seed(name_seed);
                        let job_name = person
                            .job_type_id()
                            .and_then(job_by_id)
                            .map_or("N/A".to_string(), |job| job.name.to_string());
                        let employment = if person.is_employed() {
                            format!("Employed ({job_name})")
                        } else {
                            "Unemployed".to_string()
                        };
                        let employer = person
                            .employer_company_id()
                            .and_then(|company_id| country.company_name(company_id))
                            .unwrap_or("N/A")
                            .to_string();

                        people_rows.push(PersonRowSnapshot {
                            citizen_id: person.id.to_string(),
                            name,
                            age: person.age_years().to_string(),
                            education: person.habilitation.label().to_string(),
                            employment,
                            employer,
                            salary: money(person.monthly_salary(), currency_symbol),
                            cash: money(person.cash_balance, currency_symbol),
                            loan: money(
                                person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance),
                                currency_symbol,
                            ),
                            is_selected: row_idx == ui.selected_person_row,
                            is_even: row_idx.is_multiple_of(2),
                        });
                    }
                }

                RunningTabSnapshot::Main(Box::new(MainTabSnapshot {
                    country_rows,
                    people_rows,
                    has_selected_country: selected_country.is_some(),
                    person_details_lines: person_details_lines(world, ui),
                    person_related_events: selected_person_related_events_snapshot(world, ui),
                    country_details_lines: country_details_lines(world, ui),
                    country_policy_lines: country_policy_lines(world, ui),
                }))
            }
            RunningTab::Goods => {
                let mut company_rows = Vec::new();
                if let Some(country) = selected_country {
                    let selected_abs = ui.selected_company_absolute_index();
                    for (row_idx, company) in country.companies.iter().enumerate() {
                        let good_name = good_by_id(company.produced_good_id)
                            .map_or("Unknown", |good| good.name)
                            .to_string();

                        let avg_price = if company.price_history.is_empty() {
                            money(company.unit_price, currency_symbol)
                        } else {
                            let avg = company.price_history.iter().sum::<f64>()
                                / company.price_history.len() as f64;
                            money(avg, currency_symbol)
                        };

                        company_rows.push(CompanyRowSnapshot {
                            company_id: company.id.to_string(),
                            name: company.name.to_string(),
                            good: good_name,
                            price: money(company.unit_price, currency_symbol),
                            avg_price,
                            inventory: compact_count(company.inventory_units),
                            produced: compact_count(company.last_month_units_produced),
                            sold: compact_count(company.last_month_units_sold),
                            total_sold: compact_count(
                                company.sold_units_history.iter().sum::<u32>(),
                            ),
                            export_sold: compact_count(company.last_month_export_units_sold),
                            domestic_export_split: format!(
                                "{}/{}",
                                company.last_month_domestic_units_sold,
                                company.last_month_export_units_sold
                            ),
                            sales: money(company.last_month_sales, currency_symbol),
                            is_selected: row_idx == selected_abs,
                            is_even: row_idx.is_multiple_of(2),
                        });
                    }
                }

                let (
                    company_sold_history,
                    company_employee_history,
                    company_price_history,
                    company_loan_payment_history,
                    company_money_history,
                ) = selected_company_histories(world, ui);
                let company_worker_rows = selected_company_worker_rows(world, ui, currency_symbol);
                let production_demand_rows = production_demand_rows(world, ui, currency_symbol);

                RunningTabSnapshot::Goods(Box::new(GoodsTabSnapshot {
                    company_rows,
                    company_details_lines: company_details_lines(world, ui),
                    company_worker_rows,
                    company_related_events: selected_company_related_events_snapshot(world, ui),
                    company_sold_history: company_sold_history.into(),
                    company_employee_history: company_employee_history.into(),
                    company_price_history: company_price_history.into(),
                    company_loan_payment_history: company_loan_payment_history.into(),
                    company_money_history: company_money_history.into(),
                    disaster_days: selected_country_disaster_days.clone().into(),
                    country_goods_summary_lines: country_goods_summary_lines(world, ui),
                    production_demand_summary_lines: production_demand_summary_lines(world, ui),
                    production_demand_rows,
                    goods_production_page: ui.goods_production_page,
                    total_goods_production_pages: ui.total_goods_production_pages(),
                }))
            }
            RunningTab::Banks => build_banks_tab_snapshot(world, ui, currency_symbol),
            RunningTab::Stocks => {
                let (
                    stock_rows,
                    stock_summary_lines,
                    stock_selected_index_price_history,
                    stock_selected_index_volume_history,
                    stock_country_index_history,
                    stock_bank_exposure_history,
                ) = build_stock_tab_data(world, ui, currency_symbol);

                RunningTabSnapshot::Stocks(Box::new(StocksTabSnapshot {
                    stock_rows,
                    stock_summary_lines,
                    stock_selected_index_price_history: stock_selected_index_price_history.into(),
                    stock_selected_index_volume_history: stock_selected_index_volume_history.into(),
                    stock_country_index_history: stock_country_index_history.into(),
                    stock_bank_exposure_history: stock_bank_exposure_history.into(),
                    disaster_days: selected_country_disaster_days.clone().into(),
                    stock_page: ui.stock_page,
                    stock_selected_row: ui.stock_selected_row,
                    stock_person_details_lines: stock_person_details_lines(
                        world,
                        ui,
                        currency_symbol,
                    ),
                }))
            }
            RunningTab::Fx => build_fx_tab_snapshot(world, ui),
            RunningTab::Disasters => {
                RunningTabSnapshot::Disasters(Box::new(build_disasters_tab_snapshot(world, ui)))
            }
            RunningTab::Charts => {
                build_charts_tab_snapshot(world, ui, &selected_country_disaster_days)
            }
            RunningTab::Events => RunningTabSnapshot::Events(Box::new(
                builder_events::build_events_tab_snapshot(world, ui, |_| true),
            )),
        };

        RunningSnapshot {
            core: RunningCoreSnapshot {
                day: world.day,
                generation_seed: world.generation_seed,
                currency_symbol: currency_symbol.to_string(),
                selected_country_month,
                paused: world.paused,
                active_tab: ui.active_tab,
                goods_main_page: ui.goods_main_page,
                details_view: ui.details_view,
                details_page: ui.details_page,
                selection_focus: ui.selection_focus,
                selected_country_name: selected_country_name.to_string(),
                total_population: compact_count(world.total_population() as f64),
                country_page: ui.country_page(),
                total_country_pages: ui.total_country_pages(world),
                people_page: ui.people_page,
                total_people_pages: ui.total_people_pages(world),
                company_page: ui.company_page,
                total_company_pages: ui.total_company_pages(world),
                bank_page: ui.bank_page,
                total_bank_pages: ui.total_bank_pages(world),
                country_policy_is_editing: ui.country_policy_form.is_editing,
                sim_mode: runtime_stats.mode,
                ticks_per_second: runtime_stats.ticks_per_second,
                avg_tick_duration_ms: runtime_stats.avg_tick_duration_ms,
                queue_depth: runtime_stats.queue_depth,
                tick_queue_lag_ms: runtime_stats.tick_queue_lag_ms,
                snapshot_publish_count: runtime_stats.snapshot_publish_count,
                avg_snapshot_build_ms: runtime_stats.avg_snapshot_build_ms,
                max_snapshot_age_ms: runtime_stats.max_snapshot_age_ms,
                invariant_warnings,
                events_search_active: ui.events_search_active,
                company_workers_scroll: ui.company_workers_scroll,
                bank_depositors_scroll: ui.bank_depositors_scroll,
            },
            tab,
        }
    }
}
