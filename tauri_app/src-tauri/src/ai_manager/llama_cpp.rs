use crate::ai_manager::{EngineConfig, server};
use tauri::AppHandle;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub async fn start_llama_cpp(
    _app: AppHandle, 
    config: EngineConfig,
    active_engines: Arc<Mutex<HashMap<String, EngineConfig>>>
) -> Result<(), String> {
    println!("[Candle-GGUF] Starting engine with config: {:?}", config);
    
    // Hardware Limits Application
    println!("[Candle-GGUF] Applying Hardware Target: {}, Max CPU Threads: {}, RAM Limit: {}GB", 
             config.hardware_target, config.cpu_threads, config.ram_limit_gb);
    
    // Start the native HTTP server to serve OpenAI compatible API
    server::start_http_server(config.port, active_engines).await?;
    
    Ok(())
}
