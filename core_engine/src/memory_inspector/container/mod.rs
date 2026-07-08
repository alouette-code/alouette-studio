pub mod docker;

use super::models::{InspectionConfig, TelemetryData};

#[allow(async_fn_in_trait)]
pub trait ExecutionProvider: Send + Sync {
    async fn check_health(&self) -> Result<(), String>;
    async fn create_sandbox(&self, config: &InspectionConfig, name: &str) -> Result<(), String>;
    async fn update_memory_limit(&self, name: &str, memory_mb: f64) -> Result<(), String>;
    async fn inject_chaos(&self, _name: &str) -> Result<(), String> { Ok(()) }
    async fn get_stats(&self, name: &str) -> Result<TelemetryData, String>;
    async fn destroy_sandbox(&self, name: &str) -> Result<(), String>;
}
