pub mod docker;

use super::models::{InspectionConfig, TelemetryData};

pub trait ContainerDriver {
    async fn check_daemon_health(&self) -> Result<(), String>;
    async fn create_sandbox(&self, config: &InspectionConfig, name: &str) -> Result<(), String>;
    async fn update_memory_limit(&self, name: &str, memory_mb: f64) -> Result<(), String>;
    async fn get_stats(&self, name: &str) -> Result<TelemetryData, String>;
    async fn destroy_sandbox(&self, name: &str) -> Result<(), String>;
}
