# Alouette Studio

### Integrated Development Studio and Native Process Orchestrator

A high-performance desktop application built with Rust, Tauri v2, React 18, and TypeScript. It is designed to register, execute, isolate, and monitor independent development environments with enterprise-grade sandboxing, automated toolchain management, virtual PTY terminals, an integrated Monaco editor, and a SQLite database browser.

> [!NOTE]
> The current branch (`core_server`) contains a streamlined codebase optimized for native process orchestration, security boundaries, and toolchain management. Legacy HTTP testing interfaces (such as the MiniPostman client UI) have been removed to minimize resource consumption and improve stability.

---

## Core Features

### 1. Process Management
* **Lifecycle Control:** Register, start, stop, and deregister system processes directly from the user interface.
* **Auto-Restart:** Automatically detect crashed processes and execute restarts using an exponential backoff algorithm.
* **Pre-execution Setup:** Execute initialization routines (such as package installations) before launching the main application process.
* **Recursive Teardown:** Terminate entire process trees recursively to prevent orphaned background processes.
* **Port Conflict Detection:** Query active TCP ports to identify conflicting PIDs before execution.

### 2. Resource Monitoring
* **Real-time Metrics:** Capture CPU usage percentage and memory consumption (RAM) across the entire child process hierarchy.
* **Interactive Charts:** Render performance history graphs using HTML5 Canvas powered by asynchronous Tauri IPC events.
* **Watchdog Enforcement:** Detect and terminate target processes that exceed configured resource thresholds for prolonged durations.

### 3. Proto Toolchain Isolation
* **Automated Installation:** Download and manage moonrepo's [Proto CLI](https://moonrepo.dev/proto) engine.
* **Path Environment Spoofing:** Construct isolated execution environments by overriding the PATH variable. This allows projects to run distinct versions of Node.js, Python, or Go without conflicting with host system libraries.
* **Version Pinning:** Pin specific toolchain versions per project configuration.

### 4. Interactive Terminals
* **PTY Virtualization:** Seamlessly route input and output streams through virtual PTY terminal sessions (utilizing xterm.js in the frontend and non-blocking I/O routines on the Rust backend).
* **Multi-session Layout:** Support spawning multiple concurrent terminal instances within the scope of a single project.
* **Dynamic Resizing:** Automatically synchronize terminal rows and columns during container layout changes.

### 5. Command Sandbox
Prevent destructive actions (such as `rm -rf /` or unauthorized directory escapes) during terminal sessions:
* **Tier 1a (Interceptor):** Parse command semantics, resolve virtual environment variables, and enforce project workspace boundaries.
* **Tier 1b (Engine):** Fallback token matching to secure paths.
* **Tier 2 (OS-level Isolation):** Utilize AppContainer on Windows platforms and sandbox kernels on Unix-like environments.

### 6. Cloudflare Tunnel Integration
* **Automated Binary Management:** Retrieve and execute the latest `cloudflared` package.
* **Zero-config Routing:** Expose local services to secure public URLs by reading tunnel outputs automatically.

### 7. SQLite Browser and Monaco Editor
* **Monaco Editor:** Edit configuration and source files directly inside the application workspace with syntax highlighting, automatic saving, and cursor position persistence.
* **Database Explorer:** Inspect SQLite schemas, insert or delete table rows, modify table fields, and edit cell values directly.

### 8. AI Log Diagnostics
* **Anomaly Classification:** Scan real-time log outputs to identify system errors, socket conflicts, database exceptions, out-of-memory errors, and permission failures.
* **ONNX Inference Engine:** Run the local `alouette_open-A1 v1.0.onnx` classifier via `tract-onnx` combined with regex-lite heuristic fallbacks.

---

## System Architecture

```text
┌─────────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop Shell                          │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    React Frontend (UI)                        │  │
│  │  Components | Hooks | Monaco | xterm.js | Canvas Charts       │  │
│  └───────────────────────────────────────────────────────────────┘  │
│  │              Tauri IPC Bridge (Commands + Events)             │  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                   Core Engine (Rust)                          │  │
│  │  Process Manager | Sandbox | Proto | Cloudflared | DB WAL     │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Project Structure

The codebase is organized into the following layout:

```text
alouette-server/
├── core_engine/                    # Core backend system written in Rust
│   ├── src/
│   │   ├── config.rs               # TOML parser configurations
│   │   ├── db.rs                   # SQLite storage engine using WAL mode
│   │   ├── monitor.rs              # CPU and RAM tracking routines
│   │   ├── settings.rs             # Application-wide settings configurations
│   │   ├── cloudflared_manager.rs  # Cloudflared wrapper lifecycle logic
│   │   ├── proto_manager.rs        # Toolchain installer and path isolator
│   │   ├── workspace_manager.rs    # Project workspace directory setups
│   │   └── process/                # Process execution and terminal routing
│   │       ├── executor.rs         # Subprocess spawner and stream pipelines
│   │       ├── terminal.rs         # Virtual PTY terminal managers
│   │       └── sandbox/            # Three-tier command protection policies
│
├── tauri_app/                      # Desktop shell framework (Tauri v2)
│   ├── src-tauri/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # Application initialization entrypoint
│   │       ├── ai_diagnostics.rs   # Local log classifications (ONNX + Heuristics)
│   │       ├── system_manager.rs   # Window routers and helper controllers
│   │       └── commands/           # IPC command handlers invoked by the frontend
│
│   └── ui/                         # React UI Frontend (Vite)
│       ├── src/
│       │   ├── App.tsx             # Workspace router and main tab interface
│       │   ├── index.css           # Design system tokens and styles
│       │   └── components/
│       │       ├── Header.tsx      # Title bar and window actions
│       │       ├── FileExplorer.tsx# Directory browser
│       │       ├── CodeEditor.tsx  # Monaco-based code wrapper
│       │       ├── SqliteEditor.tsx# Database admin table browser
│       │       ├── TerminalPanel.tsx# Terminal UI component (xterm.js)
│       │       ├── AdminPanel.tsx  # Central configuration dashboard
│       │       └── ExtendedDashboard.tsx # System status overview
```

---

## Getting Started

### Prerequisites
* **Rust:** Version `1.75+`
* **Node.js:** Version `18+` (including `npm` package manager)
* **Git:** Version `2.40+`
* **Windows 10/11** (or compatible Linux/macOS build toolchains)

### Running in Development Mode

1. Install frontend packages:
   ```bash
   cd tauri_app/ui
   npm install
   cd ../..
   ```

2. Start the Tauri development hot-reload server:
   ```bash
   npx --prefix tauri_app/ui run tauri dev
   ```

### Building for Production

Compile and pack the application binaries:
```bash
npx --prefix tauri_app/ui run tauri build
```
The compiled installers will be located in: `tauri_app/src-tauri/target/release/bundle/`.

---

## Development Guidelines
For guidelines regarding code styles, test execution, and architecture boundaries, refer to [CLAUDE.md](file:///d:/core_alouette_server/CLAUDE.md).
