use crate::commands::agent::{load_custom_ai_config, resolve_api_key, resolve_model_config, AgentResponse};
use crate::commands::code_rag::CodeRagState;
use crate::commands::opencode_bridge::{OpenCodeManager, ResolvedOpenCodeModel};
use crate::state::{AppState, SuperApprovalDecision};
use chrono::Local;
use core_engine::multi_agent::blackboard::memory_store::MemoryStore;
use core_engine::multi_agent::orchestration::safeguards::{
    clamp_super_agent_budget, extract_target_path, is_file_mutation_tool, SafeguardVerdict,
    WorkerLoopDetector, MAX_SUPER_AGENTS,
};
use dashmap::DashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State, WebviewWindow};
use tokio::sync::{mpsc, oneshot, RwLock, Semaphore};
use tokio::task::JoinSet;

// ─── Config & Payloads ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperMultiAgentConfig {
    #[serde(default = "default_execution_mode")]
    pub execution_mode: String, // "interactive" | "plan" | "autonomous" | "copilot"
    pub commander_model: Option<String>,
    pub worker_model: Option<String>,
    pub max_agents: Option<usize>,
    pub max_concurrency: Option<usize>,
    pub max_worker_iterations: Option<u32>,
}

fn default_execution_mode() -> String {
    "interactive".to_string()
}

impl Default for SuperMultiAgentConfig {
    fn default() -> Self {
        Self {
            execution_mode: default_execution_mode(),
            commander_model: None,
            worker_model: None,
            max_agents: Some(MAX_SUPER_AGENTS),
            max_concurrency: Some(6),
            max_worker_iterations: Some(15),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperAgentWorkerInfo {
    pub id: String,
    pub role: String,
    pub mode: String,
    pub status: String, // "queued" | "running" | "waiting_approval" | "done" | "halted" | "error" | "killed"
    pub iteration: u32,
    pub max_iterations: u32,
    pub current_action: String,
    pub files_touched: Vec<String>,
    pub error_count: u32,
    pub logs: Vec<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperSwarmSnapshot {
    pub session_id: String,
    pub phase: String, // "recon" | "running" | "supervising" | "finalizing" | "finished" | "aborted"
    pub execution_mode: String,
    pub commander_model: String,
    pub worker_model: String,
    pub wave: u32,
    pub total_spawned: usize,
    pub max_agents: usize,
    pub active_count: usize,
    pub supervisor_status: String,
    pub supervisor_alerts: Vec<String>,
    pub workers: Vec<SuperAgentWorkerInfo>,
    pub headline: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperAgentApprovalRequest {
    pub approval_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub agent_role: String,
    pub tool_name: String,
    pub args_preview: String,
    pub execution_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DispatchWorkerSpec {
    #[serde(default)]
    id: String,
    #[serde(default = "default_worker_role")]
    role: String,
    task: String,
    #[serde(default = "default_worker_mode")]
    mode: String, // "read_only" | "coder" | "verifier"
    #[serde(default)]
    scope_paths: Vec<String>,
    #[serde(alias = "max_iterations")]
    max_steps: Option<u32>,
}

fn default_worker_role() -> String {
    "Specialist Worker".to_string()
}

fn default_worker_mode() -> String {
    "read_only".to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct CommanderDirective {
    action: String, // "dispatch" | "finalize"
    #[serde(default)]
    wave_goal: Option<String>,
    #[serde(default)]
    agents: Vec<DispatchWorkerSpec>,
    #[serde(default)]
    final_report: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SupervisorDirective {
    action: String, // "continue" | "kill_worker" | "reassign_worker" | "abort_swarm"
    #[serde(default)]
    target_worker_id: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    new_task: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Clone)]
enum OpenCodeSessionEvent {
    ToolRunning {
        tool_name: String,
        input: Value,
    },
    ToolFinished {
        tool_name: String,
        success: bool,
        output: String,
    },
    PermissionAsked {
        req_id: String,
        permission: String,
        args_pretty: String,
        patterns: Vec<String>,
    },
    Idle {
        full_text: String,
    },
    Error(String),
}

#[derive(Debug, Clone)]
enum TelemetryKind {
    ToolStarted,
    ToolFinished,
    ToolError,
    SafeguardWarn,
    SafeguardHalt,
}

#[derive(Debug, Clone)]
struct WorkerTelemetry {
    agent_id: String,
    kind: TelemetryKind,
    detail: String,
}

struct WorkerOutcome {
    id: String,
    role: String,
    mode: String,
    status: String,
    summary: String,
    files_touched: Vec<String>,
}

// ─── Tauri Command: Approval Response ───────────────────────────────────

#[tauri::command]
pub async fn super_agent_respond_approval(
    approval_id: String,
    approved: bool,
    approve_all_session: Option<bool>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some((_, tx)) = app_state.super_agent_approvals.remove(&approval_id) {
        let _ = tx.send(SuperApprovalDecision {
            approved,
            approve_all_session: approve_all_session.unwrap_or(false),
        });
        Ok(())
    } else {
        Err(format!(
            "Yêu cầu phê duyệt '{}' không còn tồn tại hoặc đã được xử lý.",
            approval_id
        ))
    }
}

// ─── OpenCode SSE Multiplexer ───────────────────────────────────────────

#[derive(Default)]
struct PerSessionStreamState {
    assistant_msg_ids: HashSet<String>,
    part_types: HashMap<String, String>,
    text_parts_order: Vec<String>,
    text_parts_map: HashMap<String, String>,
    emitted_text_fallback: String,
    started_call_ids: HashSet<String>,
    completed_call_ids: HashSet<String>,
}

impl PerSessionStreamState {
    fn reset_turn(&mut self) {
        self.text_parts_order.clear();
        self.text_parts_map.clear();
        self.emitted_text_fallback.clear();
    }
}

async fn pump_opencode_swarm_sse(
    resp: reqwest::Response,
    routes: Arc<DashMap<String, mpsc::UnboundedSender<OpenCodeSessionEvent>>>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut stream = resp.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    let mut states: HashMap<String, PerSessionStreamState> = HashMap::new();

    while !stop_flag.load(Ordering::SeqCst) {
        let chunk_opt = stream.next().await;
        let bytes = match chunk_opt {
            Some(Ok(b)) => b,
            Some(Err(e)) => {
                let err_msg = format!("OpenCode SSE stream error: {}", e);
                for entry in routes.iter() {
                    let _ = entry.value().send(OpenCodeSessionEvent::Error(err_msg.clone()));
                }
                break;
            }
            None => break,
        };

        buffer.extend_from_slice(&bytes);

        while let Some(nl_idx) = buffer.iter().position(|&b| b == b'\n') {
            if stop_flag.load(Ordering::SeqCst) {
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
                    if let Some(sid) = info["sessionID"].as_str() {
                        if routes.contains_key(sid) {
                            let role = info["role"].as_str().unwrap_or("");
                            if role == "assistant" {
                                if let Some(mid) = info["id"].as_str() {
                                    states
                                        .entry(sid.to_string())
                                        .or_default()
                                        .assistant_msg_ids
                                        .insert(mid.to_string());
                                }
                            }
                        }
                    }
                }
                "message.part.updated" => {
                    let part = &props["part"];
                    let Some(sid) = part["sessionID"]
                        .as_str()
                        .or_else(|| props["sessionID"].as_str())
                    else {
                        continue;
                    };
                    let Some(tx) = routes.get(sid).map(|r| r.value().clone()) else {
                        continue;
                    };

                    let st = states.entry(sid.to_string()).or_default();
                    let part_id = part["id"].as_str().unwrap_or("").to_string();
                    let msg_id = part["messageID"].as_str().unwrap_or("");
                    let part_type = part["type"].as_str().unwrap_or("").to_string();

                    if !part_id.is_empty() && !part_type.is_empty() {
                        st.part_types.insert(part_id.clone(), part_type.clone());
                    }

                    match part_type.as_str() {
                        "text" => {
                            if st.assistant_msg_ids.contains(msg_id) {
                                if let Some(txt) = part["text"].as_str() {
                                    if !st.text_parts_map.contains_key(&part_id) {
                                        st.text_parts_order.push(part_id.clone());
                                    }
                                    st.text_parts_map.insert(part_id, txt.to_string());
                                }
                            }
                        }
                        "tool" => {
                            let call_id = part["callID"].as_str().unwrap_or(&part_id).to_string();
                            let tool_name = part["tool"].as_str().unwrap_or("tool").to_string();
                            let state = &part["state"];
                            let status = state["status"].as_str().unwrap_or("");

                            if status == "running" && !st.started_call_ids.contains(&call_id) {
                                st.started_call_ids.insert(call_id.clone());
                                let _ = tx.send(OpenCodeSessionEvent::ToolRunning {
                                    tool_name,
                                    input: state["input"].clone(),
                                });
                            } else if (status == "completed" || status == "error")
                                && !st.completed_call_ids.contains(&call_id)
                            {
                                st.completed_call_ids.insert(call_id);
                                let success = status == "completed";
                                let output = if success {
                                    state["output"].as_str().unwrap_or("").to_string()
                                } else {
                                    state["error"].as_str().unwrap_or("Tool error").to_string()
                                };
                                let _ = tx.send(OpenCodeSessionEvent::ToolFinished {
                                    tool_name,
                                    success,
                                    output,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                "message.part.delta" => {
                    let Some(sid) = props["sessionID"].as_str() else {
                        continue;
                    };
                    if !routes.contains_key(sid) {
                        continue;
                    }
                    let st = states.entry(sid.to_string()).or_default();
                    let msg_id = props["messageID"].as_str().unwrap_or("");
                    if !st.assistant_msg_ids.is_empty() && !st.assistant_msg_ids.contains(msg_id) {
                        continue;
                    }
                    let part_id = props["partID"].as_str().unwrap_or("");
                    let field = props["field"].as_str().unwrap_or("");
                    let delta = props["delta"].as_str().unwrap_or("");
                    if field != "text" || delta.is_empty() {
                        continue;
                    }
                    let p_type = st.part_types.get(part_id).map(|s| s.as_str()).unwrap_or("text");
                    if p_type != "reasoning" {
                        st.emitted_text_fallback.push_str(delta);
                    }
                }
                "permission.asked" => {
                    let Some(sid) = props["sessionID"].as_str() else {
                        continue;
                    };
                    let Some(tx) = routes.get(sid).map(|r| r.value().clone()) else {
                        continue;
                    };
                    let req_id = props["id"].as_str().unwrap_or("").to_string();
                    let permission = props["permission"].as_str().unwrap_or("tool").to_string();
                    let patterns: Vec<String> = props["patterns"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let args_pretty = serde_json::to_string_pretty(&json!({
                        "patterns": props["patterns"],
                        "metadata": props["metadata"],
                    }))
                    .unwrap_or_else(|_| "{}".to_string());

                    let _ = tx.send(OpenCodeSessionEvent::PermissionAsked {
                        req_id,
                        permission,
                        args_pretty,
                        patterns,
                    });
                }
                "session.idle" => {
                    let Some(sid) = props["sessionID"].as_str() else {
                        continue;
                    };
                    let Some(tx) = routes.get(sid).map(|r| r.value().clone()) else {
                        continue;
                    };
                    let st = states.entry(sid.to_string()).or_default();
                    let ordered_text: String = st
                        .text_parts_order
                        .iter()
                        .filter_map(|id| st.text_parts_map.get(id))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let full_text = if !ordered_text.trim().is_empty() {
                        ordered_text
                    } else {
                        st.emitted_text_fallback.clone()
                    };
                    st.reset_turn();
                    let _ = tx.send(OpenCodeSessionEvent::Idle { full_text });
                }
                "session.error" => {
                    let Some(sid) = props["sessionID"].as_str() else {
                        continue;
                    };
                    let Some(tx) = routes.get(sid).map(|r| r.value().clone()) else {
                        continue;
                    };
                    let err_msg = props["error"]["data"]["message"]
                        .as_str()
                        .or_else(|| props["error"]["message"].as_str())
                        .or_else(|| props["error"]["name"].as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| props["error"].to_string());
                    let _ = tx.send(OpenCodeSessionEvent::Error(format!("OpenCode Error: {}", err_msg)));
                }
                _ => {}
            }
        }
    }
}

// ─── Shared Live Swarm State ────────────────────────────────────────────

struct SwarmStateTracker {
    snapshot: RwLock<SuperSwarmSnapshot>,
    window: WebviewWindow,
    last_emit_ms: AtomicUsize,
}

impl SwarmStateTracker {
    fn new(
        session_id: String,
        execution_mode: String,
        commander_model: String,
        worker_model: String,
        max_agents: usize,
        window: WebviewWindow,
    ) -> Self {
        let initial = SuperSwarmSnapshot {
            session_id,
            phase: "recon".to_string(),
            execution_mode,
            commander_model,
            worker_model,
            wave: 0,
            total_spawned: 0,
            max_agents,
            active_count: 0,
            supervisor_status: "OpenCode Commander đang khảo sát dự án...".to_string(),
            supervisor_alerts: Vec::new(),
            workers: Vec::new(),
            headline: "Commander (OpenCode) đang phân tích yêu cầu và cấu trúc dự án...".to_string(),
        };
        Self {
            snapshot: RwLock::new(initial),
            window,
            last_emit_ms: AtomicUsize::new(0),
        }
    }

    async fn emit_now(&self) {
        let snap = self.snapshot.read().await.clone();
        let _ = self.window.emit("super-agent-event", &snap);
        let now = Local::now().timestamp_millis().max(0) as usize;
        self.last_emit_ms.store(now, Ordering::Relaxed);
    }

    async fn emit_throttled(&self) {
        let now = Local::now().timestamp_millis().max(0) as usize;
        let prev = self.last_emit_ms.load(Ordering::Relaxed);
        if now.saturating_sub(prev) >= 120 {
            self.emit_now().await;
        }
    }

    async fn set_phase(&self, phase: &str, headline: &str) {
        {
            let mut s = self.snapshot.write().await;
            s.phase = phase.to_string();
            s.headline = headline.to_string();
        }
        self.emit_now().await;
    }

    async fn add_supervisor_alert(&self, status: &str, alert: Option<String>) {
        {
            let mut s = self.snapshot.write().await;
            s.supervisor_status = status.to_string();
            if let Some(msg) = alert {
                s.supervisor_alerts.push(msg);
                if s.supervisor_alerts.len() > 30 {
                    let overflow = s.supervisor_alerts.len() - 30;
                    s.supervisor_alerts.drain(0..overflow);
                }
            }
        }
        self.emit_now().await;
    }

    async fn register_worker(&self, info: SuperAgentWorkerInfo) {
        {
            let mut s = self.snapshot.write().await;
            s.total_spawned += 1;
            s.workers.push(info);
        }
        self.emit_now().await;
    }

    async fn update_worker<F>(&self, worker_id: &str, immediate: bool, mut f: F)
    where
        F: FnMut(&mut SuperAgentWorkerInfo),
    {
        {
            let mut s = self.snapshot.write().await;
            if let Some(w) = s.workers.iter_mut().find(|w| w.id == worker_id) {
                f(w);
            }
            s.active_count = s
                .workers
                .iter()
                .filter(|w| w.status == "running" || w.status == "waiting_approval")
                .count();
        }
        if immediate {
            self.emit_now().await;
        } else {
            self.emit_throttled().await;
        }
    }

    async fn compact_supervisor_digest(&self) -> String {
        let s = self.snapshot.read().await;
        let mut lines = vec![format!(
            "Wave {} | Spawned {}/{} | Active {}",
            s.wave, s.total_spawned, s.max_agents, s.active_count
        )];
        for w in &s.workers {
            let last_log = w.logs.last().map(|l| l.as_str()).unwrap_or("");
            lines.push(format!(
                "- [{} | {} | {}] step {}/{} | errs={} | files={:?} | action={} | last={}",
                w.id,
                w.role,
                w.status,
                w.iteration,
                w.max_iterations,
                w.error_count,
                w.files_touched,
                w.current_action,
                last_log
            ));
        }
        lines.join("\n")
    }
}

fn push_worker_log(w: &mut SuperAgentWorkerInfo, msg: String) {
    w.logs.push(msg);
    if w.logs.len() > 25 {
        let drain_n = w.logs.len() - 25;
        w.logs.drain(0..drain_n);
    }
}

fn format_model_label(m: &ResolvedOpenCodeModel) -> String {
    format!("{}/{}", m.provider_id, m.model_id)
}

fn opencode_agent_for_worker(worker_mode: &str, execution_mode: &str) -> &'static str {
    if execution_mode == "plan" || worker_mode == "read_only" {
        return "alouette_readonly";
    }
    match execution_mode {
        "autonomous" => "alouette_autonomous",
        "copilot" => "alouette_copilot",
        _ => "alouette_interactive",
    }
}

fn is_pattern_in_scope(cwd: &PathBuf, scope_paths: &[String], pattern: &str) -> bool {
    if scope_paths.is_empty() {
        return true;
    }
    let mut det = WorkerLoopDetector::new(cwd.clone(), scope_paths.to_vec());
    !matches!(
        det.check_before_tool("write", &json!({ "filePath": pattern })),
        SafeguardVerdict::Halt { .. }
    )
}

// ─── Worker Execution Task (OpenCode Session) ───────────────────────────

#[allow(clippy::too_many_arguments)]
async fn execute_opencode_worker_task(
    spec: DispatchWorkerSpec,
    execution_mode: String,
    session_id: String,
    directory: String,
    base_url: String,
    auth: String,
    opencode_manager: Arc<OpenCodeManager>,
    worker_model: ResolvedOpenCodeModel,
    routes: Arc<DashMap<String, mpsc::UnboundedSender<OpenCodeSessionEvent>>>,
    memory: MemoryStore,
    global_cancel: Arc<AtomicBool>,
    worker_cancel: Arc<AtomicBool>,
    session_auto_approve: Arc<AtomicBool>,
    approvals_map: Arc<DashMap<String, oneshot::Sender<SuperApprovalDecision>>>,
    semaphore: Arc<Semaphore>,
    tracker: Arc<SwarmStateTracker>,
    telemetry_tx: mpsc::UnboundedSender<WorkerTelemetry>,
) -> WorkerOutcome {
    let effective_mode = if execution_mode == "plan" {
        "read_only".to_string()
    } else {
        match spec.mode.as_str() {
            "coder" | "verifier" | "read_only" => spec.mode.clone(),
            _ => "read_only".to_string(),
        }
    };
    let oc_agent = opencode_agent_for_worker(&effective_mode, &execution_mode);
    let max_steps = spec.max_steps.unwrap_or(15).clamp(3, 30);

    // Wait on semaphore before starting OpenCode worker turn
    let permit = match semaphore.acquire().await {
        Ok(p) => p,
        Err(_) => {
            return WorkerOutcome {
                id: spec.id,
                role: spec.role,
                mode: effective_mode,
                status: "error".to_string(),
                summary: "Semaphore closed".to_string(),
                files_touched: Vec::new(),
            };
        }
    };

    if global_cancel.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) {
        drop(permit);
        tracker
            .update_worker(&spec.id, true, |w| {
                w.status = "killed".to_string();
                w.current_action = "Đã hủy trước khi chạy".to_string();
            })
            .await;
        return WorkerOutcome {
            id: spec.id,
            role: spec.role,
            mode: effective_mode,
            status: "killed".to_string(),
            summary: "Worker bị hủy trước khi khởi tạo OpenCode session.".to_string(),
            files_touched: Vec::new(),
        };
    }

    let worker_sid = match opencode_manager
        .create_isolated_session(&base_url, &auth, &directory)
        .await
    {
        Ok(sid) => sid,
        Err(e) => {
            drop(permit);
            tracker
                .update_worker(&spec.id, true, |w| {
                    w.status = "error".to_string();
                    w.current_action = "Lỗi tạo OpenCode session".to_string();
                    push_worker_log(w, e.clone());
                })
                .await;
            return WorkerOutcome {
                id: spec.id,
                role: spec.role,
                mode: effective_mode,
                status: "error".to_string(),
                summary: e,
                files_touched: Vec::new(),
            };
        }
    };

    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<OpenCodeSessionEvent>();
    routes.insert(worker_sid.clone(), ev_tx);

    let shared_keys = memory.keys().await;
    let shared_ctx = if shared_keys.is_empty() {
        String::new()
    } else {
        let mut buf = String::from("\n## Shared Blackboard Context\n");
        for k in shared_keys.into_iter().take(6) {
            if let Some(val) = memory.get(&k).await {
                let snippet: String = val.chars().take(600).collect();
                buf.push_str(&format!("- {}: {}\n", k, snippet));
            }
        }
        buf
    };

    let scope_note = if spec.scope_paths.is_empty() {
        "Chỉ chỉnh sửa các tệp thực sự liên quan đến nhiệm vụ của bạn.".to_string()
    } else {
        format!(
            "CHỈ ĐƯỢC PHÉP ghi/chỉnh sửa tệp nằm trong phạm vi: {:?}",
            spec.scope_paths
        )
    };

    let role_rule = match effective_mode.as_str() {
        "read_only" => "Bạn là worker READ-ONLY. Chỉ đọc/phân tích codebase, tuyệt đối không sửa file hay chạy lệnh thay đổi hệ thống.",
        "verifier" => "Bạn là worker VERIFIER. Hãy đọc code liên quan và chạy lệnh kiểm thử/build để xác minh.",
        _ => "Bạn là worker CODER. Hãy thực hiện chỉnh sửa code gọn gàng, chính xác trong phạm vi được giao.",
    };

    let system_prompt = format!(
        "Bạn là Worker '{}' (ID: '{}', OpenCode agent: '{}') trong đội ngũ Super Multi-Agent.\n\
         {}\n\
         - {}\n\
         - Giới hạn tối đa ~{} bước tool. Nếu gặp cùng một lỗi 2 lần, hãy dừng lại và báo cáo trung thực thay vì lặp lại vô ích.\n\
         - Khi xong, trả về tóm tắt ngắn gọn kết quả và danh sách tệp đã kiểm tra/sửa.{}",
        spec.role, spec.id, oc_agent, role_rule, scope_note, max_steps, shared_ctx
    );

    tracker
        .update_worker(&spec.id, true, |w| {
            w.status = "running".to_string();
            w.current_action = format!("OpenCode session {} đang chạy...", &worker_sid[..worker_sid.len().min(8)]);
            push_worker_log(w, format!("OpenCode [{}] | scope: {:?}", oc_agent, spec.scope_paths));
        })
        .await;

    if let Err(e) = opencode_manager
        .send_prompt_async_raw(
            &base_url,
            &auth,
            &directory,
            &worker_sid,
            &worker_model,
            oc_agent,
            Some(&system_prompt),
            &spec.task,
        )
        .await
    {
        routes.remove(&worker_sid);
        drop(permit);
        tracker
            .update_worker(&spec.id, true, |w| {
                w.status = "error".to_string();
                w.current_action = "Lỗi gửi prompt OpenCode".to_string();
                push_worker_log(w, e.clone());
            })
            .await;
        return WorkerOutcome {
            id: spec.id,
            role: spec.role,
            mode: effective_mode,
            status: "error".to_string(),
            summary: e,
            files_touched: Vec::new(),
        };
    }

    let cwd_path = PathBuf::from(&directory);
    let mut loop_detector = WorkerLoopDetector::new(cwd_path.clone(), spec.scope_paths.clone());
    let mut files_touched: Vec<String> = Vec::new();
    let mut step_count: u32 = 0;
    let final_status: String;
    let final_summary: String;

    loop {
        if global_cancel.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) {
            opencode_manager
                .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                .await;
            final_status = "killed".to_string();
            final_summary = "Worker OpenCode đã bị dừng theo tín hiệu hủy hoặc Supervisor.".to_string();
            break;
        }

        let ev_opt = tokio::select! {
            ev = ev_rx.recv() => ev,
            _ = async {
                loop {
                    if global_cancel.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
                }
            } => {
                opencode_manager
                    .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                    .await;
                final_status = "killed".to_string();
                final_summary = "Worker OpenCode đã bị dừng khẩn cấp.".to_string();
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                opencode_manager
                    .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                    .await;
                final_status = "halted".to_string();
                final_summary = "Timeout 300s trong phiên OpenCode.".to_string();
                break;
            }
        };

        let Some(ev) = ev_opt else {
            final_status = "error".to_string();
            final_summary = "Mất kết nối kênh SSE OpenCode.".to_string();
            break;
        };

        match ev {
            OpenCodeSessionEvent::ToolRunning {
                tool_name,
                input,
            } => {
                step_count += 1;
                if step_count > max_steps {
                    let reason = format!(
                        "Đạt giới hạn {} bước tool trong OpenCode (anti-loop).",
                        max_steps
                    );
                    let _ = telemetry_tx.send(WorkerTelemetry {
                        agent_id: spec.id.clone(),
                        kind: TelemetryKind::SafeguardHalt,
                        detail: reason.clone(),
                    });
                    opencode_manager
                        .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                        .await;
                    final_status = "halted".to_string();
                    final_summary = reason;
                    break;
                }

                match loop_detector.check_before_tool(&tool_name, &input) {
                    SafeguardVerdict::Allow => {}
                    SafeguardVerdict::Warn(wmsg) => {
                        let _ = telemetry_tx.send(WorkerTelemetry {
                            agent_id: spec.id.clone(),
                            kind: TelemetryKind::SafeguardWarn,
                            detail: wmsg.clone(),
                        });
                        tracker
                            .update_worker(&spec.id, true, |w| {
                                push_worker_log(w, format!("⚠️ {}", wmsg));
                            })
                            .await;
                    }
                    SafeguardVerdict::Halt { code, reason } => {
                        let full_reason = format!("[{}] {}", code, reason);
                        let _ = telemetry_tx.send(WorkerTelemetry {
                            agent_id: spec.id.clone(),
                            kind: TelemetryKind::SafeguardHalt,
                            detail: full_reason.clone(),
                        });
                        opencode_manager
                            .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                            .await;
                        final_status = "halted".to_string();
                        final_summary = full_reason;
                        break;
                    }
                }

                if is_file_mutation_tool(&tool_name) {
                    if let Some(p) = extract_target_path(&input) {
                        if !files_touched.contains(&p) {
                            files_touched.push(p);
                        }
                    }
                }

                let _ = telemetry_tx.send(WorkerTelemetry {
                    agent_id: spec.id.clone(),
                    kind: TelemetryKind::ToolStarted,
                    detail: tool_name.clone(),
                });

                let files_snap = files_touched.clone();
                tracker
                    .update_worker(&spec.id, false, |w| {
                        w.iteration = step_count;
                        w.current_action = format!("OpenCode tool: {}", tool_name);
                        w.files_touched = files_snap.clone();
                    })
                    .await;
            }
            OpenCodeSessionEvent::ToolFinished {
                tool_name,
                success,
                output,
            } => {
                if success {
                    loop_detector.record_outcome(true, &output);
                    let _ = telemetry_tx.send(WorkerTelemetry {
                        agent_id: spec.id.clone(),
                        kind: TelemetryKind::ToolFinished,
                        detail: tool_name.clone(),
                    });
                    tracker
                        .update_worker(&spec.id, false, |w| {
                            push_worker_log(w, format!("✓ {}", tool_name));
                        })
                        .await;
                } else {
                    let verdict = loop_detector.record_outcome(false, &output);
                    let _ = telemetry_tx.send(WorkerTelemetry {
                        agent_id: spec.id.clone(),
                        kind: TelemetryKind::ToolError,
                        detail: format!("{}: {}", tool_name, output),
                    });
                    tracker
                        .update_worker(&spec.id, true, |w| {
                            w.error_count += 1;
                            push_worker_log(w, format!("✗ {}: {}", tool_name, output));
                        })
                        .await;

                    match verdict {
                        SafeguardVerdict::Warn(wmsg) => {
                            let _ = telemetry_tx.send(WorkerTelemetry {
                                agent_id: spec.id.clone(),
                                kind: TelemetryKind::SafeguardWarn,
                                detail: wmsg,
                            });
                        }
                        SafeguardVerdict::Halt { code, reason } => {
                            let full_reason = format!("[{}] {}", code, reason);
                            let _ = telemetry_tx.send(WorkerTelemetry {
                                agent_id: spec.id.clone(),
                                kind: TelemetryKind::SafeguardHalt,
                                detail: full_reason.clone(),
                            });
                            opencode_manager
                                .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                                .await;
                            final_status = "halted".to_string();
                            final_summary = full_reason;
                            break;
                        }
                        SafeguardVerdict::Allow => {}
                    }
                }
            }
            OpenCodeSessionEvent::PermissionAsked {
                req_id,
                permission,
                args_pretty,
                patterns,
            } => {
                // Plan or read_only worker must never get edit/bash permission
                if execution_mode == "plan" || effective_mode == "read_only" {
                    opencode_manager
                        .reply_permission(&base_url, &auth, &directory, &req_id, "reject")
                        .await;
                    tracker
                        .update_worker(&spec.id, true, |w| {
                            push_worker_log(w, format!("🚫 Chặn quyền '{}' (chế độ {})", permission, effective_mode));
                        })
                        .await;
                    continue;
                }

                // Scope guard on edit permissions
                if permission == "edit"
                    && !patterns.is_empty()
                    && patterns
                        .iter()
                        .any(|pat| !is_pattern_in_scope(&cwd_path, &spec.scope_paths, pat))
                {
                    opencode_manager
                        .reply_permission(&base_url, &auth, &directory, &req_id, "reject")
                        .await;
                    let msg = format!(
                        "[OUT_OF_SCOPE_WRITE] Chặn quyền sửa ngoài scope {:?}",
                        patterns
                    );
                    let _ = telemetry_tx.send(WorkerTelemetry {
                        agent_id: spec.id.clone(),
                        kind: TelemetryKind::SafeguardHalt,
                        detail: msg.clone(),
                    });
                    opencode_manager
                        .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                        .await;
                    final_status = "halted".to_string();
                    final_summary = msg;
                    break;
                }

                if execution_mode == "autonomous" || session_auto_approve.load(Ordering::SeqCst) {
                    opencode_manager
                        .reply_permission(&base_url, &auth, &directory, &req_id, "always")
                        .await;
                    continue;
                }

                // Interactive / Copilot -> forward to UI Super Approval Queue
                let approval_id = format!(
                    "sup_oc_{}_{}",
                    spec.id,
                    Local::now().timestamp_nanos_opt().unwrap_or(0)
                );
                let (tx, rx) = oneshot::channel::<SuperApprovalDecision>();
                approvals_map.insert(approval_id.clone(), tx);

                tracker
                    .update_worker(&spec.id, true, |w| {
                        w.status = "waiting_approval".to_string();
                        w.current_action = format!("Chờ duyệt: {}", permission);
                        push_worker_log(w, format!("⏳ Chờ duyệt OpenCode: {}", permission));
                    })
                    .await;

                let _ = tracker.window.emit(
                    "super-agent-approval-request",
                    &SuperAgentApprovalRequest {
                        approval_id: approval_id.clone(),
                        session_id: session_id.clone(),
                        agent_id: spec.id.clone(),
                        agent_role: spec.role.clone(),
                        tool_name: permission.clone(),
                        args_preview: args_pretty,
                        execution_mode: execution_mode.clone(),
                    },
                );

                let decision = tokio::select! {
                    res = rx => res.unwrap_or(SuperApprovalDecision { approved: false, approve_all_session: false }),
                    _ = async {
                        loop {
                            if global_cancel.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) || session_auto_approve.load(Ordering::SeqCst) {
                                break;
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
                        }
                    } => {
                        approvals_map.remove(&approval_id);
                        if session_auto_approve.load(Ordering::SeqCst) {
                            SuperApprovalDecision { approved: true, approve_all_session: true }
                        } else {
                            SuperApprovalDecision { approved: false, approve_all_session: false }
                        }
                    }
                };

                if decision.approve_all_session {
                    session_auto_approve.store(true, Ordering::SeqCst);
                }

                if global_cancel.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) {
                    opencode_manager
                        .reply_permission(&base_url, &auth, &directory, &req_id, "reject")
                        .await;
                    opencode_manager
                        .abort_isolated_session(&base_url, &auth, &directory, &worker_sid)
                        .await;
                    final_status = "killed".to_string();
                    final_summary = "Đã hủy trong lúc chờ duyệt OpenCode.".to_string();
                    break;
                }

                let reply_str = if decision.approve_all_session {
                    "always"
                } else if decision.approved {
                    "once"
                } else {
                    "reject"
                };
                opencode_manager
                    .reply_permission(&base_url, &auth, &directory, &req_id, reply_str)
                    .await;

                tracker
                    .update_worker(&spec.id, true, |w| {
                        w.status = "running".to_string();
                        push_worker_log(
                            w,
                            if decision.approved {
                                format!("✅ Đã duyệt: {}", permission)
                            } else {
                                format!("❌ Từ chối: {}", permission)
                            },
                        );
                    })
                    .await;
            }
            OpenCodeSessionEvent::Idle { full_text } => {
                final_status = "done".to_string();
                final_summary = if full_text.trim().is_empty() {
                    "Hoàn thành tác vụ trên OpenCode.".to_string()
                } else {
                    full_text
                };
                break;
            }
            OpenCodeSessionEvent::Error(err) => {
                final_status = "error".to_string();
                final_summary = err;
                break;
            }
        }
    }

    routes.remove(&worker_sid);
    drop(permit);

    let summary_short = final_summary.clone();
    let final_st = final_status.clone();
    let files_final = files_touched.clone();
    tracker
        .update_worker(&spec.id, true, |w| {
            w.status = final_st.clone();
            w.current_action = match final_st.as_str() {
                "done" => "Hoàn thành".to_string(),
                "halted" => "Đã chặn vòng lặp / lỗi".to_string(),
                "killed" => "Đã dừng".to_string(),
                _ => "Lỗi".to_string(),
            };
            w.files_touched = files_final.clone();
            w.summary = Some(summary_short.clone());
            push_worker_log(w, format!("Kết thúc [{}]", final_st));
        })
        .await;

    WorkerOutcome {
        id: spec.id,
        role: spec.role,
        mode: effective_mode,
        status: final_status,
        summary: final_summary,
        files_touched,
    }
}

// ─── OpenCode Turn Helper for Commander & Supervisor ────────────────────

async fn run_readonly_opencode_turn(
    opencode_manager: &OpenCodeManager,
    base_url: &str,
    auth: &str,
    directory: &str,
    oc_session_id: &str,
    ev_rx: &mut mpsc::UnboundedReceiver<OpenCodeSessionEvent>,
    model: &ResolvedOpenCodeModel,
    system_prompt: Option<&str>,
    prompt: &str,
    cancel_flag: &AtomicBool,
) -> Result<String, String> {
    opencode_manager
        .send_prompt_async_raw(
            base_url,
            auth,
            directory,
            oc_session_id,
            model,
            "alouette_readonly",
            system_prompt,
            prompt,
        )
        .await?;

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            opencode_manager
                .abort_isolated_session(base_url, auth, directory, oc_session_id)
                .await;
            return Err("Đã hủy theo yêu cầu người dùng.".to_string());
        }

        let ev_opt = tokio::select! {
            ev = ev_rx.recv() => ev,
            _ = async {
                loop {
                    if cancel_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            } => {
                opencode_manager
                    .abort_isolated_session(base_url, auth, directory, oc_session_id)
                    .await;
                return Err("Đã hủy theo yêu cầu người dùng.".to_string());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(240)) => {
                opencode_manager
                    .abort_isolated_session(base_url, auth, directory, oc_session_id)
                    .await;
                return Err("OpenCode session timeout (240s).".to_string());
            }
        };

        let Some(ev) = ev_opt else {
            return Err("Mất kết nối SSE OpenCode.".to_string());
        };

        match ev {
            OpenCodeSessionEvent::PermissionAsked {
                req_id,
                permission,
                ..
            } => {
                // Read-only Commander/Supervisor never edits or runs bash; allow safe reads if asked
                let reply = if permission == "edit" || permission == "bash" {
                    "reject"
                } else {
                    "always"
                };
                opencode_manager
                    .reply_permission(base_url, auth, directory, &req_id, reply)
                    .await;
            }
            OpenCodeSessionEvent::Idle { full_text } => return Ok(full_text),
            OpenCodeSessionEvent::Error(e) => return Err(e),
            _ => {}
        }
    }
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end >= start {
        Some(raw[start..=end].to_string())
    } else {
        None
    }
}

async fn evaluate_with_opencode_supervisor(
    opencode_manager: &OpenCodeManager,
    base_url: &str,
    auth: &str,
    directory: &str,
    supervisor_sid: &str,
    supervisor_rx: &mut mpsc::UnboundedReceiver<OpenCodeSessionEvent>,
    commander_model: &ResolvedOpenCodeModel,
    trigger_reason: &str,
    swarm_digest: &str,
    cancel_flag: &AtomicBool,
) -> Result<SupervisorDirective, String> {
    let sys = "Bạn là Supervisor (Mô hình Lớn Giám Sát Liên Tục trên OpenCode).\n\
               Nhiệm vụ: theo dõi trạng thái các Worker OpenCode đang chạy song song để phát hiện SỚM vòng lặp vô hạn, lặp lỗi, sửa ngoài phạm vi, hoặc ảo giác.\n\
               Trả lời DUY NHẤT một JSON object hợp lệ (không cần chạy tool, không markdown) đúng cấu trúc:\n\
               {\"action\":\"continue|kill_worker|reassign_worker|abort_swarm\",\"target_worker_id\":null,\"reason\":\"...\",\"new_task\":null,\"note\":\"...\"}";

    let prompt = format!(
        "Lý do kiểm tra: {}\n\nTrạng thái Swarm:\n{}",
        trigger_reason, swarm_digest
    );

    let raw = run_readonly_opencode_turn(
        opencode_manager,
        base_url,
        auth,
        directory,
        supervisor_sid,
        supervisor_rx,
        commander_model,
        Some(sys),
        &prompt,
        cancel_flag,
    )
    .await?;

    let cleaned = extract_json_object(&raw).unwrap_or(raw);
    serde_json::from_str::<SupervisorDirective>(&cleaned)
        .map_err(|e| format!("Supervisor JSON parse error: {} | raw={}", e, cleaned))
}

// ─── Wave Runner with Continuous Supervisor ─────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_wave_with_supervisor(
    wave_num: u32,
    wave_goal: String,
    specs: Vec<DispatchWorkerSpec>,
    execution_mode: String,
    session_id: String,
    directory: String,
    base_url: String,
    auth: String,
    opencode_manager: Arc<OpenCodeManager>,
    commander_model: ResolvedOpenCodeModel,
    worker_model: ResolvedOpenCodeModel,
    supervisor_sid: String,
    supervisor_rx: &mut mpsc::UnboundedReceiver<OpenCodeSessionEvent>,
    routes: Arc<DashMap<String, mpsc::UnboundedSender<OpenCodeSessionEvent>>>,
    memory: MemoryStore,
    global_cancel: Arc<AtomicBool>,
    session_auto_approve: Arc<AtomicBool>,
    approvals_map: Arc<DashMap<String, oneshot::Sender<SuperApprovalDecision>>>,
    semaphore: Arc<Semaphore>,
    tracker: Arc<SwarmStateTracker>,
    spawned_counter: Arc<AtomicUsize>,
    max_agents: usize,
) -> Vec<WorkerOutcome> {
    {
        let mut s = tracker.snapshot.write().await;
        s.wave = wave_num;
        s.phase = "running".to_string();
        s.headline = format!("Đợt {}: {} ({} OpenCode agents)", wave_num, wave_goal, specs.len());
    }
    tracker.emit_now().await;

    let (telemetry_tx, mut telemetry_rx) = mpsc::unbounded_channel::<WorkerTelemetry>();
    let mut join_set: JoinSet<WorkerOutcome> = JoinSet::new();
    let mut worker_cancels: HashMap<String, Arc<AtomicBool>> = HashMap::new();
    let mut active_roles: HashMap<String, String> = HashMap::new();

    for spec in specs {
        let w_cancel = Arc::new(AtomicBool::new(false));
        worker_cancels.insert(spec.id.clone(), w_cancel.clone());
        active_roles.insert(spec.id.clone(), spec.role.clone());

        let max_steps = spec.max_steps.unwrap_or(15).clamp(3, 30);
        let eff_mode = if execution_mode == "plan" {
            "read_only".to_string()
        } else {
            spec.mode.clone()
        };

        tracker
            .register_worker(SuperAgentWorkerInfo {
                id: spec.id.clone(),
                role: spec.role.clone(),
                mode: eff_mode,
                status: "queued".to_string(),
                iteration: 0,
                max_iterations: max_steps,
                current_action: "Đang chờ slot OpenCode...".to_string(),
                files_touched: Vec::new(),
                error_count: 0,
                logs: Vec::new(),
                summary: None,
            })
            .await;

        let tx_c = telemetry_tx.clone();
        let ex_m = execution_mode.clone();
        let sess = session_id.clone();
        let dir_c = directory.clone();
        let bu_c = base_url.clone();
        let au_c = auth.clone();
        let ocm_c = opencode_manager.clone();
        let w_mdl = worker_model.clone();
        let rts_c = routes.clone();
        let mem_c = memory.clone();
        let gc = global_cancel.clone();
        let saa = session_auto_approve.clone();
        let am = approvals_map.clone();
        let sem = semaphore.clone();
        let trk = tracker.clone();

        join_set.spawn(async move {
            execute_opencode_worker_task(
                spec, ex_m, sess, dir_c, bu_c, au_c, ocm_c, w_mdl, rts_c, mem_c, gc, w_cancel,
                saa, am, sem, trk, tx_c,
            )
            .await
        });
    }

    let mut outcomes: Vec<WorkerOutcome> = Vec::new();
    let mut steps_since_supervision: u32 = 0;
    let mut last_supervision = tokio::time::Instant::now();
    let mut supervisor_tick = tokio::time::interval(tokio::time::Duration::from_secs(12));
    supervisor_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        if join_set.is_empty() {
            break;
        }

        let mut trigger_supervision: Option<String> = None;

        tokio::select! {
            Some(res) = join_set.join_next() => {
                if let Ok(outcome) = res {
                    outcomes.push(outcome);
                }
            }
            Some(tel) = telemetry_rx.recv() => {
                match tel.kind {
                    TelemetryKind::SafeguardHalt => {
                        trigger_supervision = Some(format!(
                            "Cảnh báo dừng khẩn cấp từ {}: {}",
                            tel.agent_id, tel.detail
                        ));
                    }
                    TelemetryKind::SafeguardWarn => {
                        trigger_supervision = Some(format!(
                            "Cảnh báo lặp/lỗi từ {}: {}",
                            tel.agent_id, tel.detail
                        ));
                    }
                    TelemetryKind::ToolError => {
                        steps_since_supervision += 2;
                        if steps_since_supervision >= 8 {
                            trigger_supervision = Some(format!(
                                "Tích lũy lỗi trong swarm (mới nhất {}: {})",
                                tel.agent_id, tel.detail
                            ));
                        }
                    }
                    TelemetryKind::ToolFinished | TelemetryKind::ToolStarted => {
                        steps_since_supervision += 1;
                        if steps_since_supervision >= 14 {
                            trigger_supervision = Some("Kiểm tra định kỳ mỗi 14 bước tool".to_string());
                        }
                    }
                }
            }
            _ = supervisor_tick.tick() => {
                if steps_since_supervision > 0 && last_supervision.elapsed() >= tokio::time::Duration::from_secs(10) {
                    trigger_supervision = Some("Giám sát định kỳ theo chu kỳ 12s".to_string());
                }
            }
        }

        if let Some(reason) = trigger_supervision {
            if !join_set.is_empty() && !global_cancel.load(Ordering::SeqCst) {
                steps_since_supervision = 0;
                last_supervision = tokio::time::Instant::now();

                tracker
                    .add_supervisor_alert(
                        &format!("Supervisor ({}) đang kiểm tra...", format_model_label(&commander_model)),
                        None,
                    )
                    .await;

                let digest = tracker.compact_supervisor_digest().await;
                if let Ok(directive) = evaluate_with_opencode_supervisor(
                    &opencode_manager,
                    &base_url,
                    &auth,
                    &directory,
                    &supervisor_sid,
                    supervisor_rx,
                    &commander_model,
                    &reason,
                    &digest,
                    &global_cancel,
                )
                .await
                {
                    match directive.action.as_str() {
                        "kill_worker" => {
                            if let Some(ref wid) = directive.target_worker_id {
                                if let Some(wc) = worker_cancels.get(wid) {
                                    wc.store(true, Ordering::SeqCst);
                                }
                                let alert = format!(
                                    "🛑 Supervisor dừng '{}': {}",
                                    wid,
                                    directive.reason.as_deref().unwrap_or("phát hiện bất thường")
                                );
                                tracker
                                    .add_supervisor_alert("Đã can thiệp dừng worker", Some(alert))
                                    .await;
                            }
                        }
                        "reassign_worker" => {
                            if let Some(ref wid) = directive.target_worker_id {
                                if let Some(wc) = worker_cancels.get(wid) {
                                    wc.store(true, Ordering::SeqCst);
                                }
                                let cur = spawned_counter.load(Ordering::SeqCst);
                                if cur < max_agents {
                                    if let Some(new_task) = directive.new_task.clone() {
                                        let next_num =
                                            spawned_counter.fetch_add(1, Ordering::SeqCst) + 1;
                                        if next_num <= max_agents {
                                            let new_id = format!("{}_fix{}", wid, next_num);
                                            let role = active_roles
                                                .get(wid)
                                                .cloned()
                                                .unwrap_or_else(|| "Recovery Worker".to_string());
                                            let new_cancel = Arc::new(AtomicBool::new(false));
                                            worker_cancels.insert(new_id.clone(), new_cancel.clone());
                                            active_roles.insert(new_id.clone(), role.clone());

                                            let new_spec = DispatchWorkerSpec {
                                                id: new_id.clone(),
                                                role: format!("{} (Fix)", role),
                                                task: new_task,
                                                mode: if execution_mode == "plan" {
                                                    "read_only".to_string()
                                                } else {
                                                    "coder".to_string()
                                                },
                                                scope_paths: vec![],
                                                max_steps: Some(12),
                                            };

                                            tracker
                                                .register_worker(SuperAgentWorkerInfo {
                                                    id: new_spec.id.clone(),
                                                    role: new_spec.role.clone(),
                                                    mode: new_spec.mode.clone(),
                                                    status: "queued".to_string(),
                                                    iteration: 0,
                                                    max_iterations: 12,
                                                    current_action: "Supervisor điều phối thay thế..."
                                                        .to_string(),
                                                    files_touched: Vec::new(),
                                                    error_count: 0,
                                                    logs: Vec::new(),
                                                    summary: None,
                                                })
                                                .await;

                                            let alert = format!(
                                                "🔄 Supervisor thay '{}' bằng '{}': {}",
                                                wid,
                                                new_id,
                                                directive.reason.as_deref().unwrap_or("điều chỉnh hướng")
                                            );
                                            tracker
                                                .add_supervisor_alert("Đã thay thế worker", Some(alert))
                                                .await;

                                            let tx_c = telemetry_tx.clone();
                                            let ex_m = execution_mode.clone();
                                            let sess = session_id.clone();
                                            let dir_c = directory.clone();
                                            let bu_c = base_url.clone();
                                            let au_c = auth.clone();
                                            let ocm_c = opencode_manager.clone();
                                            let w_mdl = worker_model.clone();
                                            let rts_c = routes.clone();
                                            let mem_c = memory.clone();
                                            let gc = global_cancel.clone();
                                            let saa = session_auto_approve.clone();
                                            let am = approvals_map.clone();
                                            let sem = semaphore.clone();
                                            let trk = tracker.clone();

                                            join_set.spawn(async move {
                                                execute_opencode_worker_task(
                                                    new_spec, ex_m, sess, dir_c, bu_c, au_c, ocm_c,
                                                    w_mdl, rts_c, mem_c, gc, new_cancel, saa, am,
                                                    sem, trk, tx_c,
                                                )
                                                .await
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        "abort_swarm" => {
                            for wc in worker_cancels.values() {
                                wc.store(true, Ordering::SeqCst);
                            }
                            let alert = format!(
                                "🚨 Supervisor DỪNG KHẨN CẤP toàn bộ swarm: {}",
                                directive
                                    .reason
                                    .as_deref()
                                    .unwrap_or("nguy cơ lỗi dây chuyền")
                            );
                            tracker
                                .add_supervisor_alert("Đã dừng khẩn cấp swarm", Some(alert))
                                .await;
                        }
                        _ => {
                            let note = directive
                                .note
                                .unwrap_or_else(|| "Các agent OpenCode hoạt động ổn định".to_string());
                            tracker
                                .add_supervisor_alert(&format!("Ổn định: {}", note), None)
                                .await;
                        }
                    }
                } else {
                    tracker
                        .add_supervisor_alert("Giám sát tự động (circuit-breaker)", None)
                        .await;
                }
            }
        }
    }

    outcomes
}

// ─── Main Entrypoint (100% OpenCode) ────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn run_super_multi_agent_workflow(
    message: String,
    model: String,
    active_cwd: Option<String>,
    thinking_mode: Option<String>,
    super_config: Option<SuperMultiAgentConfig>,
    opencode_manager: Arc<OpenCodeManager>,
    code_rag: Arc<CodeRagState>,
    cancel_flag: Arc<AtomicBool>,
    approvals_map: Arc<DashMap<String, oneshot::Sender<SuperApprovalDecision>>>,
    window: WebviewWindow,
    session_id: String,
) -> Result<AgentResponse, String> {
    cancel_flag.store(false, Ordering::SeqCst);

    let cfg = super_config.unwrap_or_default();
    let max_agents = clamp_super_agent_budget(cfg.max_agents.unwrap_or(MAX_SUPER_AGENTS));
    let max_concurrency = cfg.max_concurrency.unwrap_or(6).clamp(1, 12);
    let execution_mode = match cfg.execution_mode.as_str() {
        "autonomous" | "plan" | "copilot" | "interactive" => cfg.execution_mode.clone(),
        _ => "interactive".to_string(),
    };

    let directory = active_cwd
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .to_string()
        });

    let ai_cfg = load_custom_ai_config();
    let commander_hint = cfg
        .commander_model
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(model.as_str());
    let worker_hint = cfg
        .worker_model
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(commander_hint);

    let commander_model = opencode_manager.resolve_model_for_swarm(&ai_cfg, commander_hint);
    let worker_model = opencode_manager.resolve_model_for_swarm(&ai_cfg, worker_hint);

    // Validate credentials for non-OpenCode providers unless pointing to local endpoint
    for (hint, resolved) in [
        (commander_hint, &commander_model),
        (worker_hint, &worker_model),
    ] {
        if resolved.provider_id != "opencode" {
            let mcfg = resolve_model_config(&ai_cfg, hint);
            if !OpenCodeManager::is_local_model_url(&mcfg.api_url) {
                let _ = resolve_api_key(&mcfg)?;
            }
        }
    }

    let (base_url, auth) = opencode_manager
        .ensure_ready(&ai_cfg, Some(&window))
        .await?;

    let tracker = Arc::new(SwarmStateTracker::new(
        session_id.clone(),
        execution_mode.clone(),
        format_model_label(&commander_model),
        format_model_label(&worker_model),
        max_agents,
        window.clone(),
    ));
    tracker.emit_now().await;

    // Connect ONE shared OpenCode SSE multiplexer BEFORE creating/running any sessions
    let sse_url = format!(
        "{}/event?directory={}",
        base_url,
        OpenCodeManager::encode_dir(&directory)
    );
    let sse_resp = opencode_manager
        .http_client()
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

    let routes: Arc<DashMap<String, mpsc::UnboundedSender<OpenCodeSessionEvent>>> =
        Arc::new(DashMap::new());
    let sse_stop = Arc::new(AtomicBool::new(false));
    let sse_handle = tokio::spawn(pump_opencode_swarm_sse(
        sse_resp,
        routes.clone(),
        sse_stop.clone(),
    ));

    // Create isolated OpenCode sessions for Commander and Supervisor
    let commander_sid = opencode_manager
        .create_isolated_session(&base_url, &auth, &directory)
        .await?;
    let supervisor_sid = opencode_manager
        .create_isolated_session(&base_url, &auth, &directory)
        .await?;

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<OpenCodeSessionEvent>();
    routes.insert(commander_sid.clone(), cmd_tx);

    let (sup_tx, mut sup_rx) = mpsc::unbounded_channel::<OpenCodeSessionEvent>();
    routes.insert(supervisor_sid.clone(), sup_tx);

    let memory = MemoryStore::new();
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let session_auto_approve = Arc::new(AtomicBool::new(execution_mode == "autonomous"));
    let spawned_counter = Arc::new(AtomicUsize::new(0));

    // Inject Code RAG project context for Commander (best-effort)
    let dir_rag = directory.clone();
    let msg_rag = message.clone();
    let rag_ctx = tokio::time::timeout(
        tokio::time::Duration::from_millis(1800),
        tokio::task::spawn_blocking(move || {
            crate::commands::code_rag::build_cursor_project_context(&code_rag, &dir_rag, &msg_rag)
        }),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .map(|(ctx, _)| ctx)
    .unwrap_or_default();

    let think_note = if thinking_mode.as_deref() == Some("high") {
        "\nSuy nghĩ kỹ từng bước trước khi phân rã nhiệm vụ."
    } else {
        ""
    };

    let commander_sys = format!(
        "Bạn là Commander (Tổng Tư Lệnh) điều phối đội ngũ Super Multi-Agent chạy trên OpenCode.\n\
         - Tổng ngân sách worker tối đa toàn phiên: {} agents (cứng, tuyệt đối không vượt quá).\n\
         - Chế độ thực thi hiện tại: '{}'. {}\n\
         - Bạn đang ở chế độ READ-ONLY (`alouette_readonly`): hãy dùng các tool đọc/tìm kiếm của OpenCode để khảo sát codebase trước khi chia việc.\n\
         - KHÔNG tự sửa code. Khi đã sẵn sàng chia việc, cuối câu trả lời BẮT BUỘC phải kết thúc bằng DUY NHẤT một khối JSON hợp lệ theo 1 trong 2 mẫu:\n\
         1) Giao việc song song cho tối đa {} worker:\n\
         ```json\n\
         {{\"action\":\"dispatch\",\"wave_goal\":\"...\",\"agents\":[{{\"id\":\"w1\",\"role\":\"...\",\"task\":\"...\",\"mode\":\"read_only|coder|verifier\",\"scope_paths\":[\"relative/dir\"],\"max_steps\":15}}]}}\n\
         ```\n\
         (BẮT BUỘC đặt `scope_paths` tách biệt giữa các `coder` để tránh đụng file!)\n\
         2) Kết thúc toàn bộ sau khi mọi đợt đã xong và kiểm chứng:\n\
         ```json\n\
         {{\"action\":\"finalize\",\"final_report\":\"Báo cáo chi tiết kết quả...\"}}\n\
         ```{}\n{}",
        max_agents,
        execution_mode,
        if execution_mode == "plan" {
            "Chỉ cho phép phân tích đọc (read_only), không được sửa file."
        } else {
            "Có thể phân công read_only, coder và verifier."
        },
        max_agents,
        think_note,
        rag_ctx
    );

    let mut current_prompt = format!(
        "Yêu cầu từ người dùng:\n{}\n\nHãy khảo sát nhanh dự án bằng tool đọc nếu cần, rồi xuất JSON `dispatch` để kích hoạt các worker OpenCode song song.",
        message
    );

    let mut wave_num: u32 = 0;
    let mut final_report: Option<String> = None;
    let max_commander_waves: u32 = 8;

    for _round in 0..max_commander_waves {
        if cancel_flag.load(Ordering::SeqCst) {
            tracker
                .set_phase("aborted", "Đã hủy theo yêu cầu người dùng.")
                .await;
            sse_stop.store(true, Ordering::SeqCst);
            sse_handle.abort();
            return Err("Super Multi-Agent đã bị hủy bởi người dùng.".to_string());
        }

        let cmd_text = match run_readonly_opencode_turn(
            &opencode_manager,
            &base_url,
            &auth,
            &directory,
            &commander_sid,
            &mut cmd_rx,
            &commander_model,
            Some(&commander_sys),
            &current_prompt,
            &cancel_flag,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                tracker.set_phase("aborted", &e).await;
                sse_stop.store(true, Ordering::SeqCst);
                sse_handle.abort();
                return Err(e);
            }
        };

        let parsed_cmd: Option<CommanderDirective> = extract_json_object(&cmd_text)
            .and_then(|j| serde_json::from_str::<CommanderDirective>(&j).ok());

        let Some(directive) = parsed_cmd else {
            // Ask Commander once to emit valid JSON, or treat plain response as final report if slots exhausted
            if spawned_counter.load(Ordering::SeqCst) > 0 {
                final_report = Some(cmd_text);
                break;
            }
            current_prompt = "Phản hồi vừa rồi chưa có khối JSON hợp lệ. Hãy trả về đúng JSON `{\"action\":\"dispatch\", ...}` hoặc `{\"action\":\"finalize\",\"final_report\":\"...\"}`.".to_string();
            continue;
        };

        if directive.action == "finalize" {
            final_report = Some(
                directive
                    .final_report
                    .unwrap_or_else(|| cmd_text.clone()),
            );
            break;
        }

        let already_spawned = spawned_counter.load(Ordering::SeqCst);
        let remaining = max_agents.saturating_sub(already_spawned);
        if remaining == 0 {
            current_prompt = format!(
                "Đã sử dụng hết {}/{} worker agents. Hãy tổng hợp mọi kết quả và trả JSON `{{\"action\":\"finalize\",\"final_report\":\"...\"}}`.",
                max_agents, max_agents
            );
            continue;
        }

        let truncated = directive.agents.len() > remaining;
        let batch: Vec<DispatchWorkerSpec> = directive
            .agents
            .into_iter()
            .take(remaining)
            .enumerate()
            .map(|(idx, mut s)| {
                let n = spawned_counter.fetch_add(1, Ordering::SeqCst) + 1;
                if s.id.trim().is_empty() {
                    s.id = format!("agent_{}", n);
                } else {
                    s.id = format!("{}_w{}_{}", s.id.trim(), wave_num + 1, idx + 1);
                }
                s
            })
            .collect();

        if batch.is_empty() {
            current_prompt = "Danh sách `agents` rỗng. Nếu đã đủ thông tin hãy trả `{\"action\":\"finalize\",...}`.".to_string();
            continue;
        }

        wave_num += 1;
        let wave_goal = directive
            .wave_goal
            .unwrap_or_else(|| format!("Thực thi đợt {}", wave_num));

        let outcomes = run_wave_with_supervisor(
            wave_num,
            wave_goal,
            batch,
            execution_mode.clone(),
            session_id.clone(),
            directory.clone(),
            base_url.clone(),
            auth.clone(),
            opencode_manager.clone(),
            commander_model.clone(),
            worker_model.clone(),
            supervisor_sid.clone(),
            &mut sup_rx,
            routes.clone(),
            memory.clone(),
            cancel_flag.clone(),
            session_auto_approve.clone(),
            approvals_map.clone(),
            semaphore.clone(),
            tracker.clone(),
            spawned_counter.clone(),
            max_agents,
        )
        .await;

        let mut report_lines = Vec::new();
        for o in &outcomes {
            memory
                .set(
                    &format!("worker_{}", o.id),
                    format!("[{}] {} -> {}", o.status, o.role, o.summary),
                )
                .await;
            report_lines.push(format!(
                "- {} ({} | mode={} | status={}) | files={:?}\n  Summary: {}",
                o.id, o.role, o.mode, o.status, o.files_touched, o.summary
            ));
        }

        let left = max_agents.saturating_sub(spawned_counter.load(Ordering::SeqCst));
        current_prompt = format!(
            "Kết quả Đợt {} ({} OpenCode workers, còn {} / {} slot):\n{}{}\n\nHãy đánh giá: nếu cần sửa lỗi/verify tiếp hãy trả JSON `dispatch`; nếu đã hoàn tất hãy trả JSON `finalize` với `final_report`.",
            wave_num,
            outcomes.len(),
            left,
            max_agents,
            if truncated {
                "⚠️ Một số worker vượt giới hạn 30 đã bị cắt bớt.\n"
            } else {
                ""
            },
            report_lines.join("\n")
        );
    }

    sse_stop.store(true, Ordering::SeqCst);
    sse_handle.abort();

    let report_text = final_report.unwrap_or_else(|| {
        "Super Multi-Agent (OpenCode) đã hoàn tất các đợt điều phối.".to_string()
    });

    tracker
        .set_phase(
            "finished",
            &format!(
                "Hoàn tất Super Multi-Agent OpenCode ({}/{} agents đã dùng)",
                spawned_counter.load(Ordering::SeqCst),
                max_agents
            ),
        )
        .await;

    Ok(AgentResponse {
        session_id,
        reply_type: "loop_result".to_string(),
        text: Some(report_text),
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
