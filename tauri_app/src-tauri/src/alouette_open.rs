use crate::state::AppState;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

pub static ALOUETTE_OPEN_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn is_alouette_open_enabled() -> bool {
    ALOUETTE_OPEN_ENABLED.load(Ordering::Relaxed)
}

pub fn set_alouette_open_enabled(enabled: bool) {
    ALOUETTE_OPEN_ENABLED.store(enabled, Ordering::Relaxed);
}

use std::sync::OnceLock;

fn get_last_processed_timestamps() -> &'static Mutex<HashMap<String, u64>> {
    static TIMESTAMPS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
    TIMESTAMPS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Spawns the Alouette Open log monitor background task.
/// Uses ONNX model inference if available, otherwise falls back to
/// lightweight heuristic keyword matching for error detection.
pub fn spawn_alouette_open_monitor(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Wait a few seconds for systems to initialize
        sleep(Duration::from_secs(5)).await;

        // ONNX model loading removed (using pure heuristics)
        println!("[Alouette Open] Using lightweight heuristic error detection.");

        loop {
            sleep(Duration::from_millis(500)).await;

            if !is_alouette_open_enabled() {
                continue;
            }

            let app_state = match app_handle.try_state::<AppState>() {
                Some(state) => state,
                None => continue,
            };

            // Lock ProcessManager to read projects
            let pm = app_state.process_manager.lock().await;
            let projects = pm.get_configs();
            let db = pm.db_manager.clone();
            drop(pm); // Release lock as soon as possible

            for project in projects {
                let project_id = project.id.clone();

                // Get logs (fetch up to latest 50 logs)
                if let Ok(logs) = db.get_logs(&project_id, 50) {
                    let mut last_timestamps = get_last_processed_timestamps().lock().unwrap();
                    let last_ts = *last_timestamps.get(&project_id).unwrap_or(&0);

                    let mut max_ts = last_ts;
                    let mut new_logs = Vec::new();

                    for log in logs {
                        if log.timestamp > last_ts {
                            if log.timestamp > max_ts {
                                max_ts = log.timestamp;
                            }
                            new_logs.push(log);
                        }
                    }

                    if !new_logs.is_empty() {
                        last_timestamps.insert(project_id.clone(), max_ts);
                        drop(last_timestamps); // release lock before processing

                        for log in new_logs {
                            let text_lower = log.text.to_lowercase();

                            // Heuristic validation
                            let is_heuristic_error = text_lower.contains("error")
                                || text_lower.contains("exception")
                                || text_lower.contains("failed")
                                || text_lower.contains("panic")
                                || text_lower.contains("critical")
                                || log.stream == "stderr";

                            if is_heuristic_error {
                                println!(
                                    "[Alouette Open] Found error in project [{}]: {}",
                                    project_id, log.text
                                );

                                // Emit Tauri event to front-end
                                let payload = serde_json::json!({
                                    "project_id": project_id,
                                    "project_name": project.name,
                                    "stream": log.stream,
                                    "text": log.text,
                                    "timestamp": log.timestamp,
                                    "cwd": project.cwd
                                });
                                let _ = app_handle.emit("alouette-open-error", payload);
                            }
                        }
                    }
                }
            }
        }
    });
}
