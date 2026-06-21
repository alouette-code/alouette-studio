pub mod docker;

use super::models::{InspectionConfig, TelemetryData};

pub trait ContainerDriver {
    fn check_daemon_health(&self) -> Result<(), String>;
    fn create_sandbox(&self, config: &InspectionConfig, name: &str) -> Result<(), String>;
    fn update_memory_limit(&self, name: &str, memory_mb: f64) -> Result<(), String>;
    fn get_stats(&self, name: &str) -> Result<TelemetryData, String>;
    fn destroy_sandbox(&self, name: &str) -> Result<(), String>;
}
