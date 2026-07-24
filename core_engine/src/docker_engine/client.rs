use bollard::Docker;
use std::process::Command;
use std::time::Duration;

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
        // Ping to check if Docker daemon is running and accessible
        if self.docker.ping().await.is_ok() {
            return Ok(());
        }

        // Attempt non-root systemctl start first
        let _ = Command::new("systemctl").arg("start").arg("docker").output();
        tokio::time::sleep(Duration::from_millis(500)).await;
        if self.docker.ping().await.is_ok() {
            return Ok(());
        }

        // Single elevated prompt combining systemctl start and socket permission fix
        let mut pk_cmd = Command::new("pkexec");
        pk_cmd.args(["sh", "-c", "systemctl start docker && chmod 666 /var/run/docker.sock"]);
        let pk_output = pk_cmd.output().map_err(|e| format!("Failed to execute pkexec: {}", e))?;

        if pk_output.status.success() {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if self.docker.ping().await.is_ok() {
                return Ok(());
            }
        }

        Err("Could not start Docker daemon automatically. Please start Docker service manually.".to_string())
    }
}
