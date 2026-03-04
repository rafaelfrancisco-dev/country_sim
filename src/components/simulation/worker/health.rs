use std::collections::VecDeque;

use super::*;

pub(super) fn push_event(events: &Arc<Mutex<VecDeque<WorkerEvent>>>, event: WorkerEvent) {
    if let Ok(mut queue) = events.lock() {
        queue.push_back(event);
    }
}

pub(super) fn mark_fatal(
    health: &Arc<Mutex<WorkerHealth>>,
    events: &Arc<Mutex<VecDeque<WorkerEvent>>>,
    reason: WorkerFatalReason,
) {
    if let Ok(mut state) = health.lock()
        && matches!(&*state, WorkerHealth::Healthy)
    {
        let message = reason.as_message();
        *state = WorkerHealth::Fatal(reason);
        drop(state);
        push_event(events, WorkerEvent::Fatal(message));
    }
}
