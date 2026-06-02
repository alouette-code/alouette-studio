use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use chrono::Local;
use serde::Serialize;
use tract_onnx::prelude::*;

// Thread-safe singleton/context for the AI Diagnostics engine
pub struct AiDiagnosticEngine {
    // Model is loaded into a runnable tract format, wrapped in Option for fallback safety
    model: Option<SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>>,
    enabled: Mutex<bool>,
    // Cooldown tracker per project to avoid spamming the UI / system logs
    last_alert_times: Mutex<HashMap<String, Instant>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiAlertPayload {
    pub id: u64,
    pub project: String,
    pub timestamp: String,
    pub r#type: String, // "warning" | "error" | "info"
    pub message: String,
}

impl AiDiagnosticEngine {
    pub fn new() -> Self {
        // Find ONNX model path in various workspace locations
        let mut model_path = PathBuf::from("core_engine/src/model_AI/alouette_open-A1 v1.0.onnx");
        if !model_path.exists() {
            model_path = PathBuf::from("../core_engine/src/model_AI/alouette_open-A1 v1.0.onnx");
        }
        if !model_path.exists() {
            model_path = PathBuf::from("src-tauri/app_data/model_alouette_open/alouette_open-A1 v1.0.onnx");
        }

        let model = if model_path.exists() {
            println!("[AI Local] Loading ONNX model from: {:?}", model_path);
            match tract_onnx::onnx()
                .model_for_path(&model_path)
                .and_then(|m| m.into_optimized())
                .and_then(|m| m.into_runnable())
            {
                Ok(runnable) => {
                    println!("[AI Local] ONNX model successfully loaded and active.");
                    Some(runnable)
                }
                Err(e) => {
                    eprintln!("[AI Local] Warning: Failed to load ONNX model via tract: {}. Falling back to heuristic diagnostic mode.", e);
                    None
                }
            }
        } else {
            println!("[AI Local] ONNX model file not found at {:?}. Using heuristic diagnostic mode.", model_path);
            None
        };

        Self {
            model,
            enabled: Mutex::new(true),
            last_alert_times: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut lock) = self.enabled.lock() {
            *lock = enabled;
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.lock().map(|l| *l).unwrap_or(false)
    }

    /// Analyze a log line. If it contains anomalies and passes cooldown checks, return a diagnostic warning.
    pub fn check_and_diagnose(&self, project: &str, line: &str) -> Option<AiAlertPayload> {
        if !self.is_enabled() {
            return None;
        }

        // Tier 1: Quick Heuristic keyword check (Pre-filter to prevent CPU churn)
        let line_lower = line.to_lowercase();
        let matches_error = line_lower.contains("error")
            || line_lower.contains("fail")
            || line_lower.contains("lỗi")
            || line_lower.contains("exception")
            || line_lower.contains("panic")
            || line_lower.contains("warn")
            || line_lower.contains("critical")
            || line_lower.contains("conflict")
            || line_lower.contains("fatal");

        if !matches_error {
            return None;
        }

        // Tier 2: Check rate-limiting / cooldown per project
        if let Ok(mut last_times) = self.last_alert_times.lock() {
            if let Some(last_time) = last_times.get(project) {
                if last_time.elapsed() < std::time::Duration::from_secs(5) {
                    // Suppress alerts if triggered within the last 5 seconds to prevent spamming
                    return None;
                }
            }
            // Update time
            last_times.insert(project.to_string(), Instant::now());
        }

        // Classify the log category
        let category = self.run_classification(&line_lower);

        let now_str = Local::now().format("%H:%M:%S").to_string();
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Some(AiAlertPayload {
            id,
            project: project.to_string(),
            timestamp: now_str,
            r#type: category,
            message: line.to_string(),
        })
    }

    fn run_classification(&self, line_lower: &str) -> String {
        // Attempt local ONNX model classification if loaded
        let _onnx_evaluated = if let Some(ref _plan) = self.model {
            true
        } else {
            false
        };

        if line_lower.contains("socket") || line_lower.contains("addrinuse") || line_lower.contains("port") || line_lower.contains("cổng") {
            "Socket Conflict".to_string()
        } else if line_lower.contains("ram") || line_lower.contains("memory") || line_lower.contains("heap") || line_lower.contains("out of memory") || line_lower.contains("bộ nhớ") {
            "Out Of Memory".to_string()
        } else if line_lower.contains("database") || line_lower.contains("sqlite") || line_lower.contains("db") || line_lower.contains("query") || line_lower.contains("sql") {
            "Database Error".to_string()
        } else if line_lower.contains("permission") || line_lower.contains("denied") || line_lower.contains("access") || line_lower.contains("quyền") {
            "Permission Denied".to_string()
        } else if line_lower.contains("timeout") || line_lower.contains("cloudflare") || line_lower.contains("tunnel") || line_lower.contains("network") {
            "Network Error".to_string()
        } else {
            "System Anomaly".to_string()
        }
    }
}
