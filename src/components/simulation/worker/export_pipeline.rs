use std::collections::VecDeque;

use super::health::push_event;
use super::*;

pub(super) struct WorkerExportState {
    export_csv_path: Option<PathBuf>,
    csv_exporter: Option<CsvExporter>,
    export_seed: Option<u64>,
    export_queue_activated: bool,
}

impl WorkerExportState {
    pub(super) fn new(export_csv: Option<PathBuf>) -> Self {
        Self {
            export_queue_activated: export_csv.is_none(),
            export_csv_path: export_csv,
            csv_exporter: None,
            export_seed: None,
        }
    }

    pub(super) fn drain_and_write(
        &mut self,
        engine: &mut GameEngine,
        events: &Arc<Mutex<VecDeque<WorkerEvent>>>,
    ) {
        if !self.export_queue_activated && matches!(engine.phase(), GamePhase::Running(_)) {
            engine.enable_export_queue();
            if self.export_seed.is_none() {
                self.export_seed = engine.world_generation_seed();
            }
            self.export_queue_activated = true;
        }

        let export_rows = engine.drain_export_queue();
        if !export_rows.is_empty() {
            if self.export_seed.is_none() {
                self.export_seed = export_rows.first().map(|row| row.generation_seed);
            }

            if let Some(exporter) = self.ensure_exporter(events) {
                for export_data in &export_rows {
                    for country_data in &export_data.countries {
                        if let Err(err) = exporter.write_row(country_data) {
                            tracing::error!("CSV write error: {err}");
                            push_event(
                                events,
                                WorkerEvent::ExportError(format!("csv write error: {err}")),
                            );
                        }
                    }
                }
            }
        }

        if self.export_csv_path.is_some() {
            let (stock_country_rows, stock_equity_rows) = engine.drain_stock_export_queues();
            let has_stock_rows = !stock_country_rows.is_empty() || !stock_equity_rows.is_empty();
            if has_stock_rows && let Some(exporter) = self.ensure_exporter(events) {
                for row in &stock_country_rows {
                    if let Err(err) = exporter.write_stock_country_row(row) {
                        tracing::error!("CSV stock country write error: {err}");
                        push_event(
                            events,
                            WorkerEvent::ExportError(format!(
                                "csv stock country write error: {err}"
                            )),
                        );
                    }
                }
                for row in &stock_equity_rows {
                    if let Err(err) = exporter.write_stock_equity_row(row) {
                        tracing::error!("CSV stock equity write error: {err}");
                        push_event(
                            events,
                            WorkerEvent::ExportError(format!(
                                "csv stock equity write error: {err}"
                            )),
                        );
                    }
                }
            }
        }
    }

    fn ensure_exporter(
        &mut self,
        events: &Arc<Mutex<VecDeque<WorkerEvent>>>,
    ) -> Option<&mut CsvExporter> {
        if self.csv_exporter.is_none()
            && let (Some(path), Some(seed)) = (&self.export_csv_path, self.export_seed)
        {
            match CsvExporter::new(path, seed) {
                Ok(exporter) => self.csv_exporter = Some(exporter),
                Err(err) => {
                    tracing::error!("failed to open CSV export: {err}");
                    push_event(
                        events,
                        WorkerEvent::ExportError(format!("csv open error: {err}")),
                    );
                }
            }
        }

        self.csv_exporter.as_mut()
    }
}
