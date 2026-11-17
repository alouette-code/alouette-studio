
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GrpcInput {
    pub url: String,
    pub service: String,
    pub method: String,
    pub payload: String,
    pub proto_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GrpcResult {
    pub status: String,
    pub message: String,
    pub elapsed_ms: u64,
}

#[tauri::command]
pub async fn grpc_call(input: GrpcInput) -> Result<GrpcResult, String> {
    let _ = (&input.payload, &input.proto_path);
    Err(format!(
        "Dynamic gRPC ({}/{} on {}) is not yet implemented with a real tonic/protox transport.",
        input.service, input.method, input.url
    ))
}
