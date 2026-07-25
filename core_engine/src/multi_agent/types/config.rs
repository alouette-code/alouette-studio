use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub default_token_budget: u64,
    pub max_workflow_duration_secs: u64,
    pub auto_approve_all: bool,
    pub auto_approve_reads: bool,
    pub auto_approve_writes: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            default_token_budget: 1_000_000,
            max_workflow_duration_secs: 600, // 10 minutes max for a workflow
            auto_approve_all: false,
            auto_approve_reads: true,
            auto_approve_writes: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub role_id: String,
    pub role_name: String,
    pub avatar: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub max_iterations: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub token_budget: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            role_id: "generic_worker".to_string(),
            role_name: "Generic Worker".to_string(),
            avatar: "🤖".to_string(),
            system_prompt: "Bạn là một Worker chuyên biệt.".to_string(),
            allowed_tools: vec![],
            max_iterations: 5,
            temperature: 0.1,
            top_p: 0.9,
            token_budget: 1_000_000,
        }
    }
}
