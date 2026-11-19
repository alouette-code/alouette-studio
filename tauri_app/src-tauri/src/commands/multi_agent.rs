use crate::commands::agent::{
    load_custom_ai_config, resolve_api_key, resolve_model_config, AgentResponse,
};
use crate::commands::opencode_bridge::OpenCodeManager;
use core_engine::agent_harness::parser::ToolCall;
use core_engine::agent_harness::{AgentHarness, ChatMessage, MessageContent};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, WebviewWindow};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentStepEvent {
    pub session_id: String,
    pub step: u32,
    pub agent_role: String,
    pub agent_name: String,
    pub avatar: String,
    pub content: String,
}

fn format_history_prompt(history: &[ChatMessage]) -> String {
    let mut out = String::new();
    for msg in history {
        let role = match msg.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => "System",
            other => other,
        };
        let text = match &msg.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::ToolCalls(tcs, text_opt) => {
                let calls_str = tcs
                    .iter()
                    .map(|tc| format!("[ToolCall: {}({})]", tc.name, tc.raw_arguments))
                    .collect::<Vec<_>>()
                    .join("\n");
                match text_opt {
                    Some(t) if !t.trim().is_empty() => format!("{}\n{}", t, calls_str),
                    _ => calls_str,
                }
            }
            MessageContent::ToolResult {
                tool_name,
                result,
                success,
                ..
            } => {
                format!("[ToolResult: {} (success={})]\n{}", tool_name, success, result)
            }
        };
        if !text.trim().is_empty() {
            out.push_str(&format!("[{}]: {}\n\n", role, text));
        }
    }
    out
}

fn parse_tool_calls_from_text(text: &str) -> Vec<ToolCall> {
    let parsed = core_engine::agent_harness::parser::parse_model_response(text);
    if !parsed.tool_calls.is_empty() {
        return parsed.tool_calls;
    }
    if let Some(tc) = parsed.tool_call {
        return vec![tc];
    }
    vec![]
}

pub async fn run_multi_agent_workflow(
    message: String,
    model: String,
    active_cwd: Option<String>,
    thinking_mode: Option<String>,
    vm_manager: Option<Arc<core_engine::vm_engine::VmManager>>,
    opencode_manager: Arc<OpenCodeManager>,
    cancel_flag: Arc<AtomicBool>,
    window: WebviewWindow,
    session_id: String,
) -> Result<AgentResponse, String> {
    cancel_flag.store(false, Ordering::SeqCst);

    let ai_cfg = load_custom_ai_config();
    let model_to_use = if !model.is_empty() {
        model.clone()
    } else if !ai_cfg.active_model.is_empty() {
        ai_cfg.active_model.clone()
    } else {
        "gemini-1.5-flash".to_string()
    };

    // Validate API key unless using OpenCode built-in/free model or local URL
    let resolved_oc = opencode_manager.resolve_model_for_swarm(&ai_cfg, &model_to_use);
    if resolved_oc.provider_id != "opencode" {
        let model_config = resolve_model_config(&ai_cfg, &model_to_use);
        if !OpenCodeManager::is_local_model_url(&model_config.api_url) {
            let _ = resolve_api_key(&model_config)?;
        }
    }

    let cwd = active_cwd
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
    let directory = cwd.to_string_lossy().to_string();

    // 1. Leader (via OpenCode)
    let mut leader_system = "Bạn là Trưởng Team (Leader 👑) điều phối đội ngũ phát triển phần mềm.\nTiếp nhận yêu cầu và phát biểu ngắn gọn giao việc cho Planner.".to_string();
    if thinking_mode.as_deref() == Some("high") {
        leader_system.push_str("\n\nThink step-by-step carefully before answering.");
    }
    let leader_text = opencode_manager
        .run_oneshot_completion(
            &ai_cfg,
            &model_to_use,
            &directory,
            &leader_system,
            &message,
            Some(&cancel_flag),
            Some(&window),
        )
        .await?;

    let _ = window.emit("multi-agent-step", &MultiAgentStepEvent {
        session_id: session_id.clone(),
        step: 1,
        agent_role: "leader".to_string(),
        agent_name: "Leader".to_string(),
        avatar: "👑".to_string(),
        content: leader_text.clone()
    });

    // 2. Initialize the core_engine's Coordinator V2
    let mut harness = AgentHarness::new(cwd.clone());
    if let Some(vm_mgr) = vm_manager {
        harness.set_vm_manager(vm_mgr);
    }
    let mut coordinator = core_engine::multi_agent::orchestration::coordinator::Coordinator::new(&mut harness);

    // Set auto-approvals options from defaults (reads approved, writes/executes require approval)
    coordinator.config.auto_approve_all = false;
    coordinator.config.auto_approve_reads = true;
    coordinator.config.auto_approve_writes = false;

    // Define the LLM executor closure using OpenCode
    let ocm = opencode_manager.clone();
    let cfg_clone = ai_cfg.clone();
    let mdl = model_to_use.clone();
    let dir = directory.clone();
    let cf = cancel_flag.clone();
    let w = window.clone();
    let tm = thinking_mode.clone();

    let llm_executor = move |sys: String, hist: Vec<ChatMessage>| {
        let ocm = ocm.clone();
        let cfg_clone = cfg_clone.clone();
        let mdl = mdl.clone();
        let dir = dir.clone();
        let cf = cf.clone();
        let w = w.clone();
        let tm = tm.clone();
        async move {
            let mut final_sys = sys;
            if tm.as_deref() == Some("high") {
                final_sys.push_str("\n\nThink step-by-step carefully before answering.");
            }
            let prompt = format_history_prompt(&hist);
            let raw_text = ocm
                .run_oneshot_completion(
                    &cfg_clone,
                    &mdl,
                    &dir,
                    &final_sys,
                    &prompt,
                    Some(&cf),
                    Some(&w),
                )
                .await?;
            let tool_calls = parse_tool_calls_from_text(&raw_text);
            let plain_text = core_engine::agent_harness::parser::parse_model_response(&raw_text).plain_text;
            Ok((plain_text, tool_calls))
        }
    };

    // Define the progress handler closure to stream updates back to the Tauri UI
    let w_progress = window.clone();
    let sess_id = session_id.clone();
    let progress_handler = move |role_id: &str, role_name: &str, avatar: &str, step: u32, content: &str| {
        let event = MultiAgentStepEvent {
            session_id: sess_id.clone(),
            step,
            agent_role: role_id.to_string(),
            agent_name: role_name.to_string(),
            avatar: avatar.to_string(),
            content: content.to_string(),
        };
        let _ = w_progress.emit("multi-agent-step", &event);
    };

    // Run the workflow
    match coordinator.run_workflow(&message, llm_executor, progress_handler).await {
        Ok(final_text) => {
            Ok(AgentResponse {
                session_id,
                reply_type: "loop_result".to_string(),
                text: Some(final_text),
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
        Err(e) => {
            Err(format!("Lỗi Multi-Agent V2: {}", e))
        }
    }
}
