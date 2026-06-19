pub mod docker;

pub trait ContainerDriver {
    fn create_sandbox(&self, image: &str, name: &str, initial_memory_mb: f64) -> Result<(), String>;
    fn update_memory_limit(&self, name: &str, memory_mb: f64) -> Result<(), String>;
    fn get_stats(&self, name: &str) -> Result<super::models::TelemetryData, String>;
    fn destroy_sandbox(&self, name: &str) -> Result<(), String>;
}
