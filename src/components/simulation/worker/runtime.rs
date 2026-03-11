use std::panic::{self, AssertUnwindSafe};

use tokio::sync::mpsc::error::TryRecvError;

use super::command_processor::apply_command;
use super::export_pipeline::WorkerExportState;
use super::health::mark_fatal;
use super::helpers::update_phase_flags;
use super::scheduler::{should_self_tick_next, spawn_clock_thread};
#[cfg(any(test, feature = "manual-harness"))]
use super::snapshot_publisher::force_publish;
use super::snapshot_publisher::publish_for_apply;
use super::health::push_event;
use super::types::{RuntimeStats, WorkerCommand, WorkerMsg};
use super::*;

impl SimulationWorker {
    pub(in crate::components::simulation) fn new(
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
        let health = Arc::new(Mutex::new(WorkerHealth::Healthy));
        let events = Arc::new(Mutex::new(VecDeque::new()));

        let (tx, mut rx) = unbounded_channel::<WorkerMsg>();

        let worker_snapshot = snapshot.clone();
        let worker_phase_is_running = phase_is_running.clone();
        let worker_world_is_paused = world_is_paused.clone();
        let worker_tick_in_flight = tick_in_flight.clone();
        let worker_shutdown = shutdown.clone();
        let worker_sim_mode_state = sim_mode_state.clone();
        let worker_pending_user_commands = pending_user_commands.clone();
        let worker_health = health.clone();
        let worker_events = events.clone();

        let handle = thread::spawn(move || {
            let worker_result = panic::catch_unwind(AssertUnwindSafe(|| {
                let mut engine = initial_engine;
                let mut ui_state = UiState::default();
                let mut runtime_stats = RuntimeStats::new(SimulationMode::from_atomic_u8(
                    worker_sim_mode_state.load(Ordering::Acquire),
                ));
                let mut last_snapshot_published_at = Instant::now();
                let mut export_state = WorkerExportState::new(export_csv);
                let mut self_tick_next = false;

                // Initialise persistence repository rooted next to the executable.
                let repo = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                    .map(crate::sim::persistence::SqliteSeaOrmSaveRepository::new);

                // Generate a random default save name.
                use rand::Rng as _;
                let slug = format!("save-{:08x}", rand::thread_rng().r#gen::<u32>());
                ui_state.save_name = slug;

                // Eagerly load the save list so the setup screen shows existing saves immediately.
                if let Some(ref r) = repo {
                    use crate::sim::persistence::SaveRepository as _;
                    if let Ok(list) = r.list_saves() {
                        ui_state.save_list = list;
                    }
                }

                'worker: loop {
                    if worker_shutdown.load(Ordering::Acquire) {
                        break 'worker;
                    }

                    let message = if self_tick_next {
                        match rx.try_recv() {
                            Ok(msg) => msg,
                            Err(TryRecvError::Empty) => {
                                self_tick_next = false;
                                WorkerMsg::Apply {
                                    command: WorkerCommand::Domain(GameCommand::Tick),
                                    enqueued_at: Instant::now(),
                                    is_user_initiated: false,
                                }
                            }
                            Err(TryRecvError::Disconnected) => {
                                mark_fatal(
                                    &worker_health,
                                    &worker_events,
                                    WorkerFatalReason::CommandChannelClosed,
                                );
                                break 'worker;
                            }
                        }
                    } else {
                        match rx.blocking_recv() {
                            Some(msg) => msg,
                            None => {
                                if !worker_shutdown.load(Ordering::Acquire) {
                                    mark_fatal(
                                        &worker_health,
                                        &worker_events,
                                        WorkerFatalReason::CommandChannelClosed,
                                    );
                                }
                                break 'worker;
                            }
                        }
                    };

                    let mode = SimulationMode::from_atomic_u8(
                        worker_sim_mode_state.load(Ordering::Acquire),
                    );
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

                            // Intercept persistence commands before regular dispatch.
                            let is_tick = if let WorkerCommand::Domain(GameCommand::Persist(pcmd)) = command.clone() {
                                apply_persistence_command(
                                    pcmd,
                                    &mut engine,
                                    &mut ui_state,
                                    &repo,
                                    &worker_events,
                                );
                                false
                            } else {
                                apply_command(
                                    command,
                                    &mut engine,
                                    &mut ui_state,
                                    &worker_sim_mode_state,
                                    &worker_events,
                                )
                            };

                            update_phase_flags(
                                &engine,
                                &worker_phase_is_running,
                                &worker_world_is_paused,
                            );

                            export_state.drain_and_write(&mut engine, &worker_events);

                            if is_tick {
                                let tick_duration_ms =
                                    command_started_at.elapsed().as_secs_f64() * 1_000.0;
                                runtime_stats.record_tick(queue_lag_ms, tick_duration_ms);
                            }

                            let mode_after = SimulationMode::from_atomic_u8(
                                worker_sim_mode_state.load(Ordering::Acquire),
                            );
                            runtime_stats.update_mode_and_queue(mode_after, rx.len());

                            if let Err(reason) = publish_for_apply(
                                is_user_initiated,
                                is_tick,
                                mode_after,
                                command_started_at,
                                &mut last_snapshot_published_at,
                                max_speed_publish_interval,
                                &engine,
                                &mut ui_state,
                                &mut runtime_stats,
                                &worker_snapshot,
                            ) {
                                mark_fatal(&worker_health, &worker_events, reason);
                                break 'worker;
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
                                let healthy = worker_health
                                    .lock()
                                    .map(|state| matches!(*state, WorkerHealth::Healthy))
                                    .unwrap_or(false);

                                if should_self_tick_next(
                                    mode_now, running, unpaused, no_pending, alive, healthy,
                                ) {
                                    self_tick_next = true;
                                } else {
                                    worker_tick_in_flight.store(false, Ordering::Release);
                                }
                            }

                            if is_user_initiated {
                                worker_pending_user_commands.fetch_sub(1, Ordering::AcqRel);
                            }
                        }
                        #[cfg(any(test, feature = "manual-harness"))]
                        WorkerMsg::ApplyAndAck(command, ack_tx) => {
                            let _ = apply_command(
                                command,
                                &mut engine,
                                &mut ui_state,
                                &worker_sim_mode_state,
                                &worker_events,
                            );

                            update_phase_flags(
                                &engine,
                                &worker_phase_is_running,
                                &worker_world_is_paused,
                            );

                            let mode_after = SimulationMode::from_atomic_u8(
                                worker_sim_mode_state.load(Ordering::Acquire),
                            );
                            runtime_stats.update_mode_and_queue(mode_after, rx.len());

                            if let Err(reason) = force_publish(
                                &engine,
                                &mut ui_state,
                                &mut runtime_stats,
                                &worker_snapshot,
                                &mut last_snapshot_published_at,
                            ) {
                                mark_fatal(&worker_health, &worker_events, reason);
                                break 'worker;
                            }

                            let _ = ack_tx.send(());
                        }
                        WorkerMsg::Shutdown => break 'worker,
                    }
                }
            }));

            if let Err(payload) = worker_result {
                mark_fatal(
                    &worker_health,
                    &worker_events,
                    WorkerFatalReason::InternalPanic(panic_payload_to_message(payload)),
                );
            }

            worker_tick_in_flight.store(false, Ordering::Release);
        });

        let clock_handle = spawn_clock_thread(
            tx.clone(),
            tick_in_flight,
            phase_is_running,
            world_is_paused,
            shutdown.clone(),
            sim_mode_state.clone(),
            pending_user_commands.clone(),
            health.clone(),
            events.clone(),
            realtime_tick_rate,
        );

        Self {
            tx,
            handle: Some(handle),
            clock_handle,
            shutdown,
            pending_user_commands,
            health,
            events,
        }
    }

    pub(in crate::components::simulation) fn health(&self) -> WorkerHealth {
        self.health
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| {
                WorkerHealth::Fatal(WorkerFatalReason::InternalPanic(
                    "worker health lock poisoned".to_string(),
                ))
            })
    }

    pub(in crate::components::simulation) fn drain_events(&self) -> Vec<WorkerEvent> {
        if let Ok(mut queue) = self.events.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    fn send_internal(
        &self,
        command: WorkerCommand,
        is_user_initiated: bool,
    ) -> Result<(), WorkerError> {
        if !matches!(self.health(), WorkerHealth::Healthy) {
            return Err(WorkerError::WORKER_UNHEALTHY);
        }

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
            if !matches!(self.health(), WorkerHealth::Healthy) {
                return Err(WorkerError::WORKER_UNHEALTHY);
            }
            return Err(WorkerError::CHANNEL_CLOSED);
        }

        Ok(())
    }

    pub(in crate::components::simulation) fn send_domain(
        &self,
        command: GameCommand,
    ) -> Result<(), WorkerError> {
        self.send_internal(WorkerCommand::Domain(command), true)
    }

    pub(in crate::components::simulation) fn send_ui(
        &self,
        command: UiCommand,
    ) -> Result<(), WorkerError> {
        self.send_internal(WorkerCommand::Ui(command), true)
    }

    #[cfg(any(test, feature = "manual-harness"))]
    pub(in crate::components::simulation) fn send_and_wait(&self, command: GameCommand) {
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

fn panic_payload_to_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

fn apply_persistence_command(
    pcmd: crate::sim::PersistenceCommand,
    engine: &mut GameEngine,
    ui_state: &mut UiState,
    repo: &Option<crate::sim::persistence::SqliteSeaOrmSaveRepository>,
    events: &Arc<Mutex<VecDeque<WorkerEvent>>>,
) {
    use crate::sim::persistence::SaveRepository as _;
    use crate::sim::PersistenceCommand;

    match pcmd {
        PersistenceCommand::SaveCurrentRun => {
            let Some(r) = repo else {
                ui_state.persistence_status = Some("Save failed: repository unavailable".into());
                return;
            };
            let Some(world) = engine.running_world() else {
                ui_state.persistence_status = Some("Save failed: not running".into());
                return;
            };
            let was_paused = world.paused;
            // Pause temporarily while saving (engine owns the pause toggle).
            if !was_paused {
                engine.apply(GameCommand::TogglePause);
            }
            let save_name = ui_state
                .active_save_name
                .clone()
                .unwrap_or_else(|| ui_state.save_name.clone());
            match r.save_world(&save_name, engine.running_world().unwrap()) {
                Ok(()) => {
                    ui_state.active_save_name = Some(save_name.clone());
                    ui_state.persistence_status =
                        Some(format!("Saved to '{save_name}'"));
                    // Refresh list so newly-created save shows up.
                    if let Ok(list) = r.list_saves() {
                        ui_state.save_list = list;
                    }
                }
                Err(e) => {
                    ui_state.persistence_status = Some(format!("Save error: {e}"));
                    push_event(events, WorkerEvent::Warning(format!("persistence save error: {e}")));
                }
            }
            if !was_paused {
                engine.apply(GameCommand::TogglePause);
            }
        }

        PersistenceCommand::SaveNameEditChar(c) => {
            ui_state.save_name.push(c);
        }
        PersistenceCommand::SaveNameBackspace => {
            ui_state.save_name.pop();
        }
        PersistenceCommand::SaveListSelectPrev => {
            if ui_state.selected_save_row > 0 {
                ui_state.selected_save_row -= 1;
            }
        }
        PersistenceCommand::SaveListSelectNext => {
            if ui_state.selected_save_row + 1 < ui_state.save_list.len() {
                ui_state.selected_save_row += 1;
            }
        }
        PersistenceCommand::SaveListRefresh => {
            if let Some(r) = repo {
                match r.list_saves() {
                    Ok(list) => {
                        ui_state.save_list = list;
                        ui_state.selected_save_row =
                            ui_state.selected_save_row.min(ui_state.save_list.len().saturating_sub(1));
                        ui_state.persistence_status = None;
                    }
                    Err(e) => {
                        ui_state.persistence_status = Some(format!("List error: {e}"));
                    }
                }
            }
        }
        PersistenceCommand::LoadSelectedSave => {
            let Some(r) = repo else {
                ui_state.persistence_status = Some("Load failed: repository unavailable".into());
                return;
            };
            let Some(descriptor) = ui_state.save_list.get(ui_state.selected_save_row).cloned() else {
                ui_state.persistence_status = Some("No save selected".into());
                return;
            };
            ui_state.persistence_status = Some(format!("Loading '{}'…", descriptor.save_name));
            match r.load_world(&descriptor.save_name) {
                Ok(world) => {
                    engine.replace_running_world(world);
                    ui_state.active_save_name = Some(descriptor.save_name.clone());
                    ui_state.persistence_status =
                        Some(format!("Loaded '{}'", descriptor.save_name));
                }
                Err(e) => {
                    ui_state.persistence_status = Some(format!("Load error: {e}"));
                    push_event(events, WorkerEvent::Warning(format!("persistence load error: {e}")));
                }
            }
        }
    }
}
