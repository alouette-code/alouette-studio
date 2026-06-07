use crate::commands::rig_bridge;
use crate::state::{AppState, LoopState};
use chrono::Local;
use core_engine::agent_harness::{
    AgentHarness, AgentSession, AgentState, ChatMessage, HarnessMode, LlmResponse, MessageContent,
    TickResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, State, WebviewWindow};

// ─── Response Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub args: String,
    pub pending_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentResponse {
    pub session_id: String,
    pub reply_type: String, // "text" | "tool_request" | "tool_batch_request" | "loop_result"
    pub text: Option<String>,
    pub tool_name: Option<String>,
    pub args: Option<String>,
    pub tools: Option<Vec<ToolInfo>>,
    pub pending_id: Option<String>,
    pub iteration: Option<u32>,
    pub total_iterations: Option<u32>,
    pub tool_result: Option<String>,
    pub approved_tool_index: Option<usize>,
}

// ─── Cancel / Interrupt ────────────────────────────────────────────────

/// Kiểm tra cờ ngắt và reset nếu đã được bật
fn check_and_reset_cancel(cancel_flag: &AtomicBool) -> bool {
    let cancelled = cancel_flag.load(Ordering::SeqCst);
    if cancelled {
        cancel_flag.store(false, Ordering::SeqCst);
    }
    cancelled
}

/// Gửi tín hiệu ngắt agent loop đang chạy
#[tauri::command]
pub async fn agent_cancel(app_state: State<'_, AppState>) -> Result<String, String> {
    app_state.agent_cancel_flag.store(true, Ordering::SeqCst);
    Ok("✓ Đã gửi tín hiệu ngắt. Agent sẽ dừng ngay lập tức.".to_string())
}

async fn wait_for_cancel(cancel_flag: std::sync::Arc<AtomicBool>) {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

enum TickExecutionResult {
    Completed(TickResult),
    Cancelled,
    Timeout,
}

async fn run_tick<F, Fut>(
    harness: &mut AgentHarness,
    session: &mut AgentSession,
    system_prompt: &str,
    cancel_flag: std::sync::Arc<AtomicBool>,
    llm_closure: F,
) -> TickExecutionResult
where
    F: Fn(String, Vec<ChatMessage>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<LlmResponse, String>> + Send + 'static,
{
    let cancel_flag_clone = cancel_flag.clone();
    tokio::select! {
        res = harness.tick(session, system_prompt, llm_closure) => {
            TickExecutionResult::Completed(res)
        }
        _ = wait_for_cancel(cancel_flag_clone) => {
            cancel_flag.store(false, Ordering::SeqCst);
            TickExecutionResult::Cancelled
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(90)) => {
            TickExecutionResult::Timeout
        }
    }
}

/// Trả về trạng thái hiện tại của agent (cho frontend poll)
#[tauri::command]
pub async fn agent_status(app_state: State<'_, AppState>) -> Result<Value, String> {
    let session_store = &app_state.agent_session;
    let guard = session_store.lock().unwrap();
    if let Some(ref session) = *guard {
        let state_str = match &session.state {
            AgentState::Idle => "idle",
            AgentState::Thinking => "thinking",
            AgentState::ExecutingTool => "executing_tool",
            AgentState::AwaitingApproval(_) => "awaiting_approval",
            AgentState::Verifying => "verifying",
            AgentState::Finished(_) => "finished",
            AgentState::Error(_) => "error",
        };
        Ok(json!({
            "session_id": session.session_id,
            "state": state_str,
            "iteration": session.iteration_count,
            "max_iterations": session.max_iterations,
            "history_len": session.history.len(),
        }))
    } else {
        Ok(json!({
            "session_id": null,
            "state": "no_session",
            "iteration": 0,
            "max_iterations": 25,
            "history_len": 0,
        }))
    }
}

// ─── Model Config ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub api_key: String,
    pub api_url: String,
    pub context_limit: usize,
    pub supports_vision: bool,
    pub temperature: f32,
    pub top_p: f32,
    pub api_standard: Option<String>,
}

fn default_active_model() -> String {
    "gemini-1.5-flash".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetail {
    pub context_limit: usize,
    pub supports_vision: bool,
    pub api_standard: Option<String>,
    pub api_url: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub api_url: Option<String>,
    pub models: std::collections::HashMap<String, ModelDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAiConfig {
    #[serde(default = "default_active_model")]
    pub active_model: String,
    pub providers: std::collections::HashMap<String, ProviderConfig>,
}

// ─── Core Agent Command ─────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_send_message(
    message: String,
    model: String,
    mode: String,
    active_cwd: Option<String>,
    thinking_mode: Option<String>,
    window: WebviewWindow,
    app_state: State<'_, AppState>,
) -> Result<AgentResponse, String> {
    let workspace = active_cwd.map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    let mut harness = AgentHarness::new(&workspace);

    // Load model config
    let ai_cfg = load_custom_ai_config();
    let model_config = resolve_model_config(&ai_cfg, &model);
    log_debug(&format!("Resolved model config: provider={}, api_standard={:?}, api_url={}", model_config.provider, model_config.api_standard, model_config.api_url));
    let api_key = resolve_api_key(&model_config)?;

    // Determine approval policy based on mode
    let is_autonomous = mode == "autonomous";
    let is_write_mode = mode == "write" || mode == "full";
    let auto_approve_reads = true;
    let auto_approve_writes = is_write_mode || is_autonomous;
    let auto_approve_all = is_autonomous;
    let max_iterations: u32 = 25;

    // Save approval policy for later (agent_approve_tool)
    {
        let mut state = app_state.agent_loop_state.lock().unwrap();
        *state = Some(LoopState {
            max_iterations,
            auto_approve_reads,
            auto_approve_writes,
            auto_approve_all,
            command_timeout_secs: 120,
            iteration_count: 0,
        });
    }

    // Initialize session
    let session_store = &app_state.agent_session;
    let mut session = {
        let mut session_guard = session_store.lock().unwrap();
        // If there's an existing session in non-terminal state, use it
        let existing = session_guard.take();
        let mut session = existing.unwrap_or_else(|| AgentSession {
            session_id: format!("sess_{}", Local::now().timestamp()),
            history: Vec::new(),
            state: AgentState::Idle,
            iteration_count: 0,
            max_iterations,
            mode: HarnessMode::Standard,
            plan: None,
            autonomous_state: None,
        });

        // Reset state to Idle for new message (unless awaiting approval)
        if !matches!(session.state, AgentState::AwaitingApproval(_)) {
            session.state = AgentState::Idle;
        }

        // Add user message to history
        session.history.push(ChatMessage {
            id: format!("usr_{}", Local::now().timestamp_millis()),
            role: "user".to_string(),
            content: MessageContent::Text(message.clone()),
            timestamp: Local::now().format("%H:%M").to_string(),
        });
        session
    };

    // Build system prompt
    let system_prompt = harness.assemble_system_prompt();

    // Clone values for LLM closure
    let api_standard = model_config
        .api_standard
        .clone()
        .unwrap_or_else(|| "gemini".to_string());
    let model_to_use = if !model.is_empty() {
        model.clone()
    } else if !ai_cfg.active_model.is_empty() {
        ai_cfg.active_model.clone()
    } else {
        "gemini-1.5-flash".to_string()
    };
    let api_key_clone = api_key.clone();
    let api_url = model_config.api_url.clone();
    let temperature = model_config.temperature;
    let top_p = model_config.top_p;

    // ─── STATE MACHINE: Tick loop ──────────────────────────────────
    // Gọi tick() liên tục cho đến khi gặp WaitForApproval, Finished, Error, hoặc Cancel
    let saved_session_id = session.session_id.clone();
    let result = loop {
        // Kiểm tra ngắt trước mỗi bước
        if check_and_reset_cancel(&app_state.agent_cancel_flag) {
            let _ = window.emit(
                "agent-cancelled",
                serde_json::json!({
                    "reason": "user_cancelled",
                }),
            );
            let mut sg = session_store.lock().unwrap();
            *sg = Some(session);
            break Ok(AgentResponse {
                session_id: saved_session_id,
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
            });
        }
        let llm_closure = {
            let api_key = api_key_clone.clone();
            let model = model_to_use.clone();
            let url = api_url.clone();
            let std = api_standard.clone();
            let window_clone = window.clone();
            let thinking_mode_clone = thinking_mode.clone();
            move |sys_prompt: String, history: Vec<ChatMessage>| {
                let api_key = api_key.clone();
                let model = model.clone();
                let url = url.clone();
                let std = std.clone();
                let w = window_clone.clone();
                let thinking_mode = thinking_mode_clone.clone();
                async move {
                    rig_bridge::call_rig(
                        &std,
                        &api_key,
                        &model,
                        &url,
                        temperature,
                        top_p,
                        &sys_prompt,
                        &history,
                        thinking_mode.as_deref(),
                        Some(&w),
                    )
                    .await
                }
            }
        };

        log_debug(&format!("Running tick... Current iteration: {}, State: {:?}", session.iteration_count, session.state));
        let tick_res_val = run_tick(&mut harness, &mut session, &system_prompt, app_state.agent_cancel_flag.clone(), llm_closure).await;
        let tick_res_type = match &tick_res_val {
            TickExecutionResult::Completed(tr) => match tr {
                TickResult::Continue { .. } => "Continue",
                TickResult::WaitForApproval { .. } => "WaitForApproval",
                TickResult::Finished { .. } => "Finished",
                TickResult::Error { .. } => "Error",
            },
            TickExecutionResult::Cancelled => "Cancelled",
            TickExecutionResult::Timeout => "Timeout",
        };
        log_debug(&format!("Tick completed. Result type: {}", tick_res_type));

        match tick_res_val {
            TickExecutionResult::Completed(tick_res) => match tick_res {
                TickResult::Continue {
                    thought,
                    tool_name,
                    tool_result,
                    iteration,
                } => {
                    let _ = window.emit(
                        "agent-iteration",
                        serde_json::json!({
                            "iteration": iteration,
                            "thought": thought.clone(),
                            "tool_name": tool_name.clone(),
                            "tool_result": tool_result.clone(),
                        }),
                    );
                    let mut log_msgs = Vec::new();
                    if let Some(ref t) = thought {
                        log_msgs.push(format!("[Suy nghĩ - Vòng {}] {}", iteration, t));
                    }
                    if let Some(ref tn) = tool_name {
                        log_msgs.push(format!("[Gọi Công cụ] Đang thực thi công cụ: {}", tn));
                    }
                    if let Some(ref tr) = tool_result {
                        let short_res = if tr.len() > 1000 {
                            format!("{}... (cắt bớt)", &tr[..1000])
                        } else {
                            tr.clone()
                        };
                        log_msgs.push(format!("[Kết quả Công cụ] Phản hồi: {}", short_res));
                    }
                    for msg in log_msgs {
                        rig_bridge::agent_log(Some(&window), &msg);
                    }
                    continue;
                }
                TickResult::WaitForApproval { tools, iteration } => {
                    let tool = tools.first().cloned();
                    let tool_name = tool.as_ref().map(|t| t.name.clone());
                    let args = tool.as_ref().map(|t| t.arguments.to_string());
                    let mut sg = session_store.lock().unwrap();
                    *sg = Some(session);

                    if tools.len() > 1 {
                        // Batch request: trả về tất cả tools
                        let batch_tools: Vec<ToolInfo> = tools
                            .iter()
                            .enumerate()
                            .map(|(i, t)| ToolInfo {
                                name: t.name.clone(),
                                args: t.arguments.to_string(),
                                pending_id: format!(
                                    "tool_{}_{}",
                                    Local::now().timestamp_millis(),
                                    i
                                ),
                            })
                            .collect();
                        rig_bridge::agent_log(
                            Some(&window),
                            &format!(
                                "[Chờ phê duyệt] Batch {} công cụ: {:?}",
                                batch_tools.len(),
                                batch_tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                            ),
                        );
                        break Ok(AgentResponse {
                            session_id: format!("pending_{}", Local::now().timestamp()),
                            reply_type: "tool_batch_request".to_string(),
                            text: None,
                            tool_name: None,
                            args: None,
                            tools: Some(batch_tools),
                            pending_id: None,
                            iteration: Some(iteration),
                            total_iterations: Some(max_iterations),
                            tool_result: None,
                            approved_tool_index: None,
                        });
                    } else {
                        rig_bridge::agent_log(
                            Some(&window),
                            &format!(
                                "[Chờ phê duyệt] Yêu cầu cấp quyền chạy công cụ: {:?}",
                                tool_name
                            ),
                        );
                        break Ok(AgentResponse {
                            session_id: format!("pending_{}", Local::now().timestamp()),
                            reply_type: "tool_request".to_string(),
                            text: None,
                            tool_name,
                            args,
                            tools: None,
                            pending_id: Some(format!("tool_{}", Local::now().timestamp_millis())),
                            iteration: Some(iteration),
                            total_iterations: Some(max_iterations),
                            tool_result: None,
                            approved_tool_index: None,
                        });
                    }
                }
                TickResult::Finished {
                    text,
                    total_iterations,
                } => {
                    let mut sg = session_store.lock().unwrap();
                    *sg = Some(session);
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!(
                            "[Hoàn thành] Agent kết thúc vòng lặp sau {} bước.",
                            total_iterations
                        ),
                    );
                    break Ok(AgentResponse {
                        session_id: saved_session_id.clone(),
                        reply_type: "loop_result".to_string(),
                        text: Some(text.clone()),
                        tool_name: None,
                        args: None,
                        tools: None,
                        pending_id: None,
                        iteration: Some(total_iterations),
                        total_iterations: Some(max_iterations),
                        tool_result: None,
                        approved_tool_index: None,
                    });
                }
                TickResult::Error { message, iteration } => {
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!("[Lỗi Agent] Lỗi ở vòng {}: {}", iteration, message),
                    );
                    break Err(format!("Agent error (iter {}): {}", iteration, message));
                }
            },
            TickExecutionResult::Cancelled => {
                let _ = window.emit(
                    "agent-cancelled",
                    serde_json::json!({
                        "reason": "user_cancelled",
                    }),
                );
                let mut sg = session_store.lock().unwrap();
                *sg = Some(session);
                rig_bridge::agent_log(Some(&window), "[Dừng] Agent đã bị dừng bởi người dùng.");
                break Ok(AgentResponse {
                    session_id: saved_session_id,
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
                });
            }
            TickExecutionResult::Timeout => {
                let mut sg = session_store.lock().unwrap();
                *sg = Some(session);
                rig_bridge::agent_log(
                    Some(&window),
                    "[Lỗi Hết Giờ] Đã hết thời gian 90 giây chờ phản hồi từ LLM.",
                );
                break Err("Đã hết thời gian chờ phản hồi từ Agent (Timeout 90 giây).".to_string());
            }
        }
    };

    result
}

// ─── Approve / Reject Tool (tiếp tục loop) ──────────────────────────────

#[tauri::command]
pub async fn agent_approve_tool(
    approved: bool,
    model: String,
    active_cwd: Option<String>,
    tool_index: Option<usize>,
    thinking_mode: Option<String>,
    window: WebviewWindow,
    app_state: State<'_, AppState>,
) -> Result<AgentResponse, String> {
    let workspace = active_cwd.map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    let mut harness = AgentHarness::new(&workspace);

    // Lấy session — kiểm tra trạng thái hiện tại
    let session_store = &app_state.agent_session;
    let is_awaiting = {
        let session_guard = session_store.lock().unwrap();
        session_guard
            .as_ref()
            .map(|s| matches!(s.state, AgentState::AwaitingApproval(_)))
            .unwrap_or(false)
    };

    if !is_awaiting {
        return Err(
            "Agent is not waiting for approval. Current state is not AwaitingApproval.".to_string(),
        );
    }

    let mut session = {
        let mut session_guard = session_store.lock().unwrap();
        session_guard
            .take()
            .ok_or_else(|| "No active session found.".to_string())?
    };

    // ─── USER REJECTED ────────────────────────────────────────────
    if !approved {
        // Kiểm tra nếu đang trong batch và có tool_index cụ thể
        if let Some(idx) = tool_index {
            // Clone pending_tools ra trước để tránh borrow của session.state
            let pending_tools = match &session.state {
                AgentState::AwaitingApproval(tools) => Some(tools.clone()),
                _ => None,
            };

            if let Some(pending_tools) = pending_tools {
                if idx < pending_tools.len() {
                    let rejected_tool = &pending_tools[idx];
                    // Ghi rejection observation vào history
                    session.history.push(ChatMessage {
                        id: format!("obs_{}", Local::now().timestamp_millis()),
                        role: "system".to_string(),
                        content: MessageContent::Text(
                            format!("<observation>User rejected tool '{}' (args: {}). Ask if they want to try a different approach.</observation>",
                                rejected_tool.name, rejected_tool.arguments)
                        ),
                        timestamp: Local::now().format("%H:%M").to_string(),
                    });

                    // Xóa tool đã reject khỏi danh sách
                    let mut remaining = pending_tools.clone();
                    remaining.remove(idx);

                    if !remaining.is_empty() {
                        // Vẫn còn tools pending, tiếp tục chờ duyệt
                        session.state = AgentState::AwaitingApproval(remaining.clone());
                        let mut session_guard = session_store.lock().unwrap();
                        *session_guard = Some(session);

                        let batch_tools: Vec<ToolInfo> = remaining
                            .iter()
                            .enumerate()
                            .map(|(i, t)| ToolInfo {
                                name: t.name.clone(),
                                args: t.arguments.to_string(),
                                pending_id: format!(
                                    "tool_{}_{}",
                                    Local::now().timestamp_millis(),
                                    i
                                ),
                            })
                            .collect();

                        rig_bridge::agent_log(
                            Some(&window),
                            &format!("[Từ chối công cụ] Đã từ chối tool trong batch, còn {} tool chờ duyệt", remaining.len()),
                        );
                        return Ok(AgentResponse {
                            session_id: format!("pending_{}", Local::now().timestamp()),
                            reply_type: "tool_batch_request".to_string(),
                            text: None,
                            tool_name: None,
                            args: None,
                            tools: Some(batch_tools),
                            pending_id: None,
                            iteration: None,
                            total_iterations: None,
                            tool_result: None,
                            approved_tool_index: Some(idx),
                        });
                    }
                }
            }
        }

        // Fallback: từ chối toàn bộ (hoặc tool cuối cùng)
        session.history.push(ChatMessage {
            id: format!("obs_{}", Local::now().timestamp_millis()),
            role: "system".to_string(),
            content: MessageContent::Text(
                "<observation>User rejected executing this tool. Ask if they want to try a different approach.</observation>".to_string()
            ),
            timestamp: Local::now().format("%H:%M").to_string(),
        });

        // Chuyển về Thinking để AI phản hồi
        session.state = AgentState::Thinking;

        // Load model config và gọi tick tiếp
        let ai_cfg = load_custom_ai_config();
        let model_config = resolve_model_config(&ai_cfg, &model);
        let api_key = resolve_api_key(&model_config).unwrap_or_default();
        let api_standard = model_config
            .api_standard
            .unwrap_or_else(|| "gemini".to_string());
        let system_prompt = harness.assemble_system_prompt();
        let model_to_use = if !model.is_empty() {
            model.clone()
        } else if !ai_cfg.active_model.is_empty() {
            ai_cfg.active_model.clone()
        } else {
            "gemini-1.5-flash".to_string()
        };
        let api_url = model_config.api_url.clone();
        let temperature = model_config.temperature;
        let top_p = model_config.top_p;
        let saved_session_id = session.session_id.clone();

        // Một tick để phản hồi
        let window_clone = window.clone();
        let thinking_mode_clone = thinking_mode.clone();
        let llm_closure = move |sys_prompt: String, history: Vec<ChatMessage>| {
            let api_key = api_key.clone();
            let model = model_to_use.clone();
            let url = api_url.clone();
            let std = api_standard.clone();
            let w = window_clone.clone();
            let thinking_mode = thinking_mode_clone.clone();
            async move {
                rig_bridge::call_rig(
                    &std,
                    &api_key,
                    &model,
                    &url,
                    temperature,
                    top_p,
                    &sys_prompt,
                    &history,
                    thinking_mode.as_deref(),
                    Some(&w),
                )
                .await
            }
        };

        match run_tick(&mut harness, &mut session, &system_prompt, app_state.agent_cancel_flag.clone(), llm_closure).await {
            TickExecutionResult::Completed(tick_res) => match tick_res {
                TickResult::Finished { text, .. } => {
                    let mut session_guard = session_store.lock().unwrap();
                    *session_guard = Some(session);
                    rig_bridge::agent_log(
                        Some(&window),
                        "[Từ chối công cụ] Agent phản hồi sau khi công cụ bị từ chối.",
                    );
                    return Ok(AgentResponse {
                        session_id: saved_session_id,
                        reply_type: "loop_result".to_string(),
                        text: Some(text),
                        tool_name: None,
                        args: None,
                        tools: None,
                        pending_id: None,
                        iteration: None,
                        total_iterations: None,
                        tool_result: None,
                        approved_tool_index: None,
                    });
                }
                TickResult::WaitForApproval { tools, iteration } => {
                    let tool = tools.first().cloned();
                    let tool_name = tool.as_ref().map(|t| t.name.clone());
                    let args = tool.as_ref().map(|t| t.arguments.to_string());
                    let mut session_guard = session_store.lock().unwrap();
                    *session_guard = Some(session);
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!(
                            "[Từ chối công cụ] Agent lại yêu cầu công cụ mới: {:?}",
                            tool_name
                        ),
                    );
                    return Ok(AgentResponse {
                        session_id: format!("pending_{}", Local::now().timestamp()),
                        reply_type: "tool_request".to_string(),
                        text: None,
                        tool_name,
                        args,
                        tools: None,
                        pending_id: Some(format!("tool_{}", Local::now().timestamp_millis())),
                        iteration: Some(iteration),
                        total_iterations: Some(25),
                        tool_result: None,
                        approved_tool_index: None,
                    });
                }
                TickResult::Error { message, .. } => {
                    let mut session_guard = session_store.lock().unwrap();
                    *session_guard = Some(session);
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!("[Từ chối công cụ] Agent gặp lỗi: {}", message),
                    );
                    return Err(format!("Agent error after rejection: {}", message));
                }
                TickResult::Continue {
                    thought: _,
                    tool_name: _,
                    tool_result: _,
                    iteration,
                } => {
                    let mut session_guard = session_store.lock().unwrap();
                    *session_guard = Some(session);
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!(
                            "[Từ chối công cụ] Agent tiếp tục suy nghĩ (vòng {}).",
                            iteration
                        ),
                    );
                    return Ok(AgentResponse {
                        session_id: saved_session_id,
                        reply_type: "text".to_string(),
                        text: Some("Tool rejected. What would you like to do instead?".to_string()),
                        tool_name: None,
                        args: None,
                        tools: None,
                        pending_id: None,
                        iteration: None,
                        total_iterations: None,
                        tool_result: None,
                        approved_tool_index: None,
                    });
                }
            },
            TickExecutionResult::Cancelled => {
                let mut session_guard = session_store.lock().unwrap();
                *session_guard = Some(session);
                rig_bridge::agent_log(Some(&window), "[Dừng] Agent đã bị dừng bởi người dùng.");
                return Ok(AgentResponse {
                    session_id: saved_session_id,
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
                });
            }
            TickExecutionResult::Timeout => {
                let mut session_guard = session_store.lock().unwrap();
                *session_guard = Some(session);
                rig_bridge::agent_log(
                    Some(&window),
                    "[Lỗi Hết Giờ] Đã hết thời gian 90 giây chờ phản hồi từ LLM.",
                );
                return Err("Đã hết thời gian chờ phản hồi từ Agent (Timeout 90 giây).".to_string());
            }
        }
    }

    // ─── USER APPROVED ──────────────────────────────────────────────

    // Nếu có tool_index, chỉ chạy tool cụ thể đó, giữ lại các tool còn lại
    if let Some(idx) = tool_index {
        // Clone pending_tools ra trước để tránh borrow của session.state
        let pending_tools = match &session.state {
            AgentState::AwaitingApproval(tools) => Some(tools.clone()),
            _ => None,
        };

        if let Some(pending_tools) = pending_tools {
            if idx < pending_tools.len() {
                let approved_tool = pending_tools[idx].clone();
                let session_id = session.session_id.clone();

                // Emit executing activity
                let _ = window.emit(
                    "agent-activity",
                    serde_json::json!({
                        "status": "executing",
                    }),
                );

                // Chỉ thực thi tool được duyệt
                let tool_result = harness.execute_tool(&session_id, &approved_tool).await;
                let (result_text, success) = match tool_result {
                    Ok(r) => (r, true),
                    Err(e) => (e, false),
                };

                let call_id = approved_tool
                    .call_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                // Push kết quả vào history
                session.history.push(ChatMessage {
                    id: call_id.clone(),
                    role: "tool".to_string(),
                    content: MessageContent::ToolResult {
                        tool_call_id: call_id,
                        tool_name: approved_tool.name.clone(),
                        result: result_text.clone(),
                        success,
                    },
                    timestamp: Local::now().format("%H:%M").to_string(),
                });

                // Xóa tool đã chạy khỏi danh sách pending
                let mut remaining = pending_tools.clone();
                remaining.remove(idx);

                if !remaining.is_empty() {
                    // Vẫn còn tools pending, tiếp tục chờ duyệt
                    session.state = AgentState::AwaitingApproval(remaining.clone());
                    let mut session_guard = session_store.lock().unwrap();
                    *session_guard = Some(session);

                    let batch_tools: Vec<ToolInfo> = remaining
                        .iter()
                        .enumerate()
                        .map(|(i, t)| ToolInfo {
                            name: t.name.clone(),
                            args: t.arguments.to_string(),
                            pending_id: format!("tool_{}_{}", Local::now().timestamp_millis(), i),
                        })
                        .collect();

                    rig_bridge::agent_log(
                        Some(&window),
                        &format!(
                            "[Chấp thuận công cụ] Tool '{}' đã chạy xong, còn {} tool chờ duyệt",
                            approved_tool.name,
                            remaining.len()
                        ),
                    );
                    return Ok(AgentResponse {
                        session_id: format!("pending_{}", Local::now().timestamp()),
                        reply_type: "tool_batch_request".to_string(),
                        text: None,
                        tool_name: None,
                        args: None,
                        tools: Some(batch_tools),
                        pending_id: None,
                        iteration: None,
                        total_iterations: None,
                        tool_result: Some(result_text),
                        approved_tool_index: Some(idx),
                    });
                } else {
                    // Hết tools pending → chuyển về Thinking để tiếp tục loop
                    session.state = AgentState::Thinking;
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!("[Chấp thuận công cụ] Tool '{}' đã chạy xong, hết tools pending, tiếp tục loop", approved_tool.name),
                    );
                }
            }
        }
    } else {
        // Không có tool_index → approve all
        session.state = AgentState::ExecutingTool;
    }

    // Emit executing activity (nếu chưa emit)
    let _ = window.emit(
        "agent-activity",
        serde_json::json!({
            "status": "executing",
        }),
    );

    // Load model config
    let ai_cfg = load_custom_ai_config();
    let model_config = resolve_model_config(&ai_cfg, &model);
    let api_key = resolve_api_key(&model_config).unwrap_or_default();
    let api_standard = model_config
        .api_standard
        .unwrap_or_else(|| "gemini".to_string());
    let system_prompt = harness.assemble_system_prompt();
    let model_to_use = if !model.is_empty() {
        model.clone()
    } else if !ai_cfg.active_model.is_empty() {
        ai_cfg.active_model.clone()
    } else {
        "gemini-1.5-flash".to_string()
    };
    let api_url = model_config.api_url.clone();
    let temperature = model_config.temperature;
    let top_p = model_config.top_p;
    let saved_session_id = session.session_id.clone();
    let max_iterations = session.max_iterations;

    // ─── STATE MACHINE: Tick loop (tiếp tục từ ExecutingTool hoặc Thinking) ─────
    loop {
        // Kiểm tra ngắt trước mỗi bước
        if check_and_reset_cancel(&app_state.agent_cancel_flag) {
            let _ = window.emit(
                "agent-cancelled",
                serde_json::json!({
                    "reason": "user_cancelled",
                }),
            );
            let mut sg = session_store.lock().unwrap();
            *sg = Some(session);
            return Ok(AgentResponse {
                session_id: saved_session_id,
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
            });
        }
        let llm_closure = {
            let api_key = api_key.clone();
            let model = model_to_use.clone();
            let url = api_url.clone();
            let std = api_standard.clone();
            let window_clone = window.clone();
            let thinking_mode_clone = thinking_mode.clone();
            move |sys_prompt: String, history: Vec<ChatMessage>| {
                let api_key = api_key.clone();
                let model = model.clone();
                let url = url.clone();
                let std = std.clone();
                let w = window_clone.clone();
                let thinking_mode = thinking_mode_clone.clone();
                async move {
                    rig_bridge::call_rig(
                        &std,
                        &api_key,
                        &model,
                        &url,
                        temperature,
                        top_p,
                        &sys_prompt,
                        &history,
                        thinking_mode.as_deref(),
                        Some(&w),
                    )
                    .await
                }
            }
        };

        match run_tick(&mut harness, &mut session, &system_prompt, app_state.agent_cancel_flag.clone(), llm_closure).await {
            TickExecutionResult::Completed(tick_res) => match tick_res {
                TickResult::Continue {
                    thought,
                    tool_name,
                    tool_result,
                    iteration,
                } => {
                    let _ = window.emit(
                        "agent-iteration",
                        serde_json::json!({
                            "iteration": iteration,
                            "thought": thought.clone(),
                            "tool_name": tool_name.clone(),
                            "tool_result": tool_result.clone(),
                        }),
                    );
                    let mut log_msgs = Vec::new();
                    if let Some(ref t) = thought {
                        log_msgs.push(format!("[Suy nghĩ - Vòng {}] {}", iteration, t));
                    }
                    if let Some(ref tn) = tool_name {
                        log_msgs.push(format!("[Gọi Công cụ] Đang thực thi công cụ: {}", tn));
                    }
                    if let Some(ref tr) = tool_result {
                        let short_res = if tr.len() > 1000 {
                            format!("{}... (cắt bớt)", &tr[..1000])
                        } else {
                            tr.clone()
                        };
                        log_msgs.push(format!("[Kết quả Công cụ] Phản hồi: {}", short_res));
                    }
                    for msg in log_msgs {
                        rig_bridge::agent_log(Some(&window), &msg);
                    }
                    continue;
                }
                TickResult::WaitForApproval { tools, iteration } => {
                    let mut sg = session_store.lock().unwrap();
                    *sg = Some(session);

                    if tools.len() > 1 {
                        let batch_tools: Vec<ToolInfo> = tools
                            .iter()
                            .enumerate()
                            .map(|(i, t)| ToolInfo {
                                name: t.name.clone(),
                                args: t.arguments.to_string(),
                                pending_id: format!(
                                    "tool_{}_{}",
                                    Local::now().timestamp_millis(),
                                    i
                                ),
                            })
                            .collect();
                        rig_bridge::agent_log(
                            Some(&window),
                            &format!(
                                "[Chờ phê duyệt] Batch {} công cụ: {:?}",
                                batch_tools.len(),
                                batch_tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                            ),
                        );
                        return Ok(AgentResponse {
                            session_id: format!("pending_{}", Local::now().timestamp()),
                            reply_type: "tool_batch_request".to_string(),
                            text: None,
                            tool_name: None,
                            args: None,
                            tools: Some(batch_tools),
                            pending_id: None,
                            iteration: Some(iteration),
                            total_iterations: Some(max_iterations),
                            tool_result: None,
                            approved_tool_index: None,
                        });
                    } else {
                        let tool = tools.first().cloned();
                        let tool_name = tool.as_ref().map(|t| t.name.clone());
                        let args = tool.as_ref().map(|t| t.arguments.to_string());
                        rig_bridge::agent_log(
                            Some(&window),
                            &format!(
                                "[Chờ phê duyệt] Yêu cầu cấp quyền chạy công cụ: {:?}",
                                tool_name
                            ),
                        );
                        return Ok(AgentResponse {
                            session_id: format!("pending_{}", Local::now().timestamp()),
                            reply_type: "tool_request".to_string(),
                            text: None,
                            tool_name,
                            args,
                            tools: None,
                            pending_id: Some(format!("tool_{}", Local::now().timestamp_millis())),
                            iteration: Some(iteration),
                            total_iterations: Some(max_iterations),
                            tool_result: None,
                            approved_tool_index: None,
                        });
                    }
                }
                TickResult::Finished {
                    text,
                    total_iterations,
                } => {
                    let _ = window.emit(
                        "agent-activity",
                        serde_json::json!({
                            "status": "idle",
                        }),
                    );
                    let mut sg = session_store.lock().unwrap();
                    *sg = Some(session);
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!(
                            "[Hoàn thành] Agent kết thúc vòng lặp sau {} bước.",
                            total_iterations
                        ),
                    );
                    return Ok(AgentResponse {
                        session_id: saved_session_id,
                        reply_type: "loop_result".to_string(),
                        text: Some(text),
                        tool_name: None,
                        args: None,
                        tools: None,
                        pending_id: None,
                        iteration: Some(total_iterations),
                        total_iterations: Some(max_iterations),
                        tool_result: None,
                        approved_tool_index: None,
                    });
                }
                TickResult::Error { message, iteration } => {
                    let _ = window.emit(
                        "agent-activity",
                        serde_json::json!({
                            "status": "error",
                        }),
                    );
                    rig_bridge::agent_log(
                        Some(&window),
                        &format!("[Lỗi Agent] Lỗi ở vòng {}: {}", iteration, message),
                    );
                    return Err(format!("Agent error (iter {}): {}", iteration, message));
                }
            },
            TickExecutionResult::Cancelled => {
                let _ = window.emit(
                    "agent-cancelled",
                    serde_json::json!({
                        "reason": "user_cancelled",
                    }),
                );
                let mut sg = session_store.lock().unwrap();
                *sg = Some(session);
                rig_bridge::agent_log(Some(&window), "[Dừng] Agent đã bị dừng bởi người dùng.");
                return Ok(AgentResponse {
                    session_id: saved_session_id,
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
                });
            }
            TickExecutionResult::Timeout => {
                let _ = window.emit(
                    "agent-activity",
                    serde_json::json!({
                        "status": "error",
                    }),
                );
                let mut sg = session_store.lock().unwrap();
                *sg = Some(session);
                rig_bridge::agent_log(
                    Some(&window),
                    "[Lỗi Hết Giờ] Đã hết thời gian 90 giây chờ phản hồi từ LLM.",
                );
                return Err("Đã hết thời gian chờ phản hồi từ Agent (Timeout 90 giây).".to_string());
            }
        }
    }
}

// ─── Session Management ─────────────────────────────────────────────────

#[tauri::command]
pub async fn agent_reset_session(app_state: State<'_, AppState>) -> Result<String, String> {
    let mut session_guard = app_state.agent_session.lock().unwrap();
    *session_guard = None;

    let mut loop_guard = app_state.agent_loop_state.lock().unwrap();
    *loop_guard = None;

    Ok("✓ Session reset successfully.".to_string())
}

#[tauri::command]
pub fn get_custom_ai_config() -> Result<CustomAiConfig, String> {
    Ok(load_custom_ai_config())
}

fn load_dot_env() {
    let mut current = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    for _ in 0..10 {
        let check_path = current.join(".env");
        if check_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&check_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, val)) = line.split_once('=') {
                        std::env::set_var(key.trim(), val.trim());
                    }
                }
            }
            break;
        }
        if !current.pop() {
            break;
        }
    }
}

fn get_encryption_key() -> Vec<u8> {
    load_dot_env();
    if let Ok(key) = std::env::var("ALOUETTE_ENCRYPTION_KEY") {
        key.into_bytes()
    } else {
        b"AlouetteSecretKey2026".to_vec()
    }
}

fn encrypt_key(plain: &str) -> String {
    if plain.is_empty() || plain == "none" {
        return plain.to_string();
    }
    let encryption_key = get_encryption_key();
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let mut xored = Vec::with_capacity(plain.len());
    for (i, byte) in plain.as_bytes().iter().enumerate() {
        xored.push(byte ^ encryption_key[i % encryption_key.len()]);
    }
    format!("enc:{}", STANDARD.encode(&xored))
}

fn decrypt_key(encrypted: &str) -> String {
    if !encrypted.starts_with("enc:") {
        return encrypted.to_string();
    }
    let encryption_key = get_encryption_key();
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let data_str = &encrypted[4..];
    if let Ok(decoded) = STANDARD.decode(data_str) {
        let mut plain = Vec::with_capacity(decoded.len());
        for (i, byte) in decoded.iter().enumerate() {
            plain.push(byte ^ encryption_key[i % encryption_key.len()]);
        }
        if let Ok(s) = String::from_utf8(plain) {
            return s;
        }
    }
    encrypted.to_string()
}

#[tauri::command]
pub fn save_custom_ai_config(mut config: CustomAiConfig) -> Result<(), String> {
    // Encrypt all API keys before saving
    for provider_config in config.providers.values_mut() {
        provider_config.api_key = encrypt_key(&provider_config.api_key);
    }

    let mut current = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut config_path = None;

    for _ in 0..10 {
        let check_path = current.join("core_engine/app_data/ai_config.yml");
        if check_path.exists() {
            config_path = Some(check_path);
            break;
        }
        if !current.pop() {
            break;
        }
    }

    let resolved_path = config_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .join("core_engine/app_data/ai_config.yml")
    });

    if let Some(parent) = resolved_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let yaml_str = serde_yaml::to_string(&config)
        .map_err(|e| format!("Failed to serialize custom AI config: {}", e))?;

    std::fs::write(&resolved_path, yaml_str)
        .map_err(|e| format!("Failed to write custom AI config to file: {}", e))?;

    Ok(())
}

// ─── Helpers ────────────────────────────────────────────────────────────

fn resolve_model_config(ai_cfg: &CustomAiConfig, model_hint: &str) -> ModelConfig {
    let model_key = if !model_hint.is_empty()
        && model_hint != "autonomous"
        && model_hint != "write"
        && model_hint != "full"
    {
        model_hint
    } else {
        &ai_cfg.active_model
    };

    for (provider_name, provider_cfg) in &ai_cfg.providers {
        if let Some(detail) = provider_cfg.models.get(model_key) {
            return ModelConfig {
                provider: provider_name.clone(),
                api_key: provider_cfg.api_key.clone(),
                api_url: detail
                    .api_url
                    .clone()
                    .or(provider_cfg.api_url.clone())
                    .unwrap_or_default(),
                context_limit: detail.context_limit,
                supports_vision: detail.supports_vision,
                temperature: detail.temperature.unwrap_or(0.2),
                top_p: detail.top_p.unwrap_or(0.95),
                api_standard: detail.api_standard.clone(),
            };
        }
    }

    // Fallback default config
    ModelConfig {
        provider: "gemini".to_string(),
        api_key: String::new(),
        api_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        context_limit: 1048576,
        supports_vision: true,
        temperature: 0.2,
        top_p: 0.95,
        api_standard: Some("gemini".to_string()),
    }
}

fn resolve_api_key(model_config: &ModelConfig) -> Result<String, String> {
    // Prefer the stored (possibly encrypted) key
    let stored = model_config.api_key.trim();
    if !stored.is_empty() && stored != "none" {
        // Decrypt if needed (decrypt_key is a no-op for plain keys)
        let plain = decrypt_key(stored);
        if !plain.is_empty() && plain != "none" {
            return Ok(plain);
        }
    }
    // Provider-aware fallback to environment variables
    let env_var = match model_config.provider.as_str() {
        "claude" => "ANTHROPIC_API_KEY",
        "openai" | "gpt-chatgpt" => "OPENAI_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "qwen" => "DASHSCOPE_API_KEY",
        _ => "GEMINI_API_KEY",
    };
    std::env::var(env_var)
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .map_err(|_| format!(
            "API Key chưa được thiết lập cho provider '{}'. Vui lòng vào phần Setting để cấu hình API Key.",
            model_config.provider
        ))
}

fn load_custom_ai_config() -> CustomAiConfig {
    let mut current = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut config_path = None;

    for _ in 0..10 {
        let check_path = current.join("core_engine/app_data/ai_config.yml");
        if check_path.exists() {
            config_path = Some(check_path);
            break;
        }
        if !current.pop() {
            break;
        }
    }

    let resolved_path = config_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .join("core_engine/app_data/ai_config.yml")
    });

    if let Ok(content) = std::fs::read_to_string(&resolved_path) {
        if let Ok(mut config) = serde_yaml::from_str::<CustomAiConfig>(&content) {
            // Decrypt all API keys after loading
            for provider_config in config.providers.values_mut() {
                provider_config.api_key = decrypt_key(&provider_config.api_key);
            }
            return config;
        }
    }

    // Fallback default config
    let mut providers = std::collections::HashMap::new();

    // DeepSeek
    let mut deepseek_models = std::collections::HashMap::new();
    deepseek_models.insert(
        "deepseek-v4-pro".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.deepseek.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    deepseek_models.insert(
        "deepseek-v4-flash".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.deepseek.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    deepseek_models.insert(
        "deepseek-v4".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.deepseek.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    deepseek_models.insert(
        "deepseek-r1".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.deepseek.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    providers.insert(
        "deepseek".to_string(),
        ProviderConfig {
            api_key: "".to_string(),
            api_url: Some("https://api.deepseek.com/v1".to_string()),
            models: deepseek_models,
        },
    );

    // Claude
    let mut claude_models = std::collections::HashMap::new();
    claude_models.insert(
        "claude-opus-4.7".to_string(),
        ModelDetail {
            context_limit: 200000,
            supports_vision: true,
            api_standard: Some("claude".to_string()),
            api_url: Some("https://api.anthropic.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    claude_models.insert(
        "claude-sonnet-5".to_string(),
        ModelDetail {
            context_limit: 200000,
            supports_vision: true,
            api_standard: Some("claude".to_string()),
            api_url: Some("https://api.anthropic.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    providers.insert(
        "claude".to_string(),
        ProviderConfig {
            api_key: "".to_string(),
            api_url: Some("https://api.anthropic.com/v1".to_string()),
            models: claude_models,
        },
    );

    // GPT
    let mut gpt_models = std::collections::HashMap::new();
    gpt_models.insert(
        "gpt-5.5".to_string(),
        ModelDetail {
            context_limit: 200000,
            supports_vision: true,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.openai.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    gpt_models.insert(
        "o1-pro".to_string(),
        ModelDetail {
            context_limit: 200000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.openai.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    gpt_models.insert(
        "o3-mini".to_string(),
        ModelDetail {
            context_limit: 200000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.openai.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    gpt_models.insert(
        "gpt-4o".to_string(),
        ModelDetail {
            context_limit: 128000,
            supports_vision: true,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://api.openai.com/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    providers.insert(
        "gpt-chatgpt".to_string(),
        ProviderConfig {
            api_key: "".to_string(),
            api_url: Some("https://api.openai.com/v1".to_string()),
            models: gpt_models,
        },
    );

    // Gemini
    let mut gemini_models = std::collections::HashMap::new();
    gemini_models.insert(
        "gemini-3.5-flash".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: true,
            api_standard: Some("gemini".to_string()),
            api_url: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    gemini_models.insert(
        "gemini-3.1-pro".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: true,
            api_standard: Some("gemini".to_string()),
            api_url: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    gemini_models.insert(
        "gemini-1.5-flash".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: true,
            api_standard: Some("gemini".to_string()),
            api_url: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    gemini_models.insert(
        "gemini-1.5-pro".to_string(),
        ModelDetail {
            context_limit: 1000000,
            supports_vision: true,
            api_standard: Some("gemini".to_string()),
            api_url: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    providers.insert(
        "gemini".to_string(),
        ProviderConfig {
            api_key: "".to_string(),
            api_url: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            models: gemini_models,
        },
    );

    // Qwen
    let mut qwen_models = std::collections::HashMap::new();
    qwen_models.insert(
        "qwen-3.7-max".to_string(),
        ModelDetail {
            context_limit: 128000,
            supports_vision: false,
            api_standard: Some("openai".to_string()),
            api_url: Some("https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        },
    );
    providers.insert(
        "qwen".to_string(),
        ProviderConfig {
            api_key: "".to_string(),
            api_url: Some("https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()),
            models: qwen_models,
        },
    );

    CustomAiConfig {
        active_model: "gemini-3.5-flash".to_string(),
        providers,
    }
}

fn log_debug(msg: &str) {
    let log_file = std::env::current_dir()
        .unwrap_or_default()
        .join("logs/debug_agent.log");
    if let Some(parent) = log_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
        use std::io::Write;
        let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"), msg);
    }
}

// ─── Agent History DB Implementation ──────────────────────────────────
use rusqlite::{params, Connection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHistoryItem {
    pub session_id: String,
    pub title: String,
    pub created_at: i64,
    pub model: String,
    pub mode: String,
    pub active_cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedSession {
    pub session_id: String,
    pub title: String,
    pub model: String,
    pub mode: String,
    pub active_cwd: Option<String>,
    pub frontend_history: serde_json::Value,
}

fn resolve_history_db_path() -> std::path::PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    for _ in 0..10 {
        let check_path = current.join("core_engine/app_data");
        if check_path.exists() {
            return check_path.join("history_agen.sql");
        }
        if !current.pop() {
            break;
        }
    }
    std::env::current_dir()
        .unwrap_or_default()
        .join("core_engine/app_data/history_agen.sql")
}

fn init_history_db(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS history_agen (
            session_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            model TEXT NOT NULL,
            mode TEXT NOT NULL,
            active_cwd TEXT,
            backend_history TEXT NOT NULL,
            frontend_history TEXT NOT NULL
        );",
        [],
    )
    .map_err(|e| format!("Failed to create history_agen table: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn agent_get_history(_app_state: State<'_, AppState>) -> Result<Vec<AgentHistoryItem>, String> {
    tokio::task::spawn_blocking(move || {
        let db_path = resolve_history_db_path();
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open history database: {}", e))?;

        init_history_db(&conn)?;

        let mut stmt = conn
            .prepare("SELECT session_id, title, created_at, model, mode, active_cwd FROM history_agen ORDER BY created_at DESC;")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let history_iter = stmt
            .query_map([], |row| {
                Ok(AgentHistoryItem {
                    session_id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    model: row.get(3)?,
                    mode: row.get(4)?,
                    active_cwd: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query history: {}", e))?;

        let mut list = Vec::new();
        for item in history_iter {
            list.push(item.map_err(|e| e.to_string())?);
        }
        Ok(list)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn load_agent_session(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<LoadedSession, String> {
    let session_store = app_state.agent_session.clone();
    tokio::task::spawn_blocking(move || {
        let db_path = resolve_history_db_path();
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open history database: {}", e))?;

        init_history_db(&conn)?;

        let mut stmt = conn
            .prepare("SELECT title, model, mode, active_cwd, backend_history, frontend_history FROM history_agen WHERE session_id = ?1;")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut rows = stmt
            .query(params![session_id])
            .map_err(|e| format!("Failed to query session: {}", e))?;

        if let Some(row) = rows.next().map_err(|e| format!("Failed to fetch row: {}", e))? {
            let title: String = row.get(0).map_err(|e| e.to_string())?;
            let model: String = row.get(1).map_err(|e| e.to_string())?;
            let mode: String = row.get(2).map_err(|e| e.to_string())?;
            let active_cwd: Option<String> = row.get(3).map_err(|e| e.to_string())?;
            let backend_history_str: String = row.get(4).map_err(|e| e.to_string())?;
            let frontend_history_str: String = row.get(5).map_err(|e| e.to_string())?;

            let backend_history: Vec<ChatMessage> = serde_json::from_str(&backend_history_str)
                .map_err(|e| format!("Failed to parse backend history: {}", e))?;

            let frontend_history: serde_json::Value = serde_json::from_str(&frontend_history_str)
                .map_err(|e| format!("Failed to parse frontend history: {}", e))?;

            let mut guard = session_store.lock().unwrap();
            *guard = Some(AgentSession {
                session_id: session_id.clone(),
                history: backend_history,
                state: AgentState::Idle,
                iteration_count: 0,
                max_iterations: 25,
                mode: match mode.as_str() {
                    "autonomous" => HarnessMode::Autonomous,
                    _ => HarnessMode::Standard,
                },
                plan: None,
                autonomous_state: None,
            });

            Ok(LoadedSession {
                session_id,
                title,
                model,
                mode,
                active_cwd,
                frontend_history,
            })
        } else {
            Err("Session not found.".to_string())
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn save_agent_session(
    session_id: String,
    title: String,
    model: String,
    mode: String,
    active_cwd: Option<String>,
    frontend_history: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let session_store = app_state.agent_session.clone();
    tokio::task::spawn_blocking(move || {
        let db_path = resolve_history_db_path();
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open history database: {}", e))?;

        init_history_db(&conn)?;

        let backend_history_json = {
            let guard = session_store.lock().unwrap();
            if let Some(ref session) = *guard {
                if session.session_id == session_id {
                    serde_json::to_string(&session.history).unwrap_or_else(|_| "[]".to_string())
                } else {
                    "[]".to_string()
                }
            } else {
                "[]".to_string()
            }
        };

        let frontend_history_str = serde_json::to_string(&frontend_history)
            .map_err(|e| format!("Failed to serialize frontend history: {}", e))?;

        let created_at = chrono::Local::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO history_agen (session_id, title, created_at, model, mode, active_cwd, backend_history, frontend_history)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            params![
                session_id,
                title,
                created_at,
                model,
                mode,
                active_cwd,
                backend_history_json,
                frontend_history_str,
            ],
        )
        .map_err(|e| format!("Failed to save session to DB: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn agent_delete_session(
    session_id: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db_path = resolve_history_db_path();
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open history database: {}", e))?;

        init_history_db(&conn)?;

        conn.execute("DELETE FROM history_agen WHERE session_id = ?1;", params![session_id])
            .map_err(|e| format!("Failed to delete session: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

