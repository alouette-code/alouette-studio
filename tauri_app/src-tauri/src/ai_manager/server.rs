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

pub struct ServerState {
    pub config: EngineConfig,
}

pub async fn start_http_server(config: EngineConfig) -> Result<(), String> {
    let port = config.port;
    let state = Arc::new(Mutex::new(ServerState { config }));

    let app = Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/chat/completions", get(chat_completions_get)) // Friendly handler for GET
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await.map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;
    
    println!("[AiServer] Native HTTP Server started and listening on {}", addr);
    
    // Spawn server in the background
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
        println!("[AiServer] Server shut down.");
    });

    Ok(())
}

async fn list_models() -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [{
            "id": "alouette-native-model",
            "object": "model",
            "owned_by": "alouette-studio"
        }]
    }))
}

async fn chat_completions_get() -> Json<Value> {
    Json(json!({
        "error": "Method Not Allowed. Please use POST for /v1/chat/completions with a JSON payload."
    }))
}

async fn chat_completions(State(_state): State<Arc<Mutex<ServerState>>>) -> Json<Value> {
    // TODO: Connect this to Candle / ONNX inference queues
    Json(json!({
        "id": "chatcmpl-alouette-native",
        "object": "chat.completion",
        "created": 1677652288,
        "model": "alouette-native-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Xin chào! Mình là Alouette Native AI Server chạy ngầm bằng Rust! Mình đã nhận được kết nối của bạn."
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 20,
            "total_tokens": 20
        }
    }))
}
