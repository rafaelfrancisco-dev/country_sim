impl SimulationWorker {
    pub(super) fn new(
        initial_engine: GameEngine,
        snapshot: Arc<RwLock<RenderSnapshot>>,
        realtime_tick_rate: f64,
        max_speed_publish_interval: Duration,
        sim_mode_state: Arc<AtomicU8>,
        export_csv: Option<PathBuf>,
    ) -> Self {
        let tick_in_flight = Arc::new(AtomicBool::new(false));
        let pending_user_commands = Arc::new(AtomicUsize::new(0));
        let phase_is_running = Arc::new(AtomicBool::new(matches!(
            initial_engine.phase(),
            GamePhase::Running(_)
        )));
        let world_is_paused = Arc::new(AtomicBool::new(matches!(
            initial_engine.phase(),
            GamePhase::Running(world) if world.paused
        )));
        let shutdown = Arc::new(AtomicBool::new(false));

        let (tx, mut rx) = unbounded_channel::<WorkerMsg>();

        let worker_snapshot = snapshot.clone();
        let worker_phase_is_running = phase_is_running.clone();
        let worker_world_is_paused = world_is_paused.clone();
        let worker_tick_in_flight = tick_in_flight.clone();
        let worker_shutdown = shutdown.clone();
        let worker_sim_mode_state = sim_mode_state.clone();
        let worker_pending_user_commands = pending_user_commands.clone();

        let handle = thread::spawn(move || {
            let mut engine = initial_engine;
            let mut ui_state = UiState::default();
            let mut runtime_stats = RuntimeStats::new(SimulationMode::from_atomic_u8(
                worker_sim_mode_state.load(Ordering::Acquire),
            ));
            let mut last_snapshot_published_at = Instant::now();
            let export_csv_path = export_csv;
            let mut csv_exporter: Option<CsvExporter> = None;
            let mut export_seed: Option<u64> = None;
            let mut export_queue_activated = export_csv_path.is_none(); // skip if no path

            let mut self_tick_next = false;

            'worker: loop {
                if worker_shutdown.load(Ordering::Acquire) {
                    break 'worker;
                }

                let message = if self_tick_next {
                    match rx.try_recv() {
                        Ok(msg) => msg,
                        Err(_) => {
                            // Channel empty; synthesize the next tick.
                            self_tick_next = false;
                            WorkerMsg::Apply {
                                command: WorkerCommand::Domain(GameCommand::Tick),
                                enqueued_at: Instant::now(),
                                is_user_initiated: false,
                            }
                        }
                    }
                } else {
                    match rx.blocking_recv() {
                        Some(msg) => msg,
                        None => break 'worker,
                    }
                };

                let mode =
                    SimulationMode::from_atomic_u8(worker_sim_mode_state.load(Ordering::Acquire));
                runtime_stats.update_mode_and_queue(mode, rx.len());

                match message {
                    WorkerMsg::Apply {
                        command,
                        enqueued_at,
                        is_user_initiated,
                    } => {
                        let command_started_at = Instant::now();
                        let queue_lag_ms = command_started_at
                            .saturating_duration_since(enqueued_at)
                            .as_secs_f64()
                            * 1_000.0;

                        let mut is_tick = false;
                        match command {
                            WorkerCommand::Domain(game_command) => {
                                is_tick = matches!(game_command, GameCommand::Tick);
                                let outcome = engine.apply(game_command);
                                for warning in outcome.warnings {
                                    warn!("simulation invariant warning: {warning}");
                                }
                            }
                            WorkerCommand::Ui(ui_command) => {
                                Self::apply_ui_command(
                                    ui_command,
                                    &engine,
                                    &mut ui_state,
                                    &worker_sim_mode_state,
                                );
                            }
                        }

                        update_phase_flags(
                            &engine,
                            &worker_phase_is_running,
                            &worker_world_is_paused,
                        );

                        // Activate the export queue on the world once the engine is Running
                        if !export_queue_activated
                            && matches!(engine.phase(), GamePhase::Running(_))
                        {
                            engine.enable_export_queue();
                            // Capture the seed as soon as the world is live so stock CSV
                            // can be written before the first monthly export fires (day 30).
                            if export_seed.is_none() {
                                export_seed = engine.world_generation_seed();
                            }
                            export_queue_activated = true;
                        }

                        // Drain export queue and write CSV rows if enabled
                        let export_rows = engine.drain_export_queue();
                        if !export_rows.is_empty() {
                            if export_seed.is_none() {
                                export_seed = export_rows.first().map(|r| r.generation_seed);
                            }
                            if let (Some(path), Some(seed)) = (&export_csv_path, export_seed) {
                                if csv_exporter.is_none() {
                                    match CsvExporter::new(path, seed) {
                                        Ok(e) => csv_exporter = Some(e),
                                        Err(err) => {
                                            tracing::error!("failed to open CSV export: {err}")
                                        }
                                    }
                                }
                                if let Some(exporter) = &mut csv_exporter {
                                    for export_data in &export_rows {
                                        for country_data in &export_data.countries {
                                            if let Err(err) = exporter.write_row(country_data) {
                                                tracing::error!("CSV write error: {err}");
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Drain daily stock export queues and write CSV rows if enabled
                        if export_csv_path.is_some() {
                            let (stock_country_rows, stock_equity_rows) =
                                engine.drain_stock_export_queues();
                            if !stock_country_rows.is_empty() {
                                if let (Some(path), Some(seed)) = (&export_csv_path, export_seed) {
                                    if csv_exporter.is_none() {
                                        match CsvExporter::new(path, seed) {
                                            Ok(e) => csv_exporter = Some(e),
                                            Err(err) => {
                                                tracing::error!("failed to open CSV export: {err}")
                                            }
                                        }
                                    }
                                    if let Some(exporter) = &mut csv_exporter {
                                        for row in &stock_country_rows {
                                            if let Err(err) = exporter.write_stock_country_row(row)
                                            {
                                                tracing::error!(
                                                    "CSV stock country write error: {err}"
                                                );
                                            }
                                        }
                                        for row in &stock_equity_rows {
                                            if let Err(err) = exporter.write_stock_equity_row(row) {
                                                tracing::error!(
                                                    "CSV stock equity write error: {err}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if is_tick {
                            let tick_duration_ms =
                                command_started_at.elapsed().as_secs_f64() * 1_000.0;
                            runtime_stats.record_tick(queue_lag_ms, tick_duration_ms);
                        }

                        let mode_after = SimulationMode::from_atomic_u8(
                            worker_sim_mode_state.load(Ordering::Acquire),
                        );
                        runtime_stats.update_mode_and_queue(mode_after, rx.len());

                        let should_publish = if is_user_initiated {
                            true
                        } else if is_tick {
                            should_publish_snapshot_for_tick(
                                mode_after,
                                command_started_at,
                                last_snapshot_published_at,
                                max_speed_publish_interval,
                            )
                        } else {
                            false
                        };

                        if should_publish {
                            let snapshot_age_ms = command_started_at
                                .saturating_duration_since(last_snapshot_published_at)
                                .as_secs_f64()
                                * 1_000.0;

                            let snapshot_started_at = Instant::now();
                            let mut ui_snapshot = UiSnapshot::from_engine_and_ui(
                                &engine,
                                &mut ui_state,
                                runtime_stats.snapshot(),
                            );
                            let snapshot_build_ms =
                                snapshot_started_at.elapsed().as_secs_f64() * 1_000.0;

                            runtime_stats
                                .record_snapshot_publish(snapshot_age_ms, snapshot_build_ms);
                            let runtime_snapshot = runtime_stats.snapshot();
                            ui_snapshot.apply_runtime_stats(runtime_snapshot);

                            if let Ok(mut shared_snapshot) = worker_snapshot.write() {
                                *shared_snapshot = RenderSnapshot { ui_snapshot };
                                last_snapshot_published_at = Instant::now();
                            } else {
                                break 'worker;
                            }
                        }

                        if is_tick {
                            let mode_now = SimulationMode::from_atomic_u8(
                                worker_sim_mode_state.load(Ordering::Acquire),
                            );
                            let running = worker_phase_is_running.load(Ordering::Acquire);
                            let unpaused = !worker_world_is_paused.load(Ordering::Acquire);
                            let no_pending =
                                worker_pending_user_commands.load(Ordering::Acquire) == 0;
                            let alive = !worker_shutdown.load(Ordering::Acquire);

                            if matches!(mode_now, SimulationMode::MaxSpeed)
                                && running
                                && unpaused
                                && no_pending
                                && alive
                            {
                                self_tick_next = true;
                            } else {
                                worker_tick_in_flight.store(false, Ordering::Release);
                            }
                        }
                        if is_user_initiated {
                            worker_pending_user_commands.fetch_sub(1, Ordering::AcqRel);
                        }
                    }
                    #[cfg(test)]
                    WorkerMsg::ApplyAndAck(command, ack_tx) => {
                        match command {
                            WorkerCommand::Domain(game_command) => {
                                let _ = engine.apply(game_command);
                            }
                            WorkerCommand::Ui(ui_command) => {
                                Self::apply_ui_command(
                                    ui_command,
                                    &engine,
                                    &mut ui_state,
                                    &worker_sim_mode_state,
                                );
                            }
                        }

                        update_phase_flags(
                            &engine,
                            &worker_phase_is_running,
                            &worker_world_is_paused,
                        );

                        let mode_after = SimulationMode::from_atomic_u8(
                            worker_sim_mode_state.load(Ordering::Acquire),
                        );
                        runtime_stats.update_mode_and_queue(mode_after, rx.len());

                        let mut ui_snapshot = UiSnapshot::from_engine_and_ui(
                            &engine,
                            &mut ui_state,
                            runtime_stats.snapshot(),
                        );
                        let runtime_snapshot = runtime_stats.snapshot();
                        ui_snapshot.apply_runtime_stats(runtime_snapshot);

                        if let Ok(mut shared_snapshot) = worker_snapshot.write() {
                            *shared_snapshot = RenderSnapshot { ui_snapshot };
                            last_snapshot_published_at = Instant::now();
                        } else {
                            break 'worker;
                        }
                        let _ = ack_tx.send(());
                    }
                    WorkerMsg::Shutdown => break 'worker,
                }
            }
            worker_tick_in_flight.store(false, Ordering::Release);
        });

        let clock_handle = if realtime_tick_rate > 0.0 {
            let clock_tx = tx.clone();
            let clock_tick_in_flight = tick_in_flight;
            let clock_phase_is_running = phase_is_running;
            let clock_world_is_paused = world_is_paused;
            let clock_shutdown = shutdown.clone();
            let clock_sim_mode_state = sim_mode_state.clone();
            let clock_pending_user_commands = pending_user_commands.clone();
            let realtime_interval = Duration::from_secs_f64(1.0 / realtime_tick_rate.max(0.1));

            Some(thread::spawn(move || {
                let mut current_mode =
                    SimulationMode::from_atomic_u8(clock_sim_mode_state.load(Ordering::Acquire));
                let mut next_realtime_tick_at = Instant::now();

                while !clock_shutdown.load(Ordering::Acquire) {
                    let mode = SimulationMode::from_atomic_u8(
                        clock_sim_mode_state.load(Ordering::Acquire),
                    );
                    if mode != current_mode {
                        current_mode = mode;
                        next_realtime_tick_at = Instant::now();
                    }

                    if !clock_phase_is_running.load(Ordering::Acquire)
                        || clock_world_is_paused.load(Ordering::Acquire)
                    {
                        thread::sleep(Duration::from_millis(2));
                        next_realtime_tick_at = Instant::now();
                        continue;
                    }

                    if clock_pending_user_commands.load(Ordering::Acquire) > 0 {
                        thread::sleep(Duration::from_micros(100));
                        continue;
                    }

                    if matches!(mode, SimulationMode::Realtime) {
                        let now = Instant::now();
                        if now < next_realtime_tick_at {
                            thread::sleep(next_realtime_tick_at.duration_since(now));
                            continue;
                        }
                        next_realtime_tick_at = now + realtime_interval;
                    }

                    if clock_tick_in_flight
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        if clock_tx
                            .send(WorkerMsg::Apply {
                                command: WorkerCommand::Domain(GameCommand::Tick),
                                enqueued_at: Instant::now(),
                                is_user_initiated: false,
                            })
                            .is_err()
                        {
                            clock_tick_in_flight.store(false, Ordering::Release);
                            break;
                        }
                    } else if matches!(mode, SimulationMode::MaxSpeed) {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }))
        } else {
            None
        };

        Self {
            tx,
            handle: Some(handle),
            clock_handle,
            shutdown,
            pending_user_commands,
        }
    }

    fn apply_ui_command(
        ui_command: UiCommand,
        engine: &GameEngine,
        ui_state: &mut UiState,
        sim_mode_state: &AtomicU8,
    ) {
        if matches!(ui_command, UiCommand::ToggleSimMode) {
            let current_mode =
                SimulationMode::from_atomic_u8(sim_mode_state.load(Ordering::Acquire));
            let next_mode = match current_mode {
                SimulationMode::Realtime => SimulationMode::MaxSpeed,
                SimulationMode::MaxSpeed => SimulationMode::Realtime,
            };
            sim_mode_state.store(next_mode.as_atomic_u8(), Ordering::Release);
            return;
        }

        if let GamePhase::Running(world) = engine.phase() {
            ui_state.apply(ui_command, world);
        }
    }

    fn send_internal(
        &self,
        command: WorkerCommand,
        is_user_initiated: bool,
    ) -> Result<(), WorkerError> {
        if is_user_initiated {
            self.pending_user_commands.fetch_add(1, Ordering::AcqRel);
        }

        if self
            .tx
            .send(WorkerMsg::Apply {
                command,
                enqueued_at: Instant::now(),
                is_user_initiated,
            })
            .is_err()
        {
            if is_user_initiated {
                self.pending_user_commands.fetch_sub(1, Ordering::AcqRel);
            }
            return Err(WorkerError::CHANNEL_CLOSED);
        }

        Ok(())
    }

    pub(super) fn send_domain(&self, command: GameCommand) -> Result<(), WorkerError> {
        self.send_internal(WorkerCommand::Domain(command), true)
    }

    pub(super) fn send_ui(&self, command: UiCommand) -> Result<(), WorkerError> {
        self.send_internal(WorkerCommand::Ui(command), true)
    }

    #[cfg(test)]
    pub(super) fn send_and_wait(&self, command: GameCommand) {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self
            .tx
            .send(WorkerMsg::ApplyAndAck(
                WorkerCommand::Domain(command),
                ack_tx,
            ))
            .is_ok()
        {
            let _ = ack_rx.recv();
        }
    }
}
