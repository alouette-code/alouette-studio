use crate::ai_manager::{EngineConfig, server};
use tauri::AppHandle;

pub async fn start_onnx(_app: AppHandle, config: EngineConfig) -> Result<(), String> {
    println!("[ONNX-ORT] Starting engine with config: {:?}", config);
    // In a real implementation:
    // 1. Configure ONNX Runtime via `ort` crate
    // 2. Load the model from `config.source_path` into memory
    
    // Start the native HTTP server to serve OpenAI compatible API
    server::start_http_server(config).await?;
    
    Ok(())
}
