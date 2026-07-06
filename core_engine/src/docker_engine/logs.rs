use bollard::container::{LogsOptions, LogOutput};
use bollard::Docker;
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub async fn stream_container_logs(
    docker: Docker,
    id: String,
    tx: mpsc::Sender<String>,
    since: i64,
) -> Result<(), String> {
    let options = Some(LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: if since > 0 { "all".to_string() } else { "100".to_string() },
        since,
        ..Default::default()
    });

    let mut stream = docker.logs(&id, options);

    while let Some(log_result) = stream.next().await {
        match log_result {
            Ok(log_output) => {
                let msg = match log_output {
                    LogOutput::StdOut { message } => String::from_utf8_lossy(&message).to_string(),
                    LogOutput::StdErr { message } => String::from_utf8_lossy(&message).to_string(),
                    LogOutput::Console { message } => String::from_utf8_lossy(&message).to_string(),
                    LogOutput::StdIn { message } => String::from_utf8_lossy(&message).to_string(),
                };
                if tx.send(msg).await.is_err() {
                    break; // Receiver dropped
                }
            }
            Err(e) => {
                let _ = tx.send(format!("Error reading logs: {}", e)).await;
                break;
            }
        }
    }

    Ok(())
}
