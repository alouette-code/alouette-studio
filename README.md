# Alouette Studio

Desktop Engineering Environment, Isolated Process Orchestrator, Virtualization Host, and Multi-Agent Runtime  
Môi trường Kỹ thuật Máy trạm, Bộ điều phối Tiến trình Cô lập, Máy chủ Ảo hóa và Hạ tầng Thực thi Đa Tác tử

Language / Ngôn ngữ: [English](#english-documentation) | [Tiếng Việt](#tài-liệu-tiếng-việt)

---

## English Documentation

### 1. System Overview

Alouette Studio is a native desktop engineering environment built as a Rust/Tauri v2 host wrapping a modular asynchronous core (`core_engine`), a TypeScript/React 19 frontend (`tauri_app/ui`), and an auxiliary OpenAI-compatible completion proxy (`server_proxy`).

The system consolidates deterministic process supervision, pre-spawn OS-level sandboxing, network fault-injection proxying, dual-engine code editing, native `git2` version control, local ONNX semantic code retrieval (Code RAG), QEMU VM orchestration with an embedded MCP server, Docker container management, eBPF/container memory telemetry, multi-protocol API diagnostics (PingZero Mini), multi-engine database inspection, WebAssembly (WASI Preview 1) plugin execution, and single/multi-agent LLM orchestration into a unified IPC architecture.

### 2. Workspace Architecture

```text
+-----------------------------------------------------------------------------+
|                        Frontend Shell (tauri_app/ui)                        |
|  React 19 + TypeScript + Vite 8 + Monaco Editor + CodeMirror 6 + xterm.js   |
|  noVNC (@novnc/novnc) + Recharts + Mermaid + React-Markdown (Sanitized)     |
+-----------------------------------------------------------------------------+
                                      |
                    Tauri v2 IPC (Commands & Event Streams)
                                      |
+-----------------------------------------------------------------------------+
|                    Tauri Host Layer (tauri_app/src-tauri)                   |
|  AppState (Arc/Mutex/RwLock/DashMap) | SQLite WAL Pool (r2d2_sqlite)        |
|  IPC Command Modules | Event Routers (Log, Status, Stats, Terminal, FS)     |
|  OpenCode Sidecar Bridge | Rig/LangChain Bridges | Axum VM MCP Server       |
|  Alouette Open Log Monitor (500ms Heuristic Scanner)                        |
+-----------------------------------------------------------------------------+
                                      |
                              Native Rust Crate API
                                      |
+-----------------------------------------------------------------------------+
|                         Core Engine (core_engine)                           |
|  ProcessManager & PTY (portable-pty)  | 3-Tier Sandbox (Tier 1a/1b/2)       |
|  ResourceMonitor, Watchdog & Breaker  | SOCKS5/HTTP Fault Simulation Proxy  |
|  Code RAG (tract-onnx + tokenizers)   | AgentHarness & Multi-Agent V2       |
|  VmManager (QEMU/QMP/QGA/vnc-rs)      | MemoryInspectorManager (bpftrace)   |
|  DockerEngine (bollard)               | WasmExtensionEngine (wasmtime WASI) |
|  ProtoManager | CloudflaredManager    | Agent Tools (CDP / OCR / Input)     |
+-----------------------------------------------------------------------------+
```

#### Layer Responsibilities

| Layer | Location | Primary Technologies | Responsibilities |
| :--- | :--- | :--- | :--- |
| UI Shell | `tauri_app/ui` | React 19.2, TypeScript 6.0, Vite 8.0, Monaco, CodeMirror 6, xterm.js, noVNC | Split code editing, interactive WebGL PTY terminals, Git diff visualization, VM VNC viewport, API/DB consoles, Swarm board, Memory telemetry charts. |
| Tauri Host | `tauri_app/src-tauri` | Tauri 2.11, Tokio 1.52, `r2d2_sqlite`, `dashmap`, `axum`, `git2`, `notify`, `sqlx` | IPC command dispatch, non-blocking event routing, WAL-mode SQLite persistence, sidecar lifecycle, database/protocol drivers, MCP HTTP endpoint. |
| Core Engine | `core_engine` | Tokio, `portable-pty`, `sysinfo`, `tract-onnx`, `wasmtime` 29, `bollard`, `vnc-rs` | Process tree supervision, command/OS sandboxing, vector RAG indexing/querying, VM/QMP/QGA control, WASI execution, multi-agent state machines. |
| Completion Proxy | `server_proxy` | Node.js, Express 4.19, `groq-sdk` | Optional local HTTP proxy on port `3001` forwarding `/v1/chat/completions` and `/v1/code/completions` for inline ghost-text completion. |

---

### 3. Core Subsystems

#### 3.1 Process Lifecycle, PTY Terminals, and Resource Watchdog
- **Process Orchestration (`core_engine/src/process/`)**: Registers, spawns, monitors, and deregisters project processes with pre-execution setup hook chains, recursive process-tree teardown (`terminate_process_tree`), and SQLite-backed log persistence (`spawn_log_persister`).
- **Interactive PTY (`core_engine/src/process/terminal.rs`)**: Uses `portable-pty` to allocate pseudo-terminals with input buffering (`sync_terminal_input_buf`), resize support, and real-time output routing to `xterm.js` (`@xterm/addon-webgl`, `@xterm/addon-fit`).
- **Telemetry & Enforcement (`core_engine/src/monitor/`)**: Polls process trees via `sysinfo`, tracks aggregate CPU/RAM utilization, and enforces resource bounds using watchdog termination and circuit-breaker primitives (`monitor/watchdog.rs`, `monitor/circuit_breaker.rs`).

#### 3.2 Three-Tier Command and Process Sandbox
Implemented in `core_engine/src/process/sandbox/`:
- **Tier 1a — Semantic Interceptor (`interceptor.rs`)**: Parses compound shell expressions (pipes, command chains, subexpressions), maps commands and aliases to risk tiers (`Safe`, `Low`, `Medium`, `High`, `Critical`, `Blocked`), resolves environment variables and `~` references, and rejects workspace boundary escapes.
- **Tier 1b — Cross-Platform Fallback Engine (`engine.rs`)**: Tokenizes commands and validates path targets when Tier 1a returns `Allow`.
- **Tier 2 — OS-Level Confinement**:
  - **Linux (`linux.rs`)**: Applies async-signal-safe `pre_exec` hooks in the forked child prior to `execve()` without root privileges: `prctl(PR_SET_NO_NEW_PRIVS, 1)`, `setrlimit(RLIMIT_AS)`, and optional `unshare(CLONE_NEWNET)`.
  - **Windows (`windows.rs`)**: Confines processes using Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) and AppContainer primitives.
  - **macOS (`macos.rs`)**: Currently stubbed as a placeholder.

#### 3.3 Network Isolation and Fault-Simulation Proxy
- **Process Network Isolation (`core_engine/src/process/network_isolate.rs`)**: Applies OS-level network isolation rules per process configuration.
- **SOCKS5/HTTP Forward Simulation Proxy (`core_engine/src/process/network_simulate_proxy.rs`)**: Spawns a local forward proxy (`127.0.0.1:0`) supporting domain/glob firewall blocklists, artificial latency, jitter, packet-loss rate, bandwidth caps (`kbps`), unstable server fault injection (connection drops and HTTP error status codes), and inbound/outbound header injection.

#### 3.4 Dual-Engine Code Editor, File Watcher, and Native Git
- **Dual Editor Runtime (`tauri_app/ui/src/components/CodeEditor.tsx`, `useEditorEngine.ts`)**: Supports live runtime switching between **Monaco Editor** (`@monaco-editor/react`) and **CodeMirror 6** (`@uiw/react-codemirror` with `@replit/codemirror-minimap` and custom Git diff gutter markers), multi-pane split layouts, cursor/scroll state recovery (`editorStateStore.ts`), and debounced inline AI ghost-text completion (`aiCompletionService.ts`).
- **Filesystem Sync (`tauri_app/src-tauri/src/commands/file_watcher.rs`)**: Uses `notify` v7 to watch workspace changes and emit debounced updates to the frontend file explorer.
- **Native Git Engine (`commands/git.rs`, `commands/git_diff.rs`)**: Uses `git2` directly for repository status, staging/unstaging, discarding changes, commit history, commit rewording, push/pull, single-commit push, and line-level diff calculation for editor gutter/minimap rendering.

#### 3.5 AI Agent Harness, OpenCode Bridge, and Multi-Agent Systems
- **Single-Agent Harness (`core_engine/src/agent_harness/`)**: Implements `AgentHarness` with five operating modes (`Standard`, `Plan`, `Coordinator`, `Worker`, `Autonomous`), context compaction (`compaction.rs`), SQLite WAL session persistence (`r2d2_sqlite`), self-healing, skills, hooks, and tool approval gates.
- **LLM Bridges (`commands/rig_bridge.rs`, `commands/langchain_rig_bridge.rs`, `commands/opencode_bridge.rs`)**:
  - Direct multi-provider execution via `rig-core` and `langchain-rust` with tool-call sequence sanitization.
  - Managed **OpenCode** sidecar runtime (`OpenCodeManager`) communicating over HTTP and SSE (`/event`), supporting dynamic model routing, turn cancellation, clipboard image attachments, message revert, and interactive tool permission prompts.
- **Multi-Agent Coordinator V2 (`core_engine/src/multi_agent/`, `commands/multi_agent.rs`)**: Deterministic state machine (`Init -> Planning -> Coding -> Verifying -> Finished/Halted/Error`) coordinating scoped roles (`secretary`, `coder`, `verifier`) over an asynchronous `MemoryStore` blackboard.
- **Super Multi-Agent Swarm (`commands/super_multi_agent.rs`, `core_engine/src/multi_agent/orchestration/safeguards.rs`, `SuperSwarmBoard.tsx`)**:
  - Orchestrates a **Commander**, **Supervisor**, and up to `MAX_SUPER_AGENTS = 30` worker instances (`read_only`, `coder`, `verifier`) over isolated OpenCode sessions multiplexed through a single SSE `/event` stream.
  - Bounds parallel execution via `tokio::sync::Semaphore`, injects Code RAG project context, enforces interactive human approval channels (`super_agent_approvals`), and runs `WorkerLoopDetector` safeguards against `OUT_OF_SCOPE_WRITE`, `FILE_THRASHING`, `DUPLICATE_CALL_LOOP`, and `CYCLIC_LOOP`.
- **Alouette Open Monitor (`tauri_app/src-tauri/src/alouette_open.rs`)**: Background 500ms polling loop inspecting the latest 50 persisted logs per project via heuristic matching (`error`, `exception`, `failed`, `panic`, `critical`, `stderr`) to emit live diagnostic notifications.

#### 3.6 Code RAG (Local Semantic Retrieval)
Implemented in `core_engine/src/code_rag/` and exposed via `commands/code_rag.rs`:
- **Local Embedding Inference (`embedding.rs`)**: Executes `BAAI/bge-small-en-v1.5` (ONNX format, 384-dimensional unit-normalized embeddings) locally on CPU using `tract-onnx` and HuggingFace `tokenizers`.
- **Tiered Extraction (`extractor.rs`, `tier.rs`, `queries/*.scm`)**: Classifies languages across `Tier1`, `Tier2`, and `Tier3`; supports Tree-sitter AST query extraction when compiled with the `tree-sitter` Cargo feature and regex-based signature extraction as fallback/default.
- **Incremental Indexing & Querying (`indexer.rs`, `db.rs`, `query.rs`)**: Maintains a local vector index with project rescan, per-file indexing, symbol-name lookup, and semantic similarity search.

#### 3.7 QEMU/KVM Virtual Machine Manager and Embedded VM MCP Server
Implemented in `core_engine/src/vm_engine/`, `commands/vm.rs`, and `commands/vm_mcp.rs`:
- **VM Lifecycle & State (`manager.rs`, `qemu_wrapper.rs`, `config.rs`)**: Manages QEMU instances via `vm.json` (with legacy `.vmx` fallback), resolves QEMU binaries, cleans up orphaned `qemu-system-*` processes on initialization, and streams VM console logs.
- **Control & Guest Channels**:
  - **QMP (`qmp_client.rs`)**: Sends structured mouse, keyboard, scroll, and wait actions (`VmInputAction`) and manages live VM snapshots (`create_vm_snapshot`, `restore_vm_snapshot`, `delete_vm_snapshot`, `list_vm_snapshots`).
  - **QGA (`qga_client.rs`)**: Communicates over Unix sockets with QEMU Guest Agent for in-guest command execution and file injection (`inject_guest_file`).
  - **VNC Streaming (`vnc_stream.rs`, `VmConsole.tsx`)**: Bridges frame capture via `vnc-rs` and interactive browser rendering via `@novnc/novnc`.
  - **Action Compiler (`action_compiler.rs`)**: Translates natural-language instructions and screen OCR observations into batched `VmInputAction` sequences via local OpenAI-compatible endpoints (Ollama / llama.cpp).
- **Embedded VM MCP Server (`commands/vm_mcp.rs`)**: Exposes a localhost Streamable HTTP MCP endpoint (`/mcp` via `axum` with bearer token authentication) allowing external/internal AI agents to inspect and drive running VMs.

#### 3.8 Docker Manager and Memory Inspector
- **Docker Engine (`core_engine/src/docker_engine/`, `commands/docker.rs`)**: Interfaces with the local Docker daemon via `bollard` for image listing, container lifecycle (`create`, `start`, `stop`, `restart`, `remove`), live log/stats streaming, and interactive `docker exec` PTY sessions.
- **Memory Inspector (`core_engine/src/memory_inspector/`, `commands/memory_inspector/`)**: Combines container/process memory telemetry, a `bpftrace`-based `EbpfTracer` (attaching uprobes to `libc.so.6:malloc`/`free`), memory stress fuzzing and ramp-down controllers (`stress/fuzzer.rs`, `stress/ramp_down.rs`), linear regression drift/`R^2` analysis (`profiling/analyzer.rs`), and frontend Pressure Chamber / Heatmap visualizations.

#### 3.9 PingZero Mini API Client and Network Diagnostics
Implemented in `commands/network.rs`, `websocket.rs`, `sse.rs`, `grpc.rs`, `oauth.rs`, and `load_test.rs`:
- **Protocol Clients**: HTTP(S) (`reqwest` with cookie jar and mTLS client cert options), WebSocket (`tokio-tungstenite` with `rustls`), Server-Sent Events (`reqwest-eventsource`), and dynamic gRPC reflection/invocation (`tonic` + `prost-reflect`).
- **Authentication & Automation**: Local OAuth 2.0 callback server (`oauth2`), workspace/collection persistence, JSON Schema validation (`jsonschema`), and pre/post request scripting.
- **Load Testing & Diagnostics**: `tokio`-based concurrent load runner (`run_load_test`), DNS lookup, TCP/ICMP host ping, SSL/TLS certificate inspection, cURL generation, JWT decoding, hashing, Base64, XML prettification, and response diffing.

#### 3.10 Universal Database and KV Store Inspector
Implemented across `commands/database.rs`, `sqlite.rs`, `db_duckdb.rs`, `db_kv.rs`, `db_mongo.rs`, and `db_redis.rs`:
- **Relational & Analytical**: SQLite (`rusqlite` / `sqlx`), PostgreSQL & MySQL (`sqlx` with `runtime-tokio-rustls` and optional X.509 SSL URI parameters), and DuckDB (`duckdb` bundled).
- **Document & Key-Value**: MongoDB (`mongodb` with `rustls-tls` and X.509 support), Redis (`redis` multiplexed Tokio connections with `SCAN`), and embedded Rust KV stores (`sled` and `redb`).
- **Capabilities**: Connection pooling/caching, schema/table/namespace enumeration, paginated inspection, arbitrary query execution, inline cell updates, row insertion/deletion, and column addition.

#### 3.11 WebAssembly (WASI Preview 1) Extension Runtime and Registry
Implemented in `core_engine/src/extension_manager/`, `commands/extension.rs`, and `commands/marketplace_cmds.rs`:
- **Sandboxed Runtime (`wasm_engine.rs`)**: Embeds `wasmtime` v29 with `wasmtime-wasi` Preview 1, asynchronous execution, and deterministic fuel metering (`10_000_000` fuel units per invocation).
- **Permission Interceptor**: Exposes `alouette_host::check_permission` to guest modules to enforce declared manifest capabilities before host calls execute.
- **Serverless Registry & Integrity (`marketplace.rs`)**: Fetches registry metadata from `https://cdn.jsdelivr.net/gh/alouette-code/alouette-extension-registry@main/index.json`, validates SHA-256 checksums, verifies Ed25519 signatures (`ed25519-dalek`), and supports GitHub PR payload generation for publishing.

#### 3.12 Toolchains, Tunnels, and Browser Automation
- **Proto Toolchain Manager (`core_engine/src/proto_manager.rs`, `commands/language.rs`)**: Provisions and isolates `proto` CLI and runtime shims inside `app_data/alouette_toolchains` with platform mismatch cleanup.
- **Cloudflared Manager (`core_engine/src/cloudflared_manager.rs`, `commands/cloudflare.rs`)**: Downloads, manages, and configures `cloudflared` tunnels.
- **Isolated Chrome & Agent Tools (`commands/browser.rs`, `core_engine/src/agent_tools/`)**: Launches Chrome with an ephemeral clean profile and `--remote-debugging-port=9222`; provides backend `chromiumoxide`, `headless_chrome`, `enigo`, and `rusty-tesseract` automation primitives.

---

### 4. Repository Layout

```text
alouette_studio/
├── Cargo.toml                         # Workspace root (core_engine, tauri_app/src-tauri)
├── core_engine/
│   ├── Cargo.toml                     # Core engine crate dependencies & features
│   └── src/
│       ├── agent_harness/             # Single-agent loop, compaction, memory, plans, skills
│       ├── agent_tools/               # Browser (CDP), mouse/keyboard, screen OCR primitives
│       ├── code_rag/                  # ONNX embedding (tract-onnx), extractors, vector DB
│       ├── db/                        # Core SQLite schema, R2D2 manager, repositories
│       ├── docker_engine/             # Bollard Docker daemon client, logs, stats, PTY exec
│       ├── extension_manager/         # Wasmtime WASI runtime, manifest, Ed25519 marketplace
│       ├── memory_inspector/          # eBPF tracer (bpftrace), fuzzer, drift analyzer
│       ├── multi_agent/               # Coordinator V2, WorkerSession, safeguards, blackboard
│       ├── process/                   # ProcessManager, PTY, 3-tier sandbox, simulation proxy
│       ├── vm_engine/                 # QEMU wrapper, QMP, QGA, VNC stream, action compiler
│       ├── cloudflared_manager.rs     # Cloudflare tunnel provisioning
│       ├── monitor.rs & monitor/      # ResourceMonitor, circuit breaker, watchdog
│       ├── proto_manager.rs           # Proto toolchain isolation
│       └── workspace_manager.rs       # Isolated workspace directory management
├── tauri_app/
│   ├── src-tauri/
│   │   ├── Cargo.toml                 # Tauri host binary dependencies
│   │   ├── tauri.conf.json            # Window, CSP, and bundle configuration
│   │   └── src/
│   │       ├── main.rs                # Tauri setup, tray, routers, command registration
│   │       ├── state.rs               # Shared AppState and SQLite WAL pool
│   │       ├── alouette_open.rs       # 500ms heuristic log monitor
│   │       └── commands/              # IPC command modules (agent, swarm, vm, db, git, etc.)
│   └── ui/
│       ├── package.json               # React 19 + Vite 8 + TypeScript dependencies
│       └── src/                       # Components, hooks, services, types
├── server_proxy/
│   ├── package.json                   # Express + groq-sdk dependencies
│   └── server.js                      # OpenAI-compatible Groq proxy (port 3001)
├── scripts/
│   ├── dev.sh                         # Tauri dev launcher (--clean support)
│   ├── download_embedding_model.sh    # Fetches Xenova/bge-small-en-v1.5 ONNX + tokenizer
│   └── test_code_rag.sh               # Runs core_engine::code_rag unit tests
└── docs/                              # Subsystem architecture notes
```

---

### 5. Build, Setup, and Execution

#### Prerequisites
- **Rust Toolchain**: Stable Rust (`edition = "2021"`) with Cargo (`rustup`).
- **Node.js & npm**: Required for `tauri_app/ui` and optional `server_proxy`.
- **Tauri System Dependencies**: Platform-specific WebView/GTK/build libraries required by Tauri v2 (on Linux: WebKitGTK, GTK3, `libssl-dev`, `pkg-config`, etc.).
- **Optional Runtime Dependencies** (required only by their respective subsystems):
  - `qemu-system-*`, `qemu-img`, and OVMF firmware for VM virtualization.
  - Docker Engine daemon accessible via local socket for Docker Manager.
  - `bpftrace` (with kernel/uprobe permissions) for live eBPF allocation tracing.
  - `opencode` binary for OpenCode sidecar agent execution and Super Multi-Agent Swarm.

#### Setup and Development Launch

```bash
# 1. Download the local ONNX embedding model for Code RAG (~33 MB)
#    Target: tauri_app/app_data/model_embedding/bge-small-en-v1.5/
bash scripts/download_embedding_model.sh

# 2. Install frontend dependencies
npm --prefix tauri_app/ui install

# 3. (Optional) Start the Groq completion proxy for inline ghost-text completion
npm --prefix server_proxy install
npm --prefix server_proxy start

# 4. Launch Alouette Studio in Tauri development mode
bash scripts/dev.sh
# Or directly:
npx --prefix tauri_app/ui tauri dev
```

#### Build and Test Commands

```bash
# Compile Rust workspace (core_engine + alouette-studio)
cargo build

# Compile core_engine with optional tree-sitter AST extraction enabled
cargo build --package core_engine --features tree-sitter

# Run Rust workspace tests
cargo test

# Run Code RAG module test suite
bash scripts/test_code_rag.sh

# Frontend type-check and production bundle
npm --prefix tauri_app/ui run build

# Build release Tauri application bundle
npx --prefix tauri_app/ui tauri build
```

---

### 6. Wasm Extension Specification

Local extensions are discovered under:
- **Linux / macOS**: `~/.alouette/extensions/<publisher_id>.<extension_id>/`
- **Windows**: `C:\Users\<User>\.alouette\extensions\<publisher_id>.<extension_id>\`

Required directory layout:

```text
<publisher_id>.<extension_id>/
├── proto-extension.json
└── plugin.wasm
```

Example `proto-extension.json`:

```json
{
  "id": "publisher_name.my-extension",
  "name": "My WASI Extension",
  "version": "1.0.0",
  "description": "Sandboxed WASI Preview 1 module",
  "runtime": {
    "type": "wasm",
    "entry": "plugin.wasm"
  },
  "capabilities": {
    "permissions": ["fs:read", "net:http"]
  }
}
```

Compiling a Rust guest module for WASI Preview 1:

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

---
---

## Tài liệu Tiếng Việt

### 1. Tổng quan Hệ thống

Alouette Studio là môi trường kỹ thuật máy trạm (desktop engineering environment) xây dựng trên kiến trúc Rust và Tauri v2, kết hợp thư viện lõi bất đồng bộ (`core_engine`), giao diện TypeScript/React 19 (`tauri_app/ui`) và dịch vụ proxy trung gian tương thích chuẩn OpenAI (`server_proxy`).

Hệ thống hợp nhất trên một biên giao tiếp IPC duy nhất các phân hệ: quản lý vòng đời tiến trình tất định, cơ chế cô lập lệnh 3 tầng ở mức hệ điều hành trước thời điểm thực thi (`pre-spawn`), proxy giả lập lỗi mạng SOCKS5/HTTP, trình soạn thảo mã nguồn kép (Monaco / CodeMirror 6), tích hợp Git nguyên bản qua `git2`, hệ thống truy vấn ngữ nghĩa mã nguồn cục bộ bằng mô hình ONNX (Code RAG), điều phối máy ảo QEMU kèm máy chủ MCP nhúng, quản lý container Docker, giám sát bộ nhớ qua `bpftrace` và phân tích hồi quy trôi bộ nhớ, công cụ kiểm thử giao thức mạng (PingZero Mini), trình duyệt đa cơ sở dữ liệu, môi trường thực thi plugin WebAssembly (WASI Preview 1) và hệ thống điều phối tác tử AI đơn luồng lẫn đa tác tử.

### 2. Kiến trúc Workspace

```text
+-----------------------------------------------------------------------------+
|                    Giao diện Người dùng (tauri_app/ui)                      |
|  React 19 + TypeScript + Vite 8 + Monaco Editor + CodeMirror 6 + xterm.js   |
|  noVNC (@novnc/novnc) + Recharts + Mermaid + React-Markdown (Sanitized)     |
+-----------------------------------------------------------------------------+
                                      |
              Biên giao tiếp Tauri v2 IPC (Commands & Event Streams)
                                      |
+-----------------------------------------------------------------------------+
|                    Lớp Máy chủ Tauri (tauri_app/src-tauri)                  |
|  AppState (Arc/Mutex/RwLock/DashMap) | Pool SQLite WAL (r2d2_sqlite)        |
|  Các module IPC Command | Bộ định tuyến sự kiện (Log, Status, Stats, FS)    |
|  OpenCode Sidecar Bridge | Rig/LangChain Bridges | Máy chủ HTTP Axum VM MCP |
|  Alouette Open Log Monitor (Quét heuristic chu kỳ 500ms)                    |
+-----------------------------------------------------------------------------+
                                      |
                        API Thư viện Rust Nguyên bản
                                      |
+-----------------------------------------------------------------------------+
|                         Thư viện Lõi (core_engine)                          |
|  ProcessManager & PTY (portable-pty)  | Sandbox 3 tầng (Tier 1a/1b/2)       |
|  ResourceMonitor, Watchdog & Breaker  | Proxy Giả lập Lỗi Mạng SOCKS5/HTTP  |
|  Code RAG (tract-onnx + tokenizers)   | AgentHarness & Multi-Agent V2       |
|  VmManager (QEMU/QMP/QGA/vnc-rs)      | MemoryInspectorManager (bpftrace)   |
|  DockerEngine (bollard)               | WasmExtensionEngine (wasmtime WASI) |
|  ProtoManager | CloudflaredManager    | Agent Tools (CDP / OCR / Input)     |
+-----------------------------------------------------------------------------+
```

#### Phân tách Trách nhiệm Các Tầng

| Tầng | Đường dẫn | Công nghệ Cốt lõi | Trách nhiệm Kỹ thuật |
| :--- | :--- | :--- | :--- |
| Giao diện (UI) | `tauri_app/ui` | React 19.2, TypeScript 6.0, Vite 8.0, Monaco, CodeMirror 6, xterm.js, noVNC | Chia khung soạn thảo, terminal PTY tăng tốc WebGL, trực quan hóa Git diff, màn hình VNC máy ảo, bảng điều khiển API/DB, bảng trạng thái Swarm, đồ thị áp suất bộ nhớ. |
| Lớp Tauri Host | `tauri_app/src-tauri` | Tauri 2.11, Tokio 1.52, `r2d2_sqlite`, `dashmap`, `axum`, `git2`, `notify`, `sqlx` | Điều phối lệnh IPC, phát luồng sự kiện bất đồng bộ, lưu trữ SQLite chế độ WAL, quản lý tiến trình phụ OpenCode, trình điều khiển CSDL/giao thức mạng, máy chủ MCP HTTP. |
| Thư viện Lõi | `core_engine` | Tokio, `portable-pty`, `sysinfo`, `tract-onnx`, `wasmtime` 29, `bollard`, `vnc-rs` | Giám sát cây tiến trình, sandbox lệnh và OS, lập chỉ mục và suy luận vector RAG, điều khiển QEMU/QMP/QGA, thực thi WASI, máy trạng thái đa tác tử. |
| Proxy Gợi ý Mã | `server_proxy` | Node.js, Express 4.19, `groq-sdk` | Proxy HTTP cục bộ tùy chọn tại cổng `3001` chuyển tiếp `/v1/chat/completions` và `/v1/code/completions` phục vụ tính năng hoàn thiện mã nội tuyến (inline ghost text). |

---

### 3. Đặc tả Kỹ thuật Các Phân hệ

#### 3.1 Quản lý Vòng đời Tiến trình, PTY Terminal và Watchdog Tài nguyên
- **Điều phối Tiến trình (`core_engine/src/process/`)**: Đăng ký, khởi chạy, theo dõi và hủy đăng ký tiến trình dự án kèm chuỗi lệnh khởi tạo (`setup` hooks), thu hồi đệ quy toàn bộ cây tiến trình con (`terminate_process_tree`) nhằm triệt tiêu tiến trình mồ côi, và ghi nhật ký bất đồng bộ xuống SQLite (`spawn_log_persister`).
- **Terminal PTY Tương tác (`core_engine/src/process/terminal.rs`)**: Sử dụng `portable-pty` cấp phát giả thiết bị đầu cuối, hỗ trợ đồng bộ bộ đệm nhập liệu (`sync_terminal_input_buf`), thay đổi kích thước khung PTY và truyền luồng xuất ra `xterm.js` (`@xterm/addon-webgl`, `@xterm/addon-fit`).
- **Đo lường & Cưỡng chế Giới hạn (`core_engine/src/monitor/`)**: Thu thập thông số CPU/RAM trên toàn cây PID bằng `sysinfo`, áp dụng cơ chế ngắt mạch (`circuit_breaker.rs`) và `watchdog.rs` để cưỡng chế dừng khi vượt ngưỡng tài nguyên cấu hình.

#### 3.2 Cơ chế Bảo vệ Sandbox 3 Tầng
Triển khai tại `core_engine/src/process/sandbox/`:
- **Tầng 1a — Bộ phân tích Ngữ nghĩa (`interceptor.rs`)**: Phân rã cây lệnh shell (pipe, chuỗi lệnh, biểu thức con), phân loại mức rủi ro (`Safe`, `Low`, `Medium`, `High`, `Critical`, `Blocked`), nội suy biến môi trường và đường dẫn `~` về đường dẫn tuyệt đối chuẩn nhằm chặn kỹ thuật vượt biên thư mục workspace.
- **Tầng 1b — Bộ kiểm tra Dự phòng Đa nền tảng (`engine.rs`)**: Tách token và kiểm tra biên đường dẫn làm lớp phòng vệ thứ hai nếu Tầng 1a trả về `Allow`.
- **Tầng 2 — Cô lập Mức Hệ điều hành**:
  - **Linux (`linux.rs`)**: Cài đặt hook `pre_exec` trong tiến trình con ngay sau `fork()` và trước `execve()` (không yêu cầu quyền root, tuân thủ tuyệt đối async-signal-safety chỉ dùng `libc::write` và lời gọi hệ thống an toàn): `prctl(PR_SET_NO_NEW_PRIVS, 1)` chặn leo thang đặc quyền, `setrlimit(RLIMIT_AS)` giới hạn không gian địa chỉ ảo, và `unshare(CLONE_NEWNET)` cô lập namespace mạng.
  - **Windows (`windows.rs`)**: Cô lập tiến trình bằng Windows Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) và AppContainer.
  - **macOS (`macos.rs`)**: Hiện ở trạng thái placeholder, chưa triển khai sandbox mức nhân.

#### 3.3 Cô lập Mạng và Proxy Giả lập Lỗi (SOCKS5/HTTP)
- **Cô lập Mạng Tiến trình (`core_engine/src/process/network_isolate.rs`)**: Thực thi quy tắc chặn truy cập mạng theo cấu hình từng tiến trình.
- **Proxy Giả lập Suy hao Mạng (`core_engine/src/process/network_simulate_proxy.rs`)**: Khởi tạo forward proxy SOCKS5/HTTP cục bộ (`127.0.0.1:0`) hỗ trợ: tường lửa chặn tên miền theo mẫu glob, chèn độ trễ (`latency_ms`), biến thiên độ trễ (`jitter_ms`), tỷ lệ rớt gói (`loss_rate`), bóp băng thông (`bandwidth_kbps`), giả lập máy chủ không ổn định (ngắt kết nối ngẫu nhiên, tiêm mã lỗi HTTP) và chèn header chiều vào/ra.

#### 3.4 Trình soạn thảo Kép, Giám sát Tệp và Git Nguyên bản
- **Kiến trúc Trình soạn thảo Kép (`tauri_app/ui/src/components/CodeEditor.tsx`, `useEditorEngine.ts`)**: Cho phép chuyển đổi nóng khi đang chạy giữa **Monaco Editor** (`@monaco-editor/react`) và **CodeMirror 6** (`@uiw/react-codemirror` kết hợp `@replit/codemirror-minimap` cùng marker trạng thái Git trên gutter), hỗ trợ chia tối đa nhiều khung soạn thảo, khôi phục vị trí con trỏ/cuộn (`editorStateStore.ts`) và gợi ý mã nội tuyến qua `http://localhost:3001` với cơ chế cửa sổ ngữ cảnh prefix/suffix và giới hạn tần suất gọi (`aiCompletionService.ts`).
- **Đồng bộ Hệ thống Tệp (`tauri_app/src-tauri/src/commands/file_watcher.rs`)**: Sử dụng crate `notify` v7 giám sát thay đổi thư mục dự án ở mức hệ điều hành và phát sự kiện đã lọc nhiễu (debounce) lên giao diện.
- **Nhân Git Nguyên bản (`commands/git.rs`, `commands/git_diff.rs`)**: Dùng trực tiếp thư viện `git2` (không phụ thuộc lệnh shell ngoài) để truy vấn trạng thái, stage/unstage, hủy thay đổi, commit, push, pull, push từng commit đơn lẻ, sửa thông điệp commit (`reword`) và tính toán diff theo từng dòng.

#### 3.5 Khung Tác tử AI, Cầu nối OpenCode và Điều phối Đa Tác tử
- **Khung Tác tử Đơn (`core_engine/src/agent_harness/`)**: Triển khai `AgentHarness` với 5 chế độ (`Standard`, `Plan`, `Coordinator`, `Worker`, `Autonomous`), nén ngữ cảnh (`compaction.rs`), lưu trữ phiên vào SQLite WAL (`r2d2_sqlite`), cơ chế tự phục hồi, kỹ năng, hooks và cổng phê duyệt thực thi công cụ.
- **Các Cầu nối LLM (`commands/rig_bridge.rs`, `commands/langchain_rig_bridge.rs`, `commands/opencode_bridge.rs`)**:
  - Gọi trực tiếp mô hình đa nhà cung cấp qua `rig-core` và `langchain-rust` kèm bộ chuẩn hóa trình tự `tool_calls`/`tool_result`.
  - Quản lý tiến trình phụ **OpenCode** (`OpenCodeManager`) qua HTTP và luồng SSE (`/event`), hỗ trợ điều phối mô hình động, ngắt lượt chạy tức thời, đính kèm ảnh từ clipboard, hoàn tác lịch sử hội thoại và cơ chế xin cấp quyền thực thi công cụ.
- **Bộ điều phối Đa Tác tử V2 (`core_engine/src/multi_agent/`, `commands/multi_agent.rs`)**: Máy trạng thái tất định (`Init -> Planning -> Coding -> Verifying -> Finished/Halted/Error`) điều phối các vai trò giới hạn quyền công cụ (`secretary`, `coder`, `verifier`) thông qua bảng nhớ dùng chung (`MemoryStore`).
- **Hệ thống Super Multi-Agent Swarm (`commands/super_multi_agent.rs`, `core_engine/src/multi_agent/orchestration/safeguards.rs`, `SuperSwarmBoard.tsx`)**:
  - Điều phối cấu trúc phân cấp gồm **Commander**, **Supervisor** và tối đa `MAX_SUPER_AGENTS = 30` worker (`read_only`, `coder`, `verifier`) chạy trên các phiên OpenCode độc lập được ghép kênh (multiplex) qua một kết nối SSE `/event` duy nhất.
  - Giới hạn độ song song bằng `tokio::sync::Semaphore`, tự động tiêm ngữ cảnh Code RAG, tích hợp cơ chế phê duyệt thủ công qua kênh `oneshot` (`super_agent_approvals`), và vận hành bộ phát hiện vòng lặp tất định `WorkerLoopDetector` nhằm ngắt sớm các lỗi `OUT_OF_SCOPE_WRITE`, `FILE_THRASHING`, `DUPLICATE_CALL_LOOP` và `CYCLIC_LOOP`.
- **Giám sát Nhật ký Alouette Open (`tauri_app/src-tauri/src/alouette_open.rs`)**: Tác vụ nền chu kỳ 500ms truy xuất 50 bản ghi nhật ký mới nhất theo từng dự án từ SQLite và đối chiếu heuristic (`error`, `exception`, `failed`, `panic`, `critical`, `stderr`) để phát cảnh báo chẩn đoán lên giao diện.

#### 3.6 Hệ thống Code RAG (Truy xuất Ngữ nghĩa Mã nguồn Cục bộ)
Triển khai tại `core_engine/src/code_rag/` và `commands/code_rag.rs`:
- **Suy luận Vector Cục bộ (`embedding.rs`)**: Nạp mô hình `BAAI/bge-small-en-v1.5` (định dạng ONNX, vector chuẩn hóa đơn vị 384 chiều) trực tiếp trên CPU thông qua `tract-onnx` và `tokenizers`.
- **Trích xuất Phân tầng (`extractor.rs`, `tier.rs`, `queries/*.scm`)**: Phân loại ngôn ngữ theo `Tier1`, `Tier2`, `Tier3`; hỗ trợ truy vấn cú pháp AST qua feature `tree-sitter` (tùy chọn lúc biên dịch) và cơ chế trích xuất chữ ký hàm bằng biểu thức chính quy làm mặc định/dự phòng.
- **Lập chỉ mục & Truy vấn (`indexer.rs`, `db.rs`, `query.rs`)**: Hỗ trợ quét lại toàn bộ dự án, lập chỉ mục tăng tiến theo tệp, tìm kiếm chính xác theo tên ký hiệu và truy vấn tương đồng ngữ nghĩa.

#### 3.7 Trình quản lý Máy ảo QEMU/KVM và Máy chủ VM MCP Nhúng
Triển khai tại `core_engine/src/vm_engine/`, `commands/vm.rs` và `commands/vm_mcp.rs`:
- **Vòng đời & Cấu hình (`manager.rs`, `qemu_wrapper.rs`, `config.rs`)**: Quản lý máy ảo dựa trên tệp `vm.json` (tương thích ngược `.vmx`), tự động dọn dẹp tiến trình `qemu-system-*` mồ côi khi khởi động, và thu thập nhật ký thực thi QEMU.
- **Kênh Điều khiển & Tương tác Guest**:
  - **QMP (`qmp_client.rs`)**: Tiêm sự kiện chuột, bàn phím, cuộn trang, chờ (`VmInputAction`) và quản lý snapshot trực tiếp (`create_vm_snapshot`, `restore_vm_snapshot`, `delete_vm_snapshot`, `list_vm_snapshots`).
  - **QGA (`qga_client.rs`)**: Giao tiếp qua Unix socket với QEMU Guest Agent để thực thi lệnh và chép tệp vào trong hệ điều hành khách (`inject_guest_file`).
  - **Luồng VNC (`vnc_stream.rs`, `VmConsole.tsx`)**: Thu khung hình qua `vnc-rs` và hiển thị tương tác trên giao diện qua `@novnc/novnc`.
  - **Trình biên dịch Hành động (`action_compiler.rs`)**: Chuyển đổi chỉ thị ngôn ngữ tự nhiên kết hợp dữ liệu OCR màn hình thành chuỗi `VmInputAction` thông qua mô hình cục bộ tương thích OpenAI (Ollama / llama.cpp).
- **Máy chủ VM MCP Nhúng (`commands/vm_mcp.rs`)**: Khởi chạy dịch vụ Streamable HTTP MCP cục bộ (`/mcp` trên `axum` kèm xác thực Bearer token) cho phép tác tử AI trực tiếp quan sát và thao tác trên máy ảo.

#### 3.8 Trình quản lý Docker và Phân tích Bộ nhớ
- **Docker Engine (`core_engine/src/docker_engine/`, `commands/docker.rs`)**: Kết nối trực tiếp Docker daemon qua crate `bollard` để liệt kê image/container, điều khiển vòng đời (`create`, `start`, `stop`, `restart`, `remove`), truyền luồng log/stats thời gian thực và mở phiên PTY `docker exec`.
- **Memory Inspector (`core_engine/src/memory_inspector/`, `commands/memory_inspector/`)**: Kết hợp đo đạc bộ nhớ container, bộ theo dõi eBPF `EbpfTracer` gọi `bpftrace` gắn uprobe vào `libc.so.6:malloc`/`free`, mô-đun fuzzing áp lực bộ nhớ và giảm tải (`stress/fuzzer.rs`, `stress/ramp_down.rs`), bộ phân tích hồi quy tuyến tính tính tốc độ rò rỉ và hệ số xác định `R^2` (`profiling/analyzer.rs`), cùng đồ thị áp suất và bản đồ nhiệt trên UI.

#### 3.9 PingZero Mini (Kiểm thử API & Chẩn đoán Mạng)
Triển khai tại `commands/network.rs`, `websocket.rs`, `sse.rs`, `grpc.rs`, `oauth.rs` và `load_test.rs`:
- **Hỗ trợ Đa Giao thức**: HTTP(S) (`reqwest` có quản lý cookie và mTLS client certificate), WebSocket (`tokio-tungstenite` với `rustls`), Server-Sent Events (`reqwest-eventsource`), và gRPC động sử dụng reflection (`tonic` + `prost-reflect`).
- **Xác thực & Kịch bản**: Máy chủ cục bộ xử lý luồng OAuth 2.0 callback (`oauth2`), lưu trữ workspace/collection, kiểm định JSON Schema (`jsonschema`) và thực thi kịch bản Pre/Post bằng JavaScript.
- **Kiểm thử Chịu tải & Công cụ Mạng**: Bộ sinh tải đồng thời dùng `tokio` (`run_load_test`), tra cứu DNS, kiểm tra Ping/TCP, thanh tra chuỗi chứng chỉ SSL/TLS, sinh lệnh cURL, giải mã JWT, tính mã băm, Base64, định dạng XML và so sánh diff phản hồi.

#### 3.10 Trình duyệt Cơ sở dữ liệu Đa Động cơ
Triển khai tại `commands/database.rs`, `sqlite.rs`, `db_duckdb.rs`, `db_kv.rs`, `db_mongo.rs` và `db_redis.rs`:
- **CSDL Quan hệ & Phân tích**: SQLite (`rusqlite` / `sqlx`), PostgreSQL và MySQL (`sqlx` dùng `runtime-tokio-rustls` hỗ trợ tham số chứng chỉ X.509), và DuckDB (`duckdb` đóng gói sẵn).
- **CSDL Tài liệu & Key-Value**: MongoDB (`mongodb` dùng `rustls-tls` và X.509), Redis (`redis` kết nối ghép kênh Tokio hỗ trợ `SCAN`), cùng các kho lưu trữ nhúng Rust (`sled` và `redb`).
- **Nghiệp vụ Hỗ trợ**: Quản lý pool kết nối, liệt kê bảng/namespace, đọc dữ liệu phân trang, thực thi truy vấn tùy biến, cập nhật từng ô dữ liệu, thêm/xóa dòng và bổ sung cột.

#### 3.11 Hệ thống Tiện ích Mở rộng WebAssembly (WASI Preview 1)
Triển khai tại `core_engine/src/extension_manager/`, `commands/extension.rs` và `commands/marketplace_cmds.rs`:
- **Môi trường Thực thi Cô lập (`wasm_engine.rs`)**: Nhúng `wasmtime` v29 hỗ trợ WASI Preview 1 bất đồng bộ và giới hạn nhiên liệu thực thi (`10_000_000` fuel units mỗi lần gọi).
- **Cổng Kiểm soát Quyền**: Cung cấp hàm host `alouette_host::check_permission` để xác thực năng lực được khai báo trong manifest trước khi cho phép gọi tài nguyên hệ thống.
- **Kho Tiện ích Phi máy chủ (`marketplace.rs`)**: Tải danh mục từ `https://cdn.jsdelivr.net/gh/alouette-code/alouette-extension-registry@main/index.json`, đối chiếu mã băm SHA-256, xác thực chữ ký số Ed25519 (`ed25519-dalek`) và tạo payload JSON phục vụ gửi Pull Request xuất bản.

#### 3.12 Quản lý Toolchain, Đường hầm Mạng và Trình duyệt Cô lập
- **Proto Toolchain Manager (`core_engine/src/proto_manager.rs`, `commands/language.rs`)**: Cài đặt và cô lập `proto` CLI cùng các runtime ngôn ngữ trong `app_data/alouette_toolchains`, tự động dọn dẹp tệp nhị phân sai lệch nền tảng.
- **Cloudflared Manager (`core_engine/src/cloudflared_manager.rs`, `commands/cloudflare.rs`)**: Tải xuống và quản lý đường hầm `cloudflared`.
- **Trình duyệt Chrome Cô lập (`commands/browser.rs`, `core_engine/src/agent_tools/`)**: Khởi chạy Chrome với hồ sơ sạch tạm thời (`chrome_profile`) và cổng gỡ lỗi `--remote-debugging-port=9222`; cung cấp các thành phần điều khiển CDP/chuột/OCR (`chromiumoxide`, `headless_chrome`, `enigo`, `rusty-tesseract`).

---

### 4. Cấu trúc Thư mục Dự án

```text
alouette_studio/
├── Cargo.toml                         # Workspace gốc (core_engine, tauri_app/src-tauri)
├── core_engine/
│   ├── Cargo.toml                     # Phụ thuộc và feature của thư viện lõi
│   └── src/
│       ├── agent_harness/             # Vòng lặp tác tử đơn, nén ngữ cảnh, bộ nhớ, kế hoạch
│       ├── agent_tools/               # Tiện ích điều khiển trình duyệt CDP, chuột/phím, OCR
│       ├── code_rag/                  # Suy luận ONNX (tract-onnx), bộ trích xuất, vector DB
│       ├── db/                        # Lược đồ SQLite lõi, R2D2 pool, repositories
│       ├── docker_engine/             # Kết nối Docker daemon qua bollard, logs, stats, PTY
│       ├── extension_manager/         # Máy ảo Wasmtime WASI, manifest, xác thực Ed25519
│       ├── memory_inspector/          # Theo dõi eBPF (bpftrace), fuzzer, phân tích hồi quy
│       ├── multi_agent/               # Coordinator V2, WorkerSession, safeguards, blackboard
│       ├── process/                   # ProcessManager, PTY, sandbox 3 tầng, proxy giả lập
│       ├── vm_engine/                 # Bọc QEMU, QMP, QGA, luồng VNC, biên dịch hành động
│       ├── cloudflared_manager.rs     # Quản lý đường hầm Cloudflare
│       ├── monitor.rs & monitor/      # ResourceMonitor, circuit breaker, watchdog
│       ├── proto_manager.rs           # Cô lập toolchain Proto
│       └── workspace_manager.rs       # Quản lý thư mục workspace cô lập
├── tauri_app/
│   ├── src-tauri/
│   │   ├── Cargo.toml                 # Phụ thuộc nhị phân Tauri host
│   │   ├── tauri.conf.json            # Cấu hình cửa sổ, CSP và đóng gói
│   │   └── src/
│   │       ├── main.rs                # Khởi tạo Tauri, khay hệ thống, router, đăng ký IPC
│   │       ├── state.rs               # AppState dùng chung và pool SQLite WAL
│   │       ├── alouette_open.rs       # Tác vụ quét lỗi nhật ký chu kỳ 500ms
│   │       └── commands/              # Các module lệnh IPC (agent, swarm, vm, db, git...)
│   └── ui/
│       ├── package.json               # Phụ thuộc React 19 + Vite 8 + TypeScript
│       └── src/                       # Components, hooks, services, types
├── server_proxy/
│   ├── package.json                   # Phụ thuộc Express + groq-sdk
│   └── server.js                      # Proxy chuẩn OpenAI cho Groq (cổng 3001)
├── scripts/
│   ├── dev.sh                         # Khởi chạy Tauri chế độ phát triển (--clean)
│   ├── download_embedding_model.sh    # Tải mô hình ONNX Xenova/bge-small-en-v1.5
│   └── test_code_rag.sh               # Chạy bộ kiểm thử đơn vị cho core_engine::code_rag
└── docs/                              # Tài liệu thiết kế chi tiết từng phân hệ
```

---

### 5. Hướng dẫn Biên dịch, Kiểm thử và Khởi chạy

#### Yêu cầu Hệ thống
- **Bộ công cụ Rust**: Rust bản ổn định (`edition = "2021"`) kèm Cargo (`rustup`).
- **Node.js và npm**: Phục vụ biên dịch `tauri_app/ui` và chạy `server_proxy` (nếu sử dụng).
- **Thư viện Nền tảng cho Tauri v2**: Trên Linux cần WebKitGTK, GTK3, `libssl-dev`, `pkg-config` và các thư viện liên quan theo tài liệu Tauri v2.
- **Thành phần Tùy chọn Theo Phân hệ**:
  - `qemu-system-*`, `qemu-img` và firmware OVMF cho phân hệ Máy ảo.
  - Docker Engine daemon đang hoạt động qua socket cục bộ cho Docker Manager.
  - `bpftrace` (cùng quyền gắn uprobe kernel) cho tính năng theo dõi cấp phát bộ nhớ eBPF.
  - Tệp thực thi `opencode` cho cầu nối tác tử OpenCode và Super Multi-Agent Swarm.

#### Thiết lập và Khởi chạy Môi trường Phát triển

```bash
# 1. Tải mô hình ONNX embedding cục bộ cho Code RAG (~33 MB)
#    Đích lưu trữ: tauri_app/app_data/model_embedding/bge-small-en-v1.5/
bash scripts/download_embedding_model.sh

# 2. Cài đặt phụ thuộc giao diện
npm --prefix tauri_app/ui install

# 3. (Tùy chọn) Khởi chạy proxy gợi ý mã nguồn nội tuyến qua Groq
npm --prefix server_proxy install
npm --prefix server_proxy start

# 4. Khởi chạy Alouette Studio ở chế độ Dev (Vite + Tauri)
bash scripts/dev.sh
# Hoặc chạy trực tiếp:
npx --prefix tauri_app/ui tauri dev
```

#### Lệnh Biên dịch và Kiểm thử

```bash
# Biên dịch toàn bộ Rust workspace (core_engine + alouette-studio)
cargo build

# Biên dịch core_engine có bật feature trích xuất AST qua tree-sitter
cargo build --package core_engine --features tree-sitter

# Chạy toàn bộ kiểm thử Rust workspace
cargo test

# Chạy bộ kiểm thử chuyên biệt cho phân hệ Code RAG
bash scripts/test_code_rag.sh

# Kiểm tra kiểu TypeScript và đóng gói giao diện React
npm --prefix tauri_app/ui run build

# Đóng gói bản phát hành (Release bundle)
npx --prefix tauri_app/ui tauri build
```

---

### 6. Đặc tả Phát triển Tiện ích Mở rộng Wasm

Thư mục cài đặt tiện ích cục bộ:
- **Linux / macOS**: `~/.alouette/extensions/<publisher_id>.<extension_id>/`
- **Windows**: `C:\Users\<User>\.alouette\extensions\<publisher_id>.<extension_id>\`

Cấu trúc thư mục bắt buộc:

```text
<publisher_id>.<extension_id>/
├── proto-extension.json
└── plugin.wasm
```

Mẫu tệp cấu hình `proto-extension.json`:

```json
{
  "id": "publisher_name.my-extension",
  "name": "My WASI Extension",
  "version": "1.0.0",
  "description": "Sandboxed WASI Preview 1 module",
  "runtime": {
    "type": "wasm",
    "entry": "plugin.wasm"
  },
  "capabilities": {
    "permissions": ["fs:read", "net:http"]
  }
}
```

Biên dịch mô-đun Rust sang đích WASI Preview 1:

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```
