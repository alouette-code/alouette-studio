<div align="center">
  <img src="logo_alouette.png" alt="Alouette Studio Logo" width="120" />
  <h1>Alouette Studio</h1>
  <p><strong>Integrated Development Studio & Native Process Orchestrator</strong></p>
  <p><i>A high-performance, enterprise-grade desktop workspace for engineering and orchestrating isolated development environments.</i></p>
  <br/>
  <p>
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
    <img src="https://img.shields.io/badge/Tauri_v2-24C6DC?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri" />
    <img src="https://img.shields.io/badge/React_19-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React" />
    <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript" />
  </p>
</div>

---

Alouette Studio redefines the developer workspace by integrating enterprise-grade toolchain isolation, a sophisticated 3-tier Sandbox command protector, an advanced Environment Simulator, and an uncompromising Native Git Engine. Experience unparalleled orchestration with built-in Cloudflare tunneling, AI Agent harnessing, and eBPF-powered memory profiling—all meticulously crafted within a sleek Monaco-based Split Editor interface.

## <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/sparkles.svg" width="24" height="24" align="top"/> Next-Generation Capabilities

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/cpu.svg" width="20" height="20" align="top"/> 1. Process Management & Workspace Orchestration
*Orchestrate your entire development lifecycle with absolute precision and stability.*
- **Lifecycle Control:** Seamlessly register, start, stop, and deregister project processes via lightning-fast IPC commands.
- **Process Guard & Restart:** Self-healing process health monitoring equipped with configurable exponential backoff.
- **Setup Hooks:** Define sophisticated setup command chains executed prior to main process initialization.
- **Deep Termination:** Advanced force-kill mechanics with recursive process tree traversal to completely eradicate orphan processes.
- **Port Diagnostics:** Automated, intelligent port scanning (`netstat` / `lsof`) to instantly identify and resolve binding conflicts.
- **Network Isolation:** Process-level network interface isolation enforced directly via OS firewall rules.
- **Workspace Auto-Cloning:** Instantly clone via Git or copy local directories for pristine, isolated workspace preparation.
- **Terminal Modes:** Seamlessly switch between rich interactive PTY terminals or lightweight log-only outputs.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/shield-alert.svg" width="20" height="20" align="top"/> 2. Sandbox — 3-Tier Command Protection
*Enterprise-grade security engine preventing destructive operations at the core.*
- **Tier 1a (Semantic Interceptor):** Performs deep semantic parsing to classify risk levels, resolve environment variables, normalize Unicode homoglyphs, and reject boundary escape attempts.
- **Tier 1b (Resolution Engine):** Fallback path resolution that strips execution boundary prefixes and validates with extreme case-insensitivity.
- **Tier 2 (OS-level Isolation):** Leverages ultimate system-level sandboxing (AppContainer on Windows, landlock/seccomp on Linux).
- **PTY Hooking:** Total interception of terminal inputs via real-time execution validation mechanisms.
- **Control Center:** A master dashboard offering synchronized control modules for Terminal, Browser, Engine, and Environment configurations.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/wifi-off.svg" width="20" height="20" align="top"/> 3. Environment Simulation Panel
*Stress-test your architecture under extreme real-world network degradations.*
- **Firewall & Routing:** Emulate complex port blocks and custom firewall rules using intelligent glob-pattern matching.
- **Degradation Injection:** Introduce artificial packet loss, high latency, jitter, and bandwidth bottlenecks via our proprietary SOCKS5/HTTP proxy.
- **Chaos Engineering:** Simulate unstable servers, HTTP error code injections (500, 502, 503), and random connection drops.
- **Resource Constraints:** Enforce unforgiving hardware boundaries (CPU %, RAM MB) coupled with watchdog termination protocols.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/columns.svg" width="20" height="20" align="top"/> 4. Premium Dual-Engine Split Editors (Monaco & CodeMirror 6)
*A sublime coding environment offering dynamic runtime switching between Monaco Editor and CodeMirror 6.*
- **Dual Text Editor Engine Architecture:** Switch effortlessly between **Monaco Editor** (rich VS Code features) and **CodeMirror 6** (ultra-lightweight, high-performance) directly in System Settings with instant auto-save and zero-latency cross-window state synchronization via custom React hooks (`useEditorEngine`) and Tauri event broadcasting.
- **Monaco Engine Integration:** Full-featured editing experience with built-in Minimap, Git diff gutter markers, overview ruler, syntax highlighting, semantic search, and formatting capabilities.
- **CodeMirror 6 Lightweight Engine:** Extremely fast, low-RAM editor engine for opening large files or working on light hardware environments.
  - **High-Performance 60FPS Code Minimap:** Integrated `@replit/codemirror-minimap` with GPU hardware acceleration (`will-change: transform`, `contain: layout style paint`) and zero-lag 0ms synchronous render pass calculation.
  - **Native Git Diff Gutter & Minimap Markers:** GitHub standard color-coded line decorations for Added (`#2da44e`), Modified (`#d29922`), Deleted (`#da3633`), Unsaved Added (`#58a6ff`), and Unsaved Modified (`#79c0ff`) lines rendered directly on both gutter and minimap canvas.
- **State Recovery:** Fluid navigation with automatic scroll and cursor position preservation across sessions.
- **Directory Explorer:** An interactive, lightning-fast recursive directory tree for flawless project traversal.
- **Split Editors:** Effortlessly divide your workspace into up to 3 side-by-side active panes with seamless drag-and-drop tab management.
- **Binary Transfer:** Highly secure Base64 encoding for local binary file transfers over Tauri IPC.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/radio.svg" width="20" height="20" align="top"/> 5. PingZero Mini API Client & Diagnostics
<div align="center">
  <img src="readme_img/ping_zero/home ping zero.png" alt="PingZero Mini Dashboard" width="600"/>
</div>

*An integrated API development powerhouse built to challenge enterprise-level REST clients.*

- <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/globe.svg" width="16" height="16" align="top"/> **Comprehensive Protocol Support:** Master REST/HTTP(S), WebSocket (WS/WSS), Server-Sent Events (SSE), and gRPC microservices effortlessly.
- <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/lock.svg" width="16" height="16" align="top"/> **Advanced Authentication:** Seamless OAuth 2.0 extraction, Mutual TLS (mTLS) with Client Certificates, and standard Auth suites.
- <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/file-json.svg" width="16" height="16" align="top"/> **OpenAPI Importer:** Instantly translate OpenAPI & Swagger definitions from URLs or raw JSON into structured Collections.
- <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/zap.svg" width="16" height="16" align="top"/> **Load Tester (Stress Testing):** Unleash raw concurrency with a `tokio`-powered engine to measure RPS, Success rates, and Latency under extreme loads.
- <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/wrench.svg" width="16" height="16" align="top"/> **Network Diagnostics:** Precision tooling including DNS Lookup, Ping/TCP Checks, and deep SSL/TLS Inspection.
- <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/code.svg" width="16" height="16" align="top"/> **Pre/Post Scripts:** Inject custom JavaScript logic to choreograph chained requests and validate complex JSON schemas.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/git-branch.svg" width="20" height="20" align="top"/> 6. Git Management & Native Diff Engine
*A beautiful, unified version control interface engineered directly into your workspace.*
- **Native Sidebar:** Command your repository visually via the elegantly integrated Git status panel.
- **Real-Time Tracking:** Instantaneous metadata monitoring for active branches and remote repositories.
- **Staging Matrix:** Fluidly manage staged and unstaged file operations with precise, per-file status indicators.
- **Commit History:** Traverse commit lineage with rich author, date, and atomic file-change inspection.
- **Native Diff Engine:** Powered by `git2`, rendering flawless addition, modification, and deletion line highlights natively in the editor—bypassing sluggish external shell invocations entirely.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/search.svg" width="20" height="20" align="top"/> 7. Code RAG System (Semantic Engine)
*Instantaneous codebase intelligence utilizing vector similarity and AST analysis.*
- **Multi-Language AST Extraction:** Deep syntactic parsing for Rust, Go, TypeScript, Python, C++, Java, and more, indexing core structural signatures and docstrings.
- **ONNX Local Embeddings:** High-performance, private vectorization powered by 384-dimensional embeddings running gracefully on non-blocking background threads.
- **Semantic Queries:** Harness semantic similarity or fast metadata keyword mapping for pinpoint retrieval across massive codebases.
- **Automated Indexing:** Real-time directory scanning and continuous background re-indexing upon filesystem changes.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/network.svg" width="20" height="20" align="top"/> 8. Python FFI Machine Learning Integration
*Execute complex AI pipelines natively through Rust-Python interoperability.*
- **Isolated ML Runtime:** Fully automated orchestration of Python virtual environments (`venv`) explicitly configured for high-performance ML workloads.
- **Zero-Friction Bootstrapping:** Autonomous dependency injection for PyTorch, Transformers, Accelerate, and FastAPI ecosystems.
- **PyO3 FFI Integration:** Unleash heavyweight pipelines (vLLM, ExLlamaV2, TensorRT-LLM) natively via Rust's Foreign Function Interface, obliterating subprocess latency.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/chrome.svg" width="20" height="20" align="top"/> 9. Google Chrome Sandbox Integration
*Uncompromisingly secure, tightly-coupled web preview environments.*
- **Isolated Web Browser:** Launch Google Chrome sessions directly into the workspace, meticulously sandboxed for flawless debugging.
- **Intelligent Resolution:** Automatic multi-path executable discovery traversing bundled resources, local binaries, and system defaults.
- **Boundary Enforcement:** Integrates profoundly with AppContainer/Landlock boundaries to guarantee total host-system isolation.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/stethoscope.svg" width="20" height="20" align="top"/> 10. Alouette Open AI Diagnostic Scanner
*Your autonomous, always-watching AI error detection sentinel.*
- **Background Log Monitor:** Relentless 500ms heartbeat diagnostic loops scrutinizing project telemetry.
- **ONNX Classification Model:** Real-time neural inference executing our proprietary `alouette_open-A1 v1.0` classification engine to instantly categorize compilations, panics, or network faults.
- **Warning Cards Alert:** Beautifully crafted, actionable UI diagnostic cards offering instant fix proposals directly in your frontend viewport.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/layout-dashboard.svg" width="20" height="20" align="top"/> 11. Welcome Page Dashboard
*Your mission control hub for unparalleled workspace observability.*
- **At-a-Glance Metrics:** Stunning visual indicators for system uptime, network port saturation, and global hardware performance profiling.
- **Quick-Access Shortcuts:** One-click deployment pipelines for new workspaces, Cloudflare tunnels, and virtual machine allocations.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/folder-sync.svg" width="20" height="20" align="top"/> 12. File Watcher & Workspace Syncer
*Lightning-fast, native filesystem synchronization.*
- **Native Watcher:** Anchored by Rust's powerful `notify` crate, delivering immediate, recursive workspace monitoring.
- **Debounced Updates:** Intelligent 300ms event grouping to eliminate frontend rendering latency and UI spam.
- **Live Syncing:** Seamless React file explorer tree updates ensuring absolute state parity with external file modifications.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/activity.svg" width="20" height="20" align="top"/> 13. Resource Monitoring & Throttling
*Unforgiving, precise hardware resource enforcement.*
- **Live Telemetry:** Experience beautifully rendered, high-framerate canvas charts tracking CPU and RAM utilization across isolated process trees.
- **Limit Throttling:** Strict CPU/RAM thresholding with aggressive watchdog termination to guarantee main-engine stability under rogue workloads.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/database.svg" width="20" height="20" align="top"/> 14. Universal Database Browser & Multi-Auth Security
*A monolithic database command center built for multi-cloud and local data architectures.*
- **Multi-Database Support:** Expansive driver compatibility spanning SQLite, PostgreSQL, MySQL, MongoDB, Redis, and Supabase ecosystems.
- **Dual-Mode Interface:** Fluid transitions between local standard modes and advanced, heavily secured Cloud Database URIs.
- **Multi-Auth Security:** Dynamic X.509/Basic authentication passing encrypted seamlessly via `rustls`.
- **Database Explorer:** A highly responsive editor to sculpt schemas, manipulate rows, and execute queries flawlessly.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/cloud.svg" width="20" height="20" align="top"/> 15. Cloudflared Tunnel Integration
*Zero-Trust networking engineered directly into the toolbar.*
- **Auto-Provisioning:** Effortlessly downloads and initializes the `cloudflared` binary on startup with offline redundancy.
- **Flexible Tunnels:** Switch instantly between anonymous `trycloudflare.com` links and highly secure Named Tunnels via Token Auth.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/hard-drive.svg" width="20" height="20" align="top"/> 16. Native Hypervisor (Virtual Machine Manager)
*Next-generation virtualization bypassing traditional emulation bottlenecks.*
- **KVM Hardware Acceleration:** Direct QEMU/KVM integration delivering near-bare-metal performance across Windows, Linux, and macOS platforms.
- **UEFI/BIOS Injections:** Dynamic `OVMF_CODE` injection mapping modern OS prerequisites with unmatched precision.
- **Live Snapshots:** Zero-downtime state freezing via QMP, ensuring perfect rollbacks with instantaneous UI responsiveness.
- **Integrated Web VNC:** Stream flawless 60FPS graphical environments directly into your IDE through WebSocket protocol.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/bot.svg" width="20" height="20" align="top"/> 17. AI Agent Loop & Assistant Interface
*An autonomous intelligent copilot orchestrated for supreme productivity.*
- **Closed-Loop Engine:** Powered by `rig-core`, supporting the titans of multi-provider LLMs (Claude, DeepSeek, ChatGPT) locked in SQLite session memory.
- **7 Operation Modes:** Adapt dynamically across Interactive, Autonomous, Coordinator, and deeply nested parallel worker states.
- **Context Injection:** Utilize `@` mentions for instant fuzzy-matching to bind intricate workspace file paths directly into LLM prompts.
- **Token Analytics:** Real-time context auditing to ensure absolute control over payload constraints and API expenditures.
- **Action Guard:** An impenetrable authorization firewall intercepting critical AI commands (file edits, system commands) requiring explicit human approval.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/microscope.svg" width="20" height="20" align="top"/> 18. Memory Inspector & Stress Profiling
*Unrivaled, surgical introspection into application memory architectures.*
- **eBPF Tracing:** Deploy kernel-level eBPF probes for pinpoint memory allocation profiling without performance penalties.
- **Fuzzing & Stress Testing:** Annihilate edge cases with built-in memory stress fuzzers engineered for extreme load validation.
- **Docker Integration:** Direct connectivity to container ecosystems for ruthless memory leak exposure.
- **Visual Analytics:** High-fidelity Pressure Chamber Charts and Heatmap Timelines projecting real-time infrastructural health.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/container.svg" width="20" height="20" align="top"/> 19. Docker Container Manager
*Native, frictionless container orchestration without leaving the IDE.*
- **Daemon Integration:** Hyperspeed direct API communication with the local Docker socket powered by `bollard`.
- **Full Lifecycle Control:** Deploy, command, restart, and decimate containers effortlessly.
- **Real-Time Insights:** Stream raw telemetry logs into ANSI-colored xterm.js terminals instantly.
- **PTY Injection:** Submerge directly into running containers via deep interactive shells for rapid-fire debugging.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/blocks.svg" width="20" height="20" align="top"/> 20. Wasm-First Serverless Hybrid Architecture & Marketplace
*Extend Alouette Studio securely using memory-isolated WebAssembly plugins and a GitHub-backed CDN registry.*
- **Wasmtime WASI Engine:** Embedded Wasmtime runtime with WASI Preview 1 support, delivering near-native performance, zero-trust memory sandboxing, and zero-crash fault tolerance.
- **Permission Firewall Interceptor:** Host API interceptor enforcing granular permissions (`fs:read`, `fs:write`, `net:http`, `terminal:exec`) prior to executing WASI calls.
- **Serverless Git Registry:** Metadata and marketplace index served globally via jsDelivr CDN backed by [alouette-code/alouette-extension-registry](https://github.com/alouette-code/alouette-extension-registry).
- **Publisher Namespace Protection:** Strict ID format (`<publisher_id>.<extension_id>`) eliminating namespace collisions and duplicate extensions.
- **Ed25519 Cryptographic Signatures:** Asymmetric key signature verification protecting developers against unauthorized extension hijacking or malicious tampering.
- **Automated 36-Char UUID Assets:** Local asset manager with maximum 500x500px resolution enforcement and collision-free 36-character UUID icon naming.
- **Serverless Submission Form:** Integrated GUI form with automatic SHA-256 binary calculation and one-click JSON payload generation for GitHub Pull Requests.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/workflow.svg" width="20" height="20" align="top"/> 21. Multi-Agent Team Collaboration Workflow (5 Agents)
*Command an entire specialized team of AI agents working in synchronized coordination.*
- **5 Specialized Agent Roles:** Automated multi-agent workflow featuring 5 distinct specialized personas:
  - 👑 **Leader (Coordinator):** Receives user objective, introduces team goals, coordinates workflow, and presents the final summary report.
  - 📋 **Planner (Architect):** Drafts step-by-step technical plans and divides approved plans into subtasks for developer agents.
  - 🔍 **Plan Reviewer (QA Specialist):** Audits proposed plans for logic flaws, security vulnerabilities, and edge cases until approved (`[APPROVED]`).
  - 💻 **Coder 1 (Developer Alpha):** Executes primary backend/core source code implementation tasks.
  - ⚡ **Coder 2 (Developer Beta):** Executes secondary/integration source code implementation tasks.
- **Token Consumption Guard:** Integrated token expenditure warning modal requiring explicit user confirmation before launching the 5-Agent collaboration pipeline.
- **Zero-Stutter 60FPS Streaming:** High-performance `requestAnimationFrame` event batching coupled with memoized `React.memo` markdown rendering, guaranteeing fluid 60FPS real-time chat updates without UI stuttering.
- **Historical Memory:** Retain deep, searchable session histories to instantly restore previous AI workflows and thought patterns.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/fingerprint.svg" width="20" height="20" align="top"/> 22. Enterprise Admin Control Center & Global Dock
*Unrivaled administrative oversight wrapped in a stunning macOS-style navigation paradigm.*
- **Zero-Trust Identity:** Advanced security flows and identity verification to lock down workspace access.
- **Global Dock Navigation:** A liquid-smooth, macOS-inspired floating dock granting instantaneous access to all critical sub-systems.
- **Unified Telemetry:** A monolithic Admin Panel consolidating global configurations, identity management, and workspace analytics into a single pane of glass.

### <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/sparkles.svg" width="20" height="20" align="top"/> 23. Smart AI Code Completion Engine (Inline Ghost Text)
*Real-time, context-aware AI code suggestions integrated directly into Monaco Editor.*
- **Inline Ghost Text:** Generates sleek, non-intrusive gray preview code directly at your cursor position—accept suggestions effortlessly with a single `Tab` press.
- **Smart 1,000-Token Context Windowing:** Intelligently analyzes code scope around your cursor using a balanced sliding window (up to 800 prefix tokens and 200 suffix tokens) to ensure highly accurate, syntactically harmonious code completions.
- **Pause-Aware Debounce Engine:** Intelligent 2-second typing pause detection waits until you complete your thought before requesting AI suggestions—eliminating unnecessary background requests, preserving system resources, and preventing rate limits.
- **Manual Trigger Shortcut (`Alt + \`):** Instantly trigger AI completions on demand anytime without waiting for auto-detection.
- **Seamless Monaco Coexistence:** Works harmoniously alongside Monaco's native autocomplete, keyword suggestions, and hover documentation without blocking or interfering with existing editor tools.
- **One-Click Toggle:** Easily enable or disable AI suggestions at any time using the quick status control in the editor toolbar.


---

## <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/layers.svg" width="24" height="24" align="top"/> Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                          Tauri Desktop Shell                            │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    React Frontend (UI)                            │  │
│  │  Components | Hooks | Monaco | xterm.js | Charts | Lucide Icons │  │
│  │  react-markdown | fuse.js | Custom VM Manager                     │  │
│  │  Welcome Dashboard Page | PingZero Mini Interface                 │  │
│  │  AI Assistant panel (with @mentions, token metrics & batches)     │  │
│  │  Memory Inspector (eBPF Profiling, Heatmap, Pressure Chamber)     │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  │              Tauri IPC Bridge (Commands + Events)                 │  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Core Engine (Rust)                              │  │
│  │  Process Manager | Sandbox | Proto | Cloudflared | R2D2 + SQLite  │  │
│  │  Agent Harness (rig-core) | Alouette Open (tract-onnx)            │  │
│  │  Network Sim Proxy | System Manager | DashMap Registry            │  │
│  │  Code RAG (VectorDb) | PyO3 Python ML Environment FFI             │  │
│  │  Notify File Watcher | Native Git2 Diff Engine                    │  │
│  │  Docker daemon client (bollard)                                   │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Responsibility |
|-------|-----------|----------------|
| **UI** | React 19 + TypeScript + Vite + xterm.js + Monaco | Desktop interface with process dashboards, terminals, file editor, SQLite browser, PingZero API client, VM manager, Welcome dashboard, AI assistant chat, Docker manager, Memory Inspector. |
| **Bridge** | Tauri v2 IPC | Type-safe command handlers, sophisticated event routers, robust state management via R2D2 pool and DashMap registries. |
| **Engine** | Rust (Tokio async) | Core logic, Sandbox enforcement, SQLite persistence (WAL), AI agent loops, ONNX inference scanner, Vector RAG search, Native diffs, PyO3 FFI integrations, Docker connectivity. |

---

## <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/rocket.svg" width="24" height="24" align="top"/> Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd alouette_studio

# Initialize core dependencies & RAG model
bash scripts/download_embedding_model.sh

# Install premium frontend dependencies
cd tauri_app/ui
npm install
cd ../..

# Launch Alouette Studio in Dev Mode (Vite + Tauri)
npx --prefix tauri_app/ui tauri dev
```

## <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/terminal.svg" width="24" height="24" align="top"/> Build & Testing

### Backend (Rust Engine)
```bash
# Compile core libraries and Desktop binary
cargo build

# Execute robust integrated test suites
cargo test
bash scripts/test_code_rag.sh
```

### Frontend (React Environment)
```bash
cd tauri_app/ui
npm run dev    # Launch UI standalone
npm run build  # Execute TypeScript rigorous compilation
```

---

## <img src="https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/box.svg" width="24" height="24" align="top"/> Wasm Extension Development Guide

### 1. Extension Directory Structure
```text
my-extension/
├── proto-extension.json    # Manifest configuration & permissions
└── plugin.wasm             # Compiled WebAssembly binary
```

### 2. Sample Manifest (`proto-extension.json`)
```json
{
  "id": "publisher_name.my-extension",
  "name": "My Custom Wasm Plugin",
  "version": "1.0.0",
  "description": "High performance WASI extension",
  "runtime": {
    "type": "wasm",
    "entry": "plugin.wasm"
  },
  "capabilities": {
    "permissions": ["fs:read", "net:http"]
  }
}
```

### 3. Compiling Rust to WASI Target
```bash
# Add WASI target
rustup target add wasm32-wasip1

# Build release WASM binary
cargo build --target wasm32-wasip1 --release
```

### 4. Local Installation Directory
Place local plugins in:
- **Linux / macOS**: `~/.alouette/extensions/<publisher>.<extension_id>/`
- **Windows**: `C:\Users\<User>\.alouette\extensions\<publisher>.<extension_id>\`

