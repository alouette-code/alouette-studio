use core_engine::memory_inspector::models::TelemetryData;
use tauri::{AppHandle, Emitter};

pub fn emit_telemetry(app: &AppHandle, telemetry: TelemetryData) {
    let _ = app.emit("memory-inspector-telemetry", telemetry);
}
