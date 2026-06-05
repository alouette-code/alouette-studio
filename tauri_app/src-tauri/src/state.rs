use core_engine::{ProcessManager, ResourceMonitor};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub process_manager: Arc<Mutex<ProcessManager>>,
    pub resource_monitor: Arc<ResourceMonitor>,
}

/// Resolve the project root (parent of src-tauri) so that
/// app_data, logs, etc. live outside Tauri's dev file watcher scope.
pub fn project_root() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf()
}

/// Convenience: get the app_data directory path.
pub fn app_data_dir() -> std::path::PathBuf {
    project_root().join("app_data")
}

pub fn log_to_app_file(msg: &str) {
    let log_dir = project_root().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("app.log");

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
        use std::io::Write;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}
