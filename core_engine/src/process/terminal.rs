use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};

use super::manager::ProcessManager;
use super::models::{TerminalOutput, TerminalSession, TerminalWriteContext};
use super::sandbox;

impl ProcessManager {
    pub fn subscribe_terminal(&self) -> broadcast::Receiver<TerminalOutput> {
        self.terminal_sender.subscribe()
    }

    pub fn get_terminal_write_context(
        &self,
        session_id: &str,
    ) -> Result<TerminalWriteContext, String> {
        let session = self
            .terminal_sessions
            .get(session_id)
            .ok_or_else(|| format!("Terminal session '{session_id}' not found"))?;
        Ok(TerminalWriteContext {
            stdin_sender: session.stdin_sender.clone(),
            terminal_sender: self.terminal_sender.clone(),
        })
    }

    pub fn get_input_buf(&self, session_id: &str) -> Option<&String> {
        self.input_buf.get(session_id)
    }

    pub fn append_input_buf(&mut self, session_id: &str, ch: &str) {
        self.input_buf
            .entry(session_id.to_string())
            .or_default()
            .push_str(ch);
    }

    pub fn clear_input_buf(&mut self, session_id: &str) {
        self.input_buf.remove(session_id);
    }

    pub fn check_input_sandbox(&self, session_id: &str) -> Result<Option<String>, String> {
        let Some(buf) = self.input_buf.get(session_id) else {
            return Ok(None);
        };
        Ok(self.check_command_sandbox(session_id, buf))
    }

    /// Evaluate a single command string against the session's workspace sandbox rules.
    pub fn check_command_sandbox(&self, session_id: &str, command: &str) -> Option<String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return None;
        }

        let cwd = self
            .sessions_cwd
            .get(session_id)
            .cloned()
            .unwrap_or_else(|| PathBuf::from("."));

        let ws = self
            .terminal_sessions
            .get(session_id)
            .map(|s| s.workspace_root.clone())
            .unwrap_or_else(|| cwd.clone());

        match sandbox::check_command(trimmed, &cwd, &ws) {
            sandbox::Verdict::Allow => None,
            sandbox::Verdict::Block { reason } => {
                eprintln!("[sandbox] BLOCKED '{}': {}", trimmed, reason);
                Some(reason)
            }
        }
    }

    /// Update tracked CWD cho mọi loại navigation command.
    /// Bao gồm: cd, sl, pushd, popd, set-location, và cả tên viết tắt.
    /// Sử dụng interpolation thông minh để resolve ~, $env:VAR, $HOME.
    pub fn update_cwd_for_cd(&mut self, session_id: &str, cmd: &str) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }

        // Chuẩn hóa: lowercase để so sánh
        let lower = trimmed.to_lowercase();

        // Navigation commands: cd, sl, pushd, popd, set-location
        let is_nav = lower.starts_with("cd ")
            || lower.starts_with("sl ")
            || lower.starts_with("pushd ")
            || lower.starts_with("popd")
            || lower.starts_with("set-location ")
            || lower == "cd"
            || lower == "sl"
            || lower == "popd"
            || lower.starts_with("set-location");

        if !is_nav {
            return;
        }

        let current = self
            .sessions_cwd
            .get(session_id)
            .cloned()
            .unwrap_or_else(|| PathBuf::from("."));

        let workspace_root = self
            .terminal_sessions
            .get(session_id)
            .map(|s| s.workspace_root.clone())
            .unwrap_or_else(|| current.clone());

        // Trích xuất path argument (nếu có)
        let rest = if lower.starts_with("set-location") {
            trimmed["set-location".len()..].trim()
        } else if lower.starts_with("pushd") {
            trimmed["pushd".len()..].trim()
        } else if lower.starts_with("popd") {
            // popd: về lại directory trước, không có argument
            // Cách đơn giản: giữ nguyên CWD (thực tế cần stack, nhưng tạm thế)
            return;
        } else {
            trimmed[2..].trim() // cd, sl
        };

        if rest.is_empty() {
            // cd/sl alone → workspace root
            self.sessions_cwd
                .insert(session_id.to_string(), workspace_root.clone());
            return;
        }

        // Bỏ quotes
        let target_raw = rest.trim_matches('"').trim_matches('\'');

        // Sử dụng interpolation giống hệt interceptor
        let interpolated = if target_raw.starts_with('~') {
            if let Some(home) = get_home_dir_for_cwd() {
                if target_raw.len() == 1 {
                    home
                } else if target_raw.as_bytes().get(1) == Some(&b'/')
                    || target_raw.as_bytes().get(1) == Some(&b'\\')
                {
                    format!("{}{}", home, &target_raw[1..])
                } else {
                    format!(r"{}\{}", home, &target_raw[1..])
                }
            } else {
                target_raw.to_string()
            }
        } else {
            // Resolve env vars
            resolve_env_vars_for_cwd(target_raw)
        };

        let target_path = PathBuf::from(&interpolated);

        let new_cwd = if target_path.is_relative() {
            let combined = current.join(&target_path);
            std::fs::canonicalize(&combined).unwrap_or(combined)
        } else {
            std::fs::canonicalize(&target_path).unwrap_or(target_path)
        };

        // Chỉ update nếu path mới tồn tại, là thư mục và nằm trong workspace
        // Nếu out-of-workspace hoặc thư mục không tồn tại, giữ nguyên CWD cũ (để tránh desync bypass)
        if new_cwd.is_dir() && new_cwd.starts_with(&workspace_root) {
            self.sessions_cwd.insert(session_id.to_string(), new_cwd);
        } else {
            eprintln!("[sandbox] CWD update blocked or path invalid: '{}' (outside workspace or not a directory)", new_cwd.display());
        }
    }

    pub async fn spawn_terminal(
        &mut self,
        session_id: &str,
        cwd: Option<&str>,
        block_internet: bool,
    ) -> Result<(), String> {
        let sid = session_id.to_string();
        let _ = self.kill_terminal(&sid).await;

        let abs_root = cwd.unwrap_or(".");
        let abs_root = std::fs::canonicalize(abs_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| abs_root.to_string());

        let proto_home = &self.proto_manager.proto_home;
        let mut envs: Vec<(String, String)> = vec![
            (
                "PROTO_HOME".into(),
                proto_home.to_string_lossy().to_string(),
            ),
            ("WORKSPACE_ROOT".into(), abs_root.clone()),
            ("TERM".into(), "xterm-256color".into()),
            ("COLORTERM".into(), "truecolor".into()),
            ("LANG".into(), "en_US.UTF-8".into()),
        ];
        if let Ok(existing_path) = std::env::var("PATH") {
            let full = std::env::join_paths(
                std::iter::once(proto_home.join("bin"))
                    .chain(std::iter::once(proto_home.join("shims")))
                    .chain(std::env::split_paths(&existing_path)),
            )
            .unwrap_or_else(|_| existing_path.into());
            envs.push(("PATH".into(), full.to_string_lossy().to_string()));
        }

        let pty_system = native_pty_system();
        let pty = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("PTY open: {e}"))?;

        let mut sandbox_wrapper_path: Option<String> = None;

        let cmd = if cfg!(target_os = "windows") {
            let prompt = r#"function global:prompt { "$((Get-Location).Path)> " }"#.to_string();

            let tmp_dir = std::env::temp_dir().join("alouette_term");
            let _ = std::fs::create_dir_all(&tmp_dir);
            let profile_path = tmp_dir.join(format!("prompt_{sid}.ps1"));
            let _ = std::fs::write(&profile_path, prompt.replace('\n', "\r\n"));
            self._prompt_files.insert(sid.clone(), profile_path.clone());

            let mut c = CommandBuilder::new("powershell.exe");
            c.arg("-NoLogo");
            c.arg("-NoExit");
            c.arg("-File");
            c.arg(profile_path.to_string_lossy().to_string());
            c
        } else {
            let shell = detect_default_shell();

            // ── Linux: Pre-spawn sandbox via wrapper script ──
            // Trên Linux, dùng sandbox wrapper thay vì shell trực tiếp
            // để inject ulimit (RLIMIT_AS) trước khi shell thật sự chạy.
            // portable-pty không hỗ trợ pre_exec, wrapper là giải pháp.
            build_terminal_cmd(&shell, block_internet, &mut sandbox_wrapper_path)
        };

        let mut cmd = cmd;
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        for (k, v) in &envs {
            cmd.env(k, v);
        }
        cmd.env_remove("npm_config_prefix");

        let shell_name = if cfg!(target_os = "windows") {
            "powershell.exe"
        } else {
            "shell"
        };
        let child: Box<dyn portable_pty::Child + Send + Sync> = pty
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Spawn {shell_name}: {e}"))?;
        // Drop slave side in parent so PTY master observes EOF cleanly when the child exits.
        drop(pty.slave);
        let pid = child.process_id().unwrap_or(0);
        eprintln!("[terminal] Spawned '{sid}' PID {pid}");

        let writer = pty
            .master
            .take_writer()
            .map_err(|e| format!("PTY writer: {e}"))?;
        let reader = pty
            .master
            .try_clone_reader()
            .map_err(|e| format!("PTY reader: {e}"))?;

        let (tx, mut rx) = mpsc::channel::<String>(256);
        let sid_w = sid.clone();
        let sid_r = sid.clone();

        std::thread::spawn(move || {
            let mut w = writer;
            while let Some(input) = rx.blocking_recv() {
                if std::io::Write::write_all(&mut w, input.as_bytes()).is_err() {
                    eprintln!("[terminal] '{sid_w}' writer closed");
                    break;
                }
                let _ = std::io::Write::flush(&mut w);
            }
        });

        let out_tx = self.terminal_sender.clone();
        let log_tx = self.log_sender.clone();
        std::thread::spawn(move || {
            let mut r = reader;
            let mut buf = [0u8; 4096];
            let mut utf8 = Utf8StreamDecoder::new();
            loop {
                match std::io::Read::read(&mut r, &mut buf) {
                    Ok(0) => {
                        let tail = utf8.finish();
                        if !tail.is_empty() {
                            let _ = out_tx.send(TerminalOutput {
                                session_id: sid_r.clone(),
                                text: tail,
                            });
                        }
                        eprintln!("[terminal] '{sid_r}' EOF");
                        break;
                    }
                    Ok(n) => {
                        let text = utf8.push(&buf[..n]);
                        if text.is_empty() {
                            continue;
                        }
                        let _ = out_tx.send(TerminalOutput {
                            session_id: sid_r.clone(),
                            text: text.clone(),
                        });

                        // Extract project ID from session ID (e.g. backend-term-default -> backend)
                        let project_id = sid_r.split("-term-").next().unwrap_or(&sid_r).to_string();
                        let _ = log_tx.send(super::models::ProcessLog {
                            project_id,
                            stream: "stdout".to_string(),
                            text,
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                        });
                    }
                    Err(e) => {
                        eprintln!("[terminal] '{sid_r}' read: {e}");
                        break;
                    }
                }
            }
        });

        self.pty_masters.insert(sid.clone(), pty.master);

        // ── Tầng 2: OS-level sandbox (pre-spawn hoặc post-spawn) ──
        // Windows: Job Object post-spawn
        // Linux: sandbox đã được inject qua wrapper script ở pre-spawn
        let job_handle: Option<usize> = {
            #[cfg(target_os = "windows")]
            {
                match super::sandbox::windows::apply_sandbox_to_process(pid) {
                    Ok(handle) => handle,
                    Err(e) => {
                        eprintln!("[terminal] Failed to apply Windows sandbox: {e}");
                        None
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                None
            }
        };

        let workspace_root = PathBuf::from(&abs_root);
        self.terminal_sessions.insert(
            sid.clone(),
            TerminalSession {
                stdin_sender: tx,
                pid,
                workspace_root: workspace_root.clone(),
                block_internet,
                _child: Some(child),
                _job_handle: job_handle,
                _sandbox_wrapper_path: sandbox_wrapper_path,
            },
        );

        // Apply network isolation if requested
        if block_internet {
            if let Err(e) = super::network_isolate::block_pid(&sid, pid) {
                eprintln!("[terminal] Failed to block internet for '{sid}': {e}");
            }
        }

        self.input_buf.insert(sid.clone(), String::new());
        self.sessions_cwd.insert(sid.clone(), workspace_root);

        let hb_tx = self.terminal_sender.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            let _ = hb_tx.send(TerminalOutput {
                session_id: sid,
                text: String::new(),
            });
        });

        Ok(())
    }

    pub async fn kill_terminal(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(s) = self.terminal_sessions.remove(session_id) {
            eprintln!("[terminal] Kill '{session_id}' PID {}", s.pid);

            // Clean up network isolation if was active
            if s.block_internet {
                let _ = super::network_isolate::unblock_session(session_id);
            }
            super::tree::terminate_process_tree(s.pid).await;

            // Clean up Linux sandbox wrapper script
            #[cfg(target_os = "linux")]
            {
                super::sandbox::linux::cleanup_wrapper(s._sandbox_wrapper_path.as_deref());
            }
        }
        self.input_buf.remove(session_id);
        self.sessions_cwd.remove(session_id);
        self.pty_masters.remove(session_id);
        if let Some(path) = self._prompt_files.remove(session_id) {
            let _ = std::fs::remove_file(&path);
        }
        Ok(())
    }

    pub fn resize_terminal(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
        if let Some(master) = self.pty_masters.get(session_id) {
            master
                .resize(portable_pty::PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("Failed to resize PTY: {e}"))?;
            Ok(())
        } else {
            Err(format!("PTY master not found for session '{session_id}'"))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helper functions cho CWD tracking (tương thích với interceptor)
// ═══════════════════════════════════════════════════════════════════

/// Lấy home directory, tương thích cross-platform
fn get_home_dir_for_cwd() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok()
    }
}

/// Resolve environment variables trong path string
fn resolve_env_vars_for_cwd(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'{') {
                chars.next();
                let mut var_name = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }
                let val = if let Some(e) = var_name.strip_prefix("env:") {
                    std::env::var(e).unwrap_or_default()
                } else {
                    std::env::var(&var_name).unwrap_or_default()
                };
                result.push_str(&val);
            } else {
                let mut var_name = String::new();
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() || next == '_' || next == ':' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                let val = if let Some(e) = var_name.strip_prefix("env:") {
                    std::env::var(e).unwrap_or_default()
                } else {
                    std::env::var(&var_name).unwrap_or_default()
                };
                result.push_str(&val);
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub async fn process_and_send_terminal_input(
    _session_id: &str,
    mut text: String,
    ctx: &TerminalWriteContext,
) -> Result<(), String> {
    if text.ends_with('\n') && !text.ends_with("\r\n") {
        text.pop();
        text.push_str("\r\n");
    }
    ctx.stdin_sender
        .send(text)
        .await
        .map_err(|e| format!("Terminal send: {e}"))
}

/// Detect the user's preferred interactive shell on Unix (honoring `$SHELL` on macOS/Linux).
pub fn detect_default_shell() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        let trimmed = shell.trim();
        if !trimmed.is_empty() && std::path::Path::new(trimmed).exists() {
            return trimmed.to_string();
        }
    }
    for candidate in ["/bin/bash", "/bin/zsh", "/usr/bin/zsh", "/bin/sh"] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    "sh".to_string()
}

/// Incremental UTF-8 decoder that buffers incomplete trailing multi-byte sequences
/// across arbitrary `Read::read` chunk boundaries.
pub struct Utf8StreamDecoder {
    pending: Vec<u8>,
}

impl Default for Utf8StreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Utf8StreamDecoder {
    pub fn new() -> Self {
        Self {
            pending: Vec::with_capacity(4),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> String {
        if chunk.is_empty() {
            return String::new();
        }
        self.pending.extend_from_slice(chunk);
        let mut out = String::new();

        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(valid) => {
                    out.push_str(valid);
                    self.pending.clear();
                    break;
                }
                Err(err) => {
                    let valid_up_to = err.valid_up_to();
                    if valid_up_to > 0 {
                        // Safety: `valid_up_to` is guaranteed valid UTF-8 by std::str::from_utf8.
                        let valid_str =
                            unsafe { std::str::from_utf8_unchecked(&self.pending[..valid_up_to]) };
                        out.push_str(valid_str);
                    }
                    match err.error_len() {
                        Some(bad_len) => {
                            out.push('\u{FFFD}');
                            self.pending.drain(..valid_up_to + bad_len);
                        }
                        None => {
                            // Incomplete multi-byte sequence at the end of the chunk; keep it for next push.
                            self.pending.drain(..valid_up_to);
                            break;
                        }
                    }
                }
            }
        }

        out
    }

    pub fn finish(&mut self) -> String {
        if self.pending.is_empty() {
            return String::new();
        }
        let tail = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        tail
    }
}

/// Helper: build terminal CommandBuilder với Linux sandbox wrapper
///
/// Trên Linux: tạo wrapper script gọi `ulimit -v` rồi exec shell thật.
/// Trên non-Linux: dùng shell trực tiếp.
fn build_terminal_cmd(
    shell: &str,
    block_internet: bool,
    wrapper_path_out: &mut Option<String>,
) -> CommandBuilder {
    let shell_base = std::path::Path::new(shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(shell);
    let supports_login_flag = matches!(shell_base, "bash" | "zsh" | "fish");

    #[cfg(target_os = "linux")]
    {
        let memory_limit_mb = 0u64; // Default: no memory limit (uses SandboxConfig)
        match super::sandbox::linux::build_sandbox_wrapper_path(
            shell,
            memory_limit_mb,
            block_internet,
        ) {
            Ok(path) => {
                let _ = wrapper_path_out.insert(path.clone());
                let mut c = CommandBuilder::new(&path);
                if supports_login_flag {
                    c.arg("-l");
                }
                c
            }
            Err(e) => {
                eprintln!("[terminal] Failed to create sandbox wrapper: {e}, using direct shell");
                let mut c = CommandBuilder::new(shell);
                if supports_login_flag {
                    c.arg("-l");
                }
                c
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = block_internet;
        let _ = wrapper_path_out;
        let mut c = CommandBuilder::new(shell);
        if supports_login_flag {
            c.arg("-l");
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_decoder_preserves_split_multibyte_chars() {
        let mut dec = Utf8StreamDecoder::new();
        // Vietnamese phrase with 3-byte UTF-8 characters split mid-rune across chunks
        let text = "Xin chào thế giới 🚀";
        let bytes = text.as_bytes();

        let mut assembled = String::new();
        for piece in bytes.chunks(2) {
            assembled.push_str(&dec.push(piece));
        }
        assembled.push_str(&dec.finish());
        assert_eq!(assembled, text);
    }

    #[test]
    fn utf8_decoder_recovers_from_invalid_byte() {
        let mut dec = Utf8StreamDecoder::new();
        let out = dec.push(b"abc\xFFxyz");
        assert_eq!(out, "abc\u{FFFD}xyz");
        assert!(dec.finish().is_empty());
    }

    #[test]
    fn detect_default_shell_returns_non_empty() {
        let shell = detect_default_shell();
        assert!(!shell.trim().is_empty());
    }
}
