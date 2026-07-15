use tauri::{AppHandle, State, Emitter};
use tokio::sync::{Mutex, mpsc};
use reqwest::Client;
use reqwest_eventsource::{EventSource, Event};
use futures_util::StreamExt;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct SseEventPayload {
    pub r#type: String, // "event", "error", "connected", "disconnected"
    pub data: String,
    pub event_id: Option<String>,
    pub event_name: Option<String>,
    pub timestamp: u64,
}

pub struct SseState {
    pub cancel_tx: Mutex<Option<mpsc::Sender<()>>>,
}

#[tauri::command]
pub async fn sse_connect(
    url: String,
    headers: std::collections::HashMap<String, String>,
    app: AppHandle,
    state: State<'_, SseState>,
) -> Result<(), String> {
    // Cancel any existing connection
    {
        let mut tx_lock = state.cancel_tx.lock().await;
        if let Some(tx) = tx_lock.take() {
            let _ = tx.send(()).await;
        }
    }

    let client_builder = Client::builder();
    let mut header_map = reqwest::header::HeaderMap::new();
    
    for (k, v) in headers {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&v) {
                header_map.insert(name, val);
            }
        }
    }

    let client = client_builder.default_headers(header_map).build().map_err(|e| e.to_string())?;
    let mut es = EventSource::new(client.get(&url)).map_err(|e| e.to_string())?;

    let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
    *state.cancel_tx.lock().await = Some(cancel_tx);

    let app_clone = app.clone();
    
    let _ = app_clone.emit("sse-message", SseEventPayload {
        r#type: "connected".into(),
        data: format!("Connecting to SSE: {}", url),
        event_id: None,
        event_name: None,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
    });

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_rx.recv() => {
                    let _ = app_clone.emit("sse-message", SseEventPayload {
                        r#type: "disconnected".into(),
                        data: "SSE connection closed by user".into(),
                        event_id: None,
                        event_name: None,
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                    });
                    break;
                }
                event = es.next() => {
                    match event {
                        Some(Ok(Event::Open)) => {
                            let _ = app_clone.emit("sse-message", SseEventPayload {
                                r#type: "connected".into(),
                                data: "Connection opened".into(),
                                event_id: None,
                                event_name: None,
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                            });
                        }
                        Some(Ok(Event::Message(message))) => {
                            let _ = app_clone.emit("sse-message", SseEventPayload {
                                r#type: "event".into(),
                                data: message.data,
                                event_id: Some(message.id),
                                event_name: Some(message.event),
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                            });
                        }
                        Some(Err(e)) => {
                            let _ = app_clone.emit("sse-message", SseEventPayload {
                                r#type: "error".into(),
                                data: e.to_string(),
                                event_id: None,
                                event_name: None,
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                            });
                            es.close();
                            break;
                        }
                        None => {
                            let _ = app_clone.emit("sse-message", SseEventPayload {
                                r#type: "disconnected".into(),
                                data: "Stream ended".into(),
                                event_id: None,
                                event_name: None,
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                            });
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn sse_disconnect(state: State<'_, SseState>) -> Result<(), String> {
    let mut tx_lock = state.cancel_tx.lock().await;
    if let Some(tx) = tx_lock.take() {
        let _ = tx.send(()).await;
    }
    Ok(())
}
