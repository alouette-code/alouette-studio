use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::memory_inspector::models::{InspectorState, TelemetryData, InspectionConfig, TaskRecord};
use crate::memory_inspector::container::{ContainerDriver, docker::DockerDriver};
use crate::memory_inspector::profiling::{Profiler, analyzer::SmartAnalyzer};
use crate::memory_inspector::stress::{StressController, ramp_down::ExponentialRampDown};

pub struct MemoryInspectorManager {
    pub state: InspectorState,
    container_driver: Box<dyn ContainerDriver + Send + Sync>,
    profiler: Arc<Mutex<dyn Profiler + Send + Sync>>,
    stress_controller: Option<Box<dyn StressController + Send + Sync>>,
    container_name: String,
    pub tasks: HashMap<String, TaskRecord>,
    pub current_task_id: Option<String>,
}

impl MemoryInspectorManager {
    pub fn new() -> Self {
        Self {
            state: InspectorState::Idle,
            container_driver: Box::new(DockerDriver),
            profiler: Arc::new(Mutex::new(SmartAnalyzer::new())),
            stress_controller: None,
            container_name: "proto-memory-inspector".to_string(),
            tasks: HashMap::new(),
            current_task_id: None,
        }
    }

    pub async fn start_inspection(&mut self, config: InspectionConfig) -> Result<String, String> {
        self.state = InspectorState::PreFlightChecks;
        
        // Check tools
        self.container_driver.check_daemon_health()?;

        self.state = InspectorState::Isolating;
        // Clean up any previous container
        let _ = self.container_driver.destroy_sandbox(&self.container_name);
        
        self.container_driver.create_sandbox(&config, &self.container_name)?;
        self.state = InspectorState::BaselineProfiling;

        let task_id = format!("task-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
        
        let record = TaskRecord {
            task_id: task_id.clone(),
            config,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            end_time: None,
            status: "Running".to_string(),
            final_diagnosis: None,
        };

        self.tasks.insert(task_id.clone(), record);
        self.current_task_id = Some(task_id.clone());

        Ok(task_id)
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
                self.state = InspectorState::GeneratingReport;
            }
            InspectorState::GeneratingReport => {
                // Analyze results
                let diagnosis = profiler.analyze();
                
                if let Some(task_id) = &self.current_task_id {
                    if let Some(record) = self.tasks.get_mut(task_id) {
                        record.status = "Finished".to_string();
                        record.end_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                        // Just map diagnosis directly if possible, else default to something.
                        // Assuming profiler.analyze() returns `Diagnosis`
                        // Wait, profiler returns `crate::memory_inspector::models::Diagnosis`
                        record.final_diagnosis = Some(diagnosis);
                    }
                }

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
