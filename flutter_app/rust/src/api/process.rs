use core_engine::config::{ProjectConfig, ProjectsConfig};
use core_engine::process::ProcessManager;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

fn app_data_dir() -> PathBuf {
    // When running inside Flutter, current_dir is typically the project root.
    // If app_data/ exists there, use it; otherwise fallback.
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidate = cwd.join("app_data");
    if candidate.exists() {
        candidate
    } else {
        // Fallback: try parent (for development from within rust/)
        cwd.parent()
            .map(|p| p.join("app_data"))
            .unwrap_or(candidate)
    }
}

fn projects_config_path() -> PathBuf {
    app_data_dir().join("projects.toml")
}

fn logs_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    cwd.join("logs")
}

static PROCESS_MANAGER: Lazy<Arc<Mutex<ProcessManager>>> = Lazy::new(|| {
    let path = logs_dir();
    Arc::new(Mutex::new(ProcessManager::new(path)))
});

/// Greeting function to verify FRB connection.
pub fn greet(name: String) -> String {
    format!("Hello {name}, from Alouette Rust Engine v0.1.0!")
}

/// Start a project process
pub async fn start_project_process(project_id: String) -> Result<(), String> {
    let mut pm = PROCESS_MANAGER.lock().await;
    let config_path = projects_config_path();
    let configs = ProjectsConfig::load_from_file(&config_path)?;
    let cfg = configs
        .projects
        .into_iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("Project '{project_id}' not found"))?;
    pm.register_project(cfg).await?;
    pm.start_process(&project_id).await?;
    Ok(())
}

/// Stop a project process
pub async fn stop_project_process(project_id: String) -> Result<(), String> {
    let mut pm = PROCESS_MANAGER.lock().await;
    pm.stop_process(&project_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all registered projects
pub async fn get_projects() -> Result<Vec<ProjectInfo>, String> {
    let config_path = projects_config_path();
    let configs = ProjectsConfig::load_from_file(&config_path)?;
    Ok(configs
        .projects
        .into_iter()
        .map(|p| ProjectInfo {
            id: p.id,
            name: p.name,
            command: p.command,
            args: p.args,
            cwd: p.cwd,
            auto_restart: p.auto_restart.unwrap_or(false),
            port: p.port.map(|p| p as i32),
        })
        .collect())
}

/// Register a new project
pub async fn register_project(
    name: String,
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    auto_restart: bool,
    port: Option<i32>,
) -> Result<String, String> {
    let config_path = projects_config_path();
    let mut configs = ProjectsConfig::load_from_file(&config_path)?;
    let id = uuid::Uuid::new_v4().to_string();
    configs.projects.push(ProjectConfig {
        id: id.clone(),
        name,
        command,
        args,
        cwd,
        setup_command: None,
        setup_args: None,
        auto_restart: Some(auto_restart),
        env: None,
        max_cpu_percent: None,
        max_ram_mb: None,
        port: port.map(|p| p as u16),
        source: None,
        terminal_mode: None,
        toolchain: None,
        toolchain_version: None,
        enable_tunnel: Some(false),
        max_log_lines: None,
    });
    configs.save_to_file(&config_path)?;
    Ok(id)
}

/// Delete a project
pub async fn deregister_project(project_id: String) -> Result<(), String> {
    let config_path = projects_config_path();
    let mut configs = ProjectsConfig::load_from_file(&config_path)?;
    configs.projects.retain(|p| p.id != project_id);
    configs.save_to_file(&config_path)
}

/// Get project process logs
pub async fn get_project_logs(project_id: String) -> Result<Vec<LogLine>, String> {
    let pm = PROCESS_MANAGER.lock().await;
    let logs = pm
        .db_manager
        .get_logs(&project_id, 1000)
        .map_err(|e| e.to_string())?;
    Ok(logs
        .into_iter()
        .map(|l| LogLine {
            text: l.text,
            stream: l.stream,
            timestamp: l.timestamp as i64,
        })
        .collect())
}

/// Force kill a process
pub async fn force_kill_process(pid: i32) -> Result<(), String> {
    core_engine::process::terminate_process_tree(pid as u32).await;
    Ok(())
}

// ── Data types exposed to Dart ──

#[derive(serde::Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub auto_restart: bool,
    pub port: Option<i32>,
}

#[derive(serde::Serialize)]
pub struct LogLine {
    pub text: String,
    pub stream: String,
    pub timestamp: i64,
}
