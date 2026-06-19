use std::process::Command;
use crate::memory_inspector::models::TelemetryData;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DockerDriver;

impl super::ContainerDriver for DockerDriver {
    fn create_sandbox(&self, image: &str, name: &str, initial_memory_mb: f64) -> Result<(), String> {
        let mem_str = format!("{}m", initial_memory_mb);
        let status = Command::new("docker")
            .args(["run", "-d", "--name", name, "-m", &mem_str, image])
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err("Failed to create docker sandbox".to_string());
        }
        Ok(())
    }

    fn update_memory_limit(&self, name: &str, memory_mb: f64) -> Result<(), String> {
        let mem_str = format!("{}m", memory_mb);
        let status = Command::new("docker")
            .args(["update", "-m", &mem_str, name])
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err("Failed to update docker memory limit".to_string());
        }
        Ok(())
    }

    fn get_stats(&self, name: &str) -> Result<TelemetryData, String> {
        // format expected: "25.5MiB / 512MiB"
        let output = Command::new("docker")
            .args(["stats", "--no-stream", "--format", "{{.MemUsage}}", name])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err("Failed to get docker stats".to_string());
        }

        let stat_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stat_str.trim().split(" / ").collect();
        
        let mut usage_mb = 0.0;
        let mut limit_mb = 0.0;
        
        if parts.len() == 2 {
            usage_mb = parse_docker_mem(parts[0]);
            limit_mb = parse_docker_mem(parts[1]);
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Ok(TelemetryData {
            timestamp: now,
            memory_usage_mb: usage_mb,
            memory_limit_mb: limit_mb,
            gc_events_detected: 0,
            crash_imminent: false,
        })
    }

    fn destroy_sandbox(&self, name: &str) -> Result<(), String> {
        let _ = Command::new("docker").args(["rm", "-f", name]).status();
        Ok(())
    }
}

fn parse_docker_mem(mem_str: &str) -> f64 {
    let s = mem_str.to_lowercase();
    if s.ends_with("mib") {
        s.replace("mib", "").trim().parse().unwrap_or(0.0)
    } else if s.ends_with("gib") {
        s.replace("gib", "").trim().parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if s.ends_with("kib") {
        s.replace("kib", "").trim().parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if s.ends_with("b") {
        s.replace("b", "").trim().parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else {
        0.0
    }
}
