use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InspectorState {
    Idle,
    PreFlightChecks,
    Isolating,
    BaselineProfiling,
    StressTesting,
    SmartInspection,
    GeneratingReport,
    Finished,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    DockerImage,
    CodeSnippet { language: String, code: String },
    ExecutableFile { host_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionConfig {
    #[serde(default)]
    pub target_type: Option<TargetType>,
    pub image: String,
    pub initial_ram_mb: f64,
    pub env_vars: std::collections::HashMap<String, String>,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
    pub network: Option<String>,
    pub cmd: Option<String>,
    pub stress_ramp_rate: f64,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub config: InspectionConfig,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub status: String,
    pub final_diagnosis: Option<Diagnosis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryData {
    pub timestamp: u64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub thread_count: u32,
    pub gc_events_detected: u32,
    pub crash_imminent: bool,
    pub status: String,
    pub activities: Vec<ProcessActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessActivity {
    pub timestamp: u64,
    pub event_type: String,
    pub pid: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingResult {
    pub min_ram_mb: f64,
    pub avg_ram_mb: f64,
    pub behavior_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Diagnosis {
    CacheEviction,
    StubbornLeak,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapPoint {
    pub timestamp: u64,
    pub intensity: f64, // 0.0 to 1.0 (1.0 = highly likely leak / denied alloc)
}
