use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::code_rag::db::{VectorDb, VectorEntry};
use crate::code_rag::extractor::{extract_functions, should_index_file};
use crate::code_rag::language_resolver::LanguageResolver;
use crate::code_rag::normalizer::FunctionEntry;
use crate::code_rag::query::dummy_embedding;
use crate::code_rag::EmbeddingModel;

/// Sự kiện index (từ file watcher)
#[derive(Debug, Clone)]
pub enum IndexEvent {
    /// File đã thay đổi → cần re-index
    FileChanged { path: PathBuf, project_id: String },
    /// File mới được tạo
    FileCreated { path: PathBuf, project_id: String },
    /// File bị xóa
    FileDeleted { path: PathBuf, project_id: String },
    /// Re-index toàn bộ project
    ProjectRescanned {
        project_id: String,
        base_path: PathBuf,
    },
}

/// Độ ưu tiên index
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexPriority {
    High, // File đang được user edit
    Low,  // Background scan
}

/// Cấu hình cho Indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Debounce thời gian (ms) - gộp sự kiện file thay đổi
    pub debounce_ms: u64,
    /// Số worker thread
    pub worker_count: usize,
    /// Kích thước batch khi upsert DB
    pub batch_size: usize,
    /// Kích thước file tối đa (bytes)
    pub max_file_size: usize,
    /// Tự động scan project khi được add
    pub auto_scan_on_project_add: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 1500,         // 1.5s debounce
            worker_count: 2,           // 2 worker threads
            batch_size: 50,            // upsert 50 entries/lần
            max_file_size: 512 * 1024, // 512KB
            auto_scan_on_project_add: true,
        }
    }
}

/// Background Indexing Engine (Producer-Consumer)
///
/// Luồng:
///   Main Thread: bắt sự kiện file → đẩy vào Message Queue
///   Worker Thread: đọc Queue → Parse → Embed → Upsert DB
pub struct Indexer {
    db: Arc<VectorDb>,
    config: IndexerConfig,
    /// Sender để đẩy event vào queue
    event_tx: mpsc::UnboundedSender<IndexEvent>,
    /// Receiver
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<IndexEvent>>>>,
    /// Cache: path → Instant (cho debounce)
    debounce_cache: Arc<RwLock<HashMap<String, Instant>>>,
    /// Đếm số lượng đã index
    stats: Arc<RwLock<IndexerStats>>,
    /// Flag để dừng worker
    running: Arc<RwLock<bool>>,
    /// Embedding model (optional) — dùng để tạo vector thực tế khi index
    embedding_model: Arc<RwLock<Option<crate::code_rag::EmbeddingModel>>>,
    /// Cache mtime + size theo project_id → (file_path → (mtime_secs, file_size))
    project_file_mtimes: Arc<RwLock<HashMap<String, HashMap<String, (u64, u64)>>>>,
}

#[derive(Debug, Clone, Default)]
pub struct IncrementalSyncStats {
    pub total_scanned_files: usize,
    pub cache_hits: usize,
    pub reindexed_files: usize,
    pub deleted_files: usize,
    pub elapsed_ms: u64,
    pub partial_remaining: usize,
}

#[derive(Debug, Clone, Default)]
pub struct IndexerStats {
    pub total_files_indexed: u64,
    pub total_functions_extracted: u64,
    pub total_errors: u64,
    pub last_index_time: Option<Duration>,
}

impl Indexer {
    pub fn new(db: Arc<VectorDb>, config: IndexerConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            db,
            config,
            event_tx: tx,
            event_rx: Arc::new(RwLock::new(Some(rx))),
            debounce_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(IndexerStats::default())),
            running: Arc::new(RwLock::new(false)),
            embedding_model: Arc::new(RwLock::new(None)),
            project_file_mtimes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gán embedding model cho Indexer (để tạo vector thực tế khi index)
    pub fn set_embedding_model(&self, model: crate::code_rag::EmbeddingModel) {
        *self.embedding_model.write() = Some(model);
    }

    /// Helper: embed text → vector, fallback về dummy nếu chưa có model
    fn embed_or_dummy(&self, text: &str) -> Vec<f32> {
        if let Some(ref model) = *self.embedding_model.read() {
            match model.embed(text) {
                Ok(v) => return v,
                Err(e) => {
                    eprintln!("[CodeRAG] Embedding failed: {}. Using dummy.", e);
                }
            }
        }
        crate::code_rag::query::dummy_embedding(text)
    }

    /// Push event vào queue
    pub fn push_event(&self, event: IndexEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Scan và index ngay lập tức (synchronous), không qua event queue.
    /// Dùng cho lần index đầu tiên khi mở project.
    pub fn scan_and_index_project(
        &self,
        base_path: &Path,
        project_id: &str,
    ) -> Result<u64, String> {
        let start = std::time::Instant::now();
        eprintln!(
            "[CodeRAG] Starting scan for project {} at {:?}",
            project_id, base_path
        );

        let entries = self.index_project(base_path, project_id)?;
        let count = entries.len() as u64;

        // Debug: đếm lang_id
        use std::collections::HashMap;
        let mut lang_counter: HashMap<String, usize> = HashMap::new();
        for entry in &entries {
            *lang_counter.entry(entry.lang_id.clone()).or_default() += 1;
        }
        eprintln!("[CodeRAG]   Languages found: {:?}", lang_counter);

        for entry in entries {
            let vector = self.embed_or_dummy(&entry.normalized_text);
            let vec_entry = VectorEntry::from((entry, vector));
            self.db.upsert(vec_entry);
        }

        // Debug: verify DB after upsert
        let by_lang = self.db.by_language("rust");
        eprintln!(
            "[CodeRAG]   DB has {} entries with lang=rust after upsert",
            by_lang.len()
        );
        for e in by_lang.iter().take(3) {
            eprintln!("[CodeRAG]     sample: {} | {}", e.func_name, e.file_path);
        }

        let elapsed = start.elapsed();
        eprintln!(
            "[CodeRAG] Scan complete: {} functions indexed in {:?}",
            count, elapsed
        );

        let mut s = self.stats.write();
        s.total_files_indexed += 1;
        s.total_functions_extracted += count;
        s.last_index_time = Some(elapsed);

        Ok(count)
    }

    /// Debounce: trả về true nếu event nên được bỏ qua (file vừa được xử lý trong cửa sổ debounce)
    fn should_debounce(&self, path: &Path, event: &IndexEvent) -> bool {
        match event {
            IndexEvent::FileChanged { .. } | IndexEvent::FileCreated { .. } => {
                let key = path.to_string_lossy().to_string();
                let mut cache = self.debounce_cache.write();
                let now = Instant::now();
                let debounce = Duration::from_millis(self.config.debounce_ms);

                if let Some(last) = cache.get(&key) {
                    if now.duration_since(*last) < debounce {
                        return true; // Drop — vừa xử lý trong 1.5s
                    }
                }

                cache.insert(key, now);
                false
            }
            _ => false,
        }
    }

    /// Index một file đơn
    fn index_file(&self, path: &Path, project_id: &str) -> Result<Vec<FunctionEntry>, String> {
        let content_bytes = std::fs::read(path).map_err(|e| format!("Read error: {}", e))?;

        // File filter
        if !should_index_file(path, &content_bytes) {
            return Ok(vec![]);
        }

        let content = String::from_utf8_lossy(&content_bytes);

        // Xác định ngôn ngữ
        let lang_cfg = LanguageResolver::resolve(path, &content)
            .ok_or_else(|| format!("Unsupported language: {:?}", path))?;

        // Debug: log từng file
        let path_str = path.to_string_lossy();
        if path_str.ends_with(".rs") {
            eprintln!(
                "[CodeRAG]   Indexing .rs file: {} → lang={}",
                path_str, lang_cfg.lang_id
            );
        }

        // Extract functions
        let result =
            extract_functions(&content, path.to_str().unwrap_or(""), project_id, &lang_cfg);

        Ok(result.entries)
    }

    /// Index toàn bộ project (scan directory)
    /// Public để có thể gọi từ bên ngoài
    pub fn index_project(
        &self,
        base_path: &Path,
        project_id: &str,
    ) -> Result<Vec<FunctionEntry>, String> {
        let mut all_entries = Vec::new();
        self.walk_dir(base_path, &mut all_entries, project_id);
        Ok(all_entries)
    }

    /// Walk directory đệ quy
    fn walk_dir(&self, dir: &Path, entries: &mut Vec<FunctionEntry>, project_id: &str) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in read_dir.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if Self::should_skip_dir(dir_name) {
                    continue;
                }
                self.walk_dir(&path, entries, project_id);
            } else if path.is_file() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with('.') && name != ".env" && name != ".gitignore" {
                        continue;
                    }
                }
                match self.index_file(&path, project_id) {
                    Ok(mut file_entries) => entries.append(&mut file_entries),
                    Err(_) => { /* skip unsupported */ }
                }
            }
        }
    }

    pub fn should_skip_dir(dir_name: &str) -> bool {
        const SKIP_DIRS: &[&str] = &[
            "node_modules",
            ".git",
            "target",
            "build",
            "dist",
            "out",
            "coverage",
            ".next",
            ".nuxt",
            ".turbo",
            ".parcel-cache",
            ".cache",
            "__pycache__",
            "vendor",
            ".venv",
            "venv",
            "env",
            ".tox",
            ".eggs",
            ".gradle",
            "Pods",
            "DerivedData",
            "idea",
            ".vscode",
            ".tauri",
            "app_data",
            "logs",
        ];
        SKIP_DIRS.contains(&dir_name)
            || dir_name.starts_with('.')
            || dir_name.starts_with("scratch_")
            || dir_name.starts_with("cmake-build")
    }

    fn collect_files_meta(
        dir: &Path,
        max_files: usize,
        out: &mut Vec<(PathBuf, u64, u64)>,
    ) {
        if out.len() >= max_files {
            return;
        }
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in read_dir.flatten() {
            if out.len() >= max_files {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if Self::should_skip_dir(dir_name) {
                    continue;
                }
                Self::collect_files_meta(&path, max_files, out);
            } else if path.is_file() {
                let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if name.starts_with('.') {
                    continue;
                }
                // Chỉ xét file có extension hỗ trợ để khỏi đọc binary/ảnh
                if path.extension().is_none()
                    || crate::code_rag::LanguageResolver::from_extension(&path).is_none()
                {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    let size = meta.len();
                    if size == 0 || size > 512 * 1024 {
                        continue;
                    }
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    out.push((path, mtime, size));
                }
            }
        }
    }

    /// Đồng bộ index tăng dần kiểu Cursor (chỉ parse & embed file mới/vừa sửa, nạp cache disk nếu có)
    pub fn sync_project_incremental(
        &self,
        base_path: &Path,
        project_id: &str,
        max_files: usize,
        time_budget_ms: u64,
    ) -> IncrementalSyncStats {
        let started = Instant::now();
        let mut sync_stats = IncrementalSyncStats::default();

        if !base_path.exists() || !base_path.is_dir() {
            return sync_stats;
        }

        // 1. Nạp cache mtimes từ RAM hoặc từ file .rag.gz trên disk
        let mut known_mtimes: HashMap<String, (u64, u64)> = {
            let guard = self.project_file_mtimes.read();
            guard.get(project_id).cloned().unwrap_or_default()
        };

        if known_mtimes.is_empty() && self.db.count_project(project_id) == 0 {
            if let Some(loaded) = self.db.load_project_cache(project_id) {
                known_mtimes = loaded;
            }
        }

        // 2. Quét metadata file hiện tại (cực nhanh, không đọc nội dung file)
        let mut disk_files = Vec::new();
        Self::collect_files_meta(base_path, max_files, &mut disk_files);
        sync_stats.total_scanned_files = disk_files.len();

        let mut disk_map: HashMap<String, (u64, u64)> = HashMap::with_capacity(disk_files.len());
        let mut to_reindex: Vec<(PathBuf, String, u64, u64)> = Vec::new();

        for (path, mtime, size) in disk_files {
            let path_str = path.to_string_lossy().to_string();
            disk_map.insert(path_str.clone(), (mtime, size));
            if let Some(&(old_mtime, old_size)) = known_mtimes.get(&path_str) {
                if old_mtime == mtime && old_size == size {
                    sync_stats.cache_hits += 1;
                    continue;
                }
            }
            to_reindex.push((path, path_str, mtime, size));
        }

        // 3. Xóa entries của các file đã bị xóa khỏi ổ đĩa
        let deleted_paths: Vec<String> = known_mtimes
            .keys()
            .filter(|p| !disk_map.contains_key(*p))
            .cloned()
            .collect();

        for del_path in &deleted_paths {
            self.db.remove_file_entries(project_id, del_path);
            known_mtimes.remove(del_path);
            sync_stats.deleted_files += 1;
        }

        // 4. Ưu tiên file ngắn/nông hơn trước và tôn trọng time_budget_ms để không block user
        to_reindex.sort_by_key(|(p, _, _, sz)| (p.components().count(), *sz));

        let total_pending = to_reindex.len();
        let embed_model_opt = self.embedding_model.read().clone();

        for (idx, (path, path_str, mtime, size)) in to_reindex.into_iter().enumerate() {
            if idx > 0 && started.elapsed().as_millis() as u64 >= time_budget_ms {
                sync_stats.partial_remaining = total_pending.saturating_sub(idx);
                break;
            }

            self.db.remove_file_entries(project_id, &path_str);
            if let Ok(entries) = self.index_file(&path, project_id) {
                let mut vec_entries = Vec::with_capacity(entries.len());
                for entry in entries {
                    let vector = if let Some(ref model) = embed_model_opt {
                        model
                            .embed(&entry.normalized_text)
                            .unwrap_or_else(|_| dummy_embedding(&entry.normalized_text))
                    } else {
                        dummy_embedding(&entry.normalized_text)
                    };
                    vec_entries.push(VectorEntry::from((entry, vector)));
                }
                self.db.upsert_batch(vec_entries);
            }
            known_mtimes.insert(path_str, (mtime, size));
            sync_stats.reindexed_files += 1;
        }

        // 5. Cập nhật RAM & lưu cache ra disk nếu có thay đổi
        let dirty = sync_stats.reindexed_files > 0 || sync_stats.deleted_files > 0;
        {
            self.project_file_mtimes
                .write()
                .insert(project_id.to_string(), known_mtimes.clone());
        }
        if dirty {
            let _ = self.db.save_project_cache(project_id, &known_mtimes);
        }

        sync_stats.elapsed_ms = started.elapsed().as_millis() as u64;
        sync_stats
    }

    /// Tạo Repo Map siêu nhẹ (danh sách file quan trọng nhất theo độ nông thư mục)
    pub fn build_repo_map(&self, base_path: &Path, max_files: usize) -> Vec<String> {
        let mut all_paths = Vec::new();
        Self::collect_repo_map_paths(base_path, base_path, 0, 5, 400, &mut all_paths);
        all_paths.sort_by_key(|p| (p.matches('/').count() + p.matches('\\').count(), p.len()));
        all_paths.truncate(max_files);
        all_paths
    }

    fn collect_repo_map_paths(
        root: &Path,
        dir: &Path,
        depth: usize,
        max_depth: usize,
        hard_cap: usize,
        out: &mut Vec<String>,
    ) {
        if depth > max_depth || out.len() >= hard_cap {
            return;
        }
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.flatten() {
            if out.len() >= hard_cap {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if Self::should_skip_dir(dir_name) {
                    continue;
                }
                Self::collect_repo_map_paths(root, &path, depth + 1, max_depth, hard_cap, out);
            } else if path.is_file() {
                let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if name.starts_with('.') {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }

    /// Chạy worker (gọi từ background task)
    pub async fn run(&self) {
        let mut rx = self.event_rx.write().take();
        let Some(rx) = rx.as_mut() else {
            return;
        };

        *self.running.write() = true;
        let running = self.running.clone();
        let db = self.db.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        let embedding_model = self.embedding_model.clone();

        while *running.read() {
            tokio::select! {
                Some(event) = rx.recv() => {
                    let now = Instant::now();

                    match event.clone() {
                        IndexEvent::FileChanged { ref path, ref project_id }
                        | IndexEvent::FileCreated { ref path, ref project_id } => {
                            // Debounce check
                            if self.should_debounce(path, &event) {
                                continue;
                            }

                            // Index file trong spawn_blocking
                            let path = path.clone();
                            let project_id = project_id.clone();
                            let db = db.clone();
                            let stats = stats.clone();
                            let max_file_size = config.max_file_size;
                            let embed_model = embedding_model.clone();

                            tokio::task::spawn_blocking(move || {
                                let content = std::fs::read(&path).ok();
                                let Some(ref content) = content else { return };

                                if content.len() > max_file_size { return; }

                                let content_str = String::from_utf8_lossy(content);
                                let lang_cfg = LanguageResolver::resolve(&path, &content_str);
                                let Some(ref lang_cfg) = lang_cfg else { return };

                                let result = extract_functions(
                                    &content_str,
                                    path.to_str().unwrap_or(""),
                                    &project_id,
                                    lang_cfg,
                                );

                                let entry_count = result.entries.len() as u64;

                                // Embed và upsert (dùng model ONNX nếu có, fallback dummy)
                                for entry in result.entries {
                                    let vector = embed_or_dummy_static(&embed_model, &entry.normalized_text);
                                    let vec_entry = VectorEntry::from((entry, vector));
                                    db.upsert(vec_entry);
                                }

                                let mut s = stats.write();
                                s.total_files_indexed += 1;
                                s.total_functions_extracted += entry_count;
                                s.last_index_time = Some(now.elapsed());
                            }).await.ok();
                        }
                        IndexEvent::FileDeleted { ref path, ref project_id } => {
                            // Xóa entries của file này
                            let path_str = path.to_str().unwrap_or("");
                            let db = db.clone();
                            let stats = stats.clone();
                            let project_id = project_id.clone();
                            let path_owned = path_str.to_string();

                            tokio::task::spawn_blocking(move || {
                                let all = db.by_project(&project_id);
                                for entry in all {
                                    if entry.file_path == path_owned {
                                        db.delete(&entry.id);
                                    }
                                }
                                let mut s = stats.write();
                                s.total_files_indexed = s.total_files_indexed.saturating_sub(1);
                            }).await.ok();
                        }
                        IndexEvent::ProjectRescanned { ref project_id, ref base_path } => {
                            let base_path = base_path.clone();
                            let project_id = project_id.clone();
                            let db = db.clone();
                            let stats = stats.clone();
                            let embed_model = embedding_model.clone();

                            // Xóa dữ liệu cũ
                            db.delete_project(&project_id);

                            // Scan toàn bộ project
                            tokio::task::spawn_blocking(move || {
                                let mut entries = Vec::new();
                                let indexer_temp = Indexer::new(
                                    db.clone(),
                                    IndexerConfig::default(),
                                );
                                let result = indexer_temp.index_project(&base_path, &project_id);
                                if let Ok(mut func_entries) = result {
                                    entries.append(&mut func_entries);
                                }

                                for entry in entries {
                                    let vector = embed_or_dummy_static(&embed_model, &entry.normalized_text);
                                    let vec_entry = VectorEntry::from((entry, vector));
                                    db.upsert(vec_entry);
                                }

                                let mut s = stats.write();
                                s.total_files_indexed += 1;
                            }).await.ok();
                        }
                    }
                }
                else => {
                    // Channel closed
                    break;
                }
            }
        }
    }

    /// Dừng worker
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Lấy stats
    pub fn stats(&self) -> IndexerStats {
        self.stats.read().clone()
    }
}

/// Helper: embed text dùng embedding model (dùng trong closures của `run`)
/// vì closures không thể borrow `self` của Indexer.
fn embed_or_dummy_static(model: &Arc<RwLock<Option<EmbeddingModel>>>, text: &str) -> Vec<f32> {
    if let Some(ref model) = *model.read() {
        match model.embed(text) {
            Ok(v) => return v,
            Err(e) => {
                eprintln!("[CodeRAG] Embedding failed in worker: {}. Using dummy.", e);
            }
        }
    }
    dummy_embedding(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup_temp_project() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let proj_id = "test_proj".to_string();

        // Tạo một file Python
        let py_file = dir.path().join("hello.py");
        std::fs::write(&py_file, b"def greet(name):\n    print(f'Hello {name}')\n").unwrap();

        // Tạo một file Rust
        let rs_file = dir.path().join("lib.rs");
        std::fs::write(
            &rs_file,
            b"pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        )
        .unwrap();

        (dir, proj_id)
    }

    #[tokio::test]
    async fn test_index_single_file() {
        let (dir, proj_id) = setup_temp_project();
        let db = Arc::new(VectorDb::new(PathBuf::from("/tmp/test_idx")));
        let indexer = Indexer::new(db.clone(), IndexerConfig::default());

        let py_file = dir.path().join("hello.py");
        let entries = indexer.index_file(&py_file, &proj_id).unwrap();
        assert!(!entries.is_empty(), "Should extract at least 1 function");
    }

    #[tokio::test]
    async fn test_index_project() {
        let (dir, proj_id) = setup_temp_project();
        let db = Arc::new(VectorDb::new(PathBuf::from("/tmp/test_idx2")));
        let indexer = Indexer::new(db.clone(), IndexerConfig::default());

        let entries = indexer.index_project(dir.path(), &proj_id).unwrap();
        assert!(
            entries.len() >= 2,
            "Should extract >=2 functions, got {}",
            entries.len()
        );
    }

    #[tokio::test]
    async fn test_push_and_process_event() {
        let (dir, proj_id) = setup_temp_project();
        let db = Arc::new(VectorDb::new(PathBuf::from("/tmp/test_idx3")));
        let indexer = Arc::new(Indexer::new(db.clone(), IndexerConfig::default()));

        // Clone để chạy worker
        let idx_clone = indexer.clone();
        tokio::spawn(async move {
            idx_clone.run().await;
        });

        // Push event
        let py_file = dir.path().join("hello.py");
        indexer.push_event(IndexEvent::FileChanged {
            path: py_file,
            project_id: proj_id.clone(),
        });

        // Chờ xử lý
        tokio::time::sleep(Duration::from_millis(500)).await;
        indexer.stop();

        let stats = indexer.stats();
        assert!(stats.total_files_indexed > 0 || stats.total_functions_extracted > 0);
    }
}
