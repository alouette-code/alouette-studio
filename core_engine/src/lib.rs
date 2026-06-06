pub mod agent_harness;
pub mod cloudflared_manager;
pub mod config;
pub mod db;
pub mod monitor;
pub mod process;
pub mod proto_manager;
pub mod settings;
pub mod workspace_manager;

// Re-export key structs for convenience
pub use agent_harness::{
    AgentHarness, AgentSession, AgentState, ChatMessage, MessageContent, TickResult,
};
pub use config::{LanguageRuntime, LanguageTool, ProjectConfig, ProjectsConfig, SandboxConfig, EnvSimulationConfig};
pub use db::DbManager;
pub use monitor::{ResourceMonitor, ResourceStats};
pub use process::{
    check_command, is_os_sandbox_supported, process_and_send_terminal_input,
    terminate_process_tree, ProcessLog, ProcessManager, ProcessState, SandboxVerdict,
    TerminalOutput, TerminalWriteContext, ChildProcessInfo, collect_child_processes,
};
pub use settings::AppSettings;
