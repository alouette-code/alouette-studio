# Alouette Studio

**Integrated Development Studio & Native Process Orchestrator** — a high-performance desktop application for managing isolated development environments with enterprise-grade toolchain isolation, real-time resource monitoring, integrated multi-session terminal access, built-in Monaco Editor, SQLite browser, robust API client, AI Agent harness with autonomous loop, and local AI-powered log diagnostic suite.

Built with **Rust + Tauri v2 + React 18 + TypeScript**.

---

## Features

### Process Management
- Register, start, stop, and deregister project processes
- Automatic restart with exponential backoff (configurable)
- Setup command execution before main process
- Force kill with process tree traversal
- Port scanning (netstat on Windows, lsof on Unix)
- Network isolation per process

### Resource Monitoring
- Real-time CPU and RAM usage per process tree
- Live canvas-based charts via Tauri events
- Configurable resource limits (max CPU %, max RAM MB)
- Watchdog enforcement: auto-terminate processes exceeding limits for 30+ seconds
- GPU performance simulation and port detail tracking

### Proto Toolchain Isolation
- Automatic download of [Proto](https://moonrepo.dev/proto) CLI (moonrepo)
- Isolated toolchain environments for Node.js, Go, and Python
- Spoofed PATH environment to prevent system tool conflicts
- Per-project toolchain version pinning (e.g., `node 20.9.0`)

### Interactive Terminals
- PTY-based interactive shell sessions (Mode A)
- Piped log stream capture (Mode B)
- Multiple terminal sessions per project
- Sandboxed within workspace directories
- Full stdin/stdout/stderr routing
- Session resizing support

### AI Agent — Autonomous Code Assistant (Agent Loop Engine)
- **Agent Loop Engine:** Full think-act-observe loop: LLM response → parse → tool call → execute → feed result → repeat
- **3 operation modes:** Interactive (approve before write), Write (auto-approve read+write), Autonomous (fully auto)
- **Real-time streaming:** Frontend receives `agent-iteration` events with thought, tool_name, tool_result in real-time
- **History compaction:** Auto-compacts conversation history when exceeding 80 messages
- **Skill tools:** `scan_directory_tree`, `scan_subdirectory`, `search_files`, `extract_symbol`, `read_file_range`, `search_symbol`
- **Memory management:** Long-term and short-term memory tracking for context persistence
- **Self-healing:** Automatic error recovery and retry logic on tool execution failures
- **Telemetry:** Execution metrics and performance tracking
- **Plan system:** Multi-step plan generation and execution
- **Prompt system:** Custom identity and tool prompts stored in `prompts/identity.txt` and `prompts/tools.txt`
- **Rig Framework integration:** Uses `rig-core 0.37` for LLM provider abstraction, supporting Google Gemini, OpenAI-compatible APIs, and local LLMs

### Alouette Open — AI-Powered Log Monitoring (Alouette A1)
- **Real-Time Log Monitoring:** High-performance background polling (500ms intervals) of process log outputs and system log streams
- **ONNX Model & Heuristic Detection:** Integrates local `alouette_open-A1 v1.0.onnx` execution via `tract-onnx` in Rust alongside heuristic fallbacks for robust and instant error detection
- **Interactive UI Cards:** Displays gorgeous, floating error alerts in the AI Agent chat panel built with Lucide React icons (`Bot`, `X`, `Search`)
- **Instant Fix Workflow:** One-click "Bắt đầu tìm hiểu" sends diagnostic reports and commands directly to the active AI Agent

### Sandbox — 3-Tier Command Protection
Designed to prevent destructive command execution (e.g., `rm -rf /`, `cd ~ && rm -rf .`, `Format C:`):

| Tier | Component | Mechanism |
|------|-----------|-----------|
| **1a** | Interceptor | Semantic analysis — parses command tree, classifies risk, resolves paths (`~`, `$env`, relative), checks workspace boundary |
| **1b** | Engine | Token-based fallback — path resolution and boundary checking |
| **2**  | OS-level | AppContainer (Windows), seccomp/landlock (Linux), sandbox_init (macOS) — platform-specific kernel isolation |

Commands are intercepted before being sent to the PTY shell. Navigation (`cd`, `sl`, `pushd`) is allowed only within the workspace boundary.

### Cloudflared Tunnel Integration
- Automatic download of latest Cloudflared binary
- Tunnel URL parsing from stderr output
- Per-project tunnel enable/disable
- Supports 2 modes: Tunnel Free (auto port link) and Named Tunnel (Cloudflare Zero Trust Token)
- Visual toggle in title bar with brand-colored Cloudflare icon
- Configuration saved in `cloudflare_config.yml`

### MiniPostman — Built-in API Client
A full-featured HTTP client embedded directly in the application:

- **Methods:** GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD
- **Request:** Query params, headers, body (JSON, text, XML, form-data, x-www-form-urlencoded, GraphQL, binary)
- **Authentication:** Bearer token, Basic Auth, API Key, OAuth 2.0, AWS Signature
- **Response:** Formatted body viewer, raw view, headers, cookies, timing breakdown, redirect chain
- **Tests:** Automated test scripts (status code, latency, JSON validity, text matching)
- **Scripts:** Pre-request and post-response JavaScript-like scripting
- **Collections:** Organize and save requests
- **Environments:** Variable management with scoped environments
- **History:** Full request history with search
- **Code Generation:** Generate cURL commands and code snippets from any request

### Network Tools
- **DNS Lookup** — resolve domain names
- **Ping** — ICMP reachability test
- **SSL Certificate Info** — inspect certificate chain
- **cURL Generator** — convert request to cURL command
- **JWT Decoder** — inspect JWT token payload
- **Hash Generator** — MD5, SHA1, SHA256, SHA512
- **Base64 Encoder/Decoder**
- **Timestamp Converter** — Unix ↔ human-readable
- **HTTP Status Code Reference**
- **JSON Schema Validator & Formatter**
- **XML Prettifier**
- **Response Diff** — compare two API responses

### SQLite Persistence
- WAL mode for concurrent read/write safety
- Automatic log persistence with pruning (5000 lines per project)
- Full CRUD operations via Tauri commands
- Foreign key cascading deletes

### File Editor
- Monaco Editor integration with syntax highlighting
- File tree explorer with recursive directory traversal
- Base64 encoding for safe binary file transfer over IPC
- Save with Ctrl+S shortcut
- Scroll position and cursor position preservation across file switches

### SQLite Browser
- List all user tables in any SQLite database
- View table structure (columns, types, primary keys)
- Edit individual cell values
- Insert new rows, delete rows
- Add columns (TEXT, INTEGER, REAL)

### Project Resources Panel
- Real-time Uptime tracking for each project
- Hardware metrics: CPU usage, RAM consumption with configurable limits
- GPU performance simulation metrics
- Port mapping and network details
- Sandbox and tunnel configuration status
- Quick-access from the database icon in the title bar

### Build Panel
- Build & deployment dashboard
- Build process orchestration and output monitoring
- Integration with project toolchain settings

### System Manager
- System-level administration and process oversight
- Background service management
- System state monitoring and diagnostics

### Split Editor Panes
- Right-click on Tab bar to split editor into 2-3 side-by-side panes
- Click any pane to activate focus
- Files open in the active pane
- Drag-and-drop tabs between panes
- Right-click menu with "Close Split Pane" option to merge

### Global Settings
Persistent application settings stored in `app_data/settings.json`:

| Category | Fields |
|----------|--------|
| **General** | Theme (dark/light), language |
| **Logs** | Max log lines, auto-scroll, active log filter |
| **Performance** | Max history points, max terminal output length, monitor interval |
| **Appearance** | Font size, sidebar widths, panel heights |
| **Application & Startup** | Keep alive (minimize to tray), auto-start with OS, run in background |
| **Resource Control** | CPU % and RAM MB limits enforcement |
| **Auto Restart** | Periodic automatic restart scheduling |
| **Telegram Bot** | Remote alerts via Telegram API Token and Chat ID |

### Thinking Mode
- **High mode (purple):** Forces deep thinking via API thinking budget configuration
- **Low mode (gray):** Lets the model decide automatically, optimizing speed and resources

### Agent History
- Chat history automatically saved to SQLite (`core_engine/app_data/history_agen.sql`)
- Session titles auto-generated from first 6 words of user's first message
- Search by typing `history agent` or `agent history` in the search bar
- Click any history entry to restore full session in the AI Agent sidebar

### Zen Browser Integration
Launch a Zen Browser window directly from the application with bundled browser resources.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Tauri Desktop Shell                            │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    React Frontend (UI)                            │  │
│  │  Components | Hooks | Monaco | xterm.js | Charts | Lucide Icons │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  │              Tauri IPC Bridge (Commands + Events)                 │  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Core Engine (Rust)                              │  │
│  │  Process Manager | Sandbox | Proto | Cloudflared | DB            │  │
│  │  Agent Harness | Alouette Open | System Manager                  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Responsibility |
|-------|-----------|----------------|
| **UI** | React 18 + TypeScript + Vite + xterm.js + Monaco | Desktop interface with process dashboards, terminals, file editor, SQLite browser, API client, admin panel, AI Agent chat |
| **Bridge** | Tauri v2 IPC | Type-safe command handlers (13 command modules), event routers (5 routers), state management |
| **Engine** | Rust (Tokio async) | Process lifecycle, sandbox enforcement, proto toolchain isolation, resource monitoring, SQLite persistence, AI agent loop, ONNX inference, system management |

---

## Project Structure

```
alouette_studio/
├── .gitignore
├── .taurignore
├── AI.json                            # AI assistant guidelines
├── CLAUDE.md                          # Project-level AI instructions
├── Cargo.toml                         # Rust workspace root (core_engine + tauri_app)
├── README.md
├── fix.js                             # Utility fix script
├── icon.png                           # Application icon
│
├── assets/                            # Static assets
│   ├── icon-app.png
│   └── icon-app-square.png
│
├── scripts/                           # Utility scripts
│   ├── generate_icons.js
│   └── generate_icons.py
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
│   │   ├── ai_config.yml              # Custom AI provider configuration
│   │   ├── alouette.db                # SQLite database (projects, logs, history)
│   │   ├── cloudflare_config.yml      # Cloudflare Tunnel configuration
│   │   ├── alouette_toolchains/       # Proto toolchain binaries
│   │   └── workspaces/                # Cloned/copied project workspaces
│   └── src/
│       ├── lib.rs                     # Crate root + public re-exports
│       ├── config.rs                  # TOML configuration deserialization
│       ├── db.rs                      # SQLite persistence layer (WAL mode)
│       ├── monitor.rs                 # Resource monitoring (CPU/RAM per process tree)
│       ├── settings.rs                # Global app settings (JSON persistence)
│       ├── cloudflared_manager.rs     # Cloudflare tunnel binary management
│       ├── proto_manager.rs           # Proto toolchain download & isolation
│       ├── workspace_manager.rs       # Git clone / local copy workspace setup
│       ├── process/                   # Process lifecycle management
│       │   ├── mod.rs                 # Module declarations + re-exports
│       │   ├── models.rs              # ProcessState, ProcessLog, TerminalOutput, ProjectInstance
│       │   ├── manager.rs             # ProcessManager struct + CRUD
│       │   ├── executor.rs            # start_process, stop_process, force_fatal_stop
│       │   ├── terminal.rs            # spawn_terminal, write_terminal, kill_terminal
│       │   ├── logging.rs             # Log rotation, stream piping
│       │   ├── tree.rs                # Process tree traversal, path utilities
│       │   ├── network_isolate.rs     # Network isolation per process
│       │   └── sandbox/               # 3-tier command protection
│       │       ├── mod.rs             # Module entry + orchestration
│       │       ├── interceptor.rs     # Tier 1a — semantic command analysis
│       │       ├── engine.rs          # Tier 1b — token-based fallback
│       │       ├── windows.rs         # Tier 2 — AppContainer (Windows)
│       │       ├── linux.rs           # Tier 2 — placeholder (Linux)
│       │       └── macos.rs           # Tier 2 — placeholder (macOS)
│       ├── agent_harness/             # AI Agent Loop Engine
│       │   ├── mod.rs                 # AgentHarness::run_agent_loop() entry point
│       │   ├── autonomous.rs          # Autonomous mode auto-approval logic
│       │   ├── compaction.rs          # History compaction when exceeding 80 messages
│       │   ├── hooks.rs               # Lifecycle hooks (pre/post tool execution)
│       │   ├── memory.rs              # Short-term + long-term memory management
│       │   ├── parser.rs              # LLM response parser (tool calls, text)
│       │   ├── plan.rs                # Multi-step plan generation & execution
│       │   ├── self_heal.rs           # Error recovery & retry logic
│       │   ├── skills.rs              # Skill tool implementations
│       │   ├── telemetry.rs           # Execution metrics & performance tracking
│       │   ├── tool_definitions.rs    # Tool definitions for LLM function calling
│       │   └── prompts/              # System prompts for LLM
│       │       ├── identity.txt       # AI identity / role prompt
│       │       └── tools.txt          # Tool descriptions prompt
│       └── bin/
│           └── test_parser.rs         # Standalone parser test binary
│
├── tauri_app/                         # Tauri v2 desktop application
│   ├── .taurignore
│   ├── zen_bundle/                    # Zen Browser resources (empty, gitignored)
│   ├── src-tauri/                     # Tauri Rust binary crate
│   │   ├── Cargo.toml
│   │   ├── build.rs                   # Tauri build script
│   │   ├── tauri.conf.json            # Tauri configuration (undecorated window)
│   │   ├── .prototools                # Proto toolchain version pin
│   │   ├── capabilities/
│   │   │   └── default.json           # Tauri v2 capability permissions
│   │   ├── icons/                     # Application icons (PNG, ICO, ICNS)
│   │   ├── gen/schemas/               # Auto-generated platform schemas
│   │   │   ├── acl-manifests.json
│   │   │   ├── capabilities.json
│   │   │   ├── desktop-schema.json
│   │   │   ├── linux-schema.json
│   │   │   └── windows-schema.json
│   │   ├── logs/                      # Runtime logs directory
│   │   ├── app_data/                  # Runtime data (gitignored)
│   │   └── src/
│   │       ├── main.rs                # Entry point + Tauri setup + exit cleanup
│   │       ├── alouette_open.rs       # Alouette A1 background log monitor (ONNX + Heuristics)
│   │       ├── events.rs              # Background event router tasks (5 routers)
│   │       ├── state.rs               # AppState struct + logging utility
│   │       ├── system_manager.rs      # System administration & state management
│   │       └── commands/              # Tauri IPC command handlers (13 modules)
│   │           ├── mod.rs             # Module declarations
│   │           ├── agent.rs           # AI Agent commands (run_agent_loop, history management)
│   │           ├── browser.rs         # Zen Browser window launcher
│   │           ├── files.rs           # File explorer + read/write file content
│   │           ├── language.rs        # Language detection & processing commands
│   │           ├── network.rs         # HTTP requests, DNS, ping, SSL, JWT, hash, etc.
│   │           ├── process.rs         # Start/stop/register/deregister projects
│   │           ├── rig_bridge.rs      # Rig LLM framework bridge (providers, models)
│   │           ├── sandbox.rs         # Sandbox configuration & status commands
│   │           ├── settings.rs        # Global settings get/save/reset
│   │           ├── sqlite.rs          # SQLite table browser + CRUD editor
│   │           └── terminal.rs        # Spawn/write/kill/resize/check terminal sessions
│   │
│   └── ui/                            # React frontend (Vite)
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── index.html
│       └── src/
│           ├── main.tsx               # React entry point
│           ├── App.tsx                # Application orchestrator
│           ├── index.css              # Global styles + CSS variables + dark/light
│           ├── constants.ts           # Static mock data + configuration
│           ├── vite-env.d.ts          # Vite type declarations
│           ├── types/
│           │   └── index.ts           # TypeScript interfaces (Project, TerminalState, etc.)
│           ├── hooks/
│           │   ├── useProjects.ts     # Project CRUD + event listeners
│           │   ├── useResources.ts    # Resource monitoring + canvas charts
│           │   └── useTerminal.ts     # Terminal session management
│           └── components/
│               ├── AdminPanel.tsx             # System administration & permissions
│               ├── AiAgent.tsx                # AI Agent chat UI with real-time iteration display
│               ├── BuildPanel.tsx             # Build & deployment dashboard
│               ├── CloudflareTunnel.tsx       # Cloudflare Tunnel config (Single Card Layout)
│               ├── CodeEditor.tsx             # Monaco file editor with syntax highlighting
│               ├── ConfigSetup.tsx            # Per-project configuration editor
│               ├── DiagnosticsPanel.tsx       # Log viewer + system diagnostics
│               ├── FileExplorer.tsx           # File tree browser
│               ├── Header.tsx                 # Custom title bar + window controls (drag, min, close)
│               ├── MiniPostman.tsx            # API Client (Postman-like HTTP client)
│               ├── MiniPostmanCodeSnippets.tsx # Code generation view for API requests
│               ├── MiniPostmanCollections.tsx  # Saved request collections manager
│               ├── MiniPostmanEnvManager.tsx   # Environment variables for API requests
│               ├── MiniPostmanNetworkTools.tsx  # DNS, ping, SSL, JWT, hash, encoder tools
│               ├── MiniPostmanScripts.tsx       # Pre-request & post-response scripts
│               ├── MiniPostmanTypes.ts          # TypeScript types for MiniPostman
│               ├── ProcessManager.tsx          # Process control dashboard
│               ├── ProjectResources.tsx         # Resource monitoring tab (CPU, RAM, GPU, Port)
│               ├── SqliteEditor.tsx            # SQLite database browser + CRUD
│               ├── TabList.tsx                 # Project tab navigation
│               ├── TerminalPanel.tsx           # PTY terminal UI (xterm.js)
│               └── brand-icon.png              # Brand logo asset
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
cargo test -p tauri_app

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

Projects are stored in SQLite (`app_data/alouette.db`) and can be exported/imported via the File menu.

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

```bash
cargo test
```

All tests pass on every commit. Test data is stored in temporary directories and cleaned up automatically.

---

## Events System

The application uses 5 background event routers:

| Router | Event Name | Purpose |
|--------|-----------|---------|
| **Log Router** | `log-line` | Streams process stdout/stderr to frontend |
| **Status Router** | `process-status` | Broadcasts process state changes |
| **Resource Router** | `resource-update` | Real-time CPU/RAM stats + watchdog |
| **Terminal Router** | `terminal-output` | Routes PTY output to frontend |
| **Init Router** | — | Preloads environment info on startup |
| **Agent Router** | `agent-iteration` | Real-time AI Agent thinking + tool execution progress |

On exit, the application gracefully terminates all running processes and terminal sessions before closing.

---

## Installed Libraries

### Core Engine (Rust/Cargo)

| Crate | Version | Purpose |
|-------|---------|---------|
| tokio | 1.35 (full) | Async runtime |
| sysinfo | 0.30 | System resource information |
| serde | 1.0 (derive) | Serialization framework |
| serde_json | 1.0 | JSON serialization |
| toml | 0.8 | TOML configuration parsing |
| chrono | 0.4 | Date/time handling |
| parking_lot | 0.12 | Fast mutex synchronization |
| reqwest | 0.11 (json, stream) | HTTP client |
| futures-util | 0.3 | Async stream utilities |
| bytes | 1.5 | Byte buffer management |
| flate2 | 1.0 | Gzip compression |
| tar | 0.4 | Tar archive handling |
| zip | 0.6 | Zip archive handling |
| portable-pty | 0.8 | Cross-platform PTY |
| fs_extra | 1.3 | Extended filesystem operations |
| async-trait | 0.1 | Async trait support |
| directories | 5.0 | Platform-specific directories |
| rusqlite | 0.31.0 (bundled) | SQLite database |
| winapi | 0.3 | Windows API bindings |

### Tauri App (Rust/Cargo)

| Crate | Version | Purpose |
|-------|---------|---------|
| tauri | 2.0 (unstable, tray-icon, image-png) | Desktop application framework |
| serde | 1.0 (derive) | Serialization framework |
| serde_json | 1.0 | JSON serialization |
| serde_yaml | 0.9 | YAML serialization |
| tokio | 1.35 (full) | Async runtime |
| core_engine | local | Local workspace dependency |
| chrono | 0.4 | Date/time handling |
| base64 | 0.21 | Base64 encoding |
| rusqlite | 0.31.0 (bundled) | SQLite database |
| reqwest | 0.11 (json, multipart, cookies) | HTTP client |
| sha2 | 0.10 | SHA-256 hashing |
| sha1 | 0.10 | SHA-1 hashing |
| md5 | 0.7 | MD5 hashing |
| regex-lite | 0.1 | Lightweight regex |
| url | 2 | URL parsing |
| quick-xml | 0.31 | XML parsing |
| jsonschema | 0.18 | JSON Schema validation |
| tract-onnx | 0.21.3 | ONNX inference engine (Alouette Open) |
| rig-core | 0.37 (derive) | LLM provider abstraction / Agent framework |
| futures-util | 0.3 | Async stream utilities |
| tauri-build | 2.0 | Tauri build script |

### Frontend (npm)

| Package | Version | Purpose |
|---------|---------|---------|
| react | ^18.2.0 | UI framework |
| react-dom | ^18.2.0 | React DOM renderer |
| @tauri-apps/api | ^2.0.0 | Tauri IPC bridge |
| @monaco-editor/react | ^4.7.0 | Monaco code editor |
| xterm | ^5.3.0 | Terminal emulator |
| @xterm/addon-fit | ^0.11.0 | xterm.js auto-fit addon |
| lucide-react | ^0.300.0 | Icon library |

**Dev dependencies:**

| Package | Version | Purpose |
|---------|---------|---------|
| typescript | ^5.2.0 | Type checking |
| vite | ^5.0.0 | Build tool |
| @vitejs/plugin-react | ^4.2.0 | Vite React plugin |
| @tauri-apps/cli | ^2.1.0 | Tauri CLI |
| @types/react | ^18.2.0 | React type definitions |
| @types/react-dom | ^18.2.0 | React DOM type definitions |

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

Additional AI agent configuration is available in `CLAUDE.md` (project-level AI instructions) and `AI.json` (detailed AI assistant guidelines with full directory structure, component purposes, and library inventory).

---
