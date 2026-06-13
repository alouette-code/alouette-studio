use crate::model_manager::SharedModelManager;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tauri::{Emitter, State, WebviewWindow};

static CANCEL_FLAG: OnceLock<AtomicBool> = OnceLock::new();

fn get_cancel_flag() -> &'static AtomicBool {
    CANCEL_FLAG.get_or_init(|| AtomicBool::new(false))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChatMessage {
    pub role: String,
    pub content: String,
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
    model_manager: State<'_, SharedModelManager>,
) -> Result<String, String> {
    get_cancel_flag().store(false, Ordering::SeqCst);

    // ── Step 1: Emit "starting" status ──
    let _ = window.emit("local-chat-status", "starting");

    // ── Step 2: Ensure model server is running (managed lifecycle) ──
    {
        let mut mgr = model_manager.lock().await;
        mgr.ensure_running().await?;
    }
    let _ = window.emit("local-chat-status", "running");

    // ── Step 3: Build request payload ──
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

    // ── Step 4: Send streaming request ──
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
        .map_err(|e| format!("Failed to connect to local model server: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_txt = response.text().await.unwrap_or_default();
        return Err(format!(
            "Local model server error ({}): {}",
            status, err_txt
        ));
    }

    // ── Step 5: Stream response tokens ──
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
