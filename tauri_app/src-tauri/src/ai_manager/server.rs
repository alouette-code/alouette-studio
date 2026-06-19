use axum::{
    routing::{get, post},
    Router,
    response::Json,
    extract::State,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::ai_manager::EngineConfig;
use std::collections::HashMap;

pub struct ServerState {
    // Map of running engine configs so the gateway knows where to route
    pub active_configs: Arc<Mutex<HashMap<String, EngineConfig>>>,
}

pub async fn start_http_server(port: u16, active_configs: Arc<Mutex<HashMap<String, EngineConfig>>>) -> Result<(), String> {
    let state = Arc::new(ServerState { active_configs });

    let app = Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/chat/completions", get(chat_completions_get))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    // We try to bind. If it fails, we assume the gateway is already running!
    match TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("[API Gateway] Native HTTP Server started on {}", addr);
            tokio::spawn(async move {
                let _ = axum::serve(listener, app).await;
            });
            Ok(())
        }
        Err(_) => {
            println!("[API Gateway] Server already running on {}. Attached new model.", addr);
            Ok(())
        }
    }
}

async fn list_models(State(state): State<Arc<ServerState>>) -> Json<Value> {
    let configs = state.active_configs.lock().await;
    let mut data = Vec::new();
    
    for (id, config) in configs.iter() {
        let display_name = if config.model_name.trim().is_empty() {
            id.clone()
        } else {
            config.model_name.clone()
        };
        
        data.push(json!({
            "id": display_name,
            "object": "model",
            "owned_by": "alouette-studio",
            "hardware_target": config.hardware_target,
            "threads": config.cpu_threads
        }));
    }

    Json(json!({
        "object": "list",
        "data": data
    }))
}

async fn chat_completions_get() -> Json<Value> {
    Json(json!({
        "error": "Method Not Allowed. Please use POST for /v1/chat/completions with a JSON payload."
    }))
}

// Struct to parse the incoming request body
#[derive(serde::Deserialize)]
struct ChatRequest {
    model: Option<String>,
    #[allow(dead_code)]
    messages: Option<Vec<Value>>,
}

async fn chat_completions(
    State(state): State<Arc<ServerState>>,
    axum::Json(payload): axum::Json<ChatRequest>,
) -> Json<Value> {
    let configs = state.active_configs.lock().await;
    
    // Find which engine to route to
    let requested_model = payload.model.unwrap_or_else(|| "default".to_string());
    
    let mut target_engine = None;
    for (_id, config) in configs.iter() {
        if config.model_name == requested_model || requested_model == "default" {
            target_engine = Some(config.clone());
            break;
        }
    }

    if let Some(engine) = target_engine {
        println!("[API Gateway] Routing request to Engine: {} (Hardware: {}, Threads: {}, RAM Limit: {}GB)", 
                 engine.engine_id, engine.hardware_target, engine.cpu_threads, engine.ram_limit_gb);
        
        // TODO: Actually call Candle or ONNX inference here based on engine.engine_id
        Json(json!({
            "id": "chatcmpl-alouette-native",
            "object": "chat.completion",
            "created": 1677652288,
            "model": engine.model_name,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": format!("Xin chào! Mình là model '{}' đang chạy bằng công cụ {}. Hệ thống đã giới hạn RAM của mình ở {}GB và dùng {} Threads trên {}.", 
                                       engine.model_name, engine.engine_id, engine.ram_limit_gb, engine.cpu_threads, engine.hardware_target)
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 0,
                "completion_tokens": 50,
                "total_tokens": 50
            }
        }))
    } else {
        Json(json!({
            "error": format!("Model '{}' is not loaded or not found in active engines.", requested_model)
        }))
    }
}
