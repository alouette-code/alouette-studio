use std::sync::Arc;
use tokio::sync::Mutex;
use crate::memory_inspector::models::{InspectorState, TelemetryData};
use crate::memory_inspector::container::{ContainerDriver, docker::DockerDriver};
use crate::memory_inspector::profiling::{Profiler, analyzer::SmartAnalyzer};
use crate::memory_inspector::stress::{StressController, ramp_down::ExponentialRampDown};

pub struct MemoryInspectorManager {
    pub state: InspectorState,
    container_driver: Box<dyn ContainerDriver + Send + Sync>,
    profiler: Arc<Mutex<dyn Profiler + Send + Sync>>,
    stress_controller: Option<Box<dyn StressController + Send + Sync>>,
    container_name: String,
}

impl MemoryInspectorManager {
    pub fn new() -> Self {
        Self {
            state: InspectorState::Idle,
            container_driver: Box::new(DockerDriver),
            profiler: Arc::new(Mutex::new(SmartAnalyzer::new())),
            stress_controller: None,
            container_name: "proto-memory-inspector".to_string(),
        }
    }

    pub async fn start_isolation(&mut self, image: &str, initial_ram: f64) -> Result<(), String> {
        self.state = InspectorState::Isolating;
        // Clean up any previous container
        let _ = self.container_driver.destroy_sandbox(&self.container_name);
        
        self.container_driver.create_sandbox(image, &self.container_name, initial_ram)?;
        self.state = InspectorState::BaselineProfiling;
        Ok(())
    }

    pub async fn tick(&mut self) -> Result<TelemetryData, String> {
        let stats = self.container_driver.get_stats(&self.container_name)?;
        
        let mut profiler = self.profiler.lock().await;
        profiler.record_telemetry(stats.clone());

        match self.state {
            InspectorState::BaselineProfiling => {
                // If baseline gathered enough data, switch to stress testing
                if let Some(baseline) = profiler.get_baseline() {
                    self.stress_controller = Some(Box::new(ExponentialRampDown::new(baseline.avg_ram_mb * 1.5)));
                    self.state = InspectorState::StressTesting;
                }
            }
            InspectorState::StressTesting => {
                if let Some(controller) = &mut self.stress_controller {
                    let next_limit = controller.calculate_next_limit(stats.memory_usage_mb);
                    let _ = self.container_driver.update_memory_limit(&self.container_name, next_limit);
                    
                    if controller.is_finished() {
                        self.state = InspectorState::SmartInspection;
                    }
                }
            }
            InspectorState::SmartInspection => {
                // Analyze results
                let _diagnosis = profiler.analyze();
                self.state = InspectorState::Finished;
            }
            _ => {}
        }

        Ok(stats)
    }

    pub async fn stop(&mut self) {
        let _ = self.container_driver.destroy_sandbox(&self.container_name);
        self.state = InspectorState::Idle;
    }
}
