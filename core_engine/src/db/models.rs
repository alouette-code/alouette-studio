// Models riêng biệt phục vụ cho tương tác Database
// Giúp tách biệt Domain logic và Data persistence logic

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDbEntity {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: String, // Stored as JSON string
    pub cwd: Option<String>,
    pub setup_command: Option<String>,
    pub setup_args: Option<String>, // JSON string
    pub auto_restart: Option<i32>, // SQLite doesn't have native boolean
    pub env: Option<String>, // JSON string
    pub max_cpu_percent: Option<u32>,
    pub max_ram_mb: Option<i64>,
    pub port: Option<u16>,
    pub source: Option<String>,
    pub terminal_mode: Option<String>,
    pub toolchain: Option<String>,
    pub toolchain_version: Option<String>,
    pub enable_tunnel: Option<i32>,
    pub max_log_lines: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDbEntity {
    pub id: i64,
    pub project_id: String,
    pub stream: String,
    pub text: String,
    pub timestamp: i64,
}
