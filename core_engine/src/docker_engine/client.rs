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
        // Ping to check if it's running and accessible
        match self.docker.ping().await {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_msg = e.to_string();
                let mut is_permission_issue = err_msg.contains("Permission denied") || err_msg.contains("os error 13");
                
                if !is_permission_issue {
                    // It might not be running. Try to start it using systemctl.
                    let mut cmd = Command::new("systemctl");
                    cmd.arg("start").arg("docker");
                    let output = cmd.output().map_err(|e| format!("Failed to execute systemctl: {}", e))?;
                    
                    if output.status.success() {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        if self.docker.ping().await.is_ok() {
                            return Ok(());
                        } else {
                            // It's running but still failing, assume it's a permission issue.
                            is_permission_issue = true;
                        }
                    } else {
                        // systemctl failed (probably needs root), try pkexec
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let mut pk_cmd = Command::new("pkexec");
                        pk_cmd.arg("systemctl").arg("start").arg("docker");
                        let pk_output = pk_cmd.output().map_err(|e| format!("Failed to execute pkexec: {}", e))?;
                        
                        if pk_output.status.success() {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            if self.docker.ping().await.is_ok() {
                                return Ok(());
                            } else {
                                is_permission_issue = true;
                            }
                        } else {
                            return Err(format!("Could not start Docker daemon. Please start it manually. Stderr: {}", stderr));
                        }
                    }
                }
                
                if is_permission_issue {
                    // We lack permissions to the socket. Elevate to fix permissions temporarily.
                    let mut cmd = Command::new("pkexec");
                    cmd.arg("chmod").arg("666").arg("/var/run/docker.sock");
                    let output = cmd.output().map_err(|e| format!("Failed to run pkexec chmod: {}", e))?;
                    
                    if output.status.success() {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        if self.docker.ping().await.is_ok() {
                            return Ok(());
                        }
                    }
                    return Err("Permission denied to /var/run/docker.sock. Please run 'sudo chmod 666 /var/run/docker.sock' manually.".to_string());
                }
                
                Err(format!("Docker error: {}", err_msg))
            }
        }
    }
}
