use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use sysinfo::System;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    pub project_id: String,
    pub cpu_percentage: f32,
    pub ram_bytes: u64,
    pub processes: Vec<crate::process::ChildProcessInfo>,
}

pub enum MonitorCommand {
    Register { project_id: String, pid: u32 },
    Deregister { project_id: String },
}

pub struct ResourceMonitor {
    cmd_tx: std::sync::mpsc::Sender<MonitorCommand>,
    stats_tx: broadcast::Sender<ResourceStats>,
}

impl ResourceMonitor {
    /// Spawns the dedicated resource-tracking worker thread and returns the controller.
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let (stats_tx, _) = broadcast::channel(100);
        let stats_tx_clone = stats_tx.clone();

        thread::spawn(move || {
            run_monitor_loop(cmd_rx, stats_tx_clone);
        });

        ResourceMonitor { cmd_tx, stats_tx }
    }

    /// Registers a parent PID to begin scanning its resource footprint.
    pub fn register(&self, project_id: String, pid: u32) {
        let _ = self
            .cmd_tx
            .send(MonitorCommand::Register { project_id, pid });
    }

    /// Stops tracking resource metrics for a specific project tab.
    pub fn deregister(&self, project_id: String) {
        let _ = self.cmd_tx.send(MonitorCommand::Deregister { project_id });
    }

    /// Subscribes to the broadcast stream of aggregated process resource stats.
    pub fn subscribe(&self) -> broadcast::Receiver<ResourceStats> {
        self.stats_tx.subscribe()
    }
}

/// The private system background thread loop that operates independent of the async executors.
fn run_monitor_loop(
    cmd_rx: std::sync::mpsc::Receiver<MonitorCommand>,
    stats_tx: broadcast::Sender<ResourceStats>,
) {
    let mut sys = System::new_all();
    let mut active_projects: HashMap<String, u32> = HashMap::new();

    // Query core count for core normalization of raw CPU percentages
    let cpus = sys.cpus();
    let core_count = if !cpus.is_empty() {
        cpus.len() as f32
    } else {
        1.0
    };

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                MonitorCommand::Register { project_id, pid } => {
                    active_projects.insert(project_id, pid);
                }
                MonitorCommand::Deregister { project_id } => {
                    active_projects.remove(&project_id);
                }
            }
        }

        // Exit loop if all output receivers are dropped
        if stats_tx.receiver_count() == 0 && active_projects.is_empty() {
            // Keep thread alive but sleep longer to avoid burning CPU when idle
            thread::sleep(Duration::from_millis(1000));
            continue;
        }

        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        for (project_id, &parent_pid) in &active_projects {
            let processes = crate::process::collect_child_processes(parent_pid, &sys, core_count);
            let total_ram: u64 = processes.iter().map(|p| p.ram_bytes).sum();
            let normalized_cpu: f32 = processes.iter().map(|p| p.cpu_percentage).sum();

            let stats = ResourceStats {
                project_id: project_id.clone(),
                cpu_percentage: normalized_cpu,
                ram_bytes: total_ram,
                processes,
            };

            let _ = stats_tx.send(stats);
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_resource_monitor_registration_and_polling() {
        let monitor = ResourceMonitor::new();
        let mut rx = monitor.subscribe();

        // Register own process ID (always active and safe to query)
        let own_pid = std::process::id();

        monitor.register("test-self".to_string(), own_pid);

        // Wait for at least one resource aggregation tick
        let tick_res = tokio::time::timeout(Duration::from_millis(2500), rx.recv()).await;

        assert!(
            tick_res.is_ok(),
            "Failed to receive resource update tick within timeout"
        );
        let stats = tick_res.unwrap().unwrap();

        assert_eq!(stats.project_id, "test-self");
        assert!(
            stats.ram_bytes > 0,
            "Self RAM RSS footprint should be positive"
        );

        monitor.deregister("test-self".to_string());
    }
}
