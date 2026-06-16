use crate::ai_manager::{EngineConfig, server};
use tauri::AppHandle;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub async fn start_onnx(
    _app: AppHandle, 
    config: EngineConfig,
    active_engines: Arc<Mutex<HashMap<String, EngineConfig>>>
) -> Result<(), String> {
    println!("[ONNX-ORT] Starting engine with config: {:?}", config);
    
    // Hardware Limits Application
    println!("[ONNX-ORT] Initializing SessionBuilder with ExecutionProvider: {}, Intra-threads: {}", 
             config.hardware_target, config.cpu_threads);
    
    // Start the native HTTP server to serve OpenAI compatible API
    server::start_http_server(config.port, active_engines).await?;
    
    Ok(())
}
