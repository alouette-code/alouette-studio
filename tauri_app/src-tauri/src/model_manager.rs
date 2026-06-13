use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::process::{Child, Command};

// ──────────────────────────────────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────────────────────────────────

const LLAMA_PORT: u16 = 8080;
const HEALTH_URL: &str = "http://127.0.0.1:8080/health";
const MAX_STARTUP_WAIT_SECS: u64 = 30;
const HEALTH_POLL_INTERVAL_MS: u64 = 500;

// ──────────────────────────────────────────────────────────────────────────────
// Managed State
// ──────────────────────────────────────────────────────────────────────────────

pub struct ModelManager {
    child: Option<Child>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self { child: None }
    }

    /// Ensure the local model server is running.
    /// If it's already healthy, returns Ok immediately.
    /// If not, kills any process on the port, spawns a new server, and waits for it.
    pub async fn ensure_running(&mut self) -> Result<(), String> {
        if self.is_healthy().await {
            return Ok(());
        }

        // 1. Kill any stale process holding our port
        self.kill_port_holder().await;

        // 2. Kill any previously tracked child that may have been orphaned
        if let Some(mut old_child) = self.child.take() {
            let _ = old_child.kill().await;
            let _ = old_child.wait().await;
        }

        // 3. Resolve paths
        let bin_path = resolve_llama_server_path();
        let model_path = resolve_model_path();
        let log_dir = crate::state::project_root().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);

        let log_file = std::fs::File::create(log_dir.join("llama-server.log"))
            .map_err(|e| format!("Cannot create log file: {}", e))?;

        // 4. Spawn the server
        let child = Command::new(&bin_path)
            .args([
                "-m",
                &model_path.to_string_lossy(),
                "-c",
                "4096",
                "--port",
                &LLAMA_PORT.to_string(),
            ])
            .stdout(Stdio::from(log_file.try_clone().map_err(|e| format!("Cannot clone log: {}", e))?))
            .stderr(Stdio::from(log_file))
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", bin_path.display(), e))?;

        self.child = Some(child);

        // 5. Wait for server to become healthy
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(MAX_STARTUP_WAIT_SECS);
        while std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
            if self.is_healthy().await {
                crate::state::log_to_app_file(&format!(
                    "Model server started on port {}",
                    LLAMA_PORT
                ));
                return Ok(());
            }

            // Check if the child process has exited early
            if let Some(ref mut child) = self.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        self.child = None;
                        return Err(format!(
                            "Model server exited prematurely with code {:?}",
                            status.code()
                        ));
                    }
                    Err(e) => {
                        self.child = None;
                        return Err(format!("Error checking model server: {}", e));
                    }
                    Ok(None) => {} // still running
                }
            }
        }

        Err(format!(
            "Model server did not become healthy within {} seconds (port {})",
            MAX_STARTUP_WAIT_SECS, LLAMA_PORT
        ))
    }

    /// Stop the model server gracefully.
    pub async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            crate::state::log_to_app_file("Model server stopped");
        }
    }

    /// Check if the model server is healthy.
    async fn is_healthy(&self) -> bool {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .ok();
        match client {
            Some(c) => match c.get(HEALTH_URL).send().await {
                Ok(res) => res.status().is_success(),
                Err(_) => false,
            },
            None => false,
        }
    }

    /// Find and kill any process listening on our port.
    async fn kill_port_holder(&self) {
        // Reuse the same logic as check_port_status from network commands
        let pid = find_pid_on_port(LLAMA_PORT).await;
        if let Some(pid) = pid {
            crate::state::log_to_app_file(&format!(
                "Killing stale process {} on port {}",
                pid, LLAMA_PORT
            ));
            core_engine::terminate_process_tree(pid).await;
            // Give it a moment to release the port
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Path Resolution
// ──────────────────────────────────────────────────────────────────────────────

fn resolve_llama_server_path() -> PathBuf {
    let hardcoded = PathBuf::from(
        "/home/nhatanh/projet/alouette_studio/core_engine/app_data/bin/llama-bin/llama-server",
    );
    if hardcoded.exists() {
        return hardcoded;
    }
    if let Some(parent) = crate::state::project_root().parent() {
        let p = parent.join("core_engine/app_data/bin/llama-bin/llama-server");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("llama-server")
}

fn resolve_model_path() -> PathBuf {
    // Try the actual Q3_K_M model (the Q4 file might not exist)
    let q3_path = PathBuf::from(
        "/home/nhatanh/projet/alouette_studio/tauri_app/app_data/model_embedding/model-small-phi-3/phi-3-mini-4k-instruct-q3_k_m.gguf",
    );
    if q3_path.exists() {
        return q3_path;
    }
    // Fallback to Q4
    let q4_path = crate::state::project_root()
        .join("app_data/model_embedding/model-small-phi-3/phi-3-mini-4k-instruct-q4_k_m.gguf");
    if q4_path.exists() {
        return q4_path;
    }
    q3_path
}

// ──────────────────────────────────────────────────────────────────────────────
// Port Detection (cross-platform)
// ──────────────────────────────────────────────────────────────────────────────

async fn find_pid_on_port(port: u16) -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("netstat").args(["-ano", "-p", "tcp"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let port_suffix = format!(":{}", port);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let local_addr = parts[1];
                    let state = parts[3];
                    let pid_str = parts[4];
                    if (local_addr.ends_with(&port_suffix) || local_addr.contains(&format!("]:{}", port)))
                        && state == "LISTENING"
                    {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if pid > 0 {
                                return Some(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("lsof")
            .args(["-t", &format!("-i:{}", port)])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                if let Ok(pid) = first_line.trim().parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }

    None
}

// ──────────────────────────────────────────────────────────────────────────────
// Factory / Convenience
// ──────────────────────────────────────────────────────────────────────────────

pub type SharedModelManager = Arc<Mutex<ModelManager>>;

pub fn create_shared() -> SharedModelManager {
    Arc::new(Mutex::new(ModelManager::new()))
}
