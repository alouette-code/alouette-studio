use crate::commands::agent::{
    load_custom_ai_config, resolve_api_key, resolve_model_config, AgentResponse, CustomAiConfig,
    LoopResult, LoopResultIteration,
};
use crate::commands::rig_bridge::agent_log;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Local;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, WebviewWindow};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, Notify};

// ─── Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachmentInput {
    pub name: String,
    pub mime: String,
    pub data_url: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedOpenCodeModel {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct PendingPermission {
    pub id: String,
    pub permission: String,
    pub args_pretty: String,
}

#[derive(Debug)]
pub enum TurnSignal {
    PermissionAsked(PendingPermission),
    Idle {
        full_text: String,
        iterations: Vec<LoopResultIteration>,
    },
    Error(String),
    Aborted,
}

pub struct ActiveTurn {
    pub session_id: String,
    pub directory: String,
    pub pending_permission: Option<PendingPermission>,
    pub signal_rx: mpsc::UnboundedReceiver<TurnSignal>,
    pub abort_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

struct OpenCodeProcess {
    child: Child,
    port: u16,
    password: String,
    config_hash: String,
}

pub struct OpenCodeManager {
    process: Mutex<Option<OpenCodeProcess>>,
    /// Map workspace path -> OpenCode session_id
    sessions: Mutex<HashMap<String, String>>,
    /// Currently running or permission-paused turn
    active_turn: Mutex<Option<ActiveTurn>>,
    /// Current active session/directory metadata for non-blocking cancel
    active_session_meta: Mutex<Option<(String, String)>>,
    /// Immediate turn cancellation signal (stops SSE emission in 0ms)
    active_cancel: Mutex<Option<(Arc<AtomicBool>, Arc<Notify>)>>,
    http: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenCodeHealthStatus {
    pub installed: bool,
    pub binary_path: Option<String>,
    pub running: bool,
    pub port: Option<u16>,
    pub version: Option<String>,
}

impl OpenCodeManager {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            process: Mutex::new(None),
            sessions: Mutex::new(HashMap::new()),
            active_turn: Mutex::new(None),
            active_session_meta: Mutex::new(None),
            active_cancel: Mutex::new(None),
            http,
        }
    }

    /// Invalidate running OpenCode instance so next call reloads updated AI config/keys.
    pub async fn invalidate_config(&self) {
        let mut guard = self.process.lock().await;
        if let Some(mut proc_info) = guard.take() {
            let _ = proc_info.child.kill().await;
        }
    }

    /// Terminate managed opencode serve process cleanly on app exit.
    pub async fn shutdown(&self) {
        self.cancel_active_turn().await;
        let mut guard = self.process.lock().await;
        if let Some(mut proc_info) = guard.take() {
            let _ = proc_info.child.kill().await;
        }
    }

    /// Check OpenCode installation & daemon status.
    pub async fn health_status(&self) -> OpenCodeHealthStatus {
        let bin = resolve_opencode_bin();
        let installed = bin.is_some();

        let proc_guard = self.process.lock().await;
        if let Some(ref p) = *proc_guard {
            let url = format!("http://127.0.0.1:{}/global/health", p.port);
            let auth = basic_auth_header(&p.password);
            if let Ok(resp) = self
                .http
                .get(&url)
                .header("Authorization", auth)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                if let Ok(val) = resp.json::<Value>().await {
                    return OpenCodeHealthStatus {
                        installed,
                        binary_path: bin.map(|b| b.to_string_lossy().to_string()),
                        running: val["healthy"].as_bool().unwrap_or(true),
                        port: Some(p.port),
                        version: val["version"].as_str().map(|s| s.to_string()),
                    };
                }
            }
        }

        OpenCodeHealthStatus {
            installed,
            binary_path: bin.map(|b| b.to_string_lossy().to_string()),
            running: false,
            port: None,
            version: None,
        }
    }

    /// Ensure `opencode serve` is running with the latest Alouette AI config & decrypted keys.
    async fn ensure_server(
        &self,
        ai_cfg: &CustomAiConfig,
        window: Option<&WebviewWindow>,
    ) -> Result<(String, String), String> {
        let (config_json, env_vars, config_hash) = build_opencode_config_and_env(ai_cfg);

        let mut proc_guard = self.process.lock().await;

        // Check if existing process is alive and config hash still matches
        if let Some(ref mut existing) = *proc_guard {
            let alive = existing.child.try_wait().ok().flatten().is_none();
            if alive && existing.config_hash == config_hash {
                let base_url = format!("http://127.0.0.1:{}", existing.port);
                let auth = basic_auth_header(&existing.password);
                return Ok((base_url, auth));
            }
            agent_log(
                window,
                "[OPENCODE] Cấu hình model/API key thay đổi hoặc process dừng, đang khởi động lại OpenCode Core...",
            );
            let _ = existing.child.kill().await;
            *proc_guard = None;
        }

        let bin_path = resolve_opencode_bin().ok_or_else(|| {
            "Không tìm thấy binary `opencode`. Vui lòng cài đặt (`npm i -g opencode-ai`) hoặc đặt biến môi trường `OPENCODE_BIN`.".to_string()
        })?;

        let port = find_free_port()?;
        let password = uuid::Uuid::new_v4().to_string();

        let mut cmd = Command::new(&bin_path);
        cmd.arg("serve")
            .arg("--port")
            .arg(port.to_string())
            .arg("--hostname")
            .arg("127.0.0.1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .env("OPENCODE_SERVER_PASSWORD", &password)
            .env("OPENCODE_CONFIG_CONTENT", &config_json);

        // Ensure parent directory of opencode (e.g. nvm node bin) is in PATH
        if let Some(parent) = bin_path.parent() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let separator = if cfg!(windows) { ";" } else { ":" };
            cmd.env(
                "PATH",
                format!("{}{}{}", parent.to_string_lossy(), separator, current_path),
            );
        }

        for (k, v) in env_vars {
            cmd.env(k, v);
        }

        agent_log(
            window,
            &format!(
                "[OPENCODE] Khởi chạy OpenCode Engine ({}) trên 127.0.0.1:{}",
                bin_path.display(),
                port
            ),
        );

        let child = cmd
            .spawn()
            .map_err(|e| format!("Không thể khởi chạy opencode serve: {}", e))?;

        let base_url = format!("http://127.0.0.1:{}", port);
        let auth = basic_auth_header(&password);

        *proc_guard = Some(OpenCodeProcess {
            child,
            port,
            password,
            config_hash,
        });
        drop(proc_guard);

        // Wait for /global/health
        let health_url = format!("{}/global/health", base_url);
        let mut ready = false;
        for _ in 0..50 {
            if let Ok(resp) = self
                .http
                .get(&health_url)
                .header("Authorization", &auth)
                .timeout(std::time::Duration::from_millis(400))
                .send()
                .await
            {
                if resp.status().is_success() {
                    ready = true;
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }

        if !ready {
            return Err("OpenCode server không phản hồi health check sau 7.5 giây.".to_string());
        }

        Ok((base_url, auth))
    }

    async fn get_or_create_session(
        &self,
        base_url: &str,
        auth: &str,
        directory: &str,
    ) -> Result<String, String> {
        {
            let guard = self.sessions.lock().await;
            if let Some(id) = guard.get(directory) {
                // Verify session still exists in running server
                let check_url = format!(
                    "{}/session/{}?directory={}",
                    base_url,
                    id,
                    urlencoding_encode(directory)
                );
                if let Ok(resp) = self
                    .http
                    .get(&check_url)
                    .header("Authorization", auth)
                    .send()
                    .await
                {
                    if resp.status().is_success() {
                        return Ok(id.clone());
                    }
                }
            }
        }

        let create_url = format!("{}/session?directory={}", base_url, urlencoding_encode(directory));
        let resp = self
            .http
            .post(&create_url)
            .header("Authorization", auth)
            .json(&json!({ "title": "Alouette Studio Session" }))
            .send()
            .await
            .map_err(|e| format!("Tạo session OpenCode thất bại: {}", e))?;

        if !resp.status().is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(format!("Lỗi tạo session OpenCode: {}", txt));
        }

        let val: Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse session OpenCode thất bại: {}", e))?;
        let session_id = val["id"]
            .as_str()
            .ok_or_else(|| "OpenCode không trả về session id".to_string())?
            .to_string();

        let mut guard = self.sessions.lock().await;
        guard.insert(directory.to_string(), session_id.clone());
        Ok(session_id)
    }

    /// Reset OpenCode session for active workspace (or all workspaces if None).
    pub async fn reset_session(&self, active_cwd: Option<&str>) {
        self.cancel_active_turn().await;
        let mut guard = self.sessions.lock().await;
        if let Some(dir) = active_cwd {
            guard.remove(dir);
        } else {
            guard.clear();
        }
    }

    /// Abort active OpenCode turn if running.
    pub async fn cancel_active_turn(&self) {
        // 1. Cut off SSE emission immediately (0ms) before HTTP abort round-trip
        if let Some((cancelled, notify)) = self.active_cancel.lock().await.take() {
            cancelled.store(true, Ordering::SeqCst);
            notify.notify_waiters();
        }

        // 2. Clear pending turn pump
        if let Some(mut turn) = self.active_turn.lock().await.take() {
            if let Some(tx) = turn.abort_tx.take() {
                let _ = tx.send(());
            }
        }

        // 3. Tell OpenCode server to abort generation upstream
        let meta = self.active_session_meta.lock().await.take();
        if let Some((session_id, directory)) = meta {
            let proc_info = {
                let guard = self.process.lock().await;
                guard
                    .as_ref()
                    .map(|p| (format!("http://127.0.0.1:{}", p.port), basic_auth_header(&p.password)))
            };
            if let Some((base_url, auth)) = proc_info {
                let abort_url = format!(
                    "{}/session/{}/abort?directory={}",
                    base_url,
                    session_id,
                    urlencoding_encode(&directory)
                );
                let _ = self
                    .http
                    .post(&abort_url)
                    .header("Authorization", &auth)
                    .send()
                    .await;
            }
        }
    }

    /// Revert OpenCode session back to just before a specific user message ID (cancelling any active turn).
    pub async fn revert_to_message(
        &self,
        message_id: Option<String>,
        active_cwd: Option<String>,
    ) -> Result<String, String> {
        self.cancel_active_turn().await;
        let directory = resolve_workspace_dir(active_cwd);

        let Some(mid) = message_id.filter(|s| !s.trim().is_empty()) else {
            return Ok("cancelled_only".to_string());
        };

        let session_id = {
            let guard = self.sessions.lock().await;
            guard.get(&directory).cloned()
        };

        let Some(sid) = session_id else {
            return Ok("no_active_session".to_string());
        };

        let proc_info = {
            let guard = self.process.lock().await;
            guard.as_ref().map(|p| {
                (
                    format!("http://127.0.0.1:{}", p.port),
                    basic_auth_header(&p.password),
                )
            })
        };

        let Some((base_url, auth)) = proc_info else {
            return Ok("server_not_running".to_string());
        };

        let revert_url = format!(
            "{}/session/{}/revert?directory={}",
            base_url,
            sid,
            urlencoding_encode(&directory)
        );
        let resp = self
            .http
            .post(&revert_url)
            .header("Authorization", &auth)
            .json(&json!({ "messageID": mid }))
            .send()
            .await
            .map_err(|e| format!("Revert session OpenCode thất bại: {}", e))?;

        if resp.status().is_success() {
            Ok("reverted".to_string())
        } else {
            // If message wasn't found in OpenCode (e.g. from older SQLite history), reset session cleanly
            let mut guard = self.sessions.lock().await;
            guard.remove(&directory);
            Ok("session_cleared".to_string())
        }
    }

    /// Send a user prompt (with optional image attachments) to OpenCode Core and stream events to Alouette UI.
    pub async fn send_message(
        &self,
        message: String,
        images: Vec<ImageAttachmentInput>,
        model_hint: String,
        mode: String,
        active_cwd: Option<String>,
        thinking_mode: Option<String>,
        code_rag: Option<Arc<crate::commands::code_rag::CodeRagState>>,
        window: WebviewWindow,
    ) -> Result<AgentResponse, String> {
        // Abort any leftover active turn before starting a fresh prompt
        self.cancel_active_turn().await;

        let directory = resolve_workspace_dir(active_cwd);
        let ai_cfg = load_custom_ai_config();

        // Validate key availability for selected model before invoking OpenCode
        let model_cfg = resolve_model_config(&ai_cfg, &model_hint);
        if !is_local_url(&model_cfg.api_url) {
            let _ = resolve_api_key(&model_cfg)?;
        }

        let resolved_model = resolve_opencode_model(&ai_cfg, &model_hint);
        let (base_url, auth) = self.ensure_server(&ai_cfg, Some(&window)).await?;
        let session_id = self
            .get_or_create_session(&base_url, &auth, &directory)
            .await?;

        let opencode_agent = match mode.as_str() {
            "autonomous" => "alouette_autonomous",
            "plan" => "plan",
            "copilot" => "alouette_copilot",
            _ => "alouette_interactive",
        };

        agent_log(
            Some(&window),
            &format!(
                "[OPENCODE] Gửi prompt tới {}/{} | agent={} | dir={}",
                resolved_model.provider_id, resolved_model.model_id, opencode_agent, directory
            ),
        );

        let _ = window.emit("agent-text-chunk", "");

        // 1. Connect SSE stream BEFORE triggering prompt_async so no early deltas are missed
        let sse_url = format!(
            "{}/event?directory={}",
            base_url,
            urlencoding_encode(&directory)
        );
        let sse_resp = self
            .http
            .get(&sse_url)
            .header("Authorization", &auth)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| format!("Không thể kết nối SSE /event của OpenCode: {}", e))?;

        if !sse_resp.status().is_success() {
            let txt = sse_resp.text().await.unwrap_or_default();
            return Err(format!("OpenCode /event error: {}", txt));
        }

        let (signal_tx, signal_rx) = mpsc::unbounded_channel::<TurnSignal>();
        let (abort_tx, abort_rx) = tokio::sync::oneshot::channel::<()>();
        let turn_cancelled = Arc::new(AtomicBool::new(false));
        let abort_notify = Arc::new(Notify::new());

        {
            let mut cancel_guard = self.active_cancel.lock().await;
            *cancel_guard = Some((turn_cancelled.clone(), abort_notify.clone()));
        }

        let auto_approve_permissions = mode == "autonomous";
        let http_clone = self.http.clone();
        let base_url_clone = base_url.clone();
        let auth_clone = auth.clone();
        let dir_clone = directory.clone();
        let sid_clone = session_id.clone();
        let window_clone = window.clone();

        tokio::spawn(async move {
            pump_opencode_sse(
                sse_resp,
                http_clone,
                base_url_clone,
                auth_clone,
                dir_clone,
                sid_clone,
                auto_approve_permissions,
                window_clone,
                signal_tx,
                abort_rx,
                turn_cancelled,
                abort_notify,
            )
            .await;
        });

        {
            let mut meta = self.active_session_meta.lock().await;
            *meta = Some((session_id.clone(), directory.clone()));
        }

        // 2. Trigger prompt_async
        let prompt_url = format!(
            "{}/session/{}/prompt_async?directory={}",
            base_url,
            session_id,
            urlencoding_encode(&directory)
        );

        let mut parts: Vec<Value> = Vec::new();
        let text_payload = if message.trim().is_empty() && !images.is_empty() {
            "Hãy xem và phân tích hình ảnh đính kèm.".to_string()
        } else {
            message
        };
        if !text_payload.trim().is_empty() {
            parts.push(json!({ "type": "text", "text": text_payload }));
        }
        for img in images {
            if !img.data_url.trim().is_empty() {
                parts.push(json!({
                    "type": "file",
                    "mime": img.mime,
                    "filename": img.name,
                    "url": img.data_url,
                }));
            }
        }

        // Cursor-style smart project context (Incremental Repo Map + Semantic RAG)
        // Bơm nhẹ vào system prompt, hoàn toàn KHÔNG đụng chạm tool built-in của OpenCode.
        let mut system_parts: Vec<String> = Vec::new();
        system_parts.push(
            "Formatting rule: When drawing architecture diagrams, module relationships, or data flows, ALWAYS use fenced ```mermaid code blocks (e.g. `flowchart TD` or `sequenceDiagram`) instead of plain ASCII box art so the IDE renders interactive clickable diagrams.".to_string(),
        );
        if thinking_mode.as_deref() == Some("high") {
            system_parts.push(
                "Think step-by-step carefully before editing code or running commands. Verify changes after editing.".to_string(),
            );
        }

        if let Some(rag_state) = code_rag {
            let dir_for_ctx = directory.clone();
            let query_for_ctx = text_payload.clone();
            let ctx_fut = tokio::task::spawn_blocking(move || {
                crate::commands::code_rag::build_cursor_project_context(
                    &rag_state,
                    &dir_for_ctx,
                    &query_for_ctx,
                )
            });

            if let Ok(Ok((ctx_str, log_summary))) =
                tokio::time::timeout(std::time::Duration::from_millis(1800), ctx_fut).await
            {
                if !log_summary.is_empty() {
                    agent_log(Some(&window), &log_summary);
                }
                if !ctx_str.is_empty() {
                    system_parts.push(ctx_str);
                }
            }
        }

        let mut body = json!({
            "model": {
                "providerID": resolved_model.provider_id,
                "modelID": resolved_model.model_id,
            },
            "agent": opencode_agent,
            "parts": parts
        });

        if !system_parts.is_empty() {
            body["system"] = json!(system_parts.join("\n\n"));
        }

        let prompt_resp = self
            .http
            .post(&prompt_url)
            .header("Authorization", &auth)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Gửi prompt_async tới OpenCode thất bại: {}", e))?;

        if !prompt_resp.status().is_success() {
            let status = prompt_resp.status();
            let err_body = prompt_resp.text().await.unwrap_or_default();
            let _ = abort_tx.send(());
            *self.active_session_meta.lock().await = None;
            return Err(format!("OpenCode prompt_async error ({}): {}", status, err_body));
        }

        // 3. Wait for Idle, PermissionAsked, Error, or Cancel
        self.wait_for_turn_event(session_id, directory, signal_rx, abort_tx)
            .await
    }

    /// Reply to a pending OpenCode permission request and resume streaming the turn.
    pub async fn approve_tool(
        &self,
        approved: bool,
        window: WebviewWindow,
    ) -> Result<AgentResponse, String> {
        let turn = self
            .active_turn
            .lock()
            .await
            .take()
            .ok_or_else(|| "Không có yêu cầu phê duyệt OpenCode nào đang chờ.".to_string())?;

        let perm = turn
            .pending_permission
            .clone()
            .ok_or_else(|| "Phiên OpenCode không ở trạng thái chờ duyệt tool.".to_string())?;

        let (base_url, auth) = {
            let guard = self.process.lock().await;
            let p = guard
                .as_ref()
                .ok_or_else(|| "OpenCode server không còn chạy.".to_string())?;
            (
                format!("http://127.0.0.1:{}", p.port),
                basic_auth_header(&p.password),
            )
        };

        let reply_value = if approved { "once" } else { "reject" };
        let reply_url = format!(
            "{}/permission/{}/reply?directory={}",
            base_url,
            perm.id,
            urlencoding_encode(&turn.directory)
        );

        agent_log(
            Some(&window),
            &format!(
                "[OPENCODE] Người dùng {} quyền '{}' ({})",
                if approved { "chấp thuận" } else { "từ chối" },
                perm.permission,
                perm.id
            ),
        );

        let resp = self
            .http
            .post(&reply_url)
            .header("Authorization", &auth)
            .json(&json!({ "reply": reply_value }))
            .send()
            .await
            .map_err(|e| format!("Gửi phản hồi permission tới OpenCode thất bại: {}", e))?;

        if !resp.status().is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(format!("OpenCode permission reply error: {}", txt));
        }

        let abort_tx = turn
            .abort_tx
            .ok_or_else(|| "Kênh điều khiển SSE đã đóng.".to_string())?;

        self.wait_for_turn_event(turn.session_id, turn.directory, turn.signal_rx, abort_tx)
            .await
    }

    async fn wait_for_turn_event(
        &self,
        session_id: String,
        directory: String,
        mut signal_rx: mpsc::UnboundedReceiver<TurnSignal>,
        abort_tx: tokio::sync::oneshot::Sender<()>,
    ) -> Result<AgentResponse, String> {
        match signal_rx.recv().await {
            Some(TurnSignal::PermissionAsked(perm)) => {
                let pending_id = perm.id.clone();
                let tool_name = perm.permission.clone();
                let args_pretty = perm.args_pretty.clone();

                let mut guard = self.active_turn.lock().await;
                *guard = Some(ActiveTurn {
                    session_id: session_id.clone(),
                    directory,
                    pending_permission: Some(perm),
                    signal_rx,
                    abort_tx: Some(abort_tx),
                });

                Ok(AgentResponse {
                    session_id,
                    reply_type: "tool_request".to_string(),
                    text: None,
                    tool_name: Some(tool_name),
                    args: Some(args_pretty),
                    tools: None,
                    pending_id: Some(pending_id),
                    iteration: None,
                    total_iterations: None,
                    tool_result: None,
                    approved_tool_index: None,
                    loop_result: None,
                })
            }
            Some(TurnSignal::Idle {
                full_text,
                iterations,
            }) => {
                let _ = abort_tx.send(());
                *self.active_cancel.lock().await = None;
                *self.active_session_meta.lock().await = None;

                let total_iters = iterations.len() as u32;
                let final_text_opt = if full_text.trim().is_empty() {
                    None
                } else {
                    Some(full_text.clone())
                };

                Ok(AgentResponse {
                    session_id,
                    reply_type: "loop_result".to_string(),
                    text: final_text_opt.clone(),
                    tool_name: None,
                    args: None,
                    tools: None,
                    pending_id: None,
                    iteration: Some(total_iters),
                    total_iterations: Some(total_iters),
                    tool_result: None,
                    approved_tool_index: None,
                    loop_result: Some(LoopResult {
                        iterations,
                        final_text: final_text_opt,
                        total_iterations: total_iters,
                        tool_calls_made: total_iters,
                        stopped_early: false,
                        stop_reason: None,
                    }),
                })
            }
            Some(TurnSignal::Error(err)) => {
                let _ = abort_tx.send(());
                *self.active_cancel.lock().await = None;
                *self.active_session_meta.lock().await = None;
                Err(err)
            }
            Some(TurnSignal::Aborted) | None => {
                *self.active_cancel.lock().await = None;
                *self.active_session_meta.lock().await = None;
                Ok(AgentResponse {
                    session_id,
                    reply_type: "cancelled".to_string(),
                    text: Some("Agent đã bị ngắt bởi người dùng.".to_string()),
                    tool_name: None,
                    args: None,
                    tools: None,
                    pending_id: None,
                    iteration: None,
                    total_iterations: None,
                    tool_result: None,
                    approved_tool_index: None,
                    loop_result: None,
                })
            }
        }
    }
}

// ─── SSE Event Pump ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn pump_opencode_sse(
    resp: reqwest::Response,
    http: reqwest::Client,
    base_url: String,
    auth: String,
    directory: String,
    session_id: String,
    auto_approve_permissions: bool,
    window: WebviewWindow,
    signal_tx: mpsc::UnboundedSender<TurnSignal>,
    mut abort_rx: tokio::sync::oneshot::Receiver<()>,
    turn_cancelled: Arc<AtomicBool>,
    abort_notify: Arc<Notify>,
) {
    let mut stream = resp.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    let mut assistant_msg_ids: HashSet<String> = HashSet::new();
    let mut part_types: HashMap<String, String> = HashMap::new();
    let mut text_parts_order: Vec<String> = Vec::new();
    let mut text_parts_map: HashMap<String, String> = HashMap::new();
    let mut thought_accum = String::new();
    let mut emitted_text_fallback = String::new();

    let mut tool_call_order: HashMap<String, u32> = HashMap::new();
    let mut next_iter_num: u32 = 1;
    let mut completed_iterations: Vec<LoopResultIteration> = Vec::new();
    let mut completed_call_ids: HashSet<String> = HashSet::new();

    loop {
        if turn_cancelled.load(Ordering::SeqCst) {
            let _ = signal_tx.send(TurnSignal::Aborted);
            break;
        }

        tokio::select! {
            _ = &mut abort_rx => {
                let _ = signal_tx.send(TurnSignal::Aborted);
                break;
            }
            _ = abort_notify.notified() => {
                let _ = signal_tx.send(TurnSignal::Aborted);
                break;
            }
            chunk_opt = stream.next() => {
                if turn_cancelled.load(Ordering::SeqCst) {
                    let _ = signal_tx.send(TurnSignal::Aborted);
                    break;
                }
                let bytes = match chunk_opt {
                    Some(Ok(b)) => b,
                    Some(Err(e)) => {
                        let _ = signal_tx.send(TurnSignal::Error(format!("OpenCode SSE stream error: {}", e)));
                        break;
                    }
                    None => {
                        break;
                    }
                };

                buffer.extend_from_slice(&bytes);

                while let Some(nl_idx) = buffer.iter().position(|&b| b == b'\n') {
                    if turn_cancelled.load(Ordering::SeqCst) {
                        let _ = signal_tx.send(TurnSignal::Aborted);
                        return;
                    }
                    let line_bytes: Vec<u8> = buffer.drain(..=nl_idx).collect();
                    let line = String::from_utf8_lossy(&line_bytes);
                    let trimmed = line.trim();
                    if !trimmed.starts_with("data:") {
                        continue;
                    }
                    let json_str = trimmed["data:".len()..].trim();
                    if json_str.is_empty() || json_str == "[DONE]" {
                        continue;
                    }

                    let Ok(ev) = serde_json::from_str::<Value>(json_str) else {
                        continue;
                    };

                    let ev_type = ev["type"].as_str().unwrap_or("");
                    let props = &ev["properties"];

                    match ev_type {
                        "message.updated" => {
                            let info = &props["info"];
                            if info["sessionID"].as_str() == Some(&session_id) {
                                let role = info["role"].as_str().unwrap_or("");
                                if role == "assistant" {
                                    if let Some(mid) = info["id"].as_str() {
                                        assistant_msg_ids.insert(mid.to_string());
                                    }
                                } else if role == "user" {
                                    if let Some(mid) = info["id"].as_str() {
                                        let _ = window.emit(
                                            "agent-user-message-id",
                                            json!({
                                                "session_id": session_id,
                                                "message_id": mid,
                                            }),
                                        );
                                    }
                                }
                            }
                        }
                        "message.part.updated" => {
                            let part = &props["part"];
                            let p_sid = part["sessionID"]
                                .as_str()
                                .or_else(|| props["sessionID"].as_str());
                            if p_sid != Some(&session_id) {
                                continue;
                            }

                            let part_id = part["id"].as_str().unwrap_or("").to_string();
                            let msg_id = part["messageID"].as_str().unwrap_or("");
                            let part_type = part["type"].as_str().unwrap_or("").to_string();

                            if !part_id.is_empty() && !part_type.is_empty() {
                                part_types.insert(part_id.clone(), part_type.clone());
                            }

                            match part_type.as_str() {
                                "text" => {
                                    if assistant_msg_ids.contains(msg_id) {
                                        if let Some(txt) = part["text"].as_str() {
                                            if !text_parts_map.contains_key(&part_id) {
                                                text_parts_order.push(part_id.clone());
                                            }
                                            text_parts_map.insert(part_id, txt.to_string());
                                        }
                                    }
                                }
                                "reasoning" => {
                                    if let Some(txt) = part["text"].as_str() {
                                        thought_accum = txt.to_string();
                                    }
                                }
                                "tool" => {
                                    let call_id = part["callID"]
                                        .as_str()
                                        .unwrap_or(&part_id)
                                        .to_string();
                                    let tool_name = part["tool"].as_str().unwrap_or("tool").to_string();
                                    let state = &part["state"];
                                    let status = state["status"].as_str().unwrap_or("");

                                    let iter_num = *tool_call_order.entry(call_id.clone()).or_insert_with(|| {
                                        let n = next_iter_num;
                                        next_iter_num += 1;
                                        n
                                    });

                                    let args_str = serde_json::to_string(&state["input"])
                                        .unwrap_or_else(|_| "{}".to_string());

                                    if status == "running" {
                                        let _ = window.emit(
                                            "agent-activity",
                                            json!({
                                                "status": "executing",
                                                "tool_name": tool_name,
                                            }),
                                        );
                                        let _ = window.emit(
                                            "agent-iteration",
                                            json!({
                                                "iteration": iter_num,
                                                "thought": if thought_accum.is_empty() { Value::Null } else { json!(thought_accum) },
                                                "tool_name": tool_name,
                                                "tool_args": args_str,
                                                "tool_result": Value::Null,
                                                "tool_success": true,
                                                "timestamp": Local::now().format("%H:%M").to_string(),
                                            }),
                                        );
                                    } else if (status == "completed" || status == "error")
                                        && !completed_call_ids.contains(&call_id)
                                    {
                                        completed_call_ids.insert(call_id);
                                        let success = status == "completed";
                                        let result_str = if success {
                                            state["output"].as_str().unwrap_or("").to_string()
                                        } else {
                                            state["error"].as_str().unwrap_or("Tool error").to_string()
                                        };
                                        let ts = Local::now().format("%H:%M").to_string();

                                        let _ = window.emit(
                                            "agent-iteration",
                                            json!({
                                                "iteration": iter_num,
                                                "thought": if thought_accum.is_empty() { Value::Null } else { json!(thought_accum) },
                                                "tool_name": tool_name,
                                                "tool_args": args_str,
                                                "tool_result": result_str,
                                                "tool_success": success,
                                                "timestamp": ts,
                                            }),
                                        );

                                        completed_iterations.push(LoopResultIteration {
                                            iteration: iter_num,
                                            thought: if thought_accum.is_empty() {
                                                None
                                            } else {
                                                Some(thought_accum.clone())
                                            },
                                            tool_name: Some(tool_name),
                                            tool_args: Some(args_str),
                                            tool_result: Some(result_str),
                                            tool_success: success,
                                            timestamp: ts,
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "message.part.delta" => {
                            if props["sessionID"].as_str() != Some(&session_id) {
                                continue;
                            }
                            let msg_id = props["messageID"].as_str().unwrap_or("");
                            if !assistant_msg_ids.is_empty() && !assistant_msg_ids.contains(msg_id) {
                                continue;
                            }
                            let part_id = props["partID"].as_str().unwrap_or("");
                            let field = props["field"].as_str().unwrap_or("");
                            let delta = props["delta"].as_str().unwrap_or("");
                            if field != "text" || delta.is_empty() {
                                continue;
                            }

                            let p_type = part_types.get(part_id).map(|s| s.as_str()).unwrap_or("text");
                            if p_type == "reasoning" {
                                thought_accum.push_str(delta);
                                let _ = window.emit("agent-thought-chunk", delta);
                            } else {
                                emitted_text_fallback.push_str(delta);
                                let _ = window.emit("agent-text-chunk", delta);
                            }
                        }
                        "permission.asked" => {
                            if props["sessionID"].as_str() != Some(&session_id) {
                                continue;
                            }
                            let req_id = props["id"].as_str().unwrap_or("").to_string();
                            let permission = props["permission"].as_str().unwrap_or("tool").to_string();

                            if auto_approve_permissions {
                                let reply_url = format!(
                                    "{}/permission/{}/reply?directory={}",
                                    base_url,
                                    req_id,
                                    urlencoding_encode(&directory)
                                );
                                let _ = http
                                    .post(&reply_url)
                                    .header("Authorization", &auth)
                                    .json(&json!({ "reply": "always" }))
                                    .send()
                                    .await;
                                continue;
                            }

                            let args_pretty = serde_json::to_string_pretty(&json!({
                                "patterns": props["patterns"],
                                "metadata": props["metadata"],
                            }))
                            .unwrap_or_else(|_| "{}".to_string());

                            let _ = signal_tx.send(TurnSignal::PermissionAsked(PendingPermission {
                                id: req_id,
                                permission,
                                args_pretty,
                            }));
                        }
                        "session.idle" => {
                            if props["sessionID"].as_str() != Some(&session_id) {
                                continue;
                            }
                            if !thought_accum.is_empty() {
                                let _ = window.emit("agent-thought-final", &thought_accum);
                            }
                            let ordered_text: String = text_parts_order
                                .iter()
                                .filter_map(|id| text_parts_map.get(id))
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("\n\n");
                            let full_text = if !ordered_text.trim().is_empty() {
                                ordered_text
                            } else {
                                emitted_text_fallback.clone()
                            };

                            let _ = window.emit(
                                "agent-stream-complete",
                                json!({
                                    "has_error": false,
                                    "error": Value::Null,
                                    "has_text": !full_text.is_empty(),
                                }),
                            );

                            let _ = signal_tx.send(TurnSignal::Idle {
                                full_text,
                                iterations: completed_iterations.clone(),
                            });
                            return;
                        }
                        "session.error" => {
                            if let Some(sid) = props["sessionID"].as_str() {
                                if sid != session_id {
                                    continue;
                                }
                            }
                            let err_msg = props["error"]["data"]["message"]
                                .as_str()
                                .or_else(|| props["error"]["message"].as_str())
                                .or_else(|| props["error"]["name"].as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| props["error"].to_string());

                            let _ = window.emit(
                                "agent-stream-complete",
                                json!({
                                    "has_error": true,
                                    "error": err_msg,
                                    "has_text": false,
                                }),
                            );
                            let _ = signal_tx.send(TurnSignal::Error(format!("OpenCode Error: {}", err_msg)));
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// ─── Config & Provider Mapping Helpers ──────────────────────────────────

fn sanitize_provider_id(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("alouette-{}", cleaned)
}

fn sanitize_env_suffix(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn infer_npm_adapter(provider_name: &str, api_standard: Option<&str>, api_url: &str) -> &'static str {
    let std_lower = api_standard.unwrap_or("").to_ascii_lowercase();
    let prov_lower = provider_name.to_ascii_lowercase();
    let url_lower = api_url.to_ascii_lowercase();

    if std_lower == "claude" || std_lower == "anthropic" || prov_lower == "claude" || prov_lower == "anthropic" {
        return "@ai-sdk/anthropic";
    }
    if std_lower == "gemini" || std_lower == "google" || prov_lower == "gemini" || prov_lower == "google" {
        return "@ai-sdk/google";
    }
    if (prov_lower == "openai" || prov_lower == "gpt-chatgpt")
        && (url_lower.is_empty() || url_lower.contains("api.openai.com"))
    {
        return "@ai-sdk/openai";
    }
    "@ai-sdk/openai-compatible"
}

fn is_local_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("0.0.0.0")
}

/// Build dynamic `OPENCODE_CONFIG_CONTENT` and secret environment variables from Alouette's `CustomAiConfig`.
fn build_opencode_config_and_env(
    ai_cfg: &CustomAiConfig,
) -> (String, Vec<(String, String)>, String) {
    let mut providers_obj = serde_json::Map::new();
    let mut env_vars: Vec<(String, String)> = Vec::new();
    let mut hash_input = String::new();

    for (provider_name, provider_cfg) in &ai_cfg.providers {
        if provider_cfg.models.is_empty() {
            continue;
        }

        let oc_provider_id = sanitize_provider_id(provider_name);
        let first_model_std = provider_cfg
            .models
            .values()
            .find_map(|m| m.api_standard.as_deref());
        let provider_url = provider_cfg
            .api_url
            .clone()
            .or_else(|| {
                provider_cfg
                    .models
                    .values()
                    .find_map(|m| m.api_url.clone())
            })
            .unwrap_or_default();

        let npm_pkg = infer_npm_adapter(provider_name, first_model_std, &provider_url);

        // Try resolving the API key via Alouette's existing resolver
        let dummy_cfg = crate::commands::agent::ModelConfig {
            provider: provider_name.clone(),
            api_key: provider_cfg.api_key.clone(),
            api_url: provider_url.clone(),
            context_limit: 128_000,
            supports_vision: false,
            temperature: 0.2,
            top_p: 0.95,
            api_standard: first_model_std.map(|s| s.to_string()),
        };
        let resolved_key = resolve_api_key(&dummy_cfg).unwrap_or_default();

        let mut options = serde_json::Map::new();
        if !provider_url.trim().is_empty() {
            options.insert("baseURL".to_string(), json!(provider_url.trim()));
        }

        if !resolved_key.is_empty() {
            let env_name = format!("ALOUETTE_OC_KEY_{}", sanitize_env_suffix(provider_name));
            options.insert("apiKey".to_string(), json!(format!("{{env:{}}}", env_name)));
            hash_input.push_str(&format!("{}:{}|", env_name, resolved_key));
            env_vars.push((env_name, resolved_key));
        } else if is_local_url(&provider_url) {
            options.insert("apiKey".to_string(), json!("alouette-local"));
        }

        let mut models_obj = serde_json::Map::new();
        for (model_id, detail) in &provider_cfg.models {
            let lower_id = model_id.to_ascii_lowercase();
            let is_reasoning = lower_id.contains("r1")
                || lower_id.contains("reasoner")
                || lower_id.contains("o1")
                || lower_id.contains("o3")
                || lower_id.contains("thinking");

            models_obj.insert(
                model_id.clone(),
                json!({
                    "name": model_id,
                    "tool_call": true,
                    "attachment": true,
                    "modalities": {
                        "input": ["text", "image"],
                        "output": ["text"]
                    },
                    "reasoning": is_reasoning,
                    "limit": {
                        "context": detail.context_limit.max(8192),
                        "output": 16384
                    }
                }),
            );
        }

        providers_obj.insert(
            oc_provider_id,
            json!({
                "npm": npm_pkg,
                "name": format!("Alouette ({})", provider_name),
                "options": Value::Object(options),
                "models": Value::Object(models_obj),
            }),
        );
    }

    let config_val = json!({
        "$schema": "https://opencode.ai/config.json",
        "provider": Value::Object(providers_obj),
        "agent": {
            "alouette_interactive": {
                "mode": "primary",
                "description": "Alouette Interactive Coding Agent (asks before edits & shell)",
                "permission": {
                    "*": "allow",
                    "edit": "ask",
                    "bash": "ask",
                    "external_directory": "ask",
                    "webfetch": "ask",
                    "question": "deny"
                }
            },
            "alouette_autonomous": {
                "mode": "primary",
                "description": "Alouette Autonomous Coding Agent (auto-approves tools)",
                "permission": {
                    "*": "allow",
                    "question": "deny"
                }
            },
            "alouette_copilot": {
                "mode": "primary",
                "description": "Alouette Copilot Agent (read-first, asks before writes)",
                "permission": {
                    "*": "allow",
                    "edit": "ask",
                    "bash": "ask",
                    "external_directory": "ask",
                    "question": "deny"
                }
            }
        }
    });

    let config_json = serde_json::to_string(&config_val).unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(config_json.as_bytes());
    hasher.update(hash_input.as_bytes());
    let config_hash = STANDARD.encode(hasher.finalize());

    (config_json, env_vars, config_hash)
}

/// Resolve Alouette's selected model string (or `provider/model` syntax) to OpenCode `(providerID, modelID)`.
fn resolve_opencode_model(ai_cfg: &CustomAiConfig, model_hint: &str) -> ResolvedOpenCodeModel {
    let trimmed = model_hint.trim();
    if let Some((prov, mdl)) = trimmed.split_once('/') {
        if !prov.is_empty() && !mdl.is_empty() {
            return ResolvedOpenCodeModel {
                provider_id: prov.to_string(),
                model_id: mdl.to_string(),
            };
        }
    }

    let target = if !trimmed.is_empty()
        && trimmed != "autonomous"
        && trimmed != "write"
        && trimmed != "full"
    {
        trimmed
    } else {
        ai_cfg.active_model.as_str()
    };

    for (provider_name, provider_cfg) in &ai_cfg.providers {
        if provider_cfg.models.contains_key(target) {
            return ResolvedOpenCodeModel {
                provider_id: sanitize_provider_id(provider_name),
                model_id: target.to_string(),
            };
        }
    }

    // Fallback to first configured provider/model
    for (provider_name, provider_cfg) in &ai_cfg.providers {
        if let Some(first_model) = provider_cfg.models.keys().next() {
            return ResolvedOpenCodeModel {
                provider_id: sanitize_provider_id(provider_name),
                model_id: first_model.clone(),
            };
        }
    }

    ResolvedOpenCodeModel {
        provider_id: "alouette-gemini".to_string(),
        model_id: "gemini-2.5-flash".to_string(),
    }
}

fn resolve_workspace_dir(active_cwd: Option<String>) -> String {
    active_cwd
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .to_string()
        })
}

fn basic_auth_header(password: &str) -> String {
    let token = STANDARD.encode(format!("opencode:{}", password));
    format!("Basic {}", token)
}

fn find_free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Không thể cấp phát cổng local cho OpenCode: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Không thể đọc cổng local: {}", e))?
        .port();
    Ok(port)
}

fn urlencoding_encode(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

/// Locate `opencode` executable across env, PATH, nvm, and user bin directories.
pub fn resolve_opencode_bin() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("OPENCODE_BIN") {
        let p = PathBuf::from(explicit);
        if p.is_file() {
            return Some(p);
        }
    }

    let exe_name = if cfg!(windows) {
        "opencode.exe"
    } else {
        "opencode"
    };

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let home_dir = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    if let Some(home) = home_dir {
        let direct_candidates = [
            home.join(".opencode/bin").join(exe_name),
            home.join(".local/bin").join(exe_name),
            home.join(".npm-global/bin").join(exe_name),
            home.join(".bun/bin").join(exe_name),
        ];
        for cand in direct_candidates {
            if cand.is_file() {
                return Some(cand);
            }
        }

        // Check ~/.nvm/versions/node/*/bin/opencode
        let nvm_dir = home.join(".nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            let mut versions: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            versions.sort();
            versions.reverse();
            for v_dir in versions {
                let cand = v_dir.join("bin").join(exe_name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }

    for sys_dir in ["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"] {
        let cand = Path::new(sys_dir).join(exe_name);
        if cand.is_file() {
            return Some(cand);
        }
    }

    None
}

// ─── Tauri Commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn opencode_health(
    app_state: tauri::State<'_, crate::state::AppState>,
) -> Result<OpenCodeHealthStatus, String> {
    Ok(app_state.opencode_manager.health_status().await)
}

#[tauri::command]
pub async fn opencode_restart(
    app_state: tauri::State<'_, crate::state::AppState>,
) -> Result<String, String> {
    app_state.opencode_manager.invalidate_config().await;
    Ok("✓ OpenCode Engine sẽ tự khởi động lại với cấu hình mới ở lần gọi tiếp theo.".to_string())
}

#[tauri::command]
pub async fn agent_revert_to_message(
    message_id: Option<String>,
    active_cwd: Option<String>,
    app_state: tauri::State<'_, crate::state::AppState>,
) -> Result<String, String> {
    app_state
        .opencode_manager
        .revert_to_message(message_id, active_cwd)
        .await
}

#[tauri::command]
pub async fn read_clipboard_image() -> Result<Option<ImageAttachmentInput>, String> {
    tokio::task::spawn_blocking(|| -> Result<Option<ImageAttachmentInput>, String> {
        #[cfg(target_os = "linux")]
        {
            // 1. Try wl-paste (Wayland)
            if let Ok(out) = std::process::Command::new("wl-paste")
                .args(["--no-newline", "--type", "image/png"])
                .output()
            {
                if out.status.success() && !out.stdout.is_empty() {
                    let b64 = STANDARD.encode(&out.stdout);
                    return Ok(Some(ImageAttachmentInput {
                        name: format!("clipboard-{}.png", chrono::Local::now().timestamp_millis()),
                        mime: "image/png".to_string(),
                        data_url: format!("data:image/png;base64,{}", b64),
                    }));
                }
            }

            // 2. Try xclip (X11)
            if let Ok(out) = std::process::Command::new("xclip")
                .args(["-selection", "clipboard", "-t", "image/png", "-o"])
                .output()
            {
                if out.status.success() && !out.stdout.is_empty() {
                    let b64 = STANDARD.encode(&out.stdout);
                    return Ok(Some(ImageAttachmentInput {
                        name: format!("clipboard-{}.png", chrono::Local::now().timestamp_millis()),
                        mime: "image/png".to_string(),
                        data_url: format!("data:image/png;base64,{}", b64),
                    }));
                }
            }

            // 3. Fallback: Python3 GTK3 (handles both Wayland and X11 natively)
            let py_script = r#"
import base64, sys
try:
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk, Gdk
    cb = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
    pixbuf = cb.wait_for_image()
    if pixbuf:
        ok, buf = pixbuf.save_to_bufferv("png", [], [])
        if ok and buf:
            sys.stdout.write(base64.b64encode(bytes(buf)).decode("ascii"))
except Exception:
    pass
"#;
            if let Ok(out) = std::process::Command::new("python3")
                .args(["-c", py_script])
                .output()
            {
                if out.status.success() && !out.stdout.is_empty() {
                    let b64 = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !b64.is_empty() {
                        return Ok(Some(ImageAttachmentInput {
                            name: format!("clipboard-{}.png", chrono::Local::now().timestamp_millis()),
                            mime: "image/png".to_string(),
                            data_url: format!("data:image/png;base64,{}", b64),
                        }));
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let ps_script = r#"
Add-Type -AssemblyName System.Windows.Forms
$img = [System.Windows.Forms.Clipboard]::GetImage()
if ($img -ne $null) {
    $ms = New-Object System.IO.MemoryStream
    $img.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($ms.ToArray())
}
"#;
            if let Ok(out) = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", ps_script])
                .output()
            {
                if out.status.success() && !out.stdout.is_empty() {
                    let b64 = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !b64.is_empty() {
                        return Ok(Some(ImageAttachmentInput {
                            name: format!("clipboard-{}.png", chrono::Local::now().timestamp_millis()),
                            mime: "image/png".to_string(),
                            data_url: format!("data:image/png;base64,{}", b64),
                        }));
                    }
                }
            }
        }

        Ok(None)
    })
    .await
    .map_err(|e| format!("Clipboard task error: {}", e))?
}

pub fn new_shared_manager() -> Arc<OpenCodeManager> {
    Arc::new(OpenCodeManager::new())
}
