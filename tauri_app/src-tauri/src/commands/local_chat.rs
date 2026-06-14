use crate::inference::{build_chat_prompt, ChatMessage};
use crate::model_manager::SharedModelManager;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, WebviewWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub async fn local_chat_send(
    message: String,
    history: Vec<LocalChatMessage>,
    window: WebviewWindow,
    model_manager: State<'_, SharedModelManager>,
) -> Result<String, String> {
    // ── Step 1: Emit "starting" status ──
    let _ = window.emit("local-chat-status", "starting");

    // ── Step 2: Ensure inference engine is loaded ──
    {
        let mut mgr = model_manager.inner.lock().await;
        mgr.ensure_running().await?;
    }
    let _ = window.emit("local-chat-status", "running");

    // ── Step 3: Build prompt ──
    let mut messages: Vec<ChatMessage> = history
        .into_iter()
        .map(|m| ChatMessage {
            role: m.role,
            content: m.content,
        })
        .collect();
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: message,
    });
    let prompt = build_chat_prompt(&messages);

    // ── Step 4: Clone the Arc before spawn_blocking ──
    let mm_arc: SharedModelManager = (*model_manager).clone();
    let cancel_flag = mm_arc.cancel_flag.clone();

    // ── Step 5: Run CPU-heavy inference in blocking thread ──
    let (result_tx, result_rx) = std::sync::mpsc::channel();

    tokio::task::spawn_blocking(move || {
        let mut mgr = mm_arc.inner.blocking_lock();
        let engine = match mgr.engine() {
            Some(e) => e,
            None => {
                let _ = result_tx.send(Err("Model not loaded".to_string()));
                return;
            }
        };

        let result = engine.generate_stream(
            &prompt,
            0.2,  // temperature
            0.95, // top_p
            512,  // max_tokens
            2048, // max_seq_len
            &mut |token: &str| {
                let _ = result_tx.send(Ok((token.to_string(), false)));
                Ok(())
            },
        );

        match result {
            Ok(full) => {
                let _ = result_tx.send(Ok((full, true)));
            }
            Err(e) => {
                let _ = result_tx.send(Err(e));
            }
        }
    });

    // ── Step 6: Stream tokens from channel to frontend ──
    let mut full_response = String::new();

    loop {
        match result_rx.recv() {
            Ok(Ok((token, is_done))) => {
                if is_done {
                    full_response = token;
                } else {
                    full_response.push_str(&token);
                    let _ = window.emit("local-chat-chunk", &token);
                }
            }
            Ok(Err(e)) => {
                let _ = window.emit("local-chat-complete", "");
                return Err(e);
            }
            Err(_) => break,
        }

        if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
    }

    let _ = window.emit("local-chat-complete", &full_response);
    Ok(full_response)
}

#[tauri::command]
pub fn local_chat_stop(model_manager: State<'_, SharedModelManager>) {
    model_manager.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
}
