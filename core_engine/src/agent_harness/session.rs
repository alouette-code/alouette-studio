use crate::agent_harness::{AgentHarness, AgentSession, AgentState, HarnessMode};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{watch, OwnedRwLockWriteGuard, RwLock};
use tokio::task::JoinHandle;

/// Metadata của một session — luôn ở RAM, kể cả khi runtime bị evict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub title: String,
    pub project_id: String,
    pub project_cwd: String,
    pub model: String,
    pub mode: String,
    pub created_at: i64,
    pub last_accessed: i64,
}

/// Runtime của một agent đang chạy — có hoặc không
pub struct SessionRuntime {
    pub harness: AgentHarness,
    pub state_sender: watch::Sender<AgentState>,
    pub state_receiver: watch::Receiver<AgentState>,
    pub task_handle: Option<JoinHandle<()>>,
}

/// Entry chính trong registry — trong RAM
pub struct SessionEntry {
    pub meta: SessionMetadata,
    pub runtime: Option<SessionRuntime>,
    pub pause_lock: Arc<RwLock<()>>,
    pub paused_guard: Option<OwnedRwLockWriteGuard<()>>,
    pub resume_count: Arc<AtomicU64>,
    pub agent_session: AgentSession,
    pub history_store: HistoryStore,
}

impl SessionEntry {
    /// Tạo mới một session entry cho một project
    pub fn new(project_id: &str, project_cwd: &str, model: &str, mode: &str) -> Self {
        let now = chrono::Local::now().timestamp();
        let (_state_sender, _state_receiver) = watch::channel(AgentState::Idle);

        Self {
            meta: SessionMetadata {
                session_id: format!("sess_{}", now),
                title: "New Chat".to_string(),
                project_id: project_id.to_string(),
                project_cwd: project_cwd.to_string(),
                model: model.to_string(),
                mode: mode.to_string(),
                created_at: now,
                last_accessed: now,
            },
            runtime: None,
            pause_lock: Arc::new(RwLock::new(())),
            paused_guard: None,
            resume_count: Arc::new(AtomicU64::new(0)),
            agent_session: AgentSession {
                session_id: format!("sess_{}", now),
                history: Vec::new(),
                state: AgentState::Idle,
                iteration_count: 0,
                max_iterations: 25,
                mode: HarnessMode::Standard,
                plan: None,
                autonomous_state: None,
                token_budget: 50000,
            },
            history_store: HistoryStore::new(),
        }
    }

    /// PAUSE: acquire write lock → tất cả read() trong agent loop sẽ block
    pub async fn pause(&mut self) {
        if self.runtime.is_some() {
            let guard = self.pause_lock.clone().write_owned().await;
            self.paused_guard = Some(guard);
        }
    }

    /// RESUME: drop write guard → agent loop unblock
    /// KHÔNG send state — agent loop tự send Resuming khi phát hiện resume_count thay đổi
    pub fn resume(&mut self) {
        if self.paused_guard.is_some() {
            self.paused_guard = None;
            self.resume_count.fetch_add(1, Ordering::Release);
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused_guard.is_some()
    }

    /// Kiểm tra có thể evict khỏi RAM không
    pub fn is_evictable(&self) -> bool {
        match &self.runtime {
            None => true,
            Some(r) => {
                // Idle (task_handle None) hoặc Paused → evict được
                r.task_handle.is_none() || self.is_paused()
            }
        }
    }
}

/// Thông tin trả về khi switch project
#[derive(Debug, Clone, Serialize)]
pub struct AgentSwitchInfo {
    pub session_id: Option<String>,
    pub has_history: bool,
    pub old_status: String,
}

/// Lưu trữ history với lazy load từ disk
pub struct HistoryStore {
    pub dirty: bool,
}

impl HistoryStore {
    pub fn new() -> Self {
        Self { dirty: false }
    }

    pub async fn flush_to_disk(&mut self) {
        // TODO: flush history to SQLite
        self.dirty = false;
    }
}

/// Counter global cho sequence number (chống stale switch request)
pub static SWITCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);
