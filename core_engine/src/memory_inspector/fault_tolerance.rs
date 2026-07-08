use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::fs;
use std::path::PathBuf;

use crate::memory_inspector::models::InspectorState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    pub task_id: String,
    pub state: InspectorState,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

/// Circuit Breaker tracks failures and prevents cascading errors
/// by tripping when a failure threshold is reached.
pub struct CircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    reset_timeout_secs: u64,
    last_failure_time: Option<u64>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout_secs: u64) -> Self {
        Self {
            failure_count: 0,
            failure_threshold,
            reset_timeout_secs,
            last_failure_time: None,
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.last_failure_time = None;
    }

    pub fn is_tripped(&self) -> bool {
        if self.failure_count >= self.failure_threshold {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if let Some(last_fail) = self.last_failure_time {
                if now - last_fail < self.reset_timeout_secs {
                    return true;
                }
            }
        }
        false
    }
}

/// CheckpointManager handles saving and loading task state.
pub struct CheckpointManager {
    base_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new(base_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&base_dir).unwrap_or_default();
        Self { base_dir }
    }

    pub async fn save_checkpoint(&self, checkpoint: &TaskCheckpoint) -> Result<(), String> {
        let path = self.base_dir.join(format!("{}.ckpt", checkpoint.task_id));
        let data = serde_json::to_string(checkpoint).map_err(|e| e.to_string())?;
        fs::write(path, data).await.map_err(|e| e.to_string())
    }

    pub async fn load_checkpoint(&self, task_id: &str) -> Result<TaskCheckpoint, String> {
        let path = self.base_dir.join(format!("{}.ckpt", task_id));
        let data = fs::read_to_string(path).await.map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }
}
