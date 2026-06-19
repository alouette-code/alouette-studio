pub mod analyzer;

pub trait Profiler {
    fn record_telemetry(&mut self, data: crate::memory_inspector::models::TelemetryData);
    fn analyze(&self) -> crate::memory_inspector::models::Diagnosis;
    fn get_baseline(&self) -> Option<crate::memory_inspector::models::ProfilingResult>;
}
