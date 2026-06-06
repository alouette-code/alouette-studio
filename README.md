# Alouette Studio

**Integrated Development Studio & Native Process Orchestrator** — a high-performance desktop workspace for running and managing isolated development environments. Built with enterprise-grade toolchain isolation, a 3-tier Sandbox command protector, an Environment Simulator (firewall, latency, packet loss, and CPU limits), a Monaco-based Split Editor, a SQLite database browser, a robust MiniPostman API client, and a dedicated "Ping Zero Min" connection diagnostics window.

Built with **Rust + Tauri v2 + React 19 + TypeScript**.

---

## Features

### 1. Process Management & Workspace Orchestration
- **Lifecycle Control:** Register, start, stop, and deregister project processes seamlessly.
- **Process Guard & Restart:** Automatic process health monitoring and restart with exponential backoff (configurable).
- **Setup Hooks:** Define and run custom setup command chains prior to main process execution.
- **Deep Termination:** Safe force-kill mechanics utilizing recursive process tree traversal to prevent orphan processes.
- **Port Scanner:** Automated port diagnostic scanner (`netstat` on Windows, `lsof` on Unix) to identify conflicting bindings.
- **Network Isolation:** Process-level network interface isolation configuration.

### 2. Sandbox — 3-Tier Command Protection
A comprehensive security system designed to prevent destructive command execution (e.g., `rm -rf /`, `Format C:`):
- **Tier 1a (Semantic Interceptor):** Performs deep semantic parsing on command trees, classifies risk levels, resolves environment variables/relative paths, normalizes Unicode homoglyphs, blocks .NET file operation patterns, parses PowerShell subexpressions, and rejects boundary escape attempts.
- **Tier 1b (Engine):** Fallback path resolution engine that strips Windows `\\?\` prefixes and performs case-insensitive boundary validations.
- **Tier 2 (OS-level Isolation):** Integrates system-level sandboxing (AppContainer on Windows, landlock/seccomp placeholders on Linux).
- **PTY Hooking:** All terminal inputs are intercepted at `terminal.rs` via `sandbox::check_command()` before execution.

#### Sandbox UI Control Center
The Sandbox dashboard consists of 5 synchronized control modules:
- *Tab All (Top-Right):* One-click master switch to enable/disable sandbox globally.
- *Setup Details (Center-Left):* Sub-feature configuration toggles.
- *Search (Middle-Right):* Instant filtering for imported projects.
- *Imported Projects List (Bottom-Right):* Per-project activation state checkboxes.
- *Section Tabs (Top-Left):* Switch between Terminal, Browser, Engine, and Setup configs.

### 3. Environment Simulation Panel
- **Testing Simulator:** Opened via the Server icon in the status bar (routes to a virtual tab `__environment__`).
- **4 Degradation Groups:**
  - *Firewall:* Emulate port blocks and custom firewall rules.
  - *Weak Network:* Introduce artificial packet loss, latency, and throughput caps.
  - *Unstable Server:* Simulate periodic connection drops and network instability.
  - *Resource Limits:* Enforce hard hardware utilization boundaries.
- **Debounced Persistence:** Configurations are automatically saved to the project's `SandboxConfig` using `save_sandbox_config` with an 800ms debounce.

### 4. File Editor & Split Editor Panes
- **Monaco Integration:** Full-featured Monaco Editor with rich syntax highlighting, search, and formatting.
- **State Recovery:** Automatic scroll position and cursor position preservation when switching between tabs.
- **Directory Explorer:** Interactive recursive directory tree with file creation, deletion, and search capability.
- **Split Editors:** Right-click on the Tab bar to split the editor into up to 3 side-by-side active panes. Click to activate focus, and drag-and-drop tabs between panes easily.
- **Binary Transfer:** Safe Base64 encoding for local binary file transfer over Tauri IPC.

### 5. "Ping Zero Min" Network Diagnostics & MiniPostman API Client
- **"Ping Zero Min" Diagnostics:** A dedicated diagnostic window (toggled via the Wifi icon in settings or AI chat) running custom native ping tools, tracking packet loss, minimum/maximum/average latency, and host reachability.
- **MiniPostman API Client:** Fully embedded REST client:
  - *Methods:* GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD.
  - *Payloads:* Query params, custom headers, and multipart/form-data/GraphQL/binary request bodies.
  - *Auth:* Bearer, Basic, API Key, OAuth 2.0, AWS Signature.
  - *Response Inspector:* Visual formatters, headers/cookies decoder, redirect chains, and timing breakdown.
  - *Post-request Scripts:* Automation script runner with testing assertions.
  - *Collections & Environments:* Save requests and manage environment variables.
  - *cURL Generator:* One-click export of any HTTP request to a cURL command.
- **Helper Tools:** DNS lookup (records analysis), SSL certificate inspector, JWT decoder, MD5/SHA hash builders, Base64 converter, XML/JSON prettifier, and a HTTP status code guide.

### 6. Git Management Panel (Git UI)
- **Native Sidebar:** Git panel toggled via the branch icon in the status bar.
- **Metadata Monitor:** Real-time active branch and remote repository tracking.
- **Staging Area:** Lists files separated into *Staged Changes* and unstaged *Changes*.
- **Quick Controls:** Direct buttons to Stage (+), Unstage (-), Discard/Revert changes, Commit (with message), Push, and Pull.

### 7. Resource Monitoring & Hardware Control
- **Watchdog Enforcement:** Tracks real-time CPU/RAM usage per process tree and displays live canvas-based performance charts.
- **Limit Enforcement:** Configures CPU % and RAM MB limits. The system watchdog automatically terminates processes exceeding limits for more than 30 seconds.
- **Title Bar Integration:** Database icon on the title bar grants quick access to uptime, hardware usage, simulated GPU metrics, and active ports.

### 8. SQLite Browser & Storage
- **WAL Persistence:** High-performance concurrent SQLite operations using WAL (Write-Ahead Logging) mode.
- **Database Browser:** View all tables, inspect schemas, add columns, insert/delete rows, and edit individual cells directly.
- **Log Pruning:** Automatic system log truncation, keeping a clean history of the last 5000 lines per project.

### 9. Cloudflared Tunnel Integration
- **Cloudflare Zero Trust:** Auto-download and parse Cloudflare tunnels from stderr.
- **Modes:** Tunnel Free (auto-links local project port) or Named Tunnel (Token-based authentication).
- **Title Bar Status:** Cloudflare cloud icon displays in orange when active, gray when inactive. Quick-click to toggle the tunnel state immediately.

### 10. System Manager & Global Configs
- **System Manager:** Administration panel for process supervision, background services, and core system diagnostics.
- **Global Settings:** Central configuration dashboard for theme/language, auto-start, keep-alive running in tray, Telegram Bot remote alerts, and history limits.
- **Zen Browser Integration:** Run sandbox-isolated Zen Browser windows directly from the workspace.

### 11. Auxiliary AI Diagnostic Companion
- **AI Log Assistant (Alouette Open):** Background log scanner executing `alouette_open-A1 v1.0.onnx` models via `tract-onnx` and heuristic rules at 500ms intervals. Emits warning alerts and diagnostic cards for quick fix actioning.
- **Agent Loop Harness:** Think-act-observe harness (`rig-core 0.38` integration) with three operation modes:
  - *Interactive:* Asks before executing write operations.
  - *Write:* Auto-approves file modification.
  - *Autonomous:* Fully auto-executes planned code changes.
- **Thinking Mode:** Brain icon toggle to activate High mode (forces deep model reasoning with `thinking_budget`) or Low mode (standard execution).
- **Agent History:** Conversations stored in SQLite (`history_agen.sql`), searchable via global bar query prefix (`history agent <keyword>`), with one-click restore.

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
| **UI** | React 19 + TypeScript + Vite + xterm.js + Monaco | Desktop interface with process dashboards, terminals, file editor, SQLite browser, API client, admin panel, AI Agent chat |
| **Bridge** | Tauri v2 IPC | Type-safe command handlers (13 command modules), event routers (5 routers), state management |
| **Engine** | Rust (Tokio async) | Process lifecycle, sandbox enforcement, proto toolchain isolation, resource monitoring, SQLite persistence, AI agent loop, ONNX inference, system management |

---

## Project Structure

```
alouette_studio/
├── .gitignore
├── .taurignore
├── AI.json                            # AI assistant guidelines
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
│       │   ├── details.rs             # Project uptime, memory/CPU statistics details
│       │   ├── network_isolate.rs     # Network isolation per process
│       │   ├── network_simulate_proxy.rs # Firewall and network degradation simulator
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
│   │           ├── git.rs             # Git operations command handler (status, diff, commit, push, pull)
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
│               ├── EnvironmentSetup.tsx       # Network and firewall degradation simulator setup dashboard
│               ├── FileExplorer.tsx           # File tree browser
│               ├── GitPanel.tsx               # Sidebar interface for staging, committing, and pushing changes
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
│               ├── WindowResizer.tsx          # Panel resizing handler
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
| tokio | 1.52 (full) | Async runtime |
| sysinfo | 0.39 | System resource information |
| serde | 1.0 (derive) | Serialization framework |
| serde_json | 1.0 | JSON serialization |
| toml | 0.9 | TOML configuration parsing |
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
| rusqlite | 0.40.0 (bundled) | SQLite database |
| winapi | 0.3 | Windows API bindings |

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
| base64 | 0.22 | Base64 encoding |
| rusqlite | 0.40.0 (bundled) | SQLite database |
| reqwest | 0.13 (json, multipart, cookies) | HTTP client |
| sha2 | 0.11 | SHA-256 hashing |
| sha1 | 0.11 | SHA-1 hashing |
| md5 | 0.8 | MD5 hashing |
| regex-lite | 0.1 | Lightweight regex |
| url | 2 | URL parsing |
| quick-xml | 0.40 | XML parsing |
| jsonschema | 0.46 | JSON Schema validation |
| tract-onnx | 0.23.0 | ONNX inference engine (Alouette Open) |
| rig-core | 0.38 (derive) | LLM provider abstraction / Agent framework |
| futures-util | 0.3 | Async stream utilities |
| tauri-build | 2.6 | Tauri build script |

### Frontend (npm)

| Package | Version | Purpose |
|---------|---------|---------|
| react | ^19.2.7 | UI framework |
| react-dom | ^19.2.7 | React DOM renderer |
| @tauri-apps/api | ^2.11.0 | Tauri IPC bridge |
| @monaco-editor/react | ^4.7.0 | Monaco code editor |
| @xterm/addon-fit | ^0.11.0 | xterm.js auto-fit addon |
| fuse.js | ^7.4.1 | Fuzzy search |
| lucide-react | ^1.17.0 | Icon library |
| xterm | ^5.3.0 | Terminal emulator |

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
