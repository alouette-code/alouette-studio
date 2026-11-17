use bollard::Docker;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    /// Serializes concurrent `ensure_started` calls so at most one elevation prompt is shown.
    static ref ENSURE_STARTED_LOCK: Mutex<()> = Mutex::new(());
}

pub struct DockerClient {
    pub docker: Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self, String> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| format!("Failed to connect to Docker: {}", e))?;
        Ok(DockerClient { docker })
    }

    pub async fn ensure_started(&self) -> Result<(), String> {
        // Fast path: Docker daemon is already running and accessible
        if self.docker.ping().await.is_ok() {
            return Ok(());
        }

        // Single-flight lock prevents concurrent callers (e.g. React StrictMode / multiple panels)
        // from spawning duplicate authentication dialogs.
        let _guard = ENSURE_STARTED_LOCK.lock().await;

        // Re-check after acquiring lock in case an earlier caller already started/fixed Docker
        if self.docker.ping().await.is_ok() {
            return Ok(());
        }

        // Non-interactive attempt first: `--no-ask-password` ensures systemctl never triggers
        // its own separate Polkit password dialog before our single `pkexec` call.
        let _ = Command::new("systemctl")
            .args(["--no-ask-password", "start", "docker"])
            .output()
            .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        if self.docker.ping().await.is_ok() {
            return Ok(());
        }

        // Single consolidated elevated prompt for starting the daemon and updating socket permissions
        let pk_output = Command::new("pkexec")
            .args([
                "sh",
                "-c",
                "systemctl start docker && chmod 666 /var/run/docker.sock",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to execute pkexec: {}", e))?;

        if pk_output.status.success() {
            for _ in 0..10 {
                if self.docker.ping().await.is_ok() {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            return Err("Docker daemon started, but the Docker socket did not respond in time.".to_string());
        }

        let stderr = String::from_utf8_lossy(&pk_output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err("Authentication cancelled or failed. Could not start Docker daemon.".to_string())
        } else {
            Err(format!("Could not start Docker daemon automatically: {}", stderr))
        }
    }
}
