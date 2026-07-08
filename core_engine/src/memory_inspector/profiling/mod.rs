pub mod analyzer;
pub mod ebpf_tracer;

pub trait Profiler: Send + Sync {
    fn record_telemetry(&mut self, data: crate::memory_inspector::models::TelemetryData);
    fn analyze(&self) -> crate::memory_inspector::models::Diagnosis;
    fn get_baseline(&self) -> Option<crate::memory_inspector::models::ProfilingResult>;
    fn recommended_sample_interval_ms(&self) -> u64;
}
