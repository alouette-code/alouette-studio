use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tauri::{Emitter, WebviewWindow};

static CANCEL_FLAG: OnceLock<AtomicBool> = OnceLock::new();

fn get_cancel_flag() -> &'static AtomicBool {
    CANCEL_FLAG.get_or_init(|| AtomicBool::new(false))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChatMessage {
    pub role: String,
    pub content: String,
}

async fn check_server_health() -> bool {
    let client = reqwest::Client::new();
    match client.get("http://127.0.0.1:8080/health").send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    }
}

fn resolve_llama_server_path() -> std::path::PathBuf {
    let paths = vec![
        "/home/nhatanh/projet/alouette_studio/core_engine/app_data/bin/llama-bin/llama-server"
            .into(),
        crate::state::project_root()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("core_engine/app_data/bin/llama-bin/llama-server"),
        crate::state::project_root().join("core_engine/app_data/bin/llama-bin/llama-server"),
    ];
    for p in paths {
        if p.exists() {
            return p;
        }
    }
    std::path::PathBuf::from("llama-server")
}

fn resolve_model_path() -> std::path::PathBuf {
    let p = std::path::PathBuf::from("/home/nhatanh/projet/alouette_studio/tauri_app/app_data/model_embedding/model-small-phi-3/phi-3-mini-4k-instruct-q2_k.gguf");
    if p.exists() {
        return p;
    }
    crate::state::project_root()
        .join("app_data/model_embedding/model-small-phi-3/phi-3-mini-4k-instruct-q2_k.gguf")
}

fn spawn_llama_server() -> Result<(), String> {
    let bin_path = resolve_llama_server_path();
    let model_path = resolve_model_path();

    let log_dir = crate::state::project_root().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = std::fs::File::create(log_dir.join("llama-server.log")).ok();

    let mut cmd = std::process::Command::new(bin_path);
    cmd.args(&[
        "-m",
        &model_path.to_string_lossy(),
        "-c",
        "4096",
        "--port",
        "8080",
    ]);

    if let Some(f) = log_file {
        cmd.stdout(std::process::Stdio::from(f.try_clone().unwrap()));
        cmd.stderr(std::process::Stdio::from(f));
    } else {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;
    Ok(())
}

async fn ensure_llama_server_running() -> Result<(), String> {
    if check_server_health().await {
        return Ok(());
    }

    spawn_llama_server()?;

    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if check_server_health().await {
            return Ok(());
        }
    }

    Err("llama-server did not start and become healthy in time (port 8080)".to_string())
}

#[tauri::command]
pub fn local_chat_stop() {
    get_cancel_flag().store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn local_chat_send(
    message: String,
    history: Vec<LocalChatMessage>,
    window: WebviewWindow,
) -> Result<String, String> {
    get_cancel_flag().store(false, Ordering::SeqCst);

    let _ = window.emit("local-chat-status", "starting");
    ensure_llama_server_running().await?;
    let _ = window.emit("local-chat-status", "running");

    let mut messages = Vec::new();
    for msg in history {
        messages.push(json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }
    messages.push(json!({
        "role": "user",
        "content": message,
    }));

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8080/v1/chat/completions")
        .json(&json!({
            "model": "phi-3-mini-4k-instruct",
            "messages": messages,
            "stream": true,
            "temperature": 0.2,
            "top_p": 0.95
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to local llama-server: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_txt = response.text().await.unwrap_or_default();
        return Err(format!("Local server error ({}): {}", status, err_txt));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut full_response = String::new();

    while let Some(chunk_res) = stream.next().await {
        if get_cancel_flag().load(Ordering::SeqCst) {
            break;
        }

        let chunk = chunk_res.map_err(|e| format!("Stream read error: {}", e))?;
        buffer.extend_from_slice(&chunk);

        while let Some(newline_idx) = buffer.iter().position(|&b| b == b'\n') {
            let line_bytes = buffer.drain(..=newline_idx).collect::<Vec<u8>>();
            let line = String::from_utf8_lossy(&line_bytes);
            let line_trimmed = line.trim();

            if line_trimmed.starts_with("data: ") {
                let data = &line_trimmed["data: ".len()..];
                if data == "[DONE]" {
                    break;
                }
                if let Ok(val) = serde_json::from_str::<Value>(data) {
                    if let Some(choices) = val["choices"].as_array() {
                        if !choices.is_empty() {
                            if let Some(content) =
                                choices[0]["delta"].get("content").and_then(|c| c.as_str())
                            {
                                full_response.push_str(content);
                                let _ = window.emit("local-chat-chunk", content);
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = window.emit("local-chat-complete", &full_response);
    Ok(full_response)
}
