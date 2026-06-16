# Alouette Studio

**Integrated Development Studio & Native Process Orchestrator** — a high-performance desktop workspace for running and managing isolated development environments. Built with enterprise-grade toolchain isolation, a 3-tier Sandbox command protector, an Environment Simulator (firewall, latency, packet loss, CPU/RAM limits), a Monaco-based Split Editor, a SQLite database browser, the robust **PingZero Mini** API client, a full Git UI, a Cloudflare Tunnel manager, and an integrated AI Agent harness with multi-provider LLM support.

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
- **Workspace Auto-Cloning:** Git clone or local directory copy for isolated workspace preparation ([workspace_manager.rs](file:///home/nhatanh/projet/alouette_studio/core_engine/src/workspace_manager.rs)).
- **PTY & Log Terminal Modes:** Choose between full interactive PTY terminals or log-only output mode per project.

### 2. Sandbox — 3-Tier Command Protection
A comprehensive security system designed to prevent destructive command execution (e.g., `rm -rf /`, `Format C:`):
- **Tier 1a (Semantic Interceptor):** Performs deep semantic parsing on command trees, classifies risk levels, resolves environment variables/relative paths, normalizes Unicode homoglyphs, blocks .NET file operation patterns, parses PowerShell subexpressions, and rejects boundary escape attempts.
- **Tier 1b (Engine):** Fallback path resolution engine that strips Windows `\\?\` prefixes and performs case-insensitive boundary validations.
- **Tier 2 (OS-level Isolation):** Integrates system-level sandboxing (AppContainer on Windows, landlock/seccomp placeholders on Linux).
- **PTY Hooking:** All terminal inputs are intercepted at [terminal.rs](file:///home/nhatanh/projet/alouette_studio/core_engine/src/process/terminal.rs) via `sandbox::check_command()` before execution.

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
- **Debounced Persistence:** Configurations are automatically saved to YAML ([env_simulation.yml](file:///home/nhatanh/projet/alouette_studio/core_engine/app_data/env_simulation.yml)) with an 800ms debounce.

### 4. File Editor & Split Editor Panes
- **Monaco Integration:** Full-featured Monaco Editor with rich syntax highlighting, search, and formatting ([CodeEditor.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/CodeEditor.tsx)).
- **State Recovery:** Automatic scroll position and cursor position preservation when switching between tabs.
- **Directory Explorer:** Interactive recursive directory tree with file/folder creation, deletion, and search capability ([FileExplorer.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/FileExplorer.tsx)).
- **Split Editors:** Right-click on the Tab bar to split the editor into up to 3 side-by-side active panes. Click to activate focus, and drag-and-drop tabs between panes easily.
- **Binary Transfer:** Safe Base64 encoding for local binary file transfer over Tauri IPC.

### 5. PingZero Mini API Client & Network Diagnostics
- **PingZero Mini API Client:** Fully embedded REST client (implemented as [MiniPostman.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/MiniPostman.tsx)):
  - *Methods:* GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD.
  - *Payloads:* Query params, custom headers, multipart/form-data, GraphQL, binary, and URL-encoded request bodies.
  - *Auth:* Bearer, Basic, API Key, OAuth 2.0, AWS Signature.
  - *Response Inspector:* Visual formatters, headers/cookies decoder, redirect chain viewer, timing breakdown (DNS lookup, TCP connect, TLS handshake, first byte, total).
  - *Post-request Scripts:* Automation script runner with testing assertions.
  - *Pre-request Scripts:* Dynamic request modification before execution.
  - *Collections & Environments:* Save requests in collections and manage environment variables per workspace.
  - *cURL Generator:* One-click export of any HTTP request to a cURL command.
  - *Response Diff:* Side-by-side comparison of two HTTP responses.
- **Helper Toolbox:**
  - **DNS Lookup / SSL Cert Inspector / Ping Host:** Essential network diagnostics tools.
  - **JWT Decoder / Hash Builder / Base64 Converter / JSON Schema Validator:** Handy developer utilities.

### 6. Git Management & Native Diff Engine
- **Native Sidebar:** Git panel toggled via the branch icon in the status bar ([GitPanel.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/GitPanel.tsx)).
- **Metadata Monitor:** Real-time active branch and remote repository tracking.
- **Staging Area:** Lists files separated into *Staged Changes* and unstaged *Changes* with per-file status indicators.
- **Quick Controls:** Direct buttons to Stage (+), Unstage (-), Discard/Revert changes, Commit (with message), Push, Pull.
- **Commit History:** View last 30 commits with hash, author, date, and subject. Click to inspect changed files in each commit.
- **Native Diff Engine:** Integrates a native file diffing capability powered by `git2` under [git_diff.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/commands/git_diff.rs) to display precise addition, modification, and deletion line highlights in the editor without invoking external shell tools.

### 7. Code RAG System (Semantic & Keyword Code Search)
- **Multi-Language AST Extraction:** Leverages AST parsing configured for multiple languages (Rust, Go, TypeScript, JavaScript, Python, C++, C#, Java, Ruby) to index function declarations, signatures, line boundaries, and docstrings.
- **ONNX Local Embeddings:** Runs local 384-dimensional vector embeddings powered by the Xenova `bge-small-en-v1.5` model via `tract-onnx` in a non-blocking background thread.
- **Semantic Code Queries:** Query index content via semantic similarity (`code_rag_query`) or fast metadata keyword queries (`code_rag_query_by_name`) for autocomplete and semantic retrieval.
- **Project Scan Control:** Supports directory scanning ([code_rag.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/commands/code_rag.rs)) and re-indexing updates automatically upon file changes.
- **Seed Code Library:** Seeded automatically with helper code templates on startup.

### 8. Local Inference Engine, Local Chatbot & Gateway
- **In-App Local Inference:** Directly runs transformer models (e.g. MiniCPM) within the Tauri sandbox using the HF Candle crate ([inference.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/inference.rs) / [minicpm.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/minicpm.rs)).
- **Local Chat Window:** Engage in direct, secure, zero-network conversational chat with local LLMs, streaming response chunks via Tauri events ([LocalChat.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/LocalChat.tsx)).
- **AI Engine Supervisor:** Start, stop, and supervise external local engines (Ollama, llama.cpp, KoboldCPP, ONNX Runtime, and python vLLM/exllamav2) under [ai_manager/mod.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/ai_manager/mod.rs) with config saving and deletion.
- **OpenAI-Compatible Local API Gateway:** Built-in HTTP API server (implemented via `axum` in [server.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/ai_manager/server.rs)) exposing standard `/v1/models` and `/v1/chat/completions` endpoints. Dynamically routes incoming queries to active local LLM backends based on resource allocation.

### 9. Python FFI Machine Learning Integration
- **Isolated ML Runtime Environment:** Automatically creates and initializes a Python virtual environment (`venv`) inside `app_data/venv` upon launching high-performance ML backends ([python_env.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/ai_manager/python_env.rs)).
- **Automatic Package Bootstrapping:** Installs PyTorch, Transformers, Accelerate, FastAPI, and Uvicorn automatically to serve as the ML engine basis.
- **PyO3 FFI Integration:** Loads heavy Python model pipelines (vLLM, ExLlamaV2, TensorRT-LLM) directly via the Rust `pyo3` Foreign Function Interface without spawning CLI subprocesses ([python_ffi.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/ai_manager/python_ffi.rs)), injecting the virtual environment site-packages path directly into `sys.path`.

### 10. Zen Browser Sandbox Integration
- **Isolated Web Browser testing:** Launch Zen Browser windows directly from the workspace for secure web previewing and debugging ([browser.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/commands/browser.rs)).
- **Executable Resolution Strategy:** Resolves the browser executable automatically via three search paths:
  1. *Production Bundle:* Looks under Tauri resource directory resources (`resources/zen_browser/`).
  2. *Development Bundle:* Looks for a local bundle directory (`tauri_app/zen_bundle/`).
  3. *System Defaults:* Looks in default system installation directories (Windows Programs, Local AppData, etc.).
- **Workspace Sandboxing:** Integrates browser tasks with AppContainer/Landlock boundaries to prevent local workspace exposure.

### 11. Alouette Open AI Diagnostic Scanner (AI Error Checker)
- **Background Log Monitor:** Runs a dedicated diagnostic scanner loop at 500ms intervals monitoring project logs in the background ([alouette_open.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/alouette_open.rs)).
- **ONNX Classification Model:** Executes the `alouette_open-A1 v1.0.onnx` neural model using `tract-onnx` on log outputs to classify whether stdout/stderr strings indicate software compilation, panic, runtime, or network errors.
- **Fallback Heuristics:** Includes a lightweight heuristic pattern scanner to serve as a fast fallback whenever the ONNX hardware target fails.
- **Warning Cards Alert:** Emits `alouette-open-error` events to trigger diagnostic warnings and instant-fix proposal cards directly in the frontend UI.

### 12. Welcome Page Dashboard
- **Landing Dashboard Hub:** Welcomes developers with a sleek, comprehensive summary panel when no projects are open ([WelcomePage.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/WelcomePage.tsx)).
- **At-a-Glance Metrics:** Displays system uptime, active network ports, CPU/RAM/GPU performance load indicators, and global configuration status.
- **Quick-Access Shortcuts:** One-click shortcuts to spawn new workspace projects, access local AI models, configure Cloudflare tunnels, or manage local virtual machine environments.

### 13. File Watcher & Workspace Syncer
- **FileSystem Watcher:** Native background watcher powered by the Rust `notify` crate ([file_watcher.rs](file:///home/nhatanh/projet/alouette_studio/tauri_app/src-tauri/src/commands/file_watcher.rs)) monitors the current workspace directory recursively.
- **Debounced Updates:** Groups modification events with a 300ms debounce to prevent frontend UI rendering overhead and spamming.
- **Live Syncing:** Emits `file-system-changed` events automatically to sync the React file explorer tree instantly when files are changed, created, or deleted externally.

### 14. Resource Monitoring & Throttling
- **Watchdog Enforcement:** Tracks real-time CPU/RAM usage per process tree and displays live canvas-based performance charts ([ProjectResources.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/ProjectResources.tsx)).
- **Limit Throttling:** Configures CPU % and RAM MB limits at project or global level. The watchdog automatically terminates processes exceeding limits for more than 30 seconds.

### 15. SQLite Browser & Caching
- **R2D2 Connection Pooling:** High-performance concurrent SQLite operations using `r2d2` pool with WAL (Write-Ahead Logging) mode ([db.rs](file:///home/nhatanh/projet/alouette_studio/core_engine/src/db.rs)).
- **Database Browser:** View all tables, inspect schemas, add columns, insert/delete rows, and edit individual cells directly ([SqliteEditor.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/SqliteEditor.tsx)).
- **Log Pruning:** Automatic system log truncation, keeping a clean history of the last N lines per project.

### 16. Cloudflared Tunnel Integration
- **Cloudflare Zero Trust:** Auto-download and update the latest `cloudflared` binary on startup with offline local fallback.
- **Modes:** Tunnel Free (auto-links local project port with `trycloudflare.com` URL) or Named Tunnel (Token-based authentication).
- **Title Bar Status:** Cloudflare cloud icon displays in orange when active, gray when inactive. Quick-click to toggle the tunnel state immediately.

### 17. System Manager & Global Configs
- **Tray & Keep-Alive:** Minimize to tray, double-click to restore, and intercept close events if "keep alive" is enabled.
- **VM Manager:** Manage local development VMs, network interfaces, and guest OS instances directly from the workspace UI ([VmManager.tsx](file:///home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/VmManager.tsx)).

### 18. AI Agent Loop & Assistant Interface
- **Autonomous AI Agent Loop:** Operates a closed-loop code modification engine powered by `rig-core 0.38` supporting multi-provider models (Claude, DeepSeek, ChatGPT, Gemini, etc.) and SQLite session persistence.
- **7 Operation Modes:** Interactive, Write, Autonomous, Plan Mode, Coordinator, Worker, and Minimal setups. Provides sub-agent coordination for nested parallel tasks.
- **Fuzzy-Matched File Mentions (@):** Type `@` in the chat input to search, score with fuzzy matching, and attach workspace files/folders directly as query context dependencies.
- **Token Analytics & Budgets:** Real-time token estimator displaying exact context allocations and total session accumulated token costs to manage API expenditure.
- **High/Low Thinking Budgets:** Toggle standard execution (low) or reasoning mode (high) to allocate reasoning tokens to reasoning models like DeepSeek-R1 or OpenAI o1/o3-mini.
- **Batch Authorization Guard:** Intercepts critical actions (file edits, system command executions, port binds, etc.) and presents them as expandable pending cards. Supports approval or rejection in batches or individually.
- **Sanitized Markdown Layouts:** Safely renders agent responses with HTML formatting, tables, list arrays, and code blocks using a sanitized pipeline (`react-markdown`, `rehype-sanitize`, and `DOMPurify`) to prevent malicious script payloads.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Tauri Desktop Shell                            │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    React Frontend (UI)                            │  │
│  │  Components | Hooks | Monaco | xterm.js | Charts | Lucide Icons │  │
│  │  react-markdown | fuse.js | Custom VM & Local AI Managers         │  │
│  │  Welcome Dashboard Page | PingZero Mini Interface                 │  │
│  │  AI Assistant panel (with @mentions, token metrics & batches)    │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  │              Tauri IPC Bridge (Commands + Events)                 │  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Core Engine (Rust)                              │  │
│  │  Process Manager | Sandbox | Proto | Cloudflared | R2D2 + SQLite │  │
│  │  Agent Harness (rig-core) | Alouette Open (tract-onnx)           │  │
│  │  Network Sim Proxy | System Manager | DashMap Registry           │  │
│  │  Code RAG (VectorDb) | Candle Local Inference & Chat              │  │
│  │  Notify File Watcher | Native Git2 Diff Engine                    │  │
│  │  OpenAI-Compatible Local API Gateway (Axum Router)               │  │
│  │  PyO3 Python ML Environment FFI integration                      │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Responsibility |
|-------|-----------|----------------|
| **UI** | React 19 + TypeScript + Vite + xterm.js + Monaco + react-markdown | Desktop interface with process dashboards, terminals, file editor, SQLite browser, PingZero Mini API client, local AI chatbot, VM manager, Welcome landing page, AI assistant chat panel |
| **Bridge** | Tauri v2 IPC | Type-safe command handlers (18 command modules), event routers (5 routers + agent + file watcher), state management (R2D2 pool, DashMap registry, Shared model manager) |
| **Engine** | Rust (Tokio async) | Process lifecycle, sandbox enforcement, proto toolchain, resource monitoring, SQLite persistence (r2d2 WAL), AI agent loop (rig-core), ONNX inference (tract-onnx) for error scanner, Code RAG indexing & similarity search, Candle local LLM inference, native git2 diffs, file notify watcher, Local Axum HTTP API gateway, PyO3 FFI interpreter |

---

## Project Structure

```
alouette_studio/
├── .gitignore
├── .prototools                       # Proto toolchain version pin
├── .taurignore
├── AI.json                            # AI assistant guidelines
├── Cargo.toml                         # Rust workspace root (core_engine + tauri_app/src-tauri)
├── README.md
├── logo_alouette.png
│
├── assets/                            # Static assets
│
├── scripts/                           # Utility scripts
│   ├── dev.sh                         # Development launcher
│   ├── download_embedding_model.sh    # Download Xenova/bge-small-en-v1.5
│   ├── generate_icons.js              # Icon generation scripts
│   ├── preprocess_rag_data.py         # Preprocess seed libraries
│   └── test_code_rag.sh               # Run Code RAG tests
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
│   │   ├── ai_config.yml              # AI provider config (Gemini, Claude, DeepSeek, GPT)
│   │   ├── alouette.db
│   │   ├── cloudflare_config.yml
│   │   ├── env_simulation.yml         # Simulated environment configs
│   │   ├── alouette_toolchains/       # Proto toolchain binaries
│   │   ├── db_RAG/                    # Code RAG seed library JSONs
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
│       │   ├── network_simulate_proxy.rs # SOCKS5/HTTP proxy: firewall, weak net, unstable server
│       │   └── sandbox/               # 3-tier command protection
│       ├── agent_harness/             # AI Agent Loop Engine
│       └── code_rag/                  # Code RAG Vector database & indexer
│           ├── mod.rs                 # Module declaration
│           ├── db.rs                  # VectorDb in-RAM / disk storage
│           ├── embedding.rs           # tract-onnx bge-small model embedding logic
│           ├── extractor.rs           # function parser & AST queries
│           ├── indexer.rs             # background indexing tasks & queues
│           ├── language_resolver.rs   # detect programming language from code
│           ├── languages.rs           # supported languages config list
│           ├── normalizer.rs          # function text normalization for embedding
│           ├── query.rs               # QueryEngine semantic matching logic
│           ├── seed.rs                # Seed library populator
│           └── queries/               # tree-sitter .scm AST queries for languages
│
└── tauri_app/                         # Tauri v2 desktop application
    ├── .taurignore
    ├── zen_bundle/                    # Zen Browser portable bundle (gitignored)
    ├── logs/                          # Runtime application logs
    ├── src-tauri/                     # Tauri Rust binary crate
    │   ├── Cargo.toml
    │   ├── build.rs
    │   ├── tauri.conf.json            # Tauri configuration (undecorated window, tray)
    │   ├── src/
    │   │   ├── main.rs                # Entry point + Tauri setup + exit cleanup
    │   │   ├── alouette_open.rs       # ONNX + heuristic log monitor (AI Error Checker)
    │   │   ├── events.rs              # 5 background event router tasks
    │   │   ├── state.rs               # AppState (R2D2 pool, DashMap registry, Arc<Mutex>)
    │   │   ├── system_manager.rs      # System tray, autostart, keep-alive, auto-restart
    │   │   ├── inference.rs           # HF Candle MiniCPM model runner
    │   │   ├── minicpm.rs             # MiniCPM transformer definition
    │   │   ├── model_manager.rs       # Model Manager state & loader
    │   │   ├── ai_manager/            # Supervisor for Ollama, llama.cpp, etc.
    │   │   │   ├── mod.rs             # start_ai_engine, stop_ai_engine commands
    │   │   │   ├── server.rs          # Axum gateway exposing OpenAI APIs locally
    │   │   │   ├── python_env.rs      # Automatic python-venv manager for heavy ML
    │   │   │   ├── python_ffi.rs      # PyO3 FFI binding loader
    │   │   │   └── config_storage.rs  # Local configurations load/save
    │   │   └── commands/              # 18 Tauri IPC command modules
    │   │       ├── mod.rs
    │   │       ├── code_rag.rs        # Code RAG Tauri commands
    │   │       ├── file_watcher.rs    # Notify filesystem change events
    │   │       ├── git_diff.rs        # Git2 diff handler
    │   │       ├── local_chat.rs      # Local chat commands
    │   │       └── ...
    │   └── app_data/                  # Runtime model weight folder (MiniCPM / RAG)
    │
    └── ui/                            # React frontend (Vite)
        ├── package.json
        ├── tsconfig.json
        ├── vite.config.ts
        └── src/
            ├── App.tsx                # Application orchestrator
            ├── index.css              # Global styles & dark/light theme
            ├── components/
            │   ├── CodeRagPanel.tsx        # Code RAG manager panel
            │   ├── CodeRagSearchWidget.tsx # Code search input UI
            │   ├── LocalAiManager.tsx      # Engine list & models download
            │   ├── LocalChat.tsx           # Local chatbot UI
            │   ├── VmManager.tsx           # Virtual machines supervisor
            │   ├── WelcomePage.tsx         # Welcome landing screen dashboard
            │   ├── MiniPostman.tsx         # PingZero Mini API Client UI
            │   ├── AiAgent.tsx             # AI Assistant main panel
            │   └── ...
            └── ...
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
| ONNX model  | BGE 1.5  | Required for Code RAG embeddings             |
| Python      | 3.10+    | Local ML virtual environment runtimes        |

---

## Quick Start

```bash
# Clone the repository
git clone <repository-url>
# Navigate to the workspace
cd alouette_studio

# Download Code RAG embedding model (bge-small-en-v1.5)
bash scripts/download_embedding_model.sh

# Install frontend dependencies
cd tauri_app/ui
npm install
cd ../..

# Run in development mode (spawns Vite server & Tauri app)
npx --prefix tauri_app/ui tauri dev
```

---

## Development & Testing

### Backend (Rust)

```bash
# Build the entire workspace
cargo build

# Run all tests (including SQLite DB tests and sandbox tests)
cargo test

# Run Code RAG checks specifically
bash scripts/test_code_rag.sh
```

### Frontend (React + TypeScript)

```bash
cd tauri_app/ui
# Start Vite dev server in standalone mode
npm run dev
# Perform TypeScript typecheck & compilation
npm run build
```

---

## IPC Command Modules

Alouette Studio exposes 80+ Tauri IPC commands registered across 18 modules:

| Module | Key Commands | Purpose |
|--------|--------------|---------|
| **process** | `start_project_process`, `stop_project_process`, `get_projects`, `register_project` | Project process lifecycle and logs |
| **terminal** | `spawn_terminal_session`, `write_to_terminal_session`, `resize_terminal_session` | Interactive PTY shell orchestration |
| **files** | `get_project_files`, `read_file_content`, `write_file_content`, `create_file` | Workspace file tree traversal & editing |
| **network** | `check_port_status`, `force_kill_process`, `send_http_request`, `dns_lookup`, `ping_host`, `ssl_certificate_info`, `jwt_decode` | Network queries, HTTP client and developer helpers |
| **git** | `git_get_status`, `git_stage_file`, `git_commit`, `git_push`, `git_pull` | Basic Git GUI controls |
| **git_diff** | `git_get_file_diff` | Computes line additions/deletions on file compared to HEAD |
| **settings** | `get_settings`, `save_settings`, `reset_settings` | Theme and global performance configs |
| **sqlite** | `get_sqlite_tables`, `get_sqlite_table_data`, `update_sqlite_cell`, `add_sqlite_column` | Database browser queries |
| **sandbox** | `load_sandbox_configs`, `save_sandbox_config`, `load_env_simulation_configs` | Safety barriers & environment degradation config |
| **cloudflare** | `load_cloudflare_config`, `save_cloudflare_config` | Zero-trust named & temporary tunnel manager |
| **language** | `get_language_runtimes`, `install_proto_tool` | Isolated Proto runtimes manager |
| **system** | `toggle_alouette_open`, `is_alouette_open_active` | Alouette Open diagnostic scanner control |
| **code_rag** | `code_rag_health`, `code_rag_query`, `code_rag_query_by_name`, `code_rag_scan_directory` | Vector DB semantic code querying and indexing |
| **local_chat** | `local_chat_send`, `local_chat_stop` | Candle local LLM inference streaming |
| **ai_manager** | `start_ai_engine`, `stop_ai_engine`, `get_ai_engine_status`, `save_ai_settings` | External LLM engines supervisor |

---

## Key Dependency Libraries

### Core Engine (Rust/Cargo)
- **tokio** (1.52): Async runtimes and channels.
- **sysinfo** (0.39): CPU & RAM metrics collection.
- **rusqlite** (0.39): Bundled SQLite interface.
- **portable-pty** (0.9): Cross-platform PTY manager.
- **tract-onnx** (0.23): ONNX model inference engine.

### Tauri App (Rust/Cargo)
- **tauri** (2.11): App shell, tray icons, system menu.
- **rig-core** (0.38): LLM Agent orchestration layer.
- **candle-core / candle-nn** (0.10): Local inference tensor execution.
- **git2** (0.19): Native libgit2 binding.
- **notify** (7.0): File system watcher.
- **r2d2 / r2d2_sqlite** (0.8): SQLite connection pool manager with WAL.
- **dashmap** (6.1): Concurrent memory registries.
- **pyo3** (0.21) & **ort** (2.0): FFI loaders for native PyTorch & ONNX execution.
- **axum** (0.7): Local API Gateway server.

### UI (React)
- **react / react-dom** (19.2): Rendering library.
- **react-markdown** (10.1) & **dompurify** (3.4): Safe HTML response layout.
- **xterm** (5.3) & **@monaco-editor/react** (4.7): Terminals and editor panes.
- **lucide-react** (1.17): Theme icon vectors.
