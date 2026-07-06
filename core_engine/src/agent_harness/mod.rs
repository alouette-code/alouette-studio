use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;

pub mod autonomous;
pub mod compaction;
pub mod hooks;
pub mod memory;
pub mod parser;
pub mod plan;
pub mod self_heal;
pub mod session;
pub mod skills;
pub mod telemetry;
pub mod tool_definitions;

// ─── Agent Loop Types ─────────────────────────────────────────────────

/// Kết quả của một vòng lặp agent hoàn chỉnh
#[derive(Debug, Clone, Serialize)]
pub struct AgentLoopResult {
    pub session_id: String,
    pub iterations: Vec<AgentLoopIteration>,
    pub final_text: Option<String>,
    pub total_iterations: u32,
    pub tool_calls_made: u32,
    pub stopped_early: bool,
    pub stop_reason: Option<String>,
}

/// Một lần lặp trong vòng lặp agent
#[derive(Debug, Clone, Serialize)]
pub struct AgentLoopIteration {
    pub iteration: u32,
    pub thought: Option<String>,
    pub tool_name: Option<String>,
    pub tool_args: Option<String>,
    pub tool_result: Option<String>,
    pub tool_success: bool,
    pub timestamp: String,
}

// (CompletionPhase removed — simplified loop: stops when AI returns plain text with no tool calls)

/// Structured response from LLM — preserves native tool_calls to avoid text parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<parser::ToolCall>,
    pub raw_text: String,
}

impl LlmResponse {
    /// Create from a plain text response (backward compat / fallback)
    pub fn from_text(text: String) -> Self {
        Self {
            text: Some(text.clone()),
            tool_calls: vec![],
            raw_text: text,
        }
    }

    /// Create from parsed tool calls and optional text
    pub fn from_parsed(
        text: Option<String>,
        tool_calls: Vec<parser::ToolCall>,
        raw_text: String,
    ) -> Self {
        Self {
            text,
            tool_calls,
            raw_text,
        }
    }
}

/// Cấu hình cho vòng lặp agent
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    pub max_iterations: u32,
    pub auto_approve_reads: bool,
    pub auto_approve_writes: bool,
    pub auto_approve_all: bool,
    pub command_timeout_secs: u64,
    pub session_id: String,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            auto_approve_reads: true,
            auto_approve_writes: false,
            auto_approve_all: false,
            command_timeout_secs: 15,
            session_id: String::new(),
        }
    }
}

/// Operating modes for the agent harness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HarnessMode {
    /// Standard interactive agent mode
    Standard,
    /// Plan mode with 5-phase workflow
    Plan,
    /// Coordinator mode for subagent delegation
    Coordinator,
    /// Worker mode executing coordinator tasks
    Worker,
    /// Autonomous loop mode (timer-based)
    Autonomous,
    /// Minimal mode (no hooks, LSP, auto-memory)
    Minimal,
}

/// Kết quả của một lần tick() — State Machine Engine
#[derive(Debug, Clone, Serialize)]
pub enum TickResult {
    /// Đã thực hiện xong một step, gọi tick() tiếp
    Continue {
        thought: Option<String>,
        tool_name: Option<String>,
        tool_result: Option<String>,
        iteration: u32,
    },
    /// Agent đang chờ user phê duyệt
    WaitForApproval {
        tools: Vec<parser::ToolCall>,
        iteration: u32,
    },
    /// Agent hoàn thành
    Finished { text: String, total_iterations: u32 },
    /// Có lỗi
    Error { message: String, iteration: u32 },
}

/// Nội dung tin nhắn — phân biệt Text thuần, Tool Calls, hoặc Kết quả Tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Văn bản thuần túy
    Text(String),
    /// Khi AI quyết định gọi một hoặc nhiều tools (Native Tool Calling)
    ToolCalls(Vec<parser::ToolCall>),
    /// Kết quả trả về sau khi chạy tool
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        result: String,
        success: bool,
    },
}

/// Trạng thái hiện tại của Agent (State Machine)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentState {
    /// Chưa bắt đầu hoặc đang chờ input mới
    Idle,
    /// Đang đợi LLM trả lời
    Thinking,
    /// Đang chạy tool tự động
    ExecutingTool,
    /// DỪNG LẠI: Chờ user bấm Approve/Reject
    AwaitingApproval(Vec<parser::ToolCall>),
    /// Đang xác minh kết quả (Dual-Verification)
    Verifying,
    /// Hoàn thành task
    Finished(String),
    /// Lỗi
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: MessageContent,
    pub timestamp: String,
}

fn default_token_budget() -> u64 {
    5_000_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: String,
    pub history: Vec<ChatMessage>,
    pub state: AgentState,
    pub iteration_count: u32,
    pub max_iterations: u32,
    pub mode: HarnessMode,
    pub plan: Option<plan::Plan>,
    pub autonomous_state: Option<autonomous::AutonomousManager>,
    #[serde(default = "default_token_budget")]
    pub token_budget: u64,
}

/// Subagent tracking for coordinator mode
#[derive(Debug, Clone)]
pub struct Subagent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub subagent_type: Option<String>,
    pub status: SubagentStatus,
    pub prompt: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubagentStatus {
    Running,
    Completed,
    Failed(String),
    Stopped,
}

/// Blast radius assessment for action safety
#[derive(Debug, Clone, PartialEq)]
pub enum BlastRadius {
    /// Local, reversible actions (editing files, running tests)
    Local,
    /// Hard to reverse (force push, delete branches, publish)
    Significant,
    /// Outward-facing, affects shared state (PR, Slack, external API)
    Outward,
}

/// The core agent harness - orchestrates AI agent execution
/// Tracks a long-running terminal command that exceeded the 20s timeout
struct RunningCommand {
    child: tokio::process::Child,
    output: Arc<Mutex<String>>,
    stderr_output: Arc<Mutex<String>>,
    stdout_handle: Option<tokio::task::JoinHandle<()>>,
    stderr_handle: Option<tokio::task::JoinHandle<()>>,
    start_time: std::time::Instant,
    command: String,
}

impl Drop for RunningCommand {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

pub struct AgentHarness {
    workspace_root: PathBuf,
    telemetry: telemetry::TelemetryManager,
    memory_manager: memory::MemoryManager,
    hook_manager: Option<hooks::HookManager>,
    skill_engine: skills::SkillEngine,
    mode: HarnessMode,
    command_timeout_secs: u64,
    pub cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    subagents: HashMap<String, Subagent>,
    running_commands: HashMap<String, RunningCommand>,
}

impl AgentHarness {
    pub fn new<P: AsRef<Path>>(workspace_root: P) -> Self {
        let canonical_root = fs::canonicalize(&workspace_root)
            .unwrap_or_else(|_| workspace_root.as_ref().to_path_buf());
        let telemetry = telemetry::TelemetryManager::new(&canonical_root);
        let memory_manager = memory::MemoryManager::new(&canonical_root);

        let skill_engine = skills::SkillEngine::new(canonical_root.clone());
        Self {
            workspace_root: canonical_root,
            telemetry,
            memory_manager,
            skill_engine,
            hook_manager: None,
            mode: HarnessMode::Standard,
            command_timeout_secs: 15,
            cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            subagents: HashMap::new(),
            running_commands: HashMap::new(),
        }
    }

    /// Set the harness operating mode
    pub fn set_mode(&mut self, mode: HarnessMode) {
        self.mode = mode;
    }

    /// Dynamically update the workspace root without dropping running commands
    pub fn set_workspace_root<P: AsRef<Path>>(&mut self, workspace_root: P) {
        let canonical_root = fs::canonicalize(&workspace_root)
            .unwrap_or_else(|_| workspace_root.as_ref().to_path_buf());
        if self.workspace_root != canonical_root {
            self.workspace_root = canonical_root.clone();
            self.telemetry = telemetry::TelemetryManager::new(&canonical_root);
            self.memory_manager = memory::MemoryManager::new(&canonical_root);
            self.skill_engine = skills::SkillEngine::new(canonical_root);
        }
    }

    /// Clean up finished or dead background processes to avoid memory leaks
    pub fn prune_running_commands(&mut self) {
        self.running_commands.retain(|_id, cmd| {
            match cmd.child.try_wait() {
                Ok(Some(_status)) => false, // Finished, remove from map
                Err(_) => false,            // Failed/corrupted, remove
                Ok(None) => {
                    // Still running, check if elapsed > 1 hour
                    if cmd.start_time.elapsed() > Duration::from_secs(3600) {
                        let _ = cmd.child.start_kill();
                        false
                    } else {
                        true
                    }
                }
            }
        });
    }

    /// Set command execution timeout (seconds)
    pub fn set_command_timeout(&mut self, timeout_secs: u64) {
        self.command_timeout_secs = timeout_secs;
    }

    /// Get the current operating mode
    pub fn mode(&self) -> &HarnessMode {
        &self.mode
    }

    /// Initialize hooks from a configuration file
    pub fn init_hooks(&mut self, config_path: &Path) -> Result<(), String> {
        self.hook_manager = Some(hooks::HookManager::from_file(config_path)?);
        Ok(())
    }

    // ─── System Prompt Assembly ───────────────────────────────────────────

    /// Assemble the full system prompt based on the current mode
    pub fn assemble_system_prompt(&self) -> String {
        let mut prompt = String::new();

        // 1. Identity
        prompt.push_str(&self.load_or_default(
            "identity.txt",
            "You are an autonomous AI coding assistant. Always think in a <thought> block before acting."
        ));
        prompt.push_str("\n\n");

        // 2. Software Engineering Focus
        prompt.push_str(&self.get_se_focus_prompt());

        // 3. Communication Style
        prompt.push_str(&self.get_communication_style_prompt());

        // 4. Action Safety
        prompt.push_str(&self.get_action_safety_prompt());

        // 5. Security Guidelines
        prompt.push_str(&self.get_security_prompt());

        // 6. Tool Usage (generated from tool_definitions)
        prompt.push_str(&tool_definitions::tools_prompt_text());

        // 8. Mode-specific prompts
        match self.mode {
            HarnessMode::Plan => prompt.push_str(&self.get_plan_mode_prompt()),
            HarnessMode::Coordinator => prompt.push_str(&self.get_coordinator_prompt()),
            HarnessMode::Worker => prompt.push_str(&self.get_worker_prompt()),
            HarnessMode::Autonomous => prompt.push_str(&self.get_autonomous_prompt()),
            HarnessMode::Minimal => {
                prompt.push_str("\n\n## Minimal Mode\n");
                prompt.push_str("Running in minimal mode: hooks, LSP, auto-memory, and background features are disabled.");
            }
            HarnessMode::Standard => {}
        }

        // 9. Memory instructions (if not minimal mode)
        if self.mode != HarnessMode::Minimal {
            prompt.push_str(&self.get_memory_prompt());
        }

        // 10. Project context (CLAUDE.md)
        let claude_md_path = self.workspace_root.join("CLAUDE.md");
        if claude_md_path.exists() {
            if let Ok(claude_context) = fs::read_to_string(&claude_md_path) {
                prompt.push_str("\n\n## Project-Specific Guidelines (CLAUDE.md)\n");
                prompt.push_str(&claude_context);
            }
        }

        // 11. Current environment
        let git_branch = self.get_git_branch();
        let git_status = self.get_git_status();

        prompt.push_str(&format!(
            "\n\n## Current Environment\n- OS: {}\n- Workspace: {}\n",
            std::env::consts::OS,
            self.workspace_root.display()
        ));

        if !git_branch.is_empty() {
            prompt.push_str(&format!("- Branch: {}\n", git_branch));
        }
        if !git_status.is_empty() {
            prompt.push_str(&format!("\n## Git Status\n```\n{}\n```\n", git_status));
        }

        // 12. Harness Instructions
        prompt.push_str("\n\n## Harness\n");
        prompt.push_str("- Text outside tool use is displayed as markdown.\n");
        prompt.push_str("- Tools run behind permission checks; a denied call means adjust, don't retry verbatim.\n");
        prompt.push_str("- `<system-reminder>` tags are injected by the harness, not the user.\n");
        prompt.push_str("- Reference code as `file_path:line_number` — it's clickable.\n");

        prompt
    }

    // ─── Mode-Specific Prompts ────────────────────────────────────────────

    fn get_se_focus_prompt(&self) -> String {
        "\n## Software Engineering Focus\nThe user primarily requests software engineering tasks: solving bugs, adding functionality, refactoring, explaining code.\nWhen given unclear instructions, interpret them in code context.\n".to_string()
    }

    fn get_communication_style_prompt(&self) -> String {
        "\n## Communication Style\n- Before your first tool call, state in one sentence what you're about to do.\n- Give short updates at key moments: findings, direction changes, blockers.\n- End-of-turn: one or two sentences. What changed and what's next.\n- Match response format to task complexity — simple questions get direct answers.\n- Default to writing no comments in code. Never write multi-paragraph docstrings.\n".to_string()
    }

    fn get_action_safety_prompt(&self) -> String {
        "\n## Action Safety\n- For hard-to-reverse or outward-facing actions, confirm first unless durably authorized.\n- Before deleting/overwriting, check the target — if it contradicts expectations, surface that.\n- Report outcomes faithfully: if tests fail, say so; if skipped, say that.\n- Destructive operations (rm -rf, force push, git reset --hard) warrant user confirmation.\n- Actions visible to others (push, PR, Slack) affect shared state — confirm first.\n".to_string()
    }

    fn get_security_prompt(&self) -> String {
        "\n## Security\n- Do not introduce security vulnerabilities: command injection, XSS, SQL injection, OWASP top 10.\n- If you notice insecure code, immediately fix it.\n- Prioritize safe, secure, and correct code.\n".to_string()
    }

    fn get_plan_mode_prompt(&self) -> String {
        "\n## Plan Mode (5-Phase)\n\
         You are in plan mode with five phases:\n\
         1. **Research** — Understand the problem, gather requirements, investigate codebase\n\
         2. **Synthesis** — Design solution approach, identify files to change\n\
         3. **Planning** — Write implementation plan with specific steps\n\
         4. **Implementation** — Execute the plan, make changes\n\
         5. **Verification** — Verify changes work (tests, typecheck, manual)\n\n\
         Track progress with clear phase transitions. Only advance when all current phase steps are complete.\n"
        .to_string()
    }

    fn get_coordinator_prompt(&self) -> String {
        "\n## Coordinator Mode\n\
         You orchestrate work across multiple subagents.\n\n\
         **Your Role:**\n\
         - Direct workers to research, implement, and verify\n\
         - Synthesize results and communicate with the user\n\
         - Answer questions directly when possible\n\n\
         **Parallelism is your superpower.** Launch independent workers concurrently.\n\n\
         **Worker Prompts:** Be self-contained. Workers can't see your conversation.\n\
         Always synthesize research findings yourself before directing follow-up work.\n\n\
         **Worker Results** arrive as `<task-notification>` user-role messages.\n\n\
         **Decision: continue vs spawn fresh:**\n\
         - Research explored exactly the files to edit → **Continue** the worker\n\
         - Research was broad but implementation narrow → **Spawn fresh**\n\
         - Correcting failure → **Continue** (worker has error context)\n\
         - Verifying different worker's code → **Spawn fresh** (fresh eyes)\n"
            .to_string()
    }

    fn get_worker_prompt(&self) -> String {
        "\n## Worker Instructions\n\
         You are executing a task assigned by the coordinator.\n\n\
         **Scope:** Complete exactly what was asked. Don't fix unrelated issues.\n\
         - Commit changes with clear messages when done. Only stage files you changed.\n\
         - Do not spawn sub-agents.\n\
         - Limit changes to what your task requires.\n\n\
         **When stuck:**\n\
         - Tool denied? Stop and report what you needed.\n\
         - Task impossible? Stop and explain why.\n\
         - Ambiguous? Pick the most likely interpretation and note your assumption.\n\n\
         **Output structure:**\n\
         1. What you did/found — specific file paths, code snippets\n\
         2. Summary: One sentence the coordinator can relay\n"
            .to_string()
    }

    fn get_autonomous_prompt(&self) -> String {
        "\n## Autonomous Mode\n\
         You are being invoked on a timer while the user is away.\n\n\
         **What to act on:**\n\
         1. In-progress PR: review comments, failing CI, merge conflicts\n\
         2. Unfinished implementation from conversation\n\
         3. Dangling questions or verification steps\n\n\
         **Action Safety:**\n\
         - Reversible actions (local edits, tests): proceed freely\n\
         - Irreversible actions (push, delete): require authorization\n\n\
         **Idle handling:** After 3 consecutive idle ticks, scale back to quick CI check.\n"
            .to_string()
    }

    fn get_memory_prompt(&self) -> String {
        r#"

## Memory

You have a persistent file-based memory system. Each memory is one file holding one fact, with frontmatter:

```markdown
---
name: <short-kebab-case-slug>
description: <one-line summary — used to decide relevance during recall>
metadata:
  type: user | feedback | project | reference
---

<the fact; for feedback/project, follow with **Why:** and **How to apply:** lines>
```

Memory types:
- `user` — who the user is (role, expertise, preferences, working style)
- `feedback` — guidance the user has given on how you should work (corrections and confirmed approaches); include the why
- `project` — ongoing work, goals, or constraints not derivable from the code or git history; convert relative dates to absolute
- `reference` — pointers to external resources (URLs, dashboards, tickets, API docs)

Before saving, check for an existing file that already covers it — update that file rather than creating a duplicate; delete memories that turn out to be wrong.

Do NOT save what the repo already records (code structure, past fixes, git history, CLAUDE.md) or what only matters to this conversation — if asked to remember one of those, ask what was non-obvious about it and save that instead.
"#.to_string()
    }

    // ─── Secure Path Validation ──────────────────────────────────────────

    /// Validate that a path is within the workspace boundary
    pub fn validate_path(&self, target_path: &Path) -> Result<PathBuf, String> {
        let absolute_target = if target_path.is_absolute() {
            target_path.to_path_buf()
        } else {
            self.workspace_root.join(target_path)
        };

        // Recursively find the closest existing parent/ancestor
        let mut ancestor = absolute_target.clone();
        let mut canonical_ancestor = None;

        while let Some(parent) = ancestor.parent() {
            if ancestor.exists() {
                if let Ok(canon) = fs::canonicalize(&ancestor) {
                    canonical_ancestor = Some(canon);
                    break;
                }
            }
            ancestor = parent.to_path_buf();
        }

        // Fallback check on root / parent if not resolved
        if canonical_ancestor.is_none() && ancestor.exists() {
            if let Ok(canon) = fs::canonicalize(&ancestor) {
                canonical_ancestor = Some(canon);
            }
        }

        let canonical_ancestor = canonical_ancestor.ok_or_else(|| {
            format!(
                "Security Error: Path parent does not exist or cannot be resolved: {}",
                absolute_target.display()
            )
        })?;

        if !canonical_ancestor.starts_with(&self.workspace_root) {
            return Err(format!(
                "Security Boundary Error: Access to '{}' is forbidden. Outside workspace '{}'.",
                absolute_target.display(),
                self.workspace_root.display()
            ));
        }

        // Prevent traversal using component checks
        for component in target_path.components() {
            if component == std::path::Component::ParentDir {
                return Err(
                    "Security Boundary Error: Parent directory traversal (..) is forbidden."
                        .to_string(),
                );
            }
        }

        Ok(absolute_target)
    }

    /// Assess blast radius of an action
    pub fn assess_blast_radius(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> BlastRadius {
        match tool_name {
            "write_file" | "edit_file" | "execute_command" | "check_port" => BlastRadius::Local,
            "get_project_files" | "read_file" => BlastRadius::Local,
            "delete_file" | "force_push" | "git_reset" => {
                // Check if arguments indicate destructive operation
                let is_destructive = arguments
                    .get("force")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                    || arguments
                        .get("hard")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                if is_destructive {
                    BlastRadius::Significant
                } else {
                    BlastRadius::Local
                }
            }
            "create_pr" | "post_message" | "push_code" | "deploy" => BlastRadius::Outward,
            _ => BlastRadius::Local,
        }
    }

    // ─── Tool Execution ──────────────────────────────────────────────────

    /// Execute a tool call safely with hook lifecycle and telemetry
    pub async fn execute_tool(
        &mut self,
        session_id: &str,
        tool: &parser::ToolCall,
    ) -> Result<String, String> {
        let start_time = std::time::Instant::now();
        let mut exit_status = "Success";

        // 1. Execute PreToolUse hooks
        if let Some(ref hook_manager) = self.hook_manager {
            let hook_input = hooks::HookInput {
                session_id: session_id.to_string(),
                tool_name: tool.name.clone(),
                tool_input: tool.arguments.clone(),
                tool_response: None,
            };
            let hook_outputs =
                hook_manager.execute_hooks(&hooks::HookEvent::PreToolUse, &tool.name, &hook_input);
            if let Some(block_reason) = hook_manager.should_block(&hook_outputs) {
                return Err(format!("PreToolUse hook blocked: {}", block_reason));
            }
        }

        // 2. Assess blast radius
        let radius = self.assess_blast_radius(&tool.name, &tool.arguments);

        // 3. Execute the tool
        let result = match tool.name.as_str() {
            "check_port" => self.execute_check_port(tool),
            "read_file" => self.execute_read_file(tool),
            "write_file" => self.execute_write_file(tool),
            "get_project_files" => self.execute_get_project_files(),
            "execute_command" => self.execute_command(session_id, tool).await,
            "check_command_status" => self.execute_check_command_status(tool).await,
            "kill_command" => self.execute_kill_command(tool).await,
            "save_memory" => self.execute_save_memory(tool),
            "search_memory" => self.execute_search_memory(tool),
            "compact_history" => self.execute_compact_history(tool),
            // Skill tools
            "scan_directory_tree" => Ok(self.skill_engine.scan_directory_tree().tree_string),
            "scan_subdirectory" => self.execute_scan_subdirectory(tool),
            "search_files" => self.execute_search_files(tool),
            "extract_symbol" => self.execute_extract_symbol(tool),
            "read_file_range" => self.execute_read_file_range(tool),
            "search_symbol" => self.execute_search_symbol(tool),
            "edit_file" => self.execute_edit_file(tool),
            _ => Err(format!("Unknown tool: {}", tool.name)),
        };

        let duration = start_time.elapsed().as_millis() as u64;
        let result_str = match &result {
            Ok(val) => val.clone(),
            Err(e) => {
                exit_status = "Failure";
                e.clone()
            }
        };

        // 4. Track telemetry
        self.telemetry.track_tool_call(
            session_id,
            &tool.name,
            exit_status == "Success",
            duration,
            None,
        );

        // 5. Execute PostToolUse hooks
        if let Some(ref hook_manager) = self.hook_manager {
            let hook_input = hooks::HookInput {
                session_id: session_id.to_string(),
                tool_name: tool.name.clone(),
                tool_input: tool.arguments.clone(),
                tool_response: Some(serde_json::json!({
                    "success": exit_status == "Success",
                    "result": result_str,
                    "duration_ms": duration,
                    "blast_radius": format!("{:?}", radius),
                })),
            };
            let event = if exit_status == "Success" {
                hooks::HookEvent::PostToolUse
            } else {
                hooks::HookEvent::PostToolUseFailure
            };
            let hook_outputs = hook_manager.execute_hooks(&event, &tool.name, &hook_input);
            if let Some(block_reason) = hook_manager.should_block(&hook_outputs) {
                return Err(format!("PostToolUse hook blocked: {}", block_reason));
            }
        }

        // 6. Self-heal if failed
        if exit_status == "Failure" {
            let healed = self_heal::SelfHealAnalyzer::analyze_failure(
                &tool.name,
                &tool.arguments,
                &result_str,
            );
            Err(healed)
        } else {
            result
        }
    }

    fn execute_check_port(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let port = tool.arguments["port"]
            .as_u64()
            .ok_or_else(|| "Missing 'port' argument".to_string())? as u16;

        let is_available = std::net::TcpListener::bind(("127.0.0.1", port)).is_ok();
        if is_available {
            Ok(format!("✓ Port {} is FREE and available.", port))
        } else {
            Ok(format!("✕ Port {} is IN USE.", port))
        }
    }

    fn execute_read_file(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let path_str = tool.arguments["path"]
            .as_str()
            .ok_or_else(|| "Missing 'path' argument".to_string())?;

        let target_path = Path::new(path_str);
        let verified_path = self.validate_path(target_path)?;

        let metadata = fs::metadata(&verified_path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;
        let file_size = metadata.len();

        let max_size = 2 * 1024 * 1024; // 2MB
        if file_size > max_size {
            return Err(format!(
                "File too large: {} bytes (max limit is {} bytes). Please use `read_file_range` or specify a smaller range.",
                file_size, max_size
            ));
        }

        let bytes = fs::read(&verified_path).map_err(|e| format!("Failed to read file: {}", e))?;
        let content = String::from_utf8(bytes).map_err(|_| {
            "Error: Binary file detected or file content contains invalid UTF-8 data. Reading binary files is not supported.".to_string()
        })?;

        Ok(content)
    }

    fn execute_write_file(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let path_str = tool.arguments["path"]
            .as_str()
            .ok_or_else(|| "Missing 'path' argument".to_string())?;
        let content = tool.arguments["content"]
            .as_str()
            .ok_or_else(|| "Missing 'content' argument".to_string())?;

        let target_path = Path::new(path_str);
        let verified_path = self.validate_path(target_path)?;

        if let Some(parent) = verified_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::write(&verified_path, content)
            .map(|_| format!("✓ Wrote {} bytes to: {}", content.len(), path_str))
            .map_err(|e| format!("Failed to write file: {}", e))
    }

    fn execute_edit_file(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let path_str = tool.arguments["path"]
            .as_str()
            .ok_or_else(|| "Missing 'path' argument".to_string())?;
        let old_content = tool.arguments["old_content"]
            .as_str()
            .ok_or_else(|| "Missing 'old_content' argument".to_string())?;
        let new_content = tool.arguments["new_content"]
            .as_str()
            .ok_or_else(|| "Missing 'new_content' argument".to_string())?;

        let start_line = tool
            .arguments
            .get("start_line")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let end_line = tool
            .arguments
            .get("end_line")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let target_path = Path::new(path_str);
        let verified_path = self.validate_path(target_path)?;

        let current_content = fs::read_to_string(&verified_path)
            .map_err(|e| format!("Failed to read file '{}': {}", path_str, e))?;

        // Detect line ending (\r\n vs \n)
        let line_ending = if current_content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };

        let lines: Vec<&str> = current_content.split(line_ending).collect();

        if let (Some(start), Some(end)) = (start_line, end_line) {
            // Line numbers are 1-based, inclusive
            if start == 0 || end == 0 || start > end || start > lines.len() || end > lines.len() {
                return Err(format!(
                    "Invalid line range {}-{} (file '{}' has {} lines)",
                    start,
                    end,
                    path_str,
                    lines.len()
                ));
            }

            let slice = &lines[start - 1..end];
            let original_slice_text = slice.join(line_ending);

            let norm_old = old_content.replace("\r\n", "\n").trim_end().to_string();
            let norm_slice = original_slice_text
                .replace("\r\n", "\n")
                .trim_end()
                .to_string();

            if norm_old != norm_slice {
                return Err(format!(
                    "Line range mismatch: The provided old_content does not match the text at lines {}-{}.\nExpected:\n\"{}\"\nFound:\n\"{}\"",
                    start, end, norm_old, norm_slice
                ));
            }

            // Perform replacement on this range
            let mut new_lines = lines.clone();
            new_lines.splice(start - 1..end, vec![new_content]);
            let replaced = new_lines.join(line_ending);
            fs::write(&verified_path, replaced)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        } else {
            // Uniqueness checking fallback
            let normalized_old = old_content.replace("\r\n", "\n");
            let normalized_current = current_content.replace("\r\n", "\n");

            let occurrences = normalized_current.matches(&normalized_old).count();
            if occurrences == 0 {
                return Err(format!(
                    "Could not find the specified old_content in '{}'. The text may have changed or indentation may differ.",
                    path_str
                ));
            } else if occurrences > 1 {
                return Err(format!(
                    "The text to replace (old_content) was found {} times in '{}'. To avoid incorrect replacements, please specify start_line and end_line, or provide a larger, unique block of surrounding code as old_content.",
                    occurrences, path_str
                ));
            }

            let replaced = normalized_current.replacen(&normalized_old, new_content, 1);
            fs::write(&verified_path, replaced)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }

        Ok(format!(
            "✓ edit_file: replaced {} chars → {} chars in '{}'",
            old_content.len(),
            new_content.len(),
            path_str
        ))
    }

    fn execute_get_project_files(&self) -> Result<String, String> {
        let mut files = Vec::new();
        self.list_dir_recursive(&self.workspace_root, &mut files, 0)?;
        if files.len() >= 1000 {
            files.push("... (truncated due to file limit of 1000)".to_string());
        }
        Ok(files.join("\n"))
    }

    async fn execute_command(
        &mut self,
        session_id: &str,
        tool: &parser::ToolCall,
    ) -> Result<String, String> {
        self.prune_running_commands();
        let command = tool.arguments["command"]
            .as_str()
            .ok_or_else(|| "Missing 'command' argument".to_string())?;

        let args: Vec<String> = if let Some(arr) = tool.arguments["args"].as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };

        let cmd_str = if args.is_empty() {
            command.to_string()
        } else {
            let escaped_args: Vec<String> = args
                .iter()
                .map(|arg| {
                    if cfg!(target_os = "windows") {
                        format!("'{}'", arg.replace('\'', "''"))
                    } else {
                        format!("'{}'", arg.replace('\'', "'\\''"))
                    }
                })
                .collect();
            format!("{} {}", command, escaped_args.join(" "))
        };

        // Build async command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = tokio::process::Command::new("powershell");
            c.env_clear();
            if let Ok(val) = std::env::var("PATH") {
                c.env("PATH", val);
            }
            if let Ok(val) = std::env::var("SystemRoot") {
                c.env("SystemRoot", val);
            }
            if let Ok(val) = std::env::var("windir") {
                c.env("windir", val);
            }
            if let Ok(val) = std::env::var("PATHEXT") {
                c.env("PATHEXT", val);
            }
            c.args(&[
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
            ]);
            c.arg(&cmd_str);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.env_clear();
            if let Ok(val) = std::env::var("PATH") {
                c.env("PATH", val);
            }
            c.arg("-c");
            c.arg(&cmd_str);
            c
        };

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr".to_string())?;

        let output = Arc::new(Mutex::new(String::new()));
        let stderr_output = Arc::new(Mutex::new(String::new()));

        // Background tasks to stream stdout/stderr
        let out_clone = output.clone();
        let stdout_handle = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut out = out_clone.lock().await;
                out.push_str(&line);
                out.push('\n');
            }
        });

        let err_clone = stderr_output.clone();
        let stderr_handle = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut out = err_clone.lock().await;
                out.push_str(&line);
                out.push('\n');
            }
        });

        let cmd_id = format!(
            "cmd_{}_{}",
            session_id,
            chrono::Local::now().timestamp_millis()
        );
        let timeout_secs = self.command_timeout_secs;
        let cancel_flag_clone = self.cancel_flag.clone();

        let cancel_fut = async move {
            while !cancel_flag_clone.load(std::sync::atomic::Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };

        // Wait for completion, timeout, or cancellation
        tokio::select! {
            res = child.wait() => {
                match res {
                    Ok(status) => {
                        let _ = stdout_handle.await;
                        let _ = stderr_handle.await;
                        let stdout_text = output.lock().await.clone();
                        let stderr_text = stderr_output.lock().await.clone();
                        let code = status.code().unwrap_or(-1);
                        Ok(format!(
                            "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                            code, stdout_text, stderr_text
                        ))
                    }
                    Err(e) => {
                        let _ = child.kill().await;
                        Err(format!("Command failed: {}", e))
                    }
                }
            }
            _ = cancel_fut => {
                let _ = child.kill().await;
                Err("Command execution was explicitly cancelled by the user.".to_string())
            }
            _ = tokio::time::sleep(Duration::from_secs(timeout_secs)) => {
                let partial = output.lock().await.clone();
                self.running_commands.insert(
                    cmd_id.clone(),
                    RunningCommand {
                        child,
                        output: output.clone(),
                        stderr_output: stderr_output.clone(),
                        stdout_handle: Some(stdout_handle),
                        stderr_handle: Some(stderr_handle),
                        start_time: std::time::Instant::now(),
                        command: cmd_str.clone(),
                    },
                );

                Ok(format!(
                    "COMMAND_STILL_RUNNING\nCommand ID: {}\nCommand: {}\nThe command is still running after {} seconds.\nUse `check_command_status` with the Command ID to check if it has finished.\n\nPartial output so far:\n{}",
                    cmd_id, cmd_str, timeout_secs, partial
                ))
            }
        }
    }

    /// Check the status of a long-running command that was started with execute_command
    async fn execute_check_command_status(
        &mut self,
        tool: &parser::ToolCall,
    ) -> Result<String, String> {
        self.prune_running_commands();
        let cmd_id = tool.arguments["command_id"]
            .as_str()
            .ok_or_else(|| "Missing 'command_id' argument".to_string())?;

        let running = self.running_commands.get_mut(cmd_id).ok_or_else(|| {
            format!(
                "No running command found with ID: '{}'. It may have already completed or expired.",
                cmd_id
            )
        })?;

        let elapsed = running.start_time.elapsed().as_secs();

        match running.child.try_wait() {
            Ok(Some(status)) => {
                // Command has finished
                if let Some(h) = running.stdout_handle.take() {
                    let _ = h.await;
                }
                if let Some(h) = running.stderr_handle.take() {
                    let _ = h.await;
                }
                let stdout_text = running.output.lock().await.clone();
                let stderr_text = running.stderr_output.lock().await.clone();
                let code = status.code().unwrap_or(-1);

                self.running_commands.remove(cmd_id);

                Ok(format!(
                    "COMMAND_COMPLETED\nExit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                    code, stdout_text, stderr_text
                ))
            }
            Ok(None) => {
                // Still running
                let partial = running.output.lock().await.clone();
                let wait_time = (elapsed as f64 * 2.0).min(300.0) as u64; // exponential backoff, max 5 min
                Ok(format!(
                    "COMMAND_STILL_RUNNING\nCommand ID: {}\nCommand: {}\nElapsed: {}s\nThe command is still running. Try again in about {}s.\n\nPartial output so far:\n{}",
                    cmd_id, running.command, elapsed, wait_time, partial
                ))
            }
            Err(e) => {
                self.running_commands.remove(cmd_id);
                Err(format!("Failed to check command status: {}", e))
            }
        }
    }

    /// Force kill a running command by ID
    async fn execute_kill_command(&mut self, tool: &parser::ToolCall) -> Result<String, String> {
        let cmd_id = tool.arguments["command_id"]
            .as_str()
            .ok_or_else(|| "Missing 'command_id' argument".to_string())?;

        if let Some(mut running) = self.running_commands.remove(cmd_id) {
            match running.child.kill().await {
                Ok(_) => Ok(format!(
                    "✓ Command '{}' (ID: {}) has been terminated.",
                    running.command, cmd_id
                )),
                Err(e) => Err(format!("Failed to kill command: {}", e)),
            }
        } else {
            Err(format!(
                "No running command found with ID: '{}'. It may have already completed or expired.",
                cmd_id
            ))
        }
    }

    fn execute_save_memory(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let name = tool.arguments["name"]
            .as_str()
            .ok_or_else(|| "Missing 'name' argument".to_string())?;
        let description = tool.arguments["description"]
            .as_str()
            .unwrap_or("Memory entry");
        let mem_type = match tool.arguments["type"].as_str().unwrap_or("reference") {
            "user" => memory::MemoryType::User,
            "feedback" => memory::MemoryType::Feedback,
            "project" => memory::MemoryType::Project,
            _ => memory::MemoryType::Reference,
        };
        let content = tool.arguments["content"]
            .as_str()
            .ok_or_else(|| "Missing 'content' argument".to_string())?;

        let path = self
            .memory_manager
            .save_memory(name, description, mem_type, content)?;
        Ok(format!("✓ Memory saved: {}", path.display()))
    }

    fn execute_search_memory(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let query = tool.arguments["query"]
            .as_str()
            .ok_or_else(|| "Missing 'query' argument".to_string())?;

        // Use ranked search for better relevance
        let scored_results = self.memory_manager.search_memories_ranked(query, 0.0);
        if scored_results.is_empty() {
            return Ok(format!("No memories found matching '{}'.", query));
        }

        let total = scored_results.len();
        // Show top 10 most relevant results
        let display_limit = 10usize;
        let shown = scored_results.len().min(display_limit);

        let mut output = format!(
            "Found {} memories for '{}' (showing top {} by relevance):\n",
            total, query, shown
        );
        for (i, (mem, score)) in scored_results.iter().take(display_limit).enumerate() {
            let relevance = if *score >= 5.0 {
                "HIGH"
            } else if *score >= 2.0 {
                "MEDIUM"
            } else {
                "LOW"
            };
            output.push_str(&format!(
                "\n{}. **{}** [{:?}] [relevance: {:.1} — {}]\n   _{}\n",
                i + 1,
                mem.name,
                mem.metadata.mem_type,
                score,
                relevance,
                mem.description
            ));
            // Show first 3 lines of content
            let preview: String = mem.content.lines().take(3).collect::<Vec<_>>().join("\n");
            if !preview.is_empty() {
                output.push_str(&format!("   {}\n", preview));
            }
        }

        if total > display_limit {
            output.push_str(&format!(
                "\n... and {} more results. Refine your query for precision.\n",
                total - display_limit
            ));
        }

        Ok(output)
    }

    fn execute_compact_history(&self, _tool: &parser::ToolCall) -> Result<String, String> {
        Ok("History compaction requested. Use compaction::CompactionManager for structured summaries.".to_string())
    }

    // ─── Skill Tool Executions ───────────────────────────────────────────

    fn execute_scan_subdirectory(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let path = tool.arguments["path"]
            .as_str()
            .ok_or_else(|| "Missing 'path' argument".to_string())?;
        let tree = self.skill_engine.scan_subdirectory(path)?;
        Ok(tree.tree_string)
    }

    fn execute_search_files(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let pattern = tool.arguments["pattern"]
            .as_str()
            .or_else(|| tool.arguments["query"].as_str())
            .ok_or_else(|| "Missing 'pattern' argument".to_string())?;

        let result = self.skill_engine.search_files(pattern);
        if result.matches.is_empty() {
            return Ok(format!("📁 No files found matching '{}'", pattern));
        }

        let mut output = format!(
            "📁 Found {} file(s) matching '{}':\n\n",
            result.total, pattern
        );
        for (i, path) in result.matches.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", i + 1, path.display()));
        }
        output.push_str(&format!("\n--- {} file(s) total ---", result.total));
        Ok(output)
    }

    fn execute_extract_symbol(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let file = tool.arguments["file"]
            .as_str()
            .ok_or_else(|| "Missing 'file' argument".to_string())?;
        let symbol = tool.arguments["symbol"]
            .as_str()
            .or_else(|| tool.arguments["name"].as_str())
            .ok_or_else(|| "Missing 'symbol' argument".to_string())?;

        let extraction = self.skill_engine.extract_symbol(file, symbol)?;

        let mut output = String::new();
        output.push_str(&format!(
            "📖 Symbol: **{}** ({}) | File: `{}` | Lines {}-{}\n\n",
            extraction.symbol,
            extraction
                .symbol_type
                .unwrap_or_else(|| "unknown".to_string()),
            extraction.file.display(),
            extraction.start_line,
            extraction.end_line
        ));

        // Context before
        if !extraction.context_before.is_empty() {
            output.push_str("Context before:\n");
            for line in extraction.context_before.lines() {
                output.push_str(&format!("  {}\n", line));
            }
            output.push('\n');
        }

        // The actual code block
        output.push_str("```\n");
        // Add line numbers
        for (i, line) in extraction.code_block.lines().enumerate() {
            let line_num = extraction.start_line + i;
            output.push_str(&format!("{:>6} | {}\n", line_num, line));
        }
        output.push_str("```\n");

        output.push_str(&format!(
            "\n--- Extracted {} lines ({}:{}-{}) ---",
            extraction.code_block.lines().count(),
            extraction.file.display(),
            extraction.start_line,
            extraction.end_line
        ));
        Ok(output)
    }

    fn execute_read_file_range(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let file = tool.arguments["file"]
            .as_str()
            .ok_or_else(|| "Missing 'file' argument".to_string())?;
        let start = tool.arguments["start_line"]
            .as_u64()
            .ok_or_else(|| "Missing 'start_line' argument".to_string())?
            as usize;
        let end = tool.arguments["end_line"]
            .as_u64()
            .ok_or_else(|| "Missing 'end_line' argument".to_string())? as usize;

        self.skill_engine.read_file_range(file, start, end)
    }

    fn execute_search_symbol(&self, tool: &parser::ToolCall) -> Result<String, String> {
        let symbol = tool.arguments["symbol"]
            .as_str()
            .or_else(|| tool.arguments["name"].as_str())
            .ok_or_else(|| "Missing 'symbol' argument".to_string())?;

        let results = self.skill_engine.search_symbol_across_project(symbol);
        if results.is_empty() {
            return Ok(format!(
                "🔍 Symbol '{}' not found anywhere in project.",
                symbol
            ));
        }

        let mut output = format!(
            "🔍 Found symbol **'{}'** in {} file(s):\n\n",
            symbol,
            results.len()
        );
        for (i, (path, line_num, line_text)) in results.iter().enumerate() {
            output.push_str(&format!(
                "  {}. `{}:{}` → {}\n",
                i + 1,
                path.display(),
                line_num,
                line_text
            ));
        }
        output.push_str(&format!(
            "\nUse `extract_symbol` with the file path and symbol name to see the full definition."
        ));
        Ok(output)
    }

    // ─── Subagent Management ─────────────────────────────────────────────

    /// Spawn a subagent for coordinator mode
    pub fn spawn_subagent(
        &mut self,
        id: &str,
        name: &str,
        description: &str,
        subagent_type: Option<String>,
        prompt: &str,
    ) {
        self.subagents.insert(
            id.to_string(),
            Subagent {
                id: id.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                subagent_type,
                status: SubagentStatus::Running,
                prompt: prompt.to_string(),
                result: None,
            },
        );
    }

    /// Mark a subagent as completed with result
    pub fn complete_subagent(&mut self, id: &str, result: String) -> Result<(), String> {
        if let Some(agent) = self.subagents.get_mut(id) {
            agent.status = SubagentStatus::Completed;
            agent.result = Some(result);
            Ok(())
        } else {
            Err(format!("Subagent '{}' not found", id))
        }
    }

    /// Mark a subagent as failed
    pub fn fail_subagent(&mut self, id: &str, error: String) -> Result<(), String> {
        if let Some(agent) = self.subagents.get_mut(id) {
            agent.status = SubagentStatus::Failed(error);
            Ok(())
        } else {
            Err(format!("Subagent '{}' not found", id))
        }
    }

    /// Get status of all subagents
    pub fn subagent_statuses(&self) -> Vec<&Subagent> {
        self.subagents.values().collect()
    }

    // ─── Plan Mode ───────────────────────────────────────────────────────

    /// Create a new plan
    pub fn create_plan(&mut self, title: &str) -> plan::Plan {
        plan::Plan::new(title)
    }

    // ─── Agent State Machine (State Machine Engine) ───────────────────
    //
    // Thay thế vòng lặp for tuần tự bằng State Machine:
    //   tick() kiểm tra AgentState và thực hiện ĐÚNG MỘT bước.
    //   Không có vòng lặp — mỗi lần gọi tick() = một transition.
    //
    // Flow:
    //   Idle → Thinking → (ToolCalls? → ExecutingTool | Text → Finished)
    //   ExecutingTool → Thinking
    //   Thinking → AwaitingApproval (nếu cần user duyệt)
    //   AwaitingApproval → ExecutingTool (khi user approve)
    //                    → Thinking (khi user reject)
    //
    /// Kiểm tra xem tool có auto-approve không dựa trên policy
    fn tool_needs_approval(
        tool: &parser::ToolCall,
        auto_approve_all: bool,
        auto_approve_reads: bool,
        auto_approve_writes: bool,
    ) -> bool {
        if auto_approve_all {
            return false;
        }
        let is_read_tool = matches!(
            tool.name.as_str(),
            "check_port"
                | "read_file"
                | "get_project_files"
                | "scan_directory_tree"
                | "scan_subdirectory"
                | "search_files"
                | "extract_symbol"
                | "read_file_range"
                | "search_symbol"
                | "search_memory"
        );
        let is_write_tool = matches!(tool.name.as_str(), "write_file" | "save_memory");

        if auto_approve_reads && is_read_tool {
            return false;
        }
        if auto_approve_writes && (is_read_tool || is_write_tool) {
            return false;
        }
        true
    }

    /// State Machine: thực hiện một bước dựa trên trạng thái hiện tại của session
    ///
    /// # Arguments
    /// * `session` - Session chứa history, state, config
    /// * `system_prompt` - System prompt
    /// * `llm_call_fn` - Closure gọi LLM API, nhận (system_prompt, history) → trả về LlmResponse
    ///
    /// # Returns
    /// `TickResult` cho biết bước tiếp theo là gì
    pub async fn tick<F1, Fut>(
        &mut self,
        session: &mut AgentSession,
        system_prompt: &str,
        llm_call_fn: F1,
    ) -> TickResult
    where
        F1: Fn(String, Vec<ChatMessage>) -> Fut,
        Fut: Future<Output = Result<LlmResponse, String>>,
    {
        // Kiểm tra token budget
        if session.token_budget == 0 {
            let err_msg =
                "Token budget exhausted! Self-healing loop broken to prevent excessive costs."
                    .to_string();
            session.state = AgentState::Error(err_msg.clone());
            return TickResult::Error {
                message: err_msg,
                iteration: session.iteration_count,
            };
        }

        // Kiểm tra max iterations
        if session.iteration_count >= session.max_iterations {
            session.state = AgentState::Finished(format!(
                "Đạt giới hạn vòng lặp ({} iterations)",
                session.max_iterations
            ));
            if let AgentState::Finished(ref text) = session.state {
                return TickResult::Finished {
                    text: text.clone(),
                    total_iterations: session.iteration_count,
                };
            }
        }

        match session.state.clone() {
            // ─── STATE: Idle ──────────────────────────────────────────────
            AgentState::Idle => {
                // Chuyển sang Thinking và gọi đệ quy tick() ngay
                session.state = AgentState::Thinking;
                Box::pin(self.tick(session, system_prompt, llm_call_fn)).await
            }

            // ─── STATE: Thinking ──────────────────────────────────────────
            AgentState::Thinking => {
                // Compact history nếu quá dài
                let max_msgs = if session.history.len() > 80 { 40 } else { 80 };
                Self::compact_history(&mut session.history, max_msgs);

                // Gọi LLM — nhận LlmResponse có cấu trúc (native tool_calls)
                let iteration = session.iteration_count + 1;
                let llm_reply =
                    match llm_call_fn(system_prompt.to_string(), session.history.clone()).await {
                        Ok(reply) => reply,
                        Err(e) => {
                            session.state = AgentState::Error(e.clone());
                            return TickResult::Error {
                                message: format!("LLM call failed: {}", e),
                                iteration,
                            };
                        }
                    };

                session.iteration_count = iteration;

                // Deduct tokens from budget based on estimation
                let est_input = (system_prompt.len()
                    + session
                        .history
                        .iter()
                        .map(|m| match &m.content {
                            MessageContent::Text(t) => t.len(),
                            MessageContent::ToolCalls(tcs) => tcs
                                .iter()
                                .map(|tc| tc.name.len() + tc.raw_arguments.len())
                                .sum(),
                            MessageContent::ToolResult { result, .. } => result.len(),
                        })
                        .sum::<usize>()) as u64
                    / 4;
                let est_output = llm_reply.raw_text.len() as u64 / 4;
                session.token_budget = session.token_budget.saturating_sub(est_input + est_output);

                if session.token_budget == 0 {
                    let err_msg = "Token budget exhausted! Self-healing loop broken to prevent excessive costs.".to_string();
                    session.state = AgentState::Error(err_msg.clone());
                    return TickResult::Error {
                        message: err_msg,
                        iteration,
                    };
                }

                // Use native tool_calls from LlmResponse directly
                // Fallback: parse raw_text only if no native tool_calls
                let all_tools = if !llm_reply.tool_calls.is_empty() {
                    llm_reply.tool_calls.clone()
                } else {
                    let parsed = parser::parse_model_response(&llm_reply.raw_text);
                    if !parsed.tool_calls.is_empty() {
                        parsed.tool_calls.clone()
                    } else if let Some(ref tc) = parsed.tool_call {
                        vec![tc.clone()]
                    } else {
                        vec![]
                    }
                };

                if !all_tools.is_empty() {
                    // Có tool calls → kiểm tra cần approve không
                    let any_needs_approval = all_tools.iter().any(|t| {
                        Self::tool_needs_approval(
                            t, false, // auto_approve_all — lấy từ session context
                            true,  // auto_approve_reads — default true
                            false, // auto_approve_writes — default false
                        )
                    });

                    // Lưu assistant message với ToolCalls vào history
                    session.history.push(ChatMessage {
                        id: format!("model_{}", chrono::Local::now().timestamp_millis()),
                        role: "assistant".to_string(),
                        content: MessageContent::ToolCalls(all_tools.clone()),
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });

                    if any_needs_approval {
                        // Dừng lại, chờ user approve
                        session.state = AgentState::AwaitingApproval(all_tools.clone());
                        return TickResult::WaitForApproval {
                            tools: all_tools,
                            iteration,
                        };
                    } else {
                        // Tự động chạy tool
                        session.state = AgentState::ExecutingTool;
                        Box::pin(self.tick(session, system_prompt, llm_call_fn)).await
                    }
                } else {
                    // Text response — hoan thanh
                    let text = llm_reply
                        .text
                        .unwrap_or_else(|| "Task completed.".to_string());

                    session.history.push(ChatMessage {
                        id: format!("model_{}", chrono::Local::now().timestamp_millis()),
                        role: "assistant".to_string(),
                        content: MessageContent::Text(text.clone()),
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });

                    session.state = AgentState::Finished(text.clone());
                    return TickResult::Finished {
                        text,
                        total_iterations: session.iteration_count,
                    };
                }
            }

            // ─── STATE: ExecutingTool ─────────────────────────────────────
            AgentState::ExecutingTool => {
                let session_id = session.session_id.clone();

                // Tìm tool calls từ message cuối cùng của assistant
                let tool_calls: Vec<parser::ToolCall> = session
                    .history
                    .iter()
                    .rev()
                    .find_map(|msg| {
                        if msg.role == "assistant" {
                            match &msg.content {
                                MessageContent::ToolCalls(tcs) => Some(tcs.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                if tool_calls.is_empty() {
                    session.state = AgentState::Error(
                        "ExecutingTool state but no tool calls found in history".to_string(),
                    );
                    return TickResult::Error {
                        message: "No tool calls to execute".to_string(),
                        iteration: session.iteration_count,
                    };
                }

                // Thực thi từng tool
                for tool in &tool_calls {
                    let tool_result = self.execute_tool(&session_id, tool).await;
                    let (result_text, success) = match tool_result {
                        Ok(r) => (r, true),
                        Err(e) => (e, false),
                    };

                    let call_id = tool
                        .call_id
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());

                    // Push result vào history
                    session.history.push(ChatMessage {
                        id: call_id.clone(),
                        role: "tool".to_string(),
                        content: MessageContent::ToolResult {
                            tool_call_id: call_id,
                            tool_name: tool.name.clone(),
                            result: result_text.clone(),
                            success,
                        },
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });

                    if !success {
                        let is_retryable =
                            self_heal::SelfHealAnalyzer::should_retry(&tool.name, &result_text);
                        if !is_retryable {
                            let err_msg = format!(
                                "Environmental or non-retryable error encountered: {}",
                                result_text
                            );
                            session.state = AgentState::Error(err_msg.clone());
                            return TickResult::Error {
                                message: err_msg,
                                iteration: session.iteration_count,
                            };
                        } else {
                            // Exponential backoff: sleep to avoid spamming target services/API keys
                            let delay_ms = 2_u64.pow(session.iteration_count.min(6)) * 250;
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                }

                // Chạy xong tool → quay lại Thinking
                session.state = AgentState::Thinking;
                Box::pin(self.tick(session, system_prompt, llm_call_fn)).await
            }

            // ─── STATE: AwaitingApproval ──────────────────────────────────
            AgentState::AwaitingApproval(tools) => {
                // Không thể tick khi đang chờ approve
                TickResult::WaitForApproval {
                    tools,
                    iteration: session.iteration_count,
                }
            }

            // ─── STATE: Finished ───────────────────────────────────────────
            AgentState::Finished(text) => TickResult::Finished {
                text,
                total_iterations: session.iteration_count,
            },

            // ─── STATE: Error ─────────────────────────────────────────────
            AgentState::Error(msg) => TickResult::Error {
                message: msg,
                iteration: session.iteration_count,
            },

            // ─── STATE: Verifying ─────────────────────────────────────────
            AgentState::Verifying => {
                // Placeholder: dual-verification (sẽ mở rộng sau)
                // Hiện tại coi như Finished
                let text = "Task verified.".to_string();
                session.state = AgentState::Finished(text.clone());
                TickResult::Finished {
                    text,
                    total_iterations: session.iteration_count,
                }
            }
        }
    }

    // ─── Memory Management ───────────────────────────────────────────────

    pub fn memory_manager(&self) -> &memory::MemoryManager {
        &self.memory_manager
    }

    pub fn skill_engine(&self) -> &skills::SkillEngine {
        &self.skill_engine
    }

    pub fn telemetry_manager(&self) -> &telemetry::TelemetryManager {
        &self.telemetry
    }

    // ─── Telemetry ───────────────────────────────────────────────────────

    pub fn get_performance_metrics(&self, session_id: &str) -> telemetry::PerformanceMetrics {
        self.telemetry.get_metrics(session_id)
    }

    pub fn get_aggregate_metrics(&self) -> telemetry::PerformanceMetrics {
        self.telemetry.get_aggregate_metrics()
    }

    // ─── History Compaction ──────────────────────────────────────────────

    pub fn compact_history(history: &mut Vec<ChatMessage>, max_messages: usize) {
        if history.len() <= max_messages {
            return;
        }

        // Always keep the first user message as context anchor
        let first_user = history.iter().find(|m| m.role == "user").cloned();

        let target_len = max_messages.saturating_sub(1); // -1 to make room for first_user
        let tail_start = history.len().saturating_sub(target_len);

        // Find a safe cut point: walk backward from tail_start to find the nearest "user" message.
        // Starting from a "user" message guarantees we do not break any assistant-tool or tool-result sequences.
        let mut safe_start = tail_start;
        for i in (0..=tail_start).rev() {
            if history[i].role == "user" {
                safe_start = i;
                break;
            }
        }

        let mut compacted: Vec<ChatMessage> = Vec::new();
        if let Some(first) = first_user {
            // Only add first_user if it's not already included in the safe_start.. range
            if safe_start == 0 || history[safe_start..].iter().all(|m| m.id != first.id) {
                compacted.push(first);
            }
        }
        compacted.extend(history[safe_start..].iter().cloned());
        *history = compacted;
    }

    // ─── Helpers ─────────────────────────────────────────────────────────

    fn load_or_default(&self, filename: &str, default: &str) -> String {
        let path = self
            .workspace_root
            .join("core_engine/src/agent_harness/prompts")
            .join(filename);
        fs::read_to_string(&path).unwrap_or_else(|_| default.to_string())
    }

    fn get_git_branch(&self) -> String {
        let output = std::process::Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.workspace_root)
            .output();
        if let Ok(out) = output {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            String::new()
        }
    }

    fn get_git_status(&self) -> String {
        let output = std::process::Command::new("git")
            .args(&["status", "--short"])
            .current_dir(&self.workspace_root)
            .output();
        if let Ok(out) = output {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            String::new()
        }
    }

    fn list_dir_recursive(
        &self,
        dir: &Path,
        files: &mut Vec<String>,
        current_depth: usize,
    ) -> Result<(), String> {
        const MAX_DEPTH: usize = 8;
        const MAX_FILES: usize = 1000;

        if current_depth > MAX_DEPTH {
            return Ok(());
        }

        if files.len() >= MAX_FILES {
            return Ok(());
        }

        if dir.is_dir() {
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                if files.len() >= MAX_FILES {
                    break;
                }
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();

                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str == ".git"
                        || name_str == ".idea"
                        || name_str == "target"
                        || name_str == "node_modules"
                        || name_str == "logs"
                    {
                        continue;
                    }
                }

                if path.is_dir() {
                    self.list_dir_recursive(&path, files, current_depth + 1)?;
                } else {
                    if let Ok(rel) = path.strip_prefix(&self.workspace_root) {
                        files.push(rel.display().to_string());
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_history_safe_cut() {
        let mut history = vec![
            ChatMessage {
                id: "1".to_string(),
                role: "user".to_string(),
                content: MessageContent::Text("hello".to_string()),
                timestamp: "".to_string(),
            },
            ChatMessage {
                id: "2".to_string(),
                role: "assistant".to_string(),
                content: MessageContent::ToolCalls(vec![]),
                timestamp: "".to_string(),
            },
            ChatMessage {
                id: "3".to_string(),
                role: "tool".to_string(),
                content: MessageContent::ToolResult {
                    tool_call_id: "x".to_string(),
                    tool_name: "read_file".to_string(),
                    result: "result".to_string(),
                    success: true,
                },
                timestamp: "".to_string(),
            },
            ChatMessage {
                id: "4".to_string(),
                role: "user".to_string(),
                content: MessageContent::Text("user message 2".to_string()),
                timestamp: "".to_string(),
            },
            ChatMessage {
                id: "5".to_string(),
                role: "assistant".to_string(),
                content: MessageContent::ToolCalls(vec![]),
                timestamp: "".to_string(),
            },
            ChatMessage {
                id: "6".to_string(),
                role: "tool".to_string(),
                content: MessageContent::ToolResult {
                    tool_call_id: "y".to_string(),
                    tool_name: "write_file".to_string(),
                    result: "written".to_string(),
                    success: true,
                },
                timestamp: "".to_string(),
            },
        ];

        AgentHarness::compact_history(&mut history, 4);

        assert_eq!(history.len(), 4);
        assert_eq!(history[0].id, "1");
        assert_eq!(history[1].id, "4");
        assert_eq!(history[2].id, "5");
        assert_eq!(history[3].id, "6");
    }
}
