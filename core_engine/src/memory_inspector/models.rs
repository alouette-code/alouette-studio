use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InspectorState {
    Idle,
    Isolating,
    BaselineProfiling,
    StressTesting,
    SmartInspection,
    Finished,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryData {
    pub timestamp: u64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub gc_events_detected: u32,
    pub crash_imminent: bool,
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
