use crate::commands::agent::{
    load_custom_ai_config, resolve_api_key, resolve_model_config, AgentResponse, LoopResult,
    LoopResultIteration,
};
use crate::commands::rig_bridge;
use core_engine::agent_harness::{ChatMessage, MessageContent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{Emitter, WebviewWindow};

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

/// Execute Multi-Agent Team Collaboration Workflow
pub async fn run_multi_agent_workflow(
    message: String,
    model: String,
    _active_cwd: Option<String>,
    thinking_mode: Option<String>,
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
    let thinking_ref = thinking_mode.as_deref();

    let mut iterations = Vec::new();
    let mut combined_history_text = String::new();

    let emit_step = |step: u32, role: &str, name: &str, avatar: &str, content: &str| {
        let event = MultiAgentStepEvent {
            session_id: session_id.clone(),
            step,
            agent_role: role.to_string(),
            agent_name: name.to_string(),
            avatar: avatar.to_string(),
            content: content.to_string(),
        };
        let _ = window.emit("multi-agent-step", &event);
        let _ = window.emit(
            "agent-console-log",
            json!({
                "message": format!("[MultiAgent] {}: {}", name, content.lines().next().unwrap_or("")),
                "timestamp": chrono::Local::now().to_rfc3339(),
            }),
        );
    };

    // ─── STEP 1: Leader Reception ───────────────────────────────────────────
    let leader_system = "Bạn là Trưởng Team (Leader 👑) điều phối đội ngũ phát triển phần mềm Multi-Agent bao gồm: Leader, Planner (Lên Kế Hoạch), Plan Reviewer (Bắt Lỗi Kế Hoạch), Coder 1 (Developer Alpha), Coder 2 (Developer Beta).\n\
Nhiệm vụ của bạn ở bước này:\n\
- Tiếp nhận yêu cầu người dùng.\n\
- Chào mừng và nêu ngắn gọn mục tiêu làm việc của cả đội ngũ.\n\
- Yêu cầu Planner bắt đầu phân tích và đưa ra bản kế hoạch chi tiết.\n\
Hãy phát biểu ngắn gọn, truyền cảm hứng và rõ ràng (dưới 150 từ).";

    let leader_input = vec![user_msg(&format!("Yêu cầu người dùng: {}", message))];
    let leader_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        leader_system,
        &leader_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let leader_text = leader_res.text.clone().unwrap_or_default();
    emit_step(1, "leader", "Trưởng Team (Leader)", "👑", &leader_text);
    combined_history_text.push_str(&format!("\n👑 Leader: {}\n", leader_text));
    iterations.push(LoopResultIteration {
        iteration: 1,
        thought: None,
        tool_name: Some("Leader_Initiate".to_string()),
        tool_args: Some(json!({"role": "leader"}).to_string()),
        tool_result: Some(leader_text.clone()),
        tool_success: true,
        timestamp: chrono::Local::now().to_rfc3339(),
    });

    // ─── STEP 2: Planner Drafts Plan ─────────────────────────────────────────
    let planner_system = "Bạn là Kế Hoạch Viên (Software Architect / Planner 📋).\n\
Nhiệm vụ của bạn:\n\
- Dựa trên yêu cầu người dùng và lời mở đầu của Leader, hãy soạn thảo một bản kế hoạch kỹ thuật chi tiết (Step-by-step Technical Plan).\n\
- Liệt kê rõ các bước, cấu trúc dự án, các module chính và công nghệ áp dụng.\n\
Bản kế hoạch cần rõ ràng, khả thi để Plan Reviewer thẩm định.";

    let planner_input = vec![user_msg(&format!(
        "Yêu cầu ban đầu: {}\n\nThảo luận trước đó:\n{}",
        message, combined_history_text
    ))];

    let planner_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        planner_system,
        &planner_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let planner_plan = planner_res.text.clone().unwrap_or_default();
    emit_step(2, "planner", "Planner (Lên Kế Hoạch)", "📋", &planner_plan);
    combined_history_text.push_str(&format!("\n📋 Planner (Draft Plan): {}\n", planner_plan));
    iterations.push(LoopResultIteration {
        iteration: 2,
        thought: None,
        tool_name: Some("Planner_DraftPlan".to_string()),
        tool_args: Some(json!({"role": "planner"}).to_string()),
        tool_result: Some(planner_plan.clone()),
        tool_success: true,
        timestamp: chrono::Local::now().to_rfc3339(),
    });

    // ─── STEP 3: Plan Reviewer Critique ──────────────────────────────────────
    let reviewer_system = "Bạn là Chuyên Gia Bắt Lỗi & Thẩm Định Kế Hoạch (Plan Reviewer 🔍 / QA Specialist).\n\
Nhiệm vụ của bạn:\n\
- Phân tích kỹ bản kế hoạch của Planner.\n\
- Chỉ ra các lỗ hổng logic, thiếu sót kịch bản, rủi ro bảo mật/hiệu năng hoặc edge cases.\n\
- Đưa ra nhận xét cụ thể và góp ý tinh chỉnh.\n\
- Kết thúc đánh giá bằng kết luận rõ ràng (ví dụ: '[APPROVED] Kế hoạch đã hoàn toàn ổn định!' hoặc '[NEEDS_REVISION] Cần bổ sung...').";

    let reviewer_input = vec![user_msg(&format!(
        "Yêu cầu người dùng: {}\n\nBản kế hoạch của Planner:\n{}",
        message, planner_plan
    ))];

    let reviewer_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        reviewer_system,
        &reviewer_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let reviewer_review = reviewer_res.text.clone().unwrap_or_default();
    emit_step(3, "reviewer", "Plan Reviewer (Bắt Lỗi Kế Hoạch)", "🔍", &reviewer_review);
    combined_history_text.push_str(&format!("\n🔍 Reviewer Critique: {}\n", reviewer_review));
    iterations.push(LoopResultIteration {
        iteration: 3,
        thought: None,
        tool_name: Some("Reviewer_AuditPlan".to_string()),
        tool_args: Some(json!({"role": "reviewer"}).to_string()),
        tool_result: Some(reviewer_review.clone()),
        tool_success: true,
        timestamp: chrono::Local::now().to_rfc3339(),
    });

    // ─── STEP 4: Planner Finalizes & Assigns Tasks ─────────────────────────
    let assignment_system = "Bạn là Kế Hoạch Viên (Planner 📋).\n\
Nhận xét từ Plan Reviewer đã được đưa ra.\n\
Nhiệm vụ của bạn ở bước này:\n\
1. Cập nhật bản kế hoạch ổn định hoàn chỉnh (xác nhận '[APPROVED] Kế hoạch đã ổn!').\n\
2. Chia bản kế hoạch thành 2 phần việc cụ thể và phân công rõ ràng:\n\
   - **Nhiệm vụ cho Coder 1 (Developer Alpha 💻)**: (ví dụ: Core Logic, Backend Services, Data Structure, Setup)\n\
   - **Nhiệm vụ cho Coder 2 (Developer Beta ⚡)**: (ví dụ: Frontend UI, Components, Styling, Integration)";

    let assignment_input = vec![user_msg(&format!(
        "Bản kế hoạch ban đầu:\n{}\n\nGóp ý của Reviewer:\n{}",
        planner_plan, reviewer_review
    ))];

    let assignment_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        assignment_system,
        &assignment_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let assignment_text = assignment_res.text.clone().unwrap_or_default();
    emit_step(4, "planner", "Planner (Phân Chia Nhiệm Vụ)", "📋", &assignment_text);
    combined_history_text.push_str(&format!("\n📋 Planner (Final Plan & Assignment): {}\n", assignment_text));
    iterations.push(LoopResultIteration {
        iteration: 4,
        thought: None,
        tool_name: Some("Planner_AssignTasks".to_string()),
        tool_args: Some(json!({"role": "planner"}).to_string()),
        tool_result: Some(assignment_text.clone()),
        tool_success: true,
        timestamp: chrono::Local::now().to_rfc3339(),
    });

    // ─── STEP 5: Coder 1 Executes ─────────────────────────────────────────────
    let coder1_system = "Bạn là Lập Trình Viên Alpha (Coder 1 💻).\n\
Hãy viết mã nguồn / triển khai phần nhiệm vụ lập trình được Planner phân công cho Coder 1.\n\
Viết code hoàn chỉnh, chuẩn mực, tối ưu và sạch đẹp.";

    let coder1_input = vec![user_msg(&format!(
        "Bản phân công nhiệm vụ của Planner:\n{}\n\nHãy thực hiện phần việc của Coder 1.",
        assignment_text
    ))];

    let coder1_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        coder1_system,
        &coder1_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let coder1_code = coder1_res.text.clone().unwrap_or_default();
    emit_step(5, "coder1", "Coder 1 (Developer Alpha)", "💻", &coder1_code);
    combined_history_text.push_str(&format!("\n💻 Coder 1 Output:\n{}\n", coder1_code));
    iterations.push(LoopResultIteration {
        iteration: 5,
        thought: None,
        tool_name: Some("Coder1_Execute".to_string()),
        tool_args: Some(json!({"role": "coder1"}).to_string()),
        tool_result: Some(coder1_code.clone()),
        tool_success: true,
        timestamp: chrono::Local::now().to_rfc3339(),
    });

    // ─── STEP 6: Coder 2 Executes ─────────────────────────────────────────────
    let coder2_system = "Bạn là Lập Trình Viên Beta (Coder 2 ⚡).\n\
Hãy viết mã nguồn / triển khai phần nhiệm vụ bổ trợ được Planner phân công cho Coder 2.\n\
Đảm bảo kết quả tương thích với phần việc của Coder 1 và đáp ứng đầy đủ bài toán.";

    let coder2_input = vec![user_msg(&format!(
        "Bản phân công của Planner:\n{}\n\nKết quả từ Coder 1:\n{}\n\nHãy thực hiện phần việc của Coder 2.",
        assignment_text, coder1_code
    ))];

    let coder2_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        coder2_system,
        &coder2_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let coder2_code = coder2_res.text.clone().unwrap_or_default();
    emit_step(6, "coder2", "Coder 2 (Developer Beta)", "⚡", &coder2_code);
    combined_history_text.push_str(&format!("\n⚡ Coder 2 Output:\n{}\n", coder2_code));
    iterations.push(LoopResultIteration {
        iteration: 6,
        thought: None,
        tool_name: Some("Coder2_Execute".to_string()),
        tool_args: Some(json!({"role": "coder2"}).to_string()),
        tool_result: Some(coder2_code.clone()),
        tool_success: true,
        timestamp: chrono::Local::now().to_rfc3339(),
    });

    // ─── STEP 7: Leader Synthesis & Final Report ──────────────────────────────
    let summary_system = "Bạn là Trưởng Team (Leader 👑).\n\
Toàn bộ đội ngũ (Planner, Reviewer, Coder 1, Coder 2) đã hoàn thành công việc.\n\
Nhiệm vụ của bạn:\n\
- Tổng hợp lại toàn bộ kết quả sản phẩm thành bản báo cáo tổng kết hoàn chỉnh, chuyên nghiệp và ngắn gọn cho người dùng.\n\
- Tổng hợp mã nguồn hoàn chỉnh (kết hợp Coder 1 và Coder 2) và hướng dẫn chạy/sử dụng nếu có.";

    let summary_input = vec![user_msg(&format!(
        "Yêu cầu ban đầu của người dùng: {}\n\nKết quả thực hiện của cả đội ngũ:\n{}",
        message, combined_history_text
    ))];

    let summary_res = rig_bridge::call_rig(
        &std_name,
        &api_key,
        &model_to_use,
        &model_config.api_url,
        model_config.temperature,
        model_config.top_p,
        summary_system,
        &summary_input,
        thinking_ref,
        Some(&window),
    )
    .await?;

    let final_report = summary_res.text.clone().unwrap_or_default();
    emit_step(7, "leader", "Trưởng Team (Báo Cáo Tổng Kết)", "👑", &final_report);

    let loop_result = LoopResult {
        iterations,
        final_text: Some(final_report.clone()),
        total_iterations: 7,
        tool_calls_made: 7,
        stopped_early: false,
        stop_reason: None,
    };

    Ok(AgentResponse {
        session_id,
        reply_type: "loop_result".to_string(),
        text: Some(final_report),
        tool_name: None,
        args: None,
        tools: None,
        pending_id: None,
        iteration: Some(7),
        total_iterations: Some(7),
        tool_result: None,
        approved_tool_index: None,
        loop_result: Some(loop_result),
    })
}
