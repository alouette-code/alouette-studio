use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Hard upper bound for total worker agents spawned in a single Super Multi-Agent run.
pub const MAX_SUPER_AGENTS: usize = 30;

/// Clamp user-requested max agents safely to `1..=MAX_SUPER_AGENTS`.
pub fn clamp_super_agent_budget(requested: usize) -> usize {
    requested.clamp(1, MAX_SUPER_AGENTS)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafeguardVerdict {
    Allow,
    Warn(String),
    Halt { code: String, reason: String },
}

/// Per-worker deterministic circuit breaker against infinite tool loops,
/// repeated error loops, file edit thrashing, and out-of-scope edits.
#[derive(Debug, Clone)]
pub struct WorkerLoopDetector {
    workspace_root: PathBuf,
    scope_paths: Vec<String>,
    recent_signatures: VecDeque<String>,
    last_signature: Option<String>,
    consecutive_identical: u32,
    consecutive_errors: u32,
    repeated_error_counts: HashMap<String, u32>,
    file_edit_counts: HashMap<String, u32>,
    max_consecutive_identical: u32,
    max_window_repeats: usize,
    max_consecutive_errors: u32,
    max_same_file_edits: u32,
}

impl WorkerLoopDetector {
    pub fn new(workspace_root: PathBuf, scope_paths: Vec<String>) -> Self {
        Self {
            workspace_root,
            scope_paths,
            recent_signatures: VecDeque::with_capacity(12),
            last_signature: None,
            consecutive_identical: 0,
            consecutive_errors: 0,
            repeated_error_counts: HashMap::new(),
            file_edit_counts: HashMap::new(),
            max_consecutive_identical: 3,
            max_window_repeats: 4,
            max_consecutive_errors: 3,
            max_same_file_edits: 6,
        }
    }

    /// Inspect a tool call *before* executing it.
    pub fn check_before_tool(&mut self, tool_name: &str, args: &Value) -> SafeguardVerdict {
        // 1. Scope guard for file mutations
        if is_file_mutation_tool(tool_name) {
            if let Some(raw_path) = extract_target_path(args) {
                if !self.is_path_allowed(&raw_path) {
                    return SafeguardVerdict::Halt {
                        code: "OUT_OF_SCOPE_WRITE".to_string(),
                        reason: format!(
                            "Agent attempted to modify '{}' outside its assigned scope ({:?}).",
                            raw_path, self.scope_paths
                        ),
                    };
                }

                let norm = normalize_rel_key(&self.workspace_root, &raw_path);
                let count = self.file_edit_counts.entry(norm.clone()).or_insert(0);
                *count += 1;
                if *count >= self.max_same_file_edits {
                    return SafeguardVerdict::Halt {
                        code: "FILE_THRASHING".to_string(),
                        reason: format!(
                            "File '{}' was edited {} times by the same worker; halting to prevent thrashing.",
                            norm, *count
                        ),
                    };
                } else if *count == self.max_same_file_edits.saturating_sub(2) {
                    return SafeguardVerdict::Warn(format!(
                        "Warning: '{}' edited {} times by this worker. Consolidate changes instead of repeated small edits.",
                        norm, *count
                    ));
                }
            }
        }

        // 2. Duplicate / cyclic tool call loop detection
        let sig = canonical_tool_signature(tool_name, args);
        if self.last_signature.as_deref() == Some(sig.as_str()) {
            self.consecutive_identical += 1;
        } else {
            self.last_signature = Some(sig.clone());
            self.consecutive_identical = 1;
        }

        if self.recent_signatures.len() >= 10 {
            self.recent_signatures.pop_front();
        }
        self.recent_signatures.push_back(sig.clone());

        if self.consecutive_identical >= self.max_consecutive_identical {
            return SafeguardVerdict::Halt {
                code: "DUPLICATE_CALL_LOOP".to_string(),
                reason: format!(
                    "Tool '{}' called {} times consecutively with identical arguments.",
                    tool_name, self.consecutive_identical
                ),
            };
        }

        let window_count = self
            .recent_signatures
            .iter()
            .filter(|s| s.as_str() == sig.as_str())
            .count();
        if window_count >= self.max_window_repeats {
            return SafeguardVerdict::Halt {
                code: "CYCLIC_LOOP".to_string(),
                reason: format!(
                    "Cyclic tool pattern detected: '{}' repeated {} times in the last {} calls.",
                    tool_name,
                    window_count,
                    self.recent_signatures.len()
                ),
            };
        }

        if self.consecutive_identical == 2 {
            return SafeguardVerdict::Warn(format!(
                "Tool '{}' was just called twice with identical arguments. Change approach or finish if done.",
                tool_name
            ));
        }

        SafeguardVerdict::Allow
    }

    /// Inspect a tool call outcome *after* execution to catch error loops.
    pub fn record_outcome(&mut self, success: bool, output: &str) -> SafeguardVerdict {
        if success {
            self.consecutive_errors = 0;
            return SafeguardVerdict::Allow;
        }

        self.consecutive_errors += 1;
        let err_sig = normalize_error_signature(output);
        let repeated = self.repeated_error_counts.entry(err_sig).or_insert(0);
        *repeated += 1;

        if self.consecutive_errors >= self.max_consecutive_errors {
            return SafeguardVerdict::Halt {
                code: "CONSECUTIVE_ERROR_LOOP".to_string(),
                reason: format!(
                    "Worker failed {} consecutive tool calls. Latest error: {}",
                    self.consecutive_errors,
                    truncate_str(output, 180)
                ),
            };
        }

        if *repeated >= 3 {
            return SafeguardVerdict::Halt {
                code: "REPEATED_ERROR_LOOP".to_string(),
                reason: format!(
                    "Identical error repeated {} times: {}",
                    *repeated,
                    truncate_str(output, 180)
                ),
            };
        }

        if self.consecutive_errors == 2 || *repeated == 2 {
            return SafeguardVerdict::Warn(format!(
                "Worker hit repeated errors (consecutive={}, identical={}). Supervisor alerted.",
                self.consecutive_errors, *repeated
            ));
        }

        SafeguardVerdict::Allow
    }

    pub fn consecutive_errors(&self) -> u32 {
        self.consecutive_errors
    }

    fn is_path_allowed(&self, raw_path: &str) -> bool {
        if self.scope_paths.is_empty() {
            return true;
        }
        let target_norm = normalize_rel_key(&self.workspace_root, raw_path);
        for scope in &self.scope_paths {
            let trimmed = scope.trim();
            if trimmed.is_empty() || trimmed == "*" || trimmed == "." || trimmed == "./" {
                return true;
            }
            let scope_norm = normalize_rel_key(&self.workspace_root, trimmed);
            if scope_norm.is_empty() {
                return true;
            }
            if target_norm == scope_norm
                || target_norm.starts_with(&format!("{}/", scope_norm.trim_end_matches('/')))
            {
                return true;
            }
        }
        false
    }
}

/// Async per-file write lock registry so concurrent workers never race on the same file.
#[derive(Default)]
pub struct FileLockRegistry {
    locks: Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
}

impl FileLockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lock_for(&self, workspace_root: &Path, raw_path: &str) -> Arc<tokio::sync::Mutex<()>> {
        let key = resolve_normalized_path(workspace_root, raw_path);
        let mut guard = self.locks.lock();
        guard
            .entry(key)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

pub fn is_file_mutation_tool(tool_name: &str) -> bool {
    matches!(
        tool_name.to_ascii_lowercase().as_str(),
        "write_file" | "replace_in_file" | "write" | "edit" | "multiedit" | "patch"
    )
}

pub fn extract_target_path(args: &Value) -> Option<String> {
    args.get("filePath")
        .or_else(|| args.get("path"))
        .or_else(|| args.get("file_path"))
        .or_else(|| args.get("target_file"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn resolve_normalized_path(workspace_root: &Path, raw_path: &str) -> PathBuf {
    let p = Path::new(raw_path);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace_root.join(p)
    };
    lexical_normalize(&joined)
}

fn normalize_rel_key(workspace_root: &Path, raw_path: &str) -> String {
    let abs = resolve_normalized_path(workspace_root, raw_path);
    let root_norm = lexical_normalize(workspace_root);
    let rel = abs.strip_prefix(&root_norm).unwrap_or(&abs);
    rel.to_string_lossy().replace('\\', "/")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !matches!(out.components().next_back(), Some(Component::RootDir) | None) {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn canonical_tool_signature(tool_name: &str, args: &Value) -> String {
    let canonical = canonicalize_json(args);
    let mut hasher = Sha256::new();
    hasher.update(tool_name.as_bytes());
    hasher.update(b":");
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

fn canonicalize_json(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}:{}", serde_json::to_string(k).unwrap_or_default(), canonicalize_json(&map[k])))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonicalize_json).collect();
            format!("[{}]", parts.join(","))
        }
        other => other.to_string(),
    }
}

fn normalize_error_signature(err: &str) -> String {
    let trimmed = err.trim().to_lowercase();
    let snippet: String = trimmed.chars().take(120).collect();
    let mut hasher = Sha256::new();
    hasher.update(snippet.as_bytes());
    hex::encode(hasher.finalize())
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_clamp_super_agent_budget() {
        assert_eq!(clamp_super_agent_budget(0), 1);
        assert_eq!(clamp_super_agent_budget(12), 12);
        assert_eq!(clamp_super_agent_budget(30), 30);
        assert_eq!(clamp_super_agent_budget(150), 30);
    }

    #[test]
    fn test_duplicate_call_loop_warns_then_halts() {
        let mut det = WorkerLoopDetector::new(PathBuf::from("/proj"), vec![]);
        let args = json!({ "path": "src/main.rs" });
        assert_eq!(det.check_before_tool("read_file", &args), SafeguardVerdict::Allow);
        assert!(matches!(
            det.check_before_tool("read_file", &args),
            SafeguardVerdict::Warn(_)
        ));
        assert!(matches!(
            det.check_before_tool("read_file", &args),
            SafeguardVerdict::Halt { ref code, .. } if code == "DUPLICATE_CALL_LOOP"
        ));
    }

    #[test]
    fn test_consecutive_error_loop_halts() {
        let mut det = WorkerLoopDetector::new(PathBuf::from("/proj"), vec![]);
        assert_eq!(det.record_outcome(false, "Error A"), SafeguardVerdict::Allow);
        assert!(matches!(
            det.record_outcome(false, "Error B"),
            SafeguardVerdict::Warn(_)
        ));
        assert!(matches!(
            det.record_outcome(false, "Error C"),
            SafeguardVerdict::Halt { ref code, .. } if code == "CONSECUTIVE_ERROR_LOOP"
        ));
    }

    #[test]
    fn test_scope_guard_blocks_outside_writes() {
        let mut det = WorkerLoopDetector::new(
            PathBuf::from("/proj"),
            vec!["src/auth".to_string()],
        );
        assert_eq!(
            det.check_before_tool("write_file", &json!({ "path": "src/auth/mod.rs" })),
            SafeguardVerdict::Allow
        );
        assert!(matches!(
            det.check_before_tool("write_file", &json!({ "path": "Cargo.toml" })),
            SafeguardVerdict::Halt { ref code, .. } if code == "OUT_OF_SCOPE_WRITE"
        ));
        // OpenCode native tool names & filePath field
        assert_eq!(
            det.check_before_tool("edit", &json!({ "filePath": "src/auth/login.rs" })),
            SafeguardVerdict::Allow
        );
        assert!(matches!(
            det.check_before_tool("write", &json!({ "filePath": "src/other/main.rs" })),
            SafeguardVerdict::Halt { ref code, .. } if code == "OUT_OF_SCOPE_WRITE"
        ));
    }
}
