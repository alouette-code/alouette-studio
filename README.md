# Alouette Studio

**Integrated Development Studio & Native Process Orchestrator** — a high-performance desktop workspace for running and managing isolated development environments. Built with enterprise-grade toolchain isolation, a 3-tier Sandbox command protector, an Environment Simulator (firewall, latency, packet loss, CPU/RAM limits), a Monaco-based Split Editor, a SQLite database browser, a robust MiniPostman API client, a full Git UI, a Cloudflare Tunnel manager, and an integrated AI Agent harness with multi-provider LLM support.

Built with **Rust + Tauri v2 + React 19 + TypeScript**.

---

## Features

### 1. Process Management & Workspace Orchestration
- **Lifecycle Control:** Register, start, stop, and deregister project processes seamlessly via IPC commands.
- **Process Guard & Restart:** Automatic process health monitoring and restart with exponential backoff (configurable).
- **Setup Hooks:** Define and run custom setup command chains prior to main process execution.
- **Deep Termination:** Safe force-kill mechanics utilizing recursive process tree traversal to prevent orphan processes.
- **Port Scanner:** Automated port diagnostic scanner (`netstat` on Windows, `lsof` on Unix) to identify conflicting bindings.
- **Network Isolation:** Process-level network interface isolation via OS firewall rules.
- **Workspace Auto-Cloning:** Git clone or local directory copy for isolated workspace preparation (`workspace_manager`).
- **PTY & Log Terminal Modes:** Choose between full interactive PTY terminals or log-only output mode per project.

### 2. Sandbox — 3-Tier Command Protection
A comprehensive security system designed to prevent destructive command execution (e.g., `rm -rf /`, `Format C:`):
- **Tier 1a (Semantic Interceptor):** Performs deep semantic parsing on command trees, classifies risk levels, resolves environment variables/relative paths, normalizes Unicode homoglyphs, blocks .NET file operation patterns, parses PowerShell subexpressions, and rejects boundary escape attempts.
- **Tier 1b (Engine):** Fallback path resolution engine that strips Windows `\\?\` prefixes and performs case-insensitive boundary validations.
- **Tier 2 (OS-level Isolation):** Integrates system-level sandboxing (AppContainer on Windows, landlock/seccomp placeholders on Linux).
- **PTY Hooking:** All terminal inputs are intercepted at `terminal.rs` via `sandbox::check_command()` before execution.

#### Sandbox UI Control Center
The Sandbox dashboard consists of 5 synchronized control modules:
- *Tab All (Top-Right):* One-click master switch to enable/disable sandbox globally.
- *Setup Details (Center-Left):* Sub-feature configuration toggles for Terminal, Browser, Engine, and Setup.
- *Search (Middle-Right):* Instant filtering for imported projects.
- *Imported Projects List (Bottom-Right):* Per-project activation state checkboxes.
- *Section Tabs (Top-Left):* Switch between Terminal, Browser, Engine, Setup, and Environment Simulation configs.

### 3. Environment Simulation Panel
- **Testing Simulator:** Opened via the Server icon in the status bar (routes to a virtual tab `__environment__`).
- **4 Degradation Groups:**
  - *Firewall:* Emulate port blocks and custom firewall rules with glob-pattern matching (e.g. `*.google.com`).
  - *Weak Network:* Introduce artificial packet loss, latency, jitter, and bandwidth throughput caps via a built-in SOCKS5/HTTP forward proxy.
  - *Unstable Server:* Simulate periodic connection drops, HTTP error code injection (500, 502, 503, etc.), and random connection rejection via a reverse proxy gateway.
  - *Resource Limits:* Enforce hard hardware utilization boundaries (CPU %, RAM MB) with watchdog termination.
- **Debounced Persistence:** Configurations are automatically saved to YAML (`env_simulation.yml`) with an 800ms debounce.

### 4. File Editor & Split Editor Panes
- **Monaco Integration:** Full-featured Monaco Editor with rich syntax highlighting, search, and formatting.
- **State Recovery:** Automatic scroll position and cursor position preservation when switching between tabs.
- **Directory Explorer:** Interactive recursive directory tree with file/folder creation, deletion, and search capability.
- **Split Editors:** Right-click on the Tab bar to split the editor into up to 3 side-by-side active panes. Click to activate focus, and drag-and-drop tabs between panes easily.
- **Binary Transfer:** Safe Base64 encoding for local binary file transfer over Tauri IPC.

### 5. MiniPostman API Client & Network Diagnostics
- **MiniPostman API Client:** Fully embedded REST client:
  - *Methods:* GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD.
  - *Payloads:* Query params, custom headers, multipart/form-data, GraphQL, binary, and URL-encoded request bodies.
  - *Auth:* Bearer, Basic, API Key, OAuth 2.0, AWS Signature.
  - *Response Inspector:* Visual formatters, headers/cookies decoder, redirect chain viewer, timing breakdown (DNS lookup, TCP connect, TLS handshake, first byte, total).
  - *Timing Breakdown:* Real-time DNS resolution time, TCP connection time, TLS handshake time, time-to-first-byte.
  - *Post-request Scripts:* Automation script runner with testing assertions.
  - *Pre-request Scripts:* Dynamic request modification before execution.
  - *Collections & Environments:* Save requests in collections and manage environment variables per workspace.
  - *cURL Generator:* One-click export of any HTTP request to a cURL command.
  - *Response Diff:* Side-by-side comparison of two HTTP responses.
- **Helper Toolbox:**
  - **DNS Lookup:** Full DNS record analysis (A, AAAA, MX, TXT, CNAME, NS, SOA).
  - **SSL Certificate Inspector:** View certificate chain, issuer, subject, validity dates, and SANs.
  - **Ping Host:** Custom ICMP ping with configurable count and timeout.
  - **JWT Decoder:** Decode JWT tokens with header/payload/signature breakdown.
  - **Hash Builder:** MD5, SHA-1, SHA-256, SHA-384, SHA-512 hashing.
  - **Base64 Converter:** Encode/decode Base64 strings.
  - **JSON Formatter & Validator:** Prettify, minify, and validate JSON with JSON Schema support.
  - **XML Prettifier:** Format and syntax-highlight XML documents.
  - **HTTP Status Code Guide:** Lookup HTTP status codes with descriptions.
  - **Timestamp Converter:** Convert between Unix timestamps and human-readable dates.
  - **cURL Generator:** Auto-generate cURL commands from request parameters.

### 6. Git Management Panel (Git UI)
- **Native Sidebar:** Git panel toggled via the branch icon in the status bar.
- **Metadata Monitor:** Real-time active branch and remote repository tracking.
- **Staging Area:** Lists files separated into *Staged Changes* and unstaged *Changes* with per-file status indicators (modified, added, deleted, untracked, renamed).
- **Quick Controls:** Direct buttons to Stage (+), Unstage (-), Discard/Revert changes, Commit (with message), Push, Pull.
- **Commit History:** View last 30 commits with hash, author, date, and subject. Click to inspect changed files in each commit.

### 7. Resource Monitoring & Hardware Control
- **Watchdog Enforcement:** Tracks real-time CPU/RAM usage per process tree and displays live canvas-based performance charts.
- **Limit Enforcement:** Configures CPU % and RAM MB limits at project or global level. The system watchdog automatically terminates processes exceeding limits for more than 30 seconds.
- **Process Details:** View per-process CPU, RAM, PID, and child process tree information.
- **Title Bar Integration:** Database icon on the title bar grants quick access to uptime, hardware usage, simulated GPU metrics, and active ports.

### 8. SQLite Browser & Storage
- **R2D2 Connection Pooling:** High-performance concurrent SQLite operations using r2d2 connection pool with WAL (Write-Ahead Logging) mode.
- **Database Browser:** View all tables, inspect schemas, add columns, insert/delete rows, and edit individual cells directly.
- **Log Pruning:** Automatic system log truncation, keeping a clean history of the last N lines per project (configurable via `max_log_lines`).

### 9. Cloudflared Tunnel Integration
- **Cloudflare Zero Trust:** Auto-download and update the latest `cloudflared` binary on startup with offline local fallback.
- **Modes:** Tunnel Free (auto-links local project port with `trycloudflare.com` URL) or Named Tunnel (Token-based authentication).
- **Title Bar Status:** Cloudflare cloud icon displays in orange when active, gray when inactive. Quick-click to toggle the tunnel state immediately.
- **Process Sandboxing:** Cloudflared processes are sandboxed at the OS level via AppContainer (Windows) or landlock/seccomp (Linux).

### 10. System Manager & Global Configs
- **System Manager:** Administration panel for process supervision, background services, and core system diagnostics.
- **Global Settings:** Central configuration dashboard for:
  - *Appearance:* Theme (dark/light), language, font size, sidebar/panel dimensions.
  - *Logs:* Max log lines, auto-scroll, log filter.
  - *Performance:* History points, terminal output length, monitoring interval.
  - *Build:* Desktop single EXE, UPX compression, Android build tool (Gradle/Bazel), build target (Desktop/Android).
  - *System:* Keep-alive in system tray, auto-start on boot, run in background, CPU/RAM limits, auto-restart with configurable interval.
- **System Tray:** Application minimizes to system tray with Show/Hide and Quit menu. Double-click tray icon to toggle window visibility.
- **Zen Browser Integration:** Launch sandbox-isolated Zen Browser windows directly from the workspace (supports bundled `zen_bundle/`, resource-packed, and system-installed Zen Browser).
- **Window Close Interception:** When "keep alive" is enabled, closing the window hides it to tray instead of quitting.

### 11. AI Agent Loop Harness
- **Multi-Provider LLM Support:** Connects to any OpenAI-compatible API endpoint or native Rig LLM providers (OpenAI, Anthropic, Cohere, etc.) via `rig-core 0.38`.
- **Operation Modes:**
  - *Interactive:* Asks before executing write operations.
  - *Write:* Auto-approves file modification.
  - *Autonomous:* Fully auto-executes planned code changes.
  - *Plan Mode:* Generates multi-step plans before execution.
  - *Coordinator:* Coordinates sub-agents for complex tasks.
  - *Worker:* Executes delegated subtasks.
  - *Minimal:* Lightweight execution mode.
- **Sub-Agent Coordination:** Spawn sub-agents for parallel task execution with status tracking and result aggregation.
- **Tool System:** Full tool-calling support including:
  - Read/write/edit files, search code, explore directories.
  - Run commands and check command status.
  - Assess blast radius before operations.
  - Save/search short-term and long-term memory.
  - Spawn and manage sub-agents.
- **Thinking Mode:** Toggle High mode (forces deep model reasoning with `thinking_budget`) or Low mode (standard execution).
- **Blast Radius Assessment:** Automatic evaluation of potential impact before executing file modifications.
- **Self-Healing:** Automatic error recovery and retry logic on tool execution failures.
- **History Compaction:** Automatic history compression when exceeding 80 messages to manage token budgets.
- **Session Management:** Conversations stored in SQLite (`history_agen` table) via r2d2 connection pooling, searchable with pagination, one-click restore, and session deletion.
- **Multi-Session Registry:** Supports multiple simultaneous agent sessions with per-project isolation.
- **Markdown Rendering:** AI responses rendered with `react-markdown` + `rehype-sanitize` + `dompurify` for safe HTML rendering.
- **Cancel Support:** Real-time agent loop cancellation via atomic flag.

### 12. Auxiliary AI Diagnostic Companion (Alouette Open)
- **Background Log Scanner:** Executes `alouette_open-A1` ONNX models via `tract-onnx` at 500ms intervals combined with heuristic rules.
- **Alert System:** Emits warning alerts and diagnostic cards for quick fix actioning.
- **Toggle Control:** Enable/disable via IPC command with status query.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Tauri Desktop Shell                            │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    React Frontend (UI)                            │  │
│  │  Components | Hooks | Monaco | xterm.js | Charts | Lucide Icons │  │
│  │  react-markdown | fuse.js                                        │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  │              Tauri IPC Bridge (Commands + Events)                 │  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Core Engine (Rust)                              │  │
│  │  Process Manager | Sandbox | Proto | Cloudflared | R2D2 + SQLite │  │
│  │  Agent Harness (rig-core) | Alouette Open (tract-onnx)           │  │
│  │  Network Sim Proxy | System Manager | DashMap Registry           │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Responsibility |
|-------|-----------|----------------|
| **UI** | React 19 + TypeScript + Vite + xterm.js + Monaco + react-markdown | Desktop interface with process dashboards, terminals, file editor, SQLite browser, API client, admin panel, AI Agent chat |
| **Bridge** | Tauri v2 IPC | Type-safe command handlers (14 command modules), event routers (5 routers + agent), state management (R2D2 pool, DashMap registry) |
| **Engine** | Rust (Tokio async) | Process lifecycle, sandbox enforcement, proto toolchain isolation, resource monitoring (sysinfo), SQLite persistence (r2d2 WAL), AI agent loop (rig-core), ONNX inference (tract-onnx), network simulation proxy, system management |

---

## Project Structure

```
alouette_studio/
├── .gitignore
├── .prototools                       # Proto toolchain version pin
├── .taurignore
├── AI.json                            # AI assistant guidelines
├── Cargo.toml                         # Rust workspace root (core_engine + tauri_app)
├── README.md
├── logo_alouette.png
│
├── assets/                            # Static assets
│
├── scripts/                           # Utility scripts
│
├── docs/                              # Technical documentation
│   ├── README.md
│   ├── cloudflared_tunnels.md
│   ├── frontend_state.md
│   ├── interactive_terminals.md
│   ├── process_lifecycle.md
│   ├── proto_toolchain.md
│   ├── resource_monitoring.md
│   └── sqlite_storage.md
│
├── core_engine/                       # Rust library crate — core system logic
│   ├── Cargo.toml
│   ├── build.rs
│   ├── app_data/                      # Runtime data storage
│   │   ├── ai_config.yml
│   │   ├── alouette.db
│   │   ├── cloudflare_config.yml
│   │   ├── env_simulation.yml         # Simulated environment configs
│   │   ├── alouette_toolchains/       # Proto toolchain binaries
│   │   └── workspaces/                # Cloned/copied project workspaces
│   └── src/
│       ├── lib.rs                     # Crate root + public re-exports
│       ├── config.rs                  # TOML configuration + SandboxConfig + EnvSimulationConfig
│       ├── db.rs                      # SQLite persistence layer (r2d2 managed)
│       ├── monitor.rs                 # Resource monitoring (CPU/RAM per process tree)
│       ├── settings.rs                # Global app settings (JSON persistence)
│       ├── cloudflared_manager.rs     # Cloudflare tunnel binary management
│       ├── proto_manager.rs           # Proto toolchain download, install & env spoofing
│       ├── workspace_manager.rs       # Git clone / local copy workspace setup
│       ├── process/                   # Process lifecycle management
│       │   ├── mod.rs                 # Module declarations + re-exports
│       │   ├── models.rs              # ProcessState, ProcessLog, TerminalOutput, ProjectInstance
│       │   ├── manager.rs             # ProcessManager struct + CRUD + config
│       │   ├── executor.rs            # start_process, stop_process, force_fatal_stop
│       │   ├── terminal.rs            # spawn_terminal, write_terminal, kill_terminal, resize
│       │   ├── logging.rs             # Log rotation, stream piping, persistence
│       │   ├── tree.rs                # Process tree traversal, path utilities
│       │   ├── details.rs             # ChildProcessInfo, collect_child_processes
│       │   ├── network_isolate.rs     # Windows firewall per-PID block/unblock
│       │   ├── network_simulate_proxy.rs # SOCKS5/HTTP proxy: firewall, weak net, unstable server
│       │   └── sandbox/               # 3-tier command protection
│       │       ├── mod.rs             # Module entry + orchestration
│       │       ├── interceptor.rs     # Tier 1a — semantic command analysis
│       │       ├── engine.rs          # Tier 1b — token-based fallback
│       │       ├── windows.rs         # Tier 2 — AppContainer (Windows)
│       │       ├── linux.rs           # Tier 2 — landlock/seccomp (Linux)
│       │       └── macos.rs           # Tier 2 — placeholder (macOS)
│       ├── agent_harness/             # AI Agent Loop Engine
│       │   ├── mod.rs                 # AgentHarness::tick(), HarnessMode, AgentState, BlastRadius
│       │   ├── autonomous.rs          # Autonomous mode auto-approval logic
│       │   ├── compaction.rs          # History compaction when exceeding 80 messages
│       │   ├── hooks.rs               # Lifecycle hooks (pre/post tool execution)
│       │   ├── memory.rs              # Short-term + long-term memory management
│       │   ├── parser.rs              # LLM response parser (tool calls, text)
│       │   ├── plan.rs                # Multi-step plan generation & execution
│       │   ├── self_heal.rs           # Error recovery & retry logic
│       │   ├── skills.rs              # Skill tool implementations (scan, search, extract)
│       │   ├── session.rs             # SessionEntry, AgentSwitchInfo, SWITCH_SEQUENCE
│       │   ├── telemetry.rs           # Execution metrics & performance tracking
│       │   ├── tool_definitions.rs    # Tool definitions for LLM function calling
│       │   └── prompts/               # System prompts for LLM
│       │       ├── identity.txt       # AI identity / role prompt
│       │       └── tools.txt          # Tool descriptions prompt
│       └── bin/
│           └── test_parser.rs         # Standalone parser test binary
│
├── tauri_app/                         # Tauri v2 desktop application
│   ├── .taurignore
│   ├── zen_bundle/                    # Zen Browser portable bundle (gitignored)
│   ├── logs/                          # Runtime application logs
│   ├── src-tauri/                     # Tauri Rust binary crate
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── tauri.conf.json            # Tauri configuration (undecorated window, tray)
│   │   ├── .prototools                # Proto toolchain version pin
│   │   ├── capabilities/
│   │   │   └── default.json
│   │   ├── icons/
│   │   ├── logs/
│   │   ├── app_data/                  # Runtime data (gitignored)
│   │   └── src/
│   │       ├── main.rs                # Entry point + Tauri setup + exit cleanup
│   │       ├── alouette_open.rs       # ONNX + heuristic log monitor
│   │       ├── events.rs              # 5 background event router tasks
│   │       ├── state.rs               # AppState (R2D2 pool, DashMap registry, Arc<Mutex>)
│   │       ├── system_manager.rs      # System tray, autostart, keep-alive, auto-restart
│   │       └── commands/              # 14 Tauri IPC command modules
│   │           ├── mod.rs
│   │           ├── agent.rs           # Agent session lifecycle, history CRUD, streaming
│   │           ├── browser.rs         # Zen Browser launcher (bundled / system)
│   │           ├── cloudflare.rs      # Cloudflare config load/save
│   │           ├── files.rs           # File explorer + CRUD
│   │           ├── git.rs             # Full git operations (status, stage, commit, push, pull, log)
│   │           ├── language.rs        # Proto runtime management (list, install, delete)
│   │           ├── network.rs         # HTTP client, DNS, ping, SSL, JWT, hash, base64, etc.
│   │           ├── process.rs         # Start/stop/register/deregister projects
│   │           ├── rig_bridge.rs      # Rig LLM + OpenAI-compatible API streaming bridge
│   │           ├── sandbox.rs         # Sandbox config & env simulation CRUD
│   │           ├── settings.rs        # Global settings get/save/reset
│   │           ├── sqlite.rs          # SQLite table browser + CRUD
│   │           └── terminal.rs        # PTY terminal session management (spawn, write, kill, resize)
│   │
│   └── ui/                            # React frontend (Vite)
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── index.html
│       └── src/
│           ├── main.tsx
│           ├── App.tsx                # Application orchestrator
│           ├── index.css              # Global styles + CSS variables + dark/light theme
│           ├── constants.ts           # Mock data + default values + side panel dimensions
│           ├── vite-env.d.ts
│           ├── types/
│           │   └── index.ts           # TypeScript interfaces (Project, TerminalState, etc.)
│           ├── hooks/
│           │   ├── useProjects.ts     # Project CRUD + event listeners
│           │   ├── useResources.ts    # Resource monitoring + canvas charts
│           │   └── useTerminal.ts     # Terminal session management
│           └── components/
│               ├── AdminPanel.tsx             # System administration & permissions
│               ├── AiAgent.tsx                # AI Agent chat UI with real-time iteration display
│               ├── CloudflareTunnel.tsx       # Cloudflare Tunnel config
│               ├── CodeEditor.tsx             # Monaco file editor
│               ├── ConfigSetup.tsx            # Per-project configuration editor
│               ├── DiagnosticsPanel.tsx       # Log viewer + system diagnostics
│               ├── EnvironmentSetup.tsx       # Network & firewall degradation simulator
│               ├── FileExplorer.tsx           # File tree browser
│               ├── GitPanel.tsx               # Git staging, commit, push, pull UI
│               ├── Header.tsx                 # Custom title bar + window controls + tray
│               ├── MiniPostman.tsx            # Full REST API client
│               ├── MiniPostmanCodeSnippets.tsx # Code generation view
│               ├── MiniPostmanCollections.tsx  # Saved collections manager
│               ├── MiniPostmanEnvManager.tsx   # Environment variables
│               ├── MiniPostmanNetworkTools.tsx  # DNS, SSL, JWT, hash, encoder tools
│               ├── MiniPostmanScripts.tsx       # Pre-request & post-response scripts
│               ├── MiniPostmanTypes.ts          # TypeScript types
│               ├── ProcessManager.tsx          # Process control dashboard
│               ├── ProjectResources.tsx         # CPU, RAM, process details
│               ├── SqliteEditor.tsx            # SQLite browser + CRUD
│               ├── TabList.tsx                 # Project tab navigation
│               ├── TerminalPanel.tsx           # xterm.js PTY terminal
│               └── WindowResizer.tsx          # Panel resizing handler
│
└── target/                            # Rust build artifacts (gitignored)
```

---

## Prerequisites

| Dependency  | Version  | Purpose                                      |
|-------------|----------|----------------------------------------------|
| Rust        | 1.75+    | Core engine + Tauri backend compilation      |
| Node.js     | 18+      | Frontend development + npm scripts           |
| npm         | 9+       | Package management for UI                    |
| Git         | 2.40+    | Workspace cloning                            |
| Windows SDK | Latest   | Windows build toolchain (MSVC)               |

### Platform Support

| Platform             | Status                        |
|----------------------|-------------------------------|
| Windows (x64)        | Primary target                |
| macOS (x64/ARM)      | Supported (conditional)       |
| Linux (x64)          | Supported (conditional)       |

---

## Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd alouette_studio

# Install frontend dependencies
cd tauri_app/ui
npm install
cd ../..

# Run in development mode
npx --prefix tauri_app/ui tauri dev
```

The Tauri development server will start the Vite dev server on port 5173 and launch the desktop application window with an undecorated custom title bar.

---

## Development

### Backend (Rust)

```bash
# Build the entire workspace
cargo build

# Run all tests
cargo test

# Run specific crate tests
cargo test -p core_engine
cargo test -p alouette-studio

# Check compilation without building
cargo check
```

### Frontend (React + TypeScript)

```bash
cd tauri_app/ui

# Start Vite dev server (standalone, without Tauri shell)
npm run dev

# TypeScript type checking + build
npm run build

# Preview production build
npm run preview
```

---

## Build & Distribution

```bash
npx --prefix tauri_app/ui tauri build

# The distributable will be in:
# tauri_app/src-tauri/target/release/bundle/
```

---

## Project Configuration

Projects are stored in SQLite (`app_data/alouette.db`) and managed via the ConfigSetup UI.

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique project identifier |
| `name` | string | Display name in tabs and UI |
| `command` | string | Executable to run |
| `args` | string[] | Command-line arguments |
| `cwd` | string | Working directory (optional) |
| `setup_command` | string | Pre-run setup command (optional) |
| `setup_args` | string[] | Setup command arguments (optional) |
| `auto_restart` | boolean | Auto-restart on crash (optional) |
| `env` | object | Environment variables (optional) |
| `max_cpu_percent` | number | CPU limit percentage (optional) |
| `max_ram_mb` | number | RAM limit in MB (optional) |
| `port` | number | Port scanner target (optional) |
| `source` | string | Git URL or local path to clone/copy (optional) |
| `terminal_mode` | string | `"pty"` or `"log"` (optional) |
| `toolchain` | string | `"node"`, `"go"`, or `"python"` (optional) |
| `toolchain_version` | string | Version pin (e.g. `"stable"`, `"20.9.0"`) (optional) |
| `enable_tunnel` | boolean | Enable Cloudflare tunnel (optional) |
| `max_log_lines` | number | Max persisted log lines (optional) |

### Sandbox Configuration (per project, stored in SQLite)

The SandboxConfig includes Terminal tab (buffer, block system commands, pipe operators, block internet, skill agent), Browser tab (cookie isolation, isolate webview, bypass CORS, browser mode), Engine tab (semantic, risk level, strict boundary, PS parsing, homoglyph normalization, block IEX), Setup tab (memory limit, timeout, CPU limit, max file size), and Environment Simulation settings (firewall, weak network, unstable server parameters).

### Environment Simulation Configuration (stored in `env_simulation.yml`)

Per-project simulated environment settings including firewall rules (glob patterns), weak network parameters (latency, jitter, loss rate, bandwidth), unstable server parameters (drop rate, periodic crash, error rate, error codes), and hardware simulation limits (CPU %, RAM MB with enable flags).

---

## Testing

The project includes automated tests covering:

- Configuration serialization/deserialization (TOML round-trip)
- SQLite persistence flow (insert, query, prune, cascade delete)
- Resource monitor registration and polling
- Process manager registration
- Process execution lifecycle
- Non-existent command handling
- Setup command failure handling
- Stop process edge cases
- Log rotation
- Environment variable injection
- Settings save/load/reset round-trip
- Agent history compaction safe cut

```bash
cargo test
```

All tests pass on every commit. Test data is stored in temporary directories and cleaned up automatically.

---

## Events System

The application uses 5 background event routers:

| Router | Event Name | Purpose |
|--------|-----------|---------|
| **Log Router** | `process-log` | Streams process stdout/stderr to frontend |
| **Status Router** | `process-status` | Broadcasts process state changes |
| **Resource Router** | `resource-update` | Real-time CPU/RAM stats + watchdog enforcement |
| **Terminal Router** | `terminal-output` | Routes PTY output to frontend |
| **Init Router** | — | Preloads proto toolchains & cloudflared on startup |
| **Agent Router** | `agent-iteration` | Real-time AI Agent thinking + tool execution progress |

On exit, the application gracefully terminates all running processes and terminal sessions before closing via `ExitRequested` interception.

---

## IPC Command Modules

The application registers 70+ Tauri IPC commands across 14 modules:

| Module | Commands |
|--------|----------|
| **process** | start/stop project, get/list projects, register/deregister |
| **terminal** | spawn, write, kill, check, resize terminal sessions |
| **files** | get project files, all files/dirs, read/write content, create file/folder |
| **network** | check port, force kill process, HTTP request, DNS lookup, ping, SSL info, JSON validate/format, Base64, cURL generate, status codes, hash, JWT decode, timestamp convert, response diff, XML prettify |
| **git** | status, stage/unstage file/all, discard file, commit, push, pull, log, commit files |
| **settings** | get/save/reset settings, hide/close window |
| **sqlite** | get tables, table data, update cell, insert/delete row, add column |
| **sandbox** | load/save sandbox configs, load/save env simulation configs |
| **agent** | send message, approve tool, reset session, get/save AI config, cancel, status, history CRUD, switch project, load history page |
| **browser** | open Zen Browser window |
| **cloudflare** | load/save tunnel config |
| **language** | get/save/delete runtimes, install proto tool |
| **system** | toggle Alouette Open, check status |
| **rig_bridge** | Rig LLM provider integration (internal) |

---

## Installed Libraries

### Core Engine (Rust/Cargo)

| Crate | Version | Purpose |
|-------|---------|---------|
| tokio | 1.52 (full) | Async runtime |
| sysinfo | 0.39 | System resource information |
| serde | 1.0 (derive) | Serialization framework |
| serde_json | 1.0 | JSON serialization |
| toml | 0.9 | TOML configuration parsing |
| serde_yaml | 0.9 | YAML serialization |
| chrono | 0.4 | Date/time handling |
| parking_lot | 0.12 | Fast mutex synchronization |
| reqwest | 0.13 (json, stream) | HTTP client |
| futures-util | 0.3 | Async stream utilities |
| bytes | 1.11 | Byte buffer management |
| flate2 | 1.0 | Gzip compression |
| tar | 0.4 | Tar archive handling |
| zip | 8.6 | Zip archive handling |
| portable-pty | 0.9 | Cross-platform PTY |
| fs_extra | 1.3 | Extended filesystem operations |
| async-trait | 0.1 | Async trait support |
| directories | 6.0 | Platform-specific directories |
| rusqlite | 0.39.0 (bundled) | SQLite database |
| winapi | 0.3 (processthreadsapi, winnt, jobapi2, handleapi) | Windows API bindings |
| rand | 0.8 | Random number generation |
| libc | 0.2 | C library FFI (prctl, PDEATHSIG) |
| secrecy | 0.8 (serde) | Secret management |

### Tauri App (Rust/Cargo)

| Crate | Version | Purpose |
|-------|---------|---------|
| tauri | 2.11.2 (unstable, tray-icon, image-png) | Desktop application framework |
| serde | 1.0 (derive) | Serialization framework |
| serde_json | 1.0 | JSON serialization |
| serde_yaml | 0.9 | YAML serialization |
| tokio | 1.52 (full) | Async runtime |
| core_engine | local | Local workspace dependency |
| chrono | 0.4 | Date/time handling |
| base64 | 0.22 | Base64 encoding/decoding |
| rusqlite | 0.39.0 (bundled) | SQLite database |
| reqwest | 0.13 (json, multipart, cookies) | HTTP client |
| sha2 | 0.11 | SHA-256/384/512 hashing |
| sha1 | 0.11 | SHA-1 hashing |
| md5 | 0.8 | MD5 hashing |
| regex-lite | 0.1 | Lightweight regex |
| url | 2 | URL parsing |
| quick-xml | 0.40 | XML parsing |
| jsonschema | 0.46 | JSON Schema validation |
| tract-onnx | 0.23.0 | ONNX inference engine (Alouette Open) |
| rig-core | 0.38 (derive) | LLM provider abstraction / Agent framework |
| futures-util | 0.3.32 | Async stream utilities |
| r2d2 | 0.8 | Generic connection pooling |
| r2d2_sqlite | 0.34.0 | SQLite r2d2 connection manager |
| dashmap | 6.1 | Concurrent HashMap (agent registry) |
| secrecy | 0.8 (serde) | Secret management |
| tauri-build | 2.6 | Tauri build script |

### Frontend (npm)

| Package | Version | Purpose |
|---------|---------|---------|
| react | ^19.2.7 | UI framework |
| react-dom | ^19.2.7 | React DOM renderer |
| react-markdown | ^10.1.0 | Markdown rendering for AI responses |
| rehype-sanitize | ^6.0.0 | HTML sanitization for markdown |
| dompurify | ^3.4.8 | DOM sanitization |
| @tauri-apps/api | ^2.11.0 | Tauri IPC bridge |
| @monaco-editor/react | ^4.7.0 | Monaco code editor |
| @xterm/addon-fit | ^0.11.0 | xterm.js auto-fit addon |
| xterm | ^5.3.0 | Terminal emulator |
| fuse.js | ^7.4.1 | Fuzzy search |
| lucide-react | ^1.17.0 | Icon library |

**Dev dependencies:**

| Package | Version | Purpose |
|---------|---------|---------|
| esbuild | ^0.28.0 | Fast bundler |
| typescript | ^6.0.3 | Type checking |
| vite | ^8.0.16 | Build tool |
| @vitejs/plugin-react | ^6.0.2 | Vite React plugin |
| @tauri-apps/cli | ^2.11.2 | Tauri CLI |
| @types/react | ^19.2.16 | React type definitions |
| @types/react-dom | ^19.2.3 | React DOM type definitions |
| @types/dompurify | ^3.0.5 | DOMPurify type definitions |

---

## Documentation

Detailed technical documentation is available in the `docs/` directory:

| Document | Description |
|----------|-------------|
| `cloudflared_tunnels.md` | Cloudflare tunnel integration and lifecycle |
| `frontend_state.md` | Frontend state management architecture |
| `interactive_terminals.md` | PTY-based terminal session design |
| `process_lifecycle.md` | Process state machine and lifecycle events |
| `proto_toolchain.md` | Proto toolchain isolation and environment spoofing |
| `resource_monitoring.md` | Resource metrics collection and watchdog |
| `sqlite_storage.md` | Database schema, WAL mode, and persistence strategy |

Additional AI agent configuration is available in `AI.json` (detailed AI assistant guidelines with full directory structure, component purposes, and library inventory).

---

## Migration Notes

- The application has migrated from direct SQLite usage to **r2d2 connection pooling** with WAL mode for concurrent database access.
- **DashMap** is used for concurrent agent session registry instead of traditional `Arc<Mutex<HashMap>>`.
- Environment Simulation configs are now persisted in **YAML format** (`env_simulation.yml`) instead of inline TOML.
- Agent history is stored in the `history_agen` table with support for **pagination** and **per-project isolation**.
- The Agent system now supports **7 operation modes** (Standard, Plan, Coordinator, Worker, Autonomous, Minimal) and **sub-agent coordination**.
- Frontend now includes **react-markdown** + **rehype-sanitize** + **dompurify** for safe AI response rendering.
