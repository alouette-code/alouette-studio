use crate::agent_harness::parser::ToolCall;
use crate::agent_harness::{ChatMessage, MessageContent};
use super::super::types::config::WorkerConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerState {
    Idle,
    Thinking,
    WaitingForToolResult(Vec<ToolCall>),
    Finished(String),
    Error(String),
}

/// Session đại diện cho vòng đời của một Micro-Agent (Worker)
pub struct WorkerSession {
    pub config: WorkerConfig,
    pub history: Vec<ChatMessage>,
    pub iteration_count: u32,
    pub state: WorkerState,
    pub estimated_tokens: u64,
}

impl WorkerSession {
    pub fn new(config: WorkerConfig, initial_prompt: String) -> Self {
        let text_len = initial_prompt.len() as u64;
        Self {
            config,
            history: vec![ChatMessage {
                id: format!("usr_{}", chrono::Local::now().timestamp_millis()),
                role: "user".to_string(),
                content: MessageContent::Text(initial_prompt),
                timestamp: chrono::Local::now().format("%H:%M").to_string(),
            }],
            iteration_count: 0,
            state: WorkerState::Idle,
            estimated_tokens: text_len / 4, // Rough estimation
        }
    }

    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.config.allowed_tools.is_empty() || self.config.allowed_tools.contains(&tool_name.to_string())
    }

    pub fn append_llm_response(&mut self, text: Option<String>, tools: Vec<ToolCall>) {
        let msg_id = format!("ast_{}", chrono::Local::now().timestamp_millis());
        let mut added_len = text.as_ref().map(|s| s.len()).unwrap_or(0);
        
        if tools.is_empty() {
            self.history.push(ChatMessage {
                id: msg_id,
                role: "assistant".to_string(),
                content: MessageContent::Text(text.unwrap_or_default()),
                timestamp: chrono::Local::now().format("%H:%M").to_string(),
            });
            self.state = WorkerState::Finished("No tools requested, worker done.".to_string());
        } else {
            added_len += tools.len() * 50; // estimate JSON size
            self.history.push(ChatMessage {
                id: msg_id,
                role: "assistant".to_string(),
                content: MessageContent::ToolCalls(tools.clone(), text),
                timestamp: chrono::Local::now().format("%H:%M").to_string(),
            });
            self.state = WorkerState::WaitingForToolResult(tools);
        }
        self.iteration_count += 1;
        self.estimated_tokens += (added_len as u64) / 4;
        self.check_and_compact();
    }

    pub fn append_tool_result(&mut self, tool_id: String, tool_name: String, result: String, success: bool) {
        self.estimated_tokens += (result.len() as u64) / 4;
        self.history.push(ChatMessage {
            id: format!("tool_res_{}", tool_id),
            role: "tool".to_string(),
            content: MessageContent::ToolResult {
                tool_call_id: tool_id,
                tool_name,
                result,
                success,
            },
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        });
        self.state = WorkerState::Idle;
        self.check_and_compact();
    }

    pub fn has_reached_limit(&self) -> bool {
        self.iteration_count >= self.config.max_iterations
    }

    /// Trimming / Compacting history if token budget is exceeded
    fn check_and_compact(&mut self) {
        if self.estimated_tokens > self.config.token_budget {
            println!("[Worker {}] CẢNH BÁO: Vượt ngưỡng Token ({}/{}), tiến hành nén ngữ cảnh...", self.config.role_name, self.estimated_tokens, self.config.token_budget);
            // Giữ lại message đầu tiên, cắt bỏ phần giữa
            if self.history.len() > 5 {
                let first_msg = self.history[0].clone();
                let last_msgs = self.history.split_off(self.history.len() - 3);
                
                self.history.clear();
                self.history.push(first_msg);
                self.history.push(ChatMessage {
                    id: "compacted".to_string(),
                    role: "system".to_string(),
                    content: MessageContent::Text("... [Phần lịch sử ở giữa đã được nén lại để tránh quá tải Token (Context Exhaustion)] ...".to_string()),
                    timestamp: chrono::Local::now().format("%H:%M").to_string(),
                });
                self.history.extend(last_msgs);
                
                // Recalculate
                self.estimated_tokens = self.config.token_budget / 2;
                println!("[Worker {}] Đã nén thành công.", self.config.role_name);
            }
        }
    }
}
