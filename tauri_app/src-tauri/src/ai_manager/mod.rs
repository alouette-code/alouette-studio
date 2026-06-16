use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

pub mod llama_cpp;
pub mod ollama;
pub mod onnx_ort;
pub mod python_env;
pub mod python_ffi;
pub mod server;
pub mod config_storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub engine_id: String,
    pub model_name: String,
    pub source_path: String,
    pub hardware_target: String,
    pub ram_limit_gb: u32,
    pub cpu_threads: u32,
    pub api_host: String,
    pub port: u16,
    pub api_route: String,
}

#[derive(Default)]
pub struct AiEngineManager {
    // Map of running engines by engine_id
    pub active_engines: Arc<Mutex<HashMap<String, EngineConfig>>>,
}

impl AiEngineManager {
    pub fn new() -> Self {
        Self {
            active_engines: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tauri::command]
pub async fn start_ai_engine(
    app: AppHandle,
    state: State<'_, AiEngineManager>,
    config: EngineConfig,
) -> Result<String, String> {
    let mut active = state.active_engines.lock().await;
    
    if active.contains_key(&config.engine_id) {
        return Err(format!("Engine '{}' is already running.", config.engine_id));
    }

    // Determine which engine to start based on ID
    match config.engine_id.as_str() {
        "ollama" => {
            ollama::start_ollama(app.clone(), config.clone()).await?;
        }
        "llamacpp" | "koboldcpp" => {
            llama_cpp::start_llama_cpp(app.clone(), config.clone(), state.active_engines.clone()).await?;
        }
        "onnx" => {
            onnx_ort::start_onnx(app.clone(), config.clone(), state.active_engines.clone()).await?;
        }
        "vllm" | "tensorrt" | "exllamav2" => {
            python_ffi::start_python_engine(app.clone(), config.clone()).await?;
        }
        _ => return Err(format!("Unknown engine ID: {}", config.engine_id)),
    }

    active.insert(config.engine_id.clone(), config.clone());
    Ok(format!("Started engine: {}", config.engine_id))
}

#[tauri::command]
pub async fn stop_ai_engine(state: State<'_, AiEngineManager>, engine_id: Option<String>) -> Result<String, String> {
    let mut active = state.active_engines.lock().await;
    let mut stopped = Vec::new();
    
    if let Some(target_id) = engine_id {
        // Stop specific engine
        if let Some(_) = active.remove(&target_id) {
            match target_id.as_str() {
                "ollama" => ollama::stop_ollama(),
                _ => println!("[AiManager] Stop function not implemented for engine: {}", target_id),
            }
            stopped.push(target_id);
        }
    } else {
        // Stop all engines
        for (id, _) in active.drain() {
            match id.as_str() {
                "ollama" => ollama::stop_ollama(),
                _ => println!("[AiManager] Stop function not implemented for engine: {}", id),
            }
            stopped.push(id);
        }
    }
    
    if stopped.is_empty() {
        Ok("No engines were stopped (not found or already stopped).".into())
    } else {
        Ok(format!("Stopped engines: {}", stopped.join(", ")))
    }
}

#[tauri::command]
pub async fn get_ai_engine_status(state: State<'_, AiEngineManager>) -> Result<Vec<EngineConfig>, String> {
    let active = state.active_engines.lock().await;
    let configs: Vec<EngineConfig> = active.values().cloned().collect();
    Ok(configs)
}

#[tauri::command]
pub async fn save_ai_settings(config: EngineConfig) -> Result<(), String> {
    config_storage::save_settings(config)
}

#[tauri::command]
pub async fn load_ai_settings() -> Result<Vec<EngineConfig>, String> {
    config_storage::load_all_settings()
}

#[tauri::command]
pub async fn delete_ai_setting(engine_id: String, model_name: Option<String>) -> Result<(), String> {
    config_storage::delete_setting(engine_id, model_name)
}
