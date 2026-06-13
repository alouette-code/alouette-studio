use core_engine::process::terminal::process_and_send_terminal_input;
use core_engine::process::ProcessManager;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

fn logs_dir() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    cwd.join("logs")
}

static PROCESS_MANAGER: Lazy<Arc<Mutex<ProcessManager>>> =
    Lazy::new(|| Arc::new(Mutex::new(ProcessManager::new(logs_dir()))));

/// Spawn a new terminal session for a project
pub async fn spawn_terminal_session(
    _project_id: String,
    cwd: Option<String>,
) -> Result<String, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let mut pm = PROCESS_MANAGER.lock().await;
    pm.spawn_terminal(&session_id, cwd.as_deref(), false)
        .await
        .map_err(|e| e.to_string())?;
    Ok(session_id)
}

/// Write data to a terminal session
pub async fn write_to_terminal_session(session_id: String, data: String) -> Result<(), String> {
    let pm = PROCESS_MANAGER.lock().await;
    let ctx = pm
        .get_terminal_write_context(&session_id)
        .map_err(|e| e.to_string())?;
    drop(pm);
    process_and_send_terminal_input(&session_id, data, &ctx).await
}

/// Kill a terminal session
pub async fn kill_terminal_session(session_id: String) -> Result<(), String> {
    let mut pm = PROCESS_MANAGER.lock().await;
    pm.kill_terminal(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Resize a terminal session
pub async fn resize_terminal_session(
    session_id: String,
    cols: i32,
    rows: i32,
) -> Result<(), String> {
    let pm = PROCESS_MANAGER.lock().await;
    pm.resize_terminal(&session_id, cols as u16, rows as u16)
        .map_err(|e| e.to_string())
}
