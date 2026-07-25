use crate::agent_harness::AgentHarness;
use crate::agent_harness::parser::ToolCall;
use super::super::errors::MultiAgentError;

/// Định tuyến các yêu cầu ToolCall từ LLM vào `AgentHarness` thật để thực thi
pub struct ToolRouter<'a> {
    harness: &'a mut AgentHarness,
}

impl<'a> ToolRouter<'a> {
    pub fn new(harness: &'a mut AgentHarness) -> Self {
        Self { harness }
    }

    /// Chạy tool thông qua Sandbox của AgentHarness (Real execution)
    pub async fn execute(&mut self, session_id: &str, tool: &ToolCall) -> Result<String, MultiAgentError> {
        // Tận dụng execute_tool của AgentHarness để hưởng lợi từ Sandbox, Telemetry và Hooks
        match self.harness.execute_tool(session_id, tool).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // LLM có thể dùng lỗi này để Self-Heal
                Err(MultiAgentError::ToolError(e))
            }
        }
    }
}
