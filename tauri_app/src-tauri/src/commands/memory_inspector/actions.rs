use core_engine::memory_inspector::MemoryInspectorManager;
use tokio::sync::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, State, Manager};
use super::events::emit_telemetry;

use core_engine::memory_inspector::models::{InspectionConfig, TaskRecord};

#[tauri::command]
pub async fn start_memory_inspection(
    app: AppHandle,
    state: State<'_, Arc<Mutex<MemoryInspectorManager>>>,
    config: InspectionConfig
) -> Result<String, String> {
    let mut manager = state.lock().await;
    let task_id = manager.start_inspection(config).await?;

    let manager_clone = state.inner().clone();
    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    
    // Spawn background task for tick loop
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let mut m = manager_clone.lock().await;
            
            // If a new task was started or inspection stopped, break loop
            if m.current_task_id.as_deref() != Some(task_id_clone.as_str()) {
                break;
            }

            match m.tick().await {
                Ok(telemetry) => {
                    let status = telemetry.status.clone();
                    emit_telemetry(&app_clone, telemetry);
                    if status == "Finished" || status == "Error" || status.starts_with("Error") {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Memory inspector tick error: {}", e);
                    break;
                }
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
pub async fn get_task_history(state: State<'_, Arc<Mutex<MemoryInspectorManager>>>) -> Result<Vec<TaskRecord>, String> {
    let manager = state.lock().await;
    let mut history: Vec<TaskRecord> = manager.tasks.values().cloned().collect();
    history.sort_by_key(|t| std::cmp::Reverse(t.start_time));
    Ok(history)
}

#[tauri::command]
pub async fn stop_memory_inspection(state: State<'_, Arc<Mutex<MemoryInspectorManager>>>) -> Result<(), String> {
    let mut manager = state.lock().await;
    manager.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn open_memory_inspector_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("memory_inspector_window") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "memory_inspector_window",
        tauri::WebviewUrl::App("index.html?window=memory-inspector".into()),
    )
    .title("Proto-Memory Inspector")
    .inner_size(1200.0, 800.0)
    .resizable(true)
    .decorations(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}
