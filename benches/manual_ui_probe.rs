use std::time::Duration;

use country_sim::components::simulation::manual_harness::run_max_speed_ui_probe;

fn main() {
    let report = run_max_speed_ui_probe(Duration::from_secs(20));
    println!(
        "MAX_SPEED_UI_PROBE days_start={} days_end={} days_advanced={} max_snapshot_age_ms={:.2} cmd_latency_avg_ms={:.2} cmd_latency_p95_ms={:.2} samples={}",
        report.day_start,
        report.day_end,
        report.day_end.saturating_sub(report.day_start),
        report.max_snapshot_age_ms,
        report.avg_command_latency_ms,
        report.p95_command_latency_ms,
        report.samples
    );
    assert!(
        report.day_end > report.day_start,
        "simulation did not advance in max-speed mode"
    );
}
