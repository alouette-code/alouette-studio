use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;

use crate::memory_inspector::models::{InspectorState, TelemetryData, InspectionConfig, TaskRecord};
use crate::memory_inspector::container::{ExecutionProvider, docker::DockerDriver};
use crate::memory_inspector::profiling::{Profiler, analyzer::HeuristicEngine, ebpf_tracer::EbpfTracer};
use crate::memory_inspector::stress::{StressController, fuzzer::ChaosFuzzer};
use crate::memory_inspector::pipeline::TelemetryPipeline;
use crate::memory_inspector::fault_tolerance::{CircuitBreaker, CheckpointManager, TaskCheckpoint};

pub enum InspectorEvent {
    StartTask { config: InspectionConfig },
    Tick,
    StopTask,
}

pub struct MemoryInspectorManager {
    pub tasks: HashMap<String, TaskRecord>,
    pub current_task_id: Option<String>,
    event_sender: mpsc::Sender<InspectorEvent>,
    pub state: Arc<Mutex<InspectorState>>,
    pub pipeline: Arc<TelemetryPipeline>,
}

impl MemoryInspectorManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let state = Arc::new(Mutex::new(InspectorState::Idle));
        let pipeline = Arc::new(TelemetryPipeline::new());
        
        let manager = Self {
            tasks: HashMap::new(),
            current_task_id: None,
            event_sender: tx.clone(),
            state: state.clone(),
            pipeline: pipeline.clone(),
        };

        // Start Event Loop
        manager.start_reactor_loop(rx, state, pipeline);

        manager
    }

    pub async fn start_inspection(&mut self, config: InspectionConfig) -> Result<String, String> {
        let task_id = format!("task-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
        
        let record = TaskRecord {
            task_id: task_id.clone(),
            config: config.clone(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            end_time: None,
            status: "Starting".to_string(),
            final_diagnosis: None,
            culprit_summary: None,
        };

        self.tasks.insert(task_id.clone(), record);
        self.current_task_id = Some(task_id.clone());

        self.event_sender.send(InspectorEvent::StartTask { config }).await.map_err(|e| e.to_string())?;

        Ok(task_id)
    }

    pub async fn stop(&mut self) {
        let _ = self.event_sender.send(InspectorEvent::StopTask).await;
        self.current_task_id = None;
    }

    fn start_reactor_loop(&self, mut rx: mpsc::Receiver<InspectorEvent>, state_mutex: Arc<Mutex<InspectorState>>, pipeline: Arc<TelemetryPipeline>) {
        let execution_provider = Arc::new(DockerDriver);
        let profiler = Arc::new(Mutex::new(HeuristicEngine::new()));
        let circuit_breaker = Arc::new(Mutex::new(CircuitBreaker::new(3, 300)));
        let checkpoint_mgr = Arc::new(CheckpointManager::new(PathBuf::from("/tmp/memory_inspector_checkpoints")));
        let mut state = InspectorState::Idle;
        let mut stress_controller: Option<Box<dyn StressController + Send + Sync>> = None;
        let container_name = "proto-memory-inspector".to_string();
        let mut current_task = "".to_string();
        let mut ebpf_tracer = EbpfTracer::new();
        let _pipeline_tx = self.event_sender.clone(); // In real implementation, pipeline has its own sender

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    InspectorEvent::StartTask { config } => {
                        if let Err(e) = execution_provider.check_health().await {
                            state = InspectorState::Error(e);
                            continue;
                        }

                        let _ = execution_provider.destroy_sandbox(&container_name).await;
                        if let Err(e) = execution_provider.create_sandbox(&config, &container_name).await {
                            state = InspectorState::Error(e);
                            continue;
                        }

                        state = InspectorState::BaselineProfiling;
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                        current_task = format!("task-{}", now);
                    }
                    InspectorEvent::Tick => {
                        if matches!(state, InspectorState::Idle | InspectorState::Finished | InspectorState::Error(_)) {
                            continue;
                        }

                        // Save checkpoint
                        let ckpt = TaskCheckpoint {
                            task_id: current_task.clone(),
                            state: state.clone(),
                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            metadata: HashMap::new(),
                        };
                        let _ = checkpoint_mgr.save_checkpoint(&ckpt).await;

                        match execution_provider.get_stats(&container_name).await {
                            Ok(stats) => {
                                circuit_breaker.lock().await.record_success();
                                let mut prof = profiler.lock().await;
                                prof.record_telemetry(stats.clone());
                                let _ = pipeline.ingest(stats).await;

                                match state {
                                    InspectorState::BaselineProfiling => {
                                        if let Some(baseline) = prof.get_baseline() {
                                            // Start eBPF tracer
                                            ebpf_tracer.set_target_pid(1); // Mock PID for demo
                                            // _ = ebpf_tracer.start(tx).await;
                                            
                                            // Switch to Chaos Fuzzer
                                            stress_controller = Some(Box::new(ChaosFuzzer::new(baseline.avg_ram_mb * 1.5, 20, 0.2)));
                                            state = InspectorState::StressTesting;
                                        }
                                    }
                                    InspectorState::StressTesting => {
                                        if let Some(controller) = &mut stress_controller {
                                            let stats_clone = pipeline.get_recent(1).await.get(0).cloned().unwrap_or_default();
                                            let next_limit = controller.calculate_next_limit(stats_clone.memory_usage_mb);
                                            let _ = execution_provider.update_memory_limit(&container_name, next_limit).await;
                                            
                                            // Inject Chaos based on Fuzzer logic
                                            // In a real implementation we would cast to downcast to ChaosFuzzer
                                            // For this kernel architecture demo, we'll just inject randomly based on time
                                            if SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() % 10 == 0 {
                                                let _ = execution_provider.inject_chaos(&container_name).await;
                                            }
                                            
                                            if controller.is_finished() {
                                                ebpf_tracer.stop();
                                                state = InspectorState::SmartInspection;
                                            }
                                        }
                                    }
                                    InspectorState::SmartInspection => {
                                        state = InspectorState::GeneratingReport;
                                    }
                                    InspectorState::GeneratingReport => {
                                        let _ = execution_provider.destroy_sandbox(&container_name).await;
                                        state = InspectorState::Finished;
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                let mut cb = circuit_breaker.lock().await;
                                cb.record_failure();
                                if cb.is_tripped() {
                                    let _ = execution_provider.destroy_sandbox(&container_name).await;
                                    state = InspectorState::Error(format!("Circuit breaker tripped due to repeated failures: {}", e));
                                }
                            }
                        }
                    }
                    InspectorEvent::StopTask => {
                        let _ = execution_provider.destroy_sandbox(&container_name).await;
                        state = InspectorState::Idle;
                    }
                }
                *state_mutex.lock().await = state.clone();
            }
        });
    }

    pub async fn tick(&mut self) -> Result<TelemetryData, String> {
        self.event_sender.send(InspectorEvent::Tick).await.map_err(|e| e.to_string())?;
        
        let state = self.state.lock().await.clone();
        if matches!(state, InspectorState::GeneratingReport | InspectorState::Finished | InspectorState::Error(_)) {
            if let Some(task_id) = &self.current_task_id {
                if let Some(task) = self.tasks.get_mut(task_id) {
                    if task.final_diagnosis.is_none() {
                        let mut engine = HeuristicEngine::new();
                        let history = self.pipeline.get_recent(5000).await;
                        for d in &history {
                            engine.record_telemetry(d.clone());
                        }
                        task.final_diagnosis = Some(engine.analyze());
                        
                        let mut freq_map = HashMap::new();
                        if let Ok(re) = regex::Regex::new(r"0x[0-9a-fA-F]+|\b\d+\b") {
                            for data in &history {
                                for act in &data.activities {
                                    let sanitized = re.replace_all(&act.details, "X").to_string();
                                    if sanitized.len() > 5 && !sanitized.contains("busybox") && !sanitized.contains("Installing") {
                                        *freq_map.entry(sanitized).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                        
                        let mut sorted_acts: Vec<_> = freq_map.into_iter().collect();
                        sorted_acts.sort_by(|a, b| b.1.cmp(&a.1));
                        if !sorted_acts.is_empty() {
                            use crate::memory_inspector::models::Culprit;
                            let top_culprits: Vec<Culprit> = sorted_acts.into_iter().take(10).map(|(d, c)| Culprit {
                                name: d,
                                count: c,
                            }).collect();
                            task.culprit_summary = Some(top_culprits);
                        }
                    }
                }
            }
        }

        let status_str = match &state {
            InspectorState::Idle => "Idle",
            InspectorState::PreFlightChecks => "PreFlightChecks",
            InspectorState::Isolating => "Isolating",
            InspectorState::BaselineProfiling => "BaselineProfiling",
            InspectorState::StressTesting => "StressTesting",
            InspectorState::SmartInspection => "SmartInspection",
            InspectorState::GeneratingReport => "GeneratingReport",
            InspectorState::Finished => "Finished",
            InspectorState::Error(_e) => "Error",
        }.to_string();

        let mut telemetry = self.pipeline.get_recent(1).await.get(0).cloned().unwrap_or_else(|| TelemetryData {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            memory_usage_mb: 0.0,
            memory_limit_mb: 0.0,
            thread_count: 0,
            gc_events_detected: 0,
            crash_imminent: false,
            status: status_str.clone(),
            activities: vec![],
        });
        telemetry.status = status_str.clone();

        if let Some(task_id) = &self.current_task_id {
            if let Some(task) = self.tasks.get_mut(task_id) {
                task.status = status_str;
            }
        }

        Ok(telemetry)
    }
}
