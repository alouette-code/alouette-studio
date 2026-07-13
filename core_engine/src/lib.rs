pub mod error;
pub mod agent_harness;
pub mod cloudflared_manager;
pub mod code_rag;
pub mod config;
pub mod db;
pub mod monitor;
pub mod process;
pub mod proto_manager;
pub mod settings;
pub mod workspace_manager;
pub mod vm_engine;
pub mod memory_inspector;
pub mod docker_engine;
pub mod agent_tools;


// Re-export key structs for convenience
pub use agent_harness::{
    AgentHarness, AgentSession, AgentState, ChatMessage, MessageContent, TickResult,
};
pub use code_rag::{
    all_languages, embedding::EmbeddingModel, extension_map, extract_functions, normalize_entry,
    seed_code_library, IndexEvent, Indexer, IndexerConfig, LanguageConfig, LanguageResolver,
    QueryEngine, QueryResult, Tier, VectorDb,
};
pub use config::{
    EnvSimulationConfig, LanguageRuntime, LanguageTool, ProjectConfig, ProjectsConfig,
    SandboxConfig,
};
pub use db::DbManager;
pub use db::models::*;
pub use db::repositories::project_repo::ProjectRepository;
pub use monitor::{ResourceMonitor, ResourceStats};
pub use process::{
    check_command, collect_child_processes, is_os_sandbox_supported,
    process_and_send_terminal_input, terminate_process_tree, ChildProcessInfo, ProcessLog,
    ProcessManager, ProcessState, SandboxVerdict, TerminalOutput, TerminalWriteContext,
};
pub use settings::AppSettings;
