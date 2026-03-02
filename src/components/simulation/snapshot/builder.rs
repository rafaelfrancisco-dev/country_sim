use super::*;

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
                SnapshotPhase::Setup(SetupSnapshot::from_setup_form(setup, false))
            }
            GamePhase::Running(world) => {
                SnapshotPhase::Running(Self::running_from_world(world, ui_state, runtime_stats))
            }
        };
        Self { phase }
    }

    pub(in crate::components::simulation) fn apply_runtime_stats(&mut self, runtime_stats: RuntimeStatsSnapshot) {
        if let SnapshotPhase::Running(world) = &mut self.phase {
            world.sim_mode = runtime_stats.mode;
            world.ticks_per_second = runtime_stats.ticks_per_second;
            world.avg_tick_duration_ms = runtime_stats.avg_tick_duration_ms;
            world.queue_depth = runtime_stats.queue_depth;
            world.tick_queue_lag_ms = runtime_stats.tick_queue_lag_ms;
            world.snapshot_publish_count = runtime_stats.snapshot_publish_count;
            world.avg_snapshot_build_ms = runtime_stats.avg_snapshot_build_ms;
            world.max_snapshot_age_ms = runtime_stats.max_snapshot_age_ms;
        }
    }

    fn running_from_world(
        world: &World,
        ui: &mut UiState,
        runtime_stats: RuntimeStatsSnapshot,
    ) -> RunningSnapshot {
        ui.ensure_valid(world);

        let selected_country = ui.selected_country(world);
        let selected_country_name = selected_country
            .map(|country| country.name.clone())
            .unwrap_or_else(|| "N/A".to_string());
        let selected_country_month = selected_country.map_or(0, |country| country.month);

        let is_main_tab = matches!(ui.active_tab, RunningTab::Main);
        let is_goods_tab = matches!(ui.active_tab, RunningTab::Goods);
        let is_stocks_tab = matches!(ui.active_tab, RunningTab::Stocks);
        let is_charts_tab = matches!(ui.active_tab, RunningTab::Charts);
        let is_events_tab = matches!(ui.active_tab, RunningTab::Events);

        let country_rows = if is_main_tab || is_goods_tab {
            let country_page_start =
                ui.country_page() * crate::components::simulation::ui_state::COUNTRY_PAGE_SIZE;
            ui.selected_countries_page(world)
                .iter()
                .enumerate()
                .map(|(page_idx, country)| {
                    let idx = country_page_start + page_idx;
                    CountryRowSnapshot {
                        name: country.name.clone(),
                        population: compact_count(country.population() as f64),
                        household_cash: money(country.household_cash_total),
                        company_cash: money(country.company_cash_total),
                        treasury: money(country.treasury_balance),
                        loans: money(country.bank.created_money_outstanding),
                        is_selected: idx == ui.selected_country,
                        is_even: idx.is_multiple_of(2),
                    }
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut people_rows = Vec::new();
        if is_main_tab && let Some(country) = selected_country {
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
                    salary: money(person.monthly_salary()),
                    cash: money(person.cash_balance),
                    loan: money(person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance)),
                    is_selected: row_idx == ui.selected_person_row,
                    is_even: row_idx.is_multiple_of(2),
                });
            }
        }

        let mut company_rows = Vec::new();
        if is_goods_tab && selected_country.is_some() {
            let company_page_start =
                ui.company_page * crate::components::simulation::ui_state::COMPANY_PAGE_SIZE;
            for (row_idx, company) in ui
                .selected_companies_page(world)
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
                let absolute_idx = company_page_start + row_idx;
                let good_name = good_by_id(company.produced_good_id)
                    .map_or("Unknown", |good| good.name)
                    .to_string();

                let avg_price = if company.price_history.is_empty() {
                    money(company.unit_price)
                } else {
                    let avg = company.price_history.iter().sum::<f64>()
                        / company.price_history.len() as f64;
                    money(avg)
                };

                company_rows.push(CompanyRowSnapshot {
                    company_id: company.id.to_string(),
                    name: company.name.clone(),
                    good: good_name,
                    price: money(company.unit_price),
                    avg_price,
                    inventory: compact_count(company.inventory_units),
                    produced: compact_count(company.last_month_units_produced),
                    sold: compact_count(company.last_month_units_sold),
                    total_sold: compact_count(company.sold_units_history.iter().sum::<u32>()),
                    export_sold: compact_count(company.last_month_export_units_sold),
                    domestic_export_split: format!(
                        "{}/{}",
                        company.last_month_domestic_units_sold,
                        company.last_month_export_units_sold
                    ),
                    sales: money(company.last_month_sales),
                    is_selected: absolute_idx == ui.selected_company_absolute_index(),
                    is_even: row_idx.is_multiple_of(2),
                });
            }
        }

        let person_details_lines = if is_main_tab {
            person_details_lines(world, ui)
        } else {
            Vec::new()
        };
        let country_details_lines = if is_main_tab {
            country_details_lines(world, ui)
        } else {
            Vec::new()
        };
        let (
            company_details_lines,
            company_sold_history,
            company_employee_history,
            company_price_history,
            company_loan_payment_history,
            company_money_history,
            country_goods_summary_lines,
        ) = if is_goods_tab {
            let (
                company_sold_history,
                company_employee_history,
                company_price_history,
                company_loan_payment_history,
                company_money_history,
            ) = selected_company_histories(world, ui);
            (
                company_details_lines(world, ui),
                company_sold_history,
                company_employee_history,
                company_price_history,
                company_loan_payment_history,
                company_money_history,
                country_goods_summary_lines(world, ui),
            )
        } else {
            (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        };

        let (
            stock_rows,
            stock_summary_lines,
            stock_selected_index_price_history,
            stock_selected_index_volume_history,
            stock_country_index_history,
            stock_bank_exposure_history,
            stock_person_details_lines,
        ) = if is_stocks_tab {
            let (
                stock_rows,
                stock_summary_lines,
                stock_selected_index_price_history,
                stock_selected_index_volume_history,
                stock_country_index_history,
                stock_bank_exposure_history,
            ) = build_stock_tab_data(world, ui);
            (
                stock_rows,
                stock_summary_lines,
                stock_selected_index_price_history,
                stock_selected_index_volume_history,
                stock_country_index_history,
                stock_bank_exposure_history,
                stock_person_details_lines(world, ui),
            )
        } else {
            (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        };

        let (chart_scope_label, chart_history) = if is_charts_tab {
            let chart_scope_label = chart_scope_label(world, ui);
            let chart_history = match ui.chart_scope {
                ChartScope::SelectedCountry => world
                    .country_history
                    .get(ui.selected_country)
                    .cloned()
                    .unwrap_or_default(),
                ChartScope::WorldAggregate => world.world_history.clone(),
            };
            (chart_scope_label, chart_history)
        } else {
            (String::new(), Vec::new())
        };

        #[cfg(feature = "ui-diagnostics")]
        let invariant_warnings = world
            .validate_invariants()
            .into_iter()
            .map(|v| format!("{}: {}", v.code, v.message))
            .collect();
        #[cfg(not(feature = "ui-diagnostics"))]
        let invariant_warnings = Vec::new();

        let (event_log_lines, events_total_count) = if is_events_tab {
            let search_lower = ui.events_search_query.to_ascii_lowercase();
            let mut event_log_lines: Vec<String> = world
                .event_log
                .iter()
                .map(|e| e.format())
                .filter(|line| {
                    search_lower.is_empty() || line.to_ascii_lowercase().contains(&search_lower)
                })
                .collect();
            if matches!(ui.events_sort, EventsSort::NewestFirst) {
                event_log_lines.reverse();
            }
            let events_total_count = event_log_lines.len();
            (event_log_lines, events_total_count)
        } else {
            (Vec::new(), 0)
        };
        let events_sort_label = match ui.events_sort {
            EventsSort::NewestFirst => "Newest First",
            EventsSort::OldestFirst => "Oldest First",
        };

        RunningSnapshot {
            day: world.day,
            generation_seed: world.generation_seed,
            selected_country_month,
            paused: world.paused,
            active_tab: ui.active_tab,
            details_view: ui.details_view,
            selection_focus: ui.selection_focus,
            selected_country_name,
            total_population: compact_count(world.total_population() as f64),
            country_page: ui.country_page(),
            total_country_pages: ui.total_country_pages(world),
            people_page: ui.people_page,
            total_people_pages: ui.total_people_pages(world),
            company_page: ui.company_page,
            total_company_pages: ui.total_company_pages(world),
            country_rows,
            people_rows,
            company_rows,
            has_selected_country: selected_country.is_some(),
            person_details_lines,
            country_details_lines,
            company_details_lines,
            company_sold_history,
            company_employee_history,
            company_price_history,
            company_loan_payment_history,
            company_money_history,
            country_goods_summary_lines,
            chart_scope_label,
            chart_history,
            sim_mode: runtime_stats.mode,
            ticks_per_second: runtime_stats.ticks_per_second,
            avg_tick_duration_ms: runtime_stats.avg_tick_duration_ms,
            queue_depth: runtime_stats.queue_depth,
            tick_queue_lag_ms: runtime_stats.tick_queue_lag_ms,
            snapshot_publish_count: runtime_stats.snapshot_publish_count,
            avg_snapshot_build_ms: runtime_stats.avg_snapshot_build_ms,
            max_snapshot_age_ms: runtime_stats.max_snapshot_age_ms,
            invariant_warnings,
            event_log_lines,
            events_total_count,
            events_scroll: ui.events_scroll,
            events_sort_label,
            events_search_active: ui.events_search_active,
            events_search_query: ui.events_search_query.clone(),
            stock_rows,
            stock_summary_lines,
            stock_selected_index_price_history,
            stock_selected_index_volume_history,
            stock_country_index_history,
            stock_bank_exposure_history,
            stock_page: ui.stock_page,
            stock_selected_row: ui.stock_selected_row,
            stock_person_details_lines,
        }
    }
}

