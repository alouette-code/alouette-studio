use core_engine::agent_harness::session::SessionEntry;
use core_engine::{ProcessManager, ResourceMonitor};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use core_engine::agent_harness::AgentSession;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopState {
    pub max_iterations: u32,
    pub auto_approve_reads: bool,
    pub auto_approve_writes: bool,
    pub auto_approve_all: bool,
    pub command_timeout_secs: u64,
    pub iteration_count: u32,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            auto_approve_reads: true,
            auto_approve_writes: false,
            auto_approve_all: false,
            command_timeout_secs: 120,
            iteration_count: 0,
        }
    }
}

pub struct AppState {
    pub process_manager: Arc<tokio::sync::Mutex<ProcessManager>>,
    pub resource_monitor: Arc<ResourceMonitor>,
    pub agent_cancel_flag: Arc<AtomicBool>,
    pub agent_session: Arc<std::sync::Mutex<Option<AgentSession>>>,
    pub agent_loop_state: Arc<std::sync::Mutex<Option<LoopState>>>,
    pub db_pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    pub agent_harness: Arc<tokio::sync::Mutex<core_engine::agent_harness::AgentHarness>>,
    // ─── NEW: Multi-session registry ───
    pub agent_registry: Arc<DashMap<String, SessionEntry>>,
    pub active_agent_project: Arc<RwLock<Option<String>>>,
    pub vm_manager: Arc<core_engine::vm_engine::VmManager>,
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
