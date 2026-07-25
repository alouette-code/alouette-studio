use serde::{Deserialize, Serialize};

/// Sự kiện để giao tiếp với Front-end qua Tauri (nếu cần)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentStepEvent {
    pub session_id: String,
    pub step: u32,
    pub agent_role: String,
    pub agent_name: String,
    pub content: String,
}
