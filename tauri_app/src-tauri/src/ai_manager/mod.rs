use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

pub mod llama_cpp;
pub mod ollama;
pub mod onnx_ort;
pub mod python_env;
pub mod python_ffi;
pub mod server;

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
    // Currently active engine if any. We only allow one active local engine at a time for simplicity.
    pub active_engine: Arc<Mutex<Option<String>>>,
}

impl AiEngineManager {
    pub fn new() -> Self {
        Self {
            active_engine: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub async fn start_ai_engine(
    app: AppHandle,
    state: State<'_, AiEngineManager>,
    config: EngineConfig,
) -> Result<String, String> {
    let mut active = state.active_engine.lock().await;
    if active.is_some() {
        return Err("An engine is already running. Please stop it first.".into());
    }

    // Determine which engine to start based on ID
    match config.engine_id.as_str() {
        "ollama" => {
            ollama::start_ollama(app.clone(), config.clone()).await?;
        }
        "llamacpp" | "koboldcpp" => {
            llama_cpp::start_llama_cpp(app.clone(), config.clone()).await?;
        }
        "onnx" => {
            onnx_ort::start_onnx(app.clone(), config.clone()).await?;
        }
        "vllm" | "tensorrt" | "exllamav2" => {
            python_ffi::start_python_engine(app.clone(), config.clone()).await?;
        }
        _ => return Err(format!("Unknown engine ID: {}", config.engine_id)),
    }

    *active = Some(config.engine_id.clone());
    Ok(format!("Started engine: {}", config.engine_id))
}

#[tauri::command]
pub async fn stop_ai_engine(state: State<'_, AiEngineManager>) -> Result<String, String> {
    let mut active = state.active_engine.lock().await;
    if let Some(engine) = active.take() {
        // Call the specific stop functions for each engine type.
        match engine.as_str() {
            "ollama" => {
                ollama::stop_ollama();
            }
            _ => {
                println!("[AiManager] Stop function not implemented for engine: {}", engine);
            }
        }
        Ok(format!("Stopped engine: {}", engine))
    } else {
        Ok("No engine is currently running.".into())
    }
}

#[tauri::command]
pub async fn get_ai_engine_status(state: State<'_, AiEngineManager>) -> Result<Option<String>, String> {
    let active = state.active_engine.lock().await;
    Ok(active.clone())
}
