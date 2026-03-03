use super::helpers::should_publish_snapshot_for_tick;
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
