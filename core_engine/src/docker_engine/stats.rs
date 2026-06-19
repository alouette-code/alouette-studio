use bollard::container::StatsOptions;
use bollard::Docker;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
}

pub async fn stream_container_stats(
    docker: Docker,
    id: String,
    tx: mpsc::Sender<ContainerStats>,
) -> Result<(), String> {
    let options = Some(StatsOptions {
        stream: true,
        one_shot: false,
    });

    let mut stream = docker.stats(&id, options);

    while let Some(stats_result) = stream.next().await {
        match stats_result {
            Ok(stats) => {
                let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64 - stats.precpu_stats.cpu_usage.total_usage as f64;
                let system_cpu_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64 - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
                let online_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
                
                let mut cpu_percent = 0.0;
                if system_cpu_delta > 0.0 && cpu_delta > 0.0 {
                    cpu_percent = (cpu_delta / system_cpu_delta) * online_cpus * 100.0;
                }

                let memory_usage_bytes = stats.memory_stats.usage.unwrap_or(0);
                let memory_limit_bytes = stats.memory_stats.limit.unwrap_or(0);

                let parsed_stats = ContainerStats {
                    cpu_percent,
                    memory_usage_bytes,
                    memory_limit_bytes,
                };

                if tx.send(parsed_stats).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    Ok(())
}
