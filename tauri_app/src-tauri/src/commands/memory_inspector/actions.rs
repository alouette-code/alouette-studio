use core_engine::memory_inspector::MemoryInspectorManager;
use tokio::sync::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, State, Manager};
use super::events::emit_telemetry;

#[tauri::command]
pub async fn start_memory_inspection(
    app: AppHandle,
    state: State<'_, Arc<Mutex<MemoryInspectorManager>>>,
    _project_id: String,
    image: String,
    initial_ram: f64
) -> Result<(), String> {
    let mut manager = state.lock().await;
    manager.start_isolation(&image, initial_ram).await?;

    let manager_clone = state.inner().clone();
    let app_clone = app.clone();
    
    // Spawn background task for tick loop
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let mut m = manager_clone.lock().await;
            
            // If idle or finished, break loop
            match m.state {
                core_engine::memory_inspector::models::InspectorState::Idle |
                core_engine::memory_inspector::models::InspectorState::Finished => break,
                _ => {}
            }

            match m.tick().await {
                Ok(telemetry) => {
                    emit_telemetry(&app_clone, telemetry);
                }
                Err(e) => {
                    eprintln!("Memory inspector tick error: {}", e);
                }
            }
        }
    });

    Ok(())
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
