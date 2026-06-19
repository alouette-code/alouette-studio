use crate::inference::MiniCpmInference;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

// ──────────────────────────────────────────────────────────────────────────────
// Managed State
// ──────────────────────────────────────────────────────────────────────────────

pub struct ModelManager {
    engine: Option<MiniCpmInference>,
    model_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,
}

impl ModelManager {
    pub fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            engine: None,
            model_dir: resolve_model_dir(),
            cancel_flag,
        }
    }

    /// Ensure the inference engine is loaded.
    /// If already loaded, returns Ok immediately.
    /// If not, loads the model (may download missing files from HuggingFace).
    pub async fn ensure_running(&mut self) -> Result<(), String> {
        if self.engine.is_some() {
            return Ok(());
        }

        crate::state::log_to_app_file(&format!(
            "Loading MiniCPM inference engine from {}...",
            self.model_dir.display()
        ));

        // Load model — this is CPU-heavy, run on blocking thread
        let model_dir = self.model_dir.clone();
        let cancel_flag = self.cancel_flag.clone();

        let engine =
            tokio::task::spawn_blocking(move || MiniCpmInference::load(&model_dir, cancel_flag))
                .await
                .map_err(|e| format!("Load task failed: {}", e))?
                .map_err(|e| format!("Failed to load MiniCPM: {}", e))?;

        self.engine = Some(engine);
        crate::state::log_to_app_file("MiniCPM inference engine loaded successfully");
        Ok(())
    }

    /// Get a mutable reference to the loaded engine
    pub fn engine(&mut self) -> Option<&mut MiniCpmInference> {
        self.engine.as_mut()
    }

    /// Stop the inference engine (unload model from memory)
    pub fn stop(&mut self) {
        self.engine = None;
        self.cancel_flag
            .store(false, std::sync::atomic::Ordering::SeqCst);
        crate::state::log_to_app_file("MiniCPM inference engine stopped");
    }

    /// Get the cancel flag for stopping generation
    #[allow(dead_code)]
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }

    /// Cancel current generation
    #[allow(dead_code)]
    pub fn cancel_generation(&self) {
        self.cancel_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Path Resolution
// ──────────────────────────────────────────────────────────────────────────────

fn resolve_model_dir() -> PathBuf {
    crate::state::project_root().join("app_data/model_embedding/model_MiniCPM5-1B")
}

// ──────────────────────────────────────────────────────────────────────────────
// Factory / Convenience
// ──────────────────────────────────────────────────────────────────────────────

pub struct ModelManagerState {
    pub inner: Mutex<ModelManager>,
    pub cancel_flag: Arc<AtomicBool>,
}

pub type SharedModelManager = Arc<ModelManagerState>;

pub fn create_shared() -> SharedModelManager {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    Arc::new(ModelManagerState {
        inner: Mutex::new(ModelManager::new(cancel_flag.clone())),
        cancel_flag,
    })
}
