use std::fs;

use super::snapshot_publisher::should_publish_snapshot_for_tick;
use super::*;

fn wait_for_running_phase(snapshot: &Arc<RwLock<RenderSnapshot>>) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let is_running = {
            let current = snapshot.read().expect("snapshot lock poisoned");
            matches!(
                current.ui_snapshot.phase,
                crate::components::simulation::snapshot::SnapshotPhase::Running(_)
            )
        };
        if is_running {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for running phase");
}

fn wait_for_worker_fatal(worker: &SimulationWorker) -> WorkerFatalReason {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if let WorkerHealth::Fatal(reason) = worker.health() {
            return reason;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for worker fatal state");
}

#[test]
fn worker_applies_commands_in_order() {
    let initial_engine = GameEngine::default();
    let mut initial_ui_state = UiState::default();
    let runtime_stats =
        RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0);
    let snapshot = Arc::new(RwLock::new(RenderSnapshot {
        ui_snapshot: UiSnapshot::from_engine_and_ui(
            &initial_engine,
            &mut initial_ui_state,
            runtime_stats,
        ),
    }));
    let worker = SimulationWorker::new(
        initial_engine,
        snapshot.clone(),
        0.0,
        Duration::from_millis(500),
        Arc::new(AtomicU8::new(SimulationMode::Realtime.as_atomic_u8())),
        None,
    );

    worker.send_and_wait(GameCommand::SetupEditChar('1'));
    worker.send_and_wait(GameCommand::SetupNextField);
    worker.send_and_wait(GameCommand::SetupEditChar('1'));
    worker.send_and_wait(GameCommand::SetupEditChar('0'));
    worker.send_and_wait(GameCommand::SetupSubmit);
    wait_for_running_phase(&snapshot);
    worker.send_and_wait(GameCommand::Tick);
    worker.send_and_wait(GameCommand::Tick);

    let snapshot = snapshot.read().expect("snapshot lock poisoned").clone();
    let crate::components::simulation::snapshot::SnapshotPhase::Running(world) =
        snapshot.ui_snapshot.phase
    else {
        panic!("expected running phase");
    };
    assert_eq!(world.day, 2);
}

#[test]
fn max_speed_auto_tick_publish_is_cadenced() {
    let now = Instant::now();
    assert!(should_publish_snapshot_for_tick(
        SimulationMode::Realtime,
        now,
        now,
        Duration::from_millis(500)
    ));
    assert!(!should_publish_snapshot_for_tick(
        SimulationMode::MaxSpeed,
        now,
        now,
        Duration::from_millis(500)
    ));
    assert!(should_publish_snapshot_for_tick(
        SimulationMode::MaxSpeed,
        now + Duration::from_millis(600),
        now,
        Duration::from_millis(500)
    ));
}

#[test]
fn worker_transitions_to_fatal_when_snapshot_lock_is_poisoned() {
    let initial_engine = GameEngine::default();
    let mut initial_ui_state = UiState::default();
    let runtime_stats =
        RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0);
    let snapshot = Arc::new(RwLock::new(RenderSnapshot {
        ui_snapshot: UiSnapshot::from_engine_and_ui(
            &initial_engine,
            &mut initial_ui_state,
            runtime_stats,
        ),
    }));

    let poison_result = std::panic::catch_unwind({
        let poisoned_snapshot = snapshot.clone();
        move || {
            let _guard = poisoned_snapshot.write().expect("snapshot write lock");
            panic!("poison snapshot lock");
        }
    });
    assert!(poison_result.is_err());

    let worker = SimulationWorker::new(
        initial_engine,
        snapshot,
        0.0,
        Duration::from_millis(500),
        Arc::new(AtomicU8::new(SimulationMode::Realtime.as_atomic_u8())),
        None,
    );
    worker
        .send_domain(GameCommand::SetupEditChar('1'))
        .expect("initial enqueue should succeed");

    let fatal_reason = wait_for_worker_fatal(&worker);
    assert_eq!(fatal_reason, WorkerFatalReason::SnapshotLockPoisoned);

    let fatal_events = worker.drain_events();
    assert!(
        fatal_events
            .iter()
            .any(|event| matches!(event, WorkerEvent::Fatal(_))),
        "expected fatal worker event"
    );

    let enqueue_after_fatal = worker.send_domain(GameCommand::SetupNextField);
    assert_eq!(enqueue_after_fatal, Err(WorkerError::WORKER_UNHEALTHY));
}

#[test]
fn worker_emits_export_errors_without_becoming_fatal() {
    let initial_engine = GameEngine::default();
    let mut initial_ui_state = UiState::default();
    let runtime_stats =
        RuntimeStatsSnapshot::new(SimulationMode::Realtime, 0.0, 0.0, 0, 0.0, 0, 0.0, 0.0);
    let snapshot = Arc::new(RwLock::new(RenderSnapshot {
        ui_snapshot: UiSnapshot::from_engine_and_ui(
            &initial_engine,
            &mut initial_ui_state,
            runtime_stats,
        ),
    }));

    let mut export_path = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    export_path.push(format!(
        "country-sim-export-error-{}-{}.tmp",
        std::process::id(),
        nonce
    ));
    fs::write(&export_path, b"not-a-directory").expect("write temp file");

    let worker = SimulationWorker::new(
        initial_engine,
        snapshot.clone(),
        0.0,
        Duration::from_millis(500),
        Arc::new(AtomicU8::new(SimulationMode::Realtime.as_atomic_u8())),
        Some(export_path.clone()),
    );

    worker.send_and_wait(GameCommand::SetupEditChar('1'));
    worker.send_and_wait(GameCommand::SetupNextField);
    worker.send_and_wait(GameCommand::SetupEditChar('1'));
    worker.send_and_wait(GameCommand::SetupEditChar('0'));
    worker.send_and_wait(GameCommand::SetupSubmit);
    wait_for_running_phase(&snapshot);

    for _ in 0..35 {
        worker
            .send_domain(GameCommand::Tick)
            .expect("tick enqueue should succeed");
    }

    let mut saw_export_error = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        for event in worker.drain_events() {
            if matches!(event, WorkerEvent::ExportError(_)) {
                saw_export_error = true;
            }
        }
        if saw_export_error {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(saw_export_error, "expected export error event");
    assert_eq!(worker.health(), WorkerHealth::Healthy);

    let _ = fs::remove_file(export_path);
}
