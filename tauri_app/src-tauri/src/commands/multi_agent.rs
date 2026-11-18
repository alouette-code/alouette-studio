use crate::commands::agent::{
    load_custom_ai_config, resolve_api_key, resolve_model_config, AgentResponse, run_tick, TickExecutionResult,
};
use crate::commands::rig_bridge;
use core_engine::agent_harness::{AgentHarness, AgentSession, AgentState, ChatMessage, HarnessMode, MessageContent, TickResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{Emitter, WebviewWindow};
use chrono::Local;
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

fn user_msg(text: &str) -> ChatMessage {
    ChatMessage {
        id: format!("usr_{}", chrono::Local::now().timestamp_millis()),
        role: "user".to_string(),
        content: MessageContent::Text(text.to_string()),
        timestamp: chrono::Local::now().format("%H:%M").to_string(),
    }
}

async fn run_subagent_loop(
    role_id: &str,
    role_name: &str,
    avatar: &str,
    system_prompt: &str,
    task_prompt: &str,
    model: &str,
    api_key: &str,
    api_url: &str,
    api_standard: &str,
    temperature: f32,
    top_p: f32,
    thinking_mode: Option<String>,
    window: &WebviewWindow,
    active_cwd: &std::path::Path,
) -> Result<String, String> {
    let mut harness = AgentHarness::new(active_cwd.to_path_buf());
    
    let session_id = format!("{}_{}", role_id, Local::now().timestamp_millis());
    let mut session = AgentSession {
        session_id: session_id.clone(),
        history: vec![user_msg(task_prompt)],
        state: AgentState::Idle,
        iteration_count: 0,
        max_iterations: 15,
        mode: HarnessMode::Standard,
        plan: None,
        autonomous_state: None,
        token_budget: 1_000_000,
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let mut final_text = String::new();

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        let llm_closure = {
            let key = api_key.to_string();
            let mdl = model.to_string();
            let url = api_url.to_string();
            let std = api_standard.to_string();
            let w = window.clone();
            let tm = thinking_mode.clone();
            move |sys: String, hist: Vec<ChatMessage>| {
                let key = key.clone();
                let mdl = mdl.clone();
                let url = url.clone();
                let std = std.clone();
                let w = w.clone();
                let tm = tm.clone();
                async move {
                    rig_bridge::call_rig(&std, &key, &mdl, &url, temperature, top_p, &sys, &hist, tm.as_deref(), Some(&w)).await
                }
            }
        };

        let tick_res = run_tick(&mut harness, &mut session, system_prompt, cancel_flag.clone(), llm_closure).await;
        
        match tick_res {
            TickExecutionResult::Completed(TickResult::Continue { thought, tool_name, tool_result, iteration }) => {
                // Emit multi-agent-step to show activity
                if let Some(ref t) = thought {
                    let ev = MultiAgentStepEvent {
                        session_id: session_id.clone(),
                        step: iteration,
                        agent_role: role_id.to_string(),
                        agent_name: role_name.to_string(),
                        avatar: avatar.to_string(),
                        content: format!("Suy nghĩ: {}", t),
                    };
                    let _ = window.emit("multi-agent-step", &ev);
                }
                if let Some(ref tn) = tool_name {
                    let ev = MultiAgentStepEvent {
                        session_id: session_id.clone(),
                        step: iteration,
                        agent_role: role_id.to_string(),
                        agent_name: role_name.to_string(),
                        avatar: avatar.to_string(),
                        content: format!("Đang chạy tool: {}", tn),
                    };
                    let _ = window.emit("multi-agent-step", &ev);
                }
            }
            TickExecutionResult::Completed(TickResult::Finished { text, .. }) => {
                final_text = text;
                break;
            }
            TickExecutionResult::Completed(TickResult::WaitForApproval { .. }) => {
                // Auto-approve or reject in multi-agent? We should probably auto-approve for simplicity, or return error.
                // For MVP, if it hits wait for approval, we just abort or tell it it's approved.
                // But run_tick already handles auto_approve = true for some tools.
                // If it needs user approval, we just break.
                final_text = "Halted: Tool requires user approval in Multi-Agent mode.".to_string();
                break;
            }
            TickExecutionResult::Completed(TickResult::Error { message, .. }) => {
                return Err(message);
            }
            TickExecutionResult::Cancelled => {
                return Err("Cancelled".to_string());
            }
            TickExecutionResult::Timeout => {
                return Err("Timeout".to_string());
            }
        }
    }
    
    let ev = MultiAgentStepEvent {
        session_id: session_id.clone(),
        step: 999,
        agent_role: role_id.to_string(),
        agent_name: role_name.to_string(),
        avatar: avatar.to_string(),
        content: final_text.clone(),
    };
    let _ = window.emit("multi-agent-step", &ev);

    Ok(final_text)
}

pub async fn run_multi_agent_workflow(
    message: String,
    model: String,
    active_cwd: Option<String>,
    thinking_mode: Option<String>,
    vm_manager: Option<Arc<core_engine::vm_engine::VmManager>>,
    window: WebviewWindow,
    session_id: String,
) -> Result<AgentResponse, String> {
    let ai_cfg = load_custom_ai_config();
    let model_config = resolve_model_config(&ai_cfg, &model);
    let model_to_use = if !model.is_empty() {
        model.clone()
    } else if !ai_cfg.active_model.is_empty() {
        ai_cfg.active_model.clone()
    } else {
        "gemini-1.5-flash".to_string()
    };
    let api_key = resolve_api_key(&model_config)?;
    let std_name = model_config.api_standard.clone().unwrap_or_else(|| "openai".to_string());
    
    let cwd = active_cwd.map(std::path::PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap());

    // 1. Leader (Just text, no tools needed)
    let leader_system = "Bạn là Trưởng Team (Leader 👑) điều phối đội ngũ phát triển phần mềm.\nTiếp nhận yêu cầu và phát biểu ngắn gọn giao việc cho Planner.";
    let leader_res = rig_bridge::call_rig(
        &std_name, &api_key, &model_to_use, &model_config.api_url, model_config.temperature, model_config.top_p,
        leader_system, &[user_msg(&message)], thinking_mode.as_deref(), Some(&window)
    ).await?;
    let leader_text = leader_res.text.unwrap_or_default();
    
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

    // Define the LLM executor closure using Rig Bridge
    let key = api_key.to_string();
    let mdl = model_to_use.to_string();
    let url = model_config.api_url.to_string();
    let std = std_name.to_string();
    let w = window.clone();
    let tm = thinking_mode.clone();
    let temp = model_config.temperature;
    let top_p = model_config.top_p;

    let llm_executor = move |sys: String, hist: Vec<ChatMessage>| {
        let key = key.clone();
        let mdl = mdl.clone();
        let url = url.clone();
        let std = std.clone();
        let w = w.clone();
        let tm = tm.clone();
        async move {
            let res = rig_bridge::call_rig(&std, &key, &mdl, &url, temp, top_p, &sys, &hist, tm.as_deref(), Some(&w)).await?;
            Ok((res.text, res.tool_calls))
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
