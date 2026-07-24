use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use crate::memory_inspector::models::TelemetryData;

pub struct EbpfTracer {
    target_pid: Option<u32>,
    abort_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl EbpfTracer {
    pub fn new() -> Self {
        Self {
            target_pid: None,
            abort_tx: None,
        }
    }

    pub fn set_target_pid(&mut self, pid: u32) {
        self.target_pid = Some(pid);
    }

    pub async fn start(&mut self, tx: mpsc::Sender<TelemetryData>) -> Result<(), String> {
        let pid = self.target_pid.ok_or("No target PID set for eBPF Tracer")?;
        
        let script = r#"
            uprobe:/lib/x86_64-linux-gnu/libc.so.6:malloc {
                @mallocs[pid] = count();
            }
            uprobe:/lib/x86_64-linux-gnu/libc.so.6:free {
                @frees[pid] = count();
            }
            profile:s:1 {
                print(@mallocs);
                print(@frees);
                clear(@mallocs);
                clear(@frees);
            }
        "#;

        let mut child = Command::new("bpftrace")
            .arg("-e")
            .arg(script)
            .arg("-p")
            .arg(pid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start bpftrace: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel::<()>();
        self.abort_tx = Some(abort_tx);

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            
            loop {
                tokio::select! {
                    _ = &mut abort_rx => {
                        let _ = child.kill().await;
                        break;
                    }
                    line = reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                // Real implementation would parse the bpftrace output and emit TelemetryData
                                // For now, we simulate parsing and emitting an activity
                                if l.contains("@mallocs") || l.contains("@frees") {
                                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                                    let data = TelemetryData {
                                        timestamp: now,
                                        memory_usage_mb: 0.0, // Should be populated by main pipeline
                                        memory_limit_mb: 0.0,
                                        thread_count: 0,
                                        gc_events_detected: 0,
                                        crash_imminent: false,
                                        status: "eBPF Tracing".to_string(),
                                        activities: vec![
                                            crate::memory_inspector::models::ProcessActivity {
                                                timestamp: now,
                                                event_type: "eBPF".to_string(),
                                                pid: pid.to_string(),
                                                details: l.clone(),
                                            }
                                        ],
                                        drift_rate_kb_per_sec: None,
                                        regression_r2: None,
                                    };
                                    let _ = tx.send(data).await;
                                }
                            }
                            _ => { break; }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.abort_tx.take() {
            let _ = tx.send(());
        }
    }
}
