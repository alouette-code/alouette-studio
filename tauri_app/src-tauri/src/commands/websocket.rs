use tauri::{AppHandle, State, Emitter};
use tokio::sync::{mpsc, Mutex};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct WsEventPayload {
    pub r#type: String, // "received", "system", "error"
    pub data: String,
    pub timestamp: u64,
}

pub struct WsState {
    pub sender: Mutex<Option<mpsc::Sender<Message>>>,
}

#[tauri::command]
pub async fn ws_connect(
    url: String,
    app: AppHandle,
    state: State<'_, WsState>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(&url).await.map_err(|e| e.to_string())?;
    
    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // Save sender to state
    *state.sender.lock().await = Some(tx);

    let app_clone = app.clone();
    
    // Spawn writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Spawn reader task
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let _ = app_clone.emit("ws-message", WsEventPayload {
                        r#type: "received".into(),
                        data: text.to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    });
                }
                Ok(Message::Binary(bin)) => {
                    let _ = app_clone.emit("ws-message", WsEventPayload {
                        r#type: "received".into(),
                        data: format!("<Binary data: {} bytes>", bin.len()),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    });
                }
                Err(e) => {
                    let _ = app_clone.emit("ws-message", WsEventPayload {
                        r#type: "error".into(),
                        data: format!("Error: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    });
                    break;
                }
                _ => {}
            }
        }
        let _ = app_clone.emit("ws-message", WsEventPayload {
            r#type: "error".into(),
            data: "WebSocket disconnected".into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        });
    });

    Ok(())
}

#[tauri::command]
pub async fn ws_send(
    message: String,
    state: State<'_, WsState>,
) -> Result<(), String> {
    let sender = state.sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(Message::Text(message.into())).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Not connected".into())
    }
}

#[tauri::command]
pub async fn ws_disconnect(
    state: State<'_, WsState>,
) -> Result<(), String> {
    let mut sender = state.sender.lock().await;
    if let Some(tx) = sender.take() {
        let _ = tx.send(Message::Close(None)).await;
    }
    Ok(())
}
