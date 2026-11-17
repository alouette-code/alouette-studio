use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::code_rag::normalizer::FunctionEntry;

/// Entry lưu trong vector DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Từ FunctionEntry.id
    pub id: String,
    /// Vector embedding (f32 array)
    pub vector: Vec<f32>,
    /// Text đã chuẩn hóa
    pub normalized_text: String,
    /// Language ID để filter
    pub lang_id: String,
    /// Project ID để filter
    pub project_id: String,
    /// Tên function
    pub func_name: String,
    /// Đường dẫn file
    pub file_path: String,
    /// Line start
    pub line_start: usize,
    /// Line end
    pub line_end: usize,
    /// Signature
    pub signature: String,
    /// Docstring (optional)
    pub docstring: Option<String>,
}

/// Entry nhẹ — không chứa vector (384 f32) — dùng cho name-based query
/// Giảm ~90% dung lượng clone so với VectorEntry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntryMeta {
    pub id: String,
    pub normalized_text: String,
    pub lang_id: String,
    pub project_id: String,
    pub func_name: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
    pub docstring: Option<String>,
}

impl From<VectorEntry> for VectorEntryMeta {
    fn from(e: VectorEntry) -> Self {
        VectorEntryMeta {
            id: e.id,
            normalized_text: e.normalized_text,
            lang_id: e.lang_id,
            project_id: e.project_id,
            func_name: e.func_name,
            file_path: e.file_path,
            line_start: e.line_start,
            line_end: e.line_end,
            signature: e.signature,
            docstring: e.docstring,
        }
    }
}

/// Kết quả query từ vector DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMatch {
    pub entry: VectorEntry,
    /// Similarity score (0..1, higher = better)
    pub score: f32,
}

/// Disk-based vector storage (LanceDB / custom)
///
/// Kiến trúc:
/// - Disk: lưu toàn bộ vector trên SSD (LanceDB hoặc custom format)
/// - RAM: chỉ giữ Metadata Index (project_id → [ids], lang_id → [ids])
pub struct VectorDb {
    #[allow(dead_code)]
    db_path: PathBuf,
    /// RAM cache: lang_id → set of entry IDs
    lang_index: Arc<RwLock<std::collections::HashMap<String, Vec<String>>>>,
    /// RAM cache: project_id → set of entry IDs
    project_index: Arc<RwLock<std::collections::HashMap<String, Vec<String>>>>,
    /// Toàn bộ entries (trong RAM cho POC, sau này chuyển sang disk)
    entries: Arc<RwLock<std::collections::HashMap<String, VectorEntry>>>,
}

impl VectorDb {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            lang_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            project_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            entries: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Upsert một entry
    pub fn upsert(&self, entry: VectorEntry) {
        let id = entry.id.clone();
        let lang_id = entry.lang_id.clone();
        let project_id = entry.project_id.clone();

        // Lưu entry (chỉ thêm vào index nếu là key mới để tránh trùng lặp ID)
        let is_new = self.entries.write().insert(id.clone(), entry).is_none();

        if is_new {
            {
                let mut li = self.lang_index.write();
                let list = li.entry(lang_id).or_default();
                if !list.contains(&id) {
                    list.push(id.clone());
                }
            }
            {
                let mut pi = self.project_index.write();
                let list = pi.entry(project_id).or_default();
                if !list.contains(&id) {
                    list.push(id);
                }
            }
        }
    }

    /// Batch upsert
    pub fn upsert_batch(&self, entries: Vec<VectorEntry>) {
        for entry in entries {
            self.upsert(entry);
        }
    }

    /// Xóa entry theo id
    pub fn delete(&self, id: &str) {
        self.entries.write().remove(id);
    }

    /// Xóa toàn bộ entries của một file cụ thể trong project
    pub fn remove_file_entries(&self, project_id: &str, file_path: &str) {
        let ids_in_proj: Vec<String> = {
            let pi = self.project_index.read();
            pi.get(project_id).cloned().unwrap_or_default()
        };
        if ids_in_proj.is_empty() {
            return;
        }

        let mut removed_ids = std::collections::HashSet::new();
        {
            let mut entries = self.entries.write();
            for id in &ids_in_proj {
                if let Some(e) = entries.get(id) {
                    if e.file_path == file_path {
                        entries.remove(id);
                        removed_ids.insert(id.clone());
                    }
                }
            }
        }

        if !removed_ids.is_empty() {
            let mut pi = self.project_index.write();
            if let Some(list) = pi.get_mut(project_id) {
                list.retain(|id| !removed_ids.contains(id));
            }
        }
    }

    /// Đếm nhanh số lượng entry của một project
    pub fn count_project(&self, project_id: &str) -> usize {
        self.project_index
            .read()
            .get(project_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Xóa toàn bộ entries của một project
    pub fn delete_project(&self, project_id: &str) {
        let ids_to_remove: Vec<String> = {
            let pi = self.project_index.read();
            pi.get(project_id).cloned().unwrap_or_default()
        };

        {
            let mut entries = self.entries.write();
            for id in &ids_to_remove {
                entries.remove(id);
            }
        }

        self.project_index.write().remove(project_id);
    }

    /// Lấy entry theo id
    pub fn get(&self, id: &str) -> Option<VectorEntry> {
        self.entries.read().get(id).cloned()
    }

    /// Lấy tất cả entries (bao gồm vector — nặng)
    pub fn all_entries(&self) -> Vec<VectorEntry> {
        self.entries.read().values().cloned().collect()
    }

    /// Lấy entries theo lang_id (bao gồm vector)
    pub fn by_language(&self, lang_id: &str) -> Vec<VectorEntry> {
        let ids: Vec<String> = {
            let li = self.lang_index.read();
            li.get(lang_id).cloned().unwrap_or_default()
        };

        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| entries.get(id).cloned())
            .collect()
    }

    /// Lấy entries theo project_id (bao gồm vector)
    pub fn by_project(&self, project_id: &str) -> Vec<VectorEntry> {
        let ids: Vec<String> = {
            let pi = self.project_index.read();
            pi.get(project_id).cloned().unwrap_or_default()
        };

        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| entries.get(id).cloned())
            .collect()
    }

    // ── Meta-only methods (không clone vector) ──

    /// Lấy tất cả entries dạng meta (KHÔNG có vector) — NHẸ
    pub fn all_entries_meta(&self) -> Vec<VectorEntryMeta> {
        self.entries
            .read()
            .values()
            .map(|e| {
                let entry: &VectorEntry = e;
                VectorEntryMeta {
                    id: entry.id.clone(),
                    normalized_text: entry.normalized_text.clone(),
                    lang_id: entry.lang_id.clone(),
                    project_id: entry.project_id.clone(),
                    func_name: entry.func_name.clone(),
                    file_path: entry.file_path.clone(),
                    line_start: entry.line_start,
                    line_end: entry.line_end,
                    signature: entry.signature.clone(),
                    docstring: entry.docstring.clone(),
                }
            })
            .collect()
    }

    /// Lấy entries theo lang_id dạng meta (KHÔNG có vector)
    pub fn by_language_meta(&self, lang_id: &str) -> Vec<VectorEntryMeta> {
        let ids: Vec<String> = {
            let li = self.lang_index.read();
            li.get(lang_id).cloned().unwrap_or_default()
        };

        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| {
                entries.get(id).map(|e| VectorEntryMeta {
                    id: e.id.clone(),
                    normalized_text: e.normalized_text.clone(),
                    lang_id: e.lang_id.clone(),
                    project_id: e.project_id.clone(),
                    func_name: e.func_name.clone(),
                    file_path: e.file_path.clone(),
                    line_start: e.line_start,
                    line_end: e.line_end,
                    signature: e.signature.clone(),
                    docstring: e.docstring.clone(),
                })
            })
            .collect()
    }

    /// Lấy entries theo project_id dạng meta (KHÔNG có vector)
    pub fn by_project_meta(&self, project_id: &str) -> Vec<VectorEntryMeta> {
        let ids: Vec<String> = {
            let pi = self.project_index.read();
            pi.get(project_id).cloned().unwrap_or_default()
        };

        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| {
                entries.get(id).map(|e| VectorEntryMeta {
                    id: e.id.clone(),
                    normalized_text: e.normalized_text.clone(),
                    lang_id: e.lang_id.clone(),
                    project_id: e.project_id.clone(),
                    func_name: e.func_name.clone(),
                    file_path: e.file_path.clone(),
                    line_start: e.line_start,
                    line_end: e.line_end,
                    signature: e.signature.clone(),
                    docstring: e.docstring.clone(),
                })
            })
            .collect()
    }

    /// Brute-force cosine similarity search (cho POC).
    /// Sau này thay bằng ANN index (LanceDB).
    pub fn search(
        &self,
        query_vector: &[f32],
        lang_filter: Option<&str>,
        project_filter: Option<&str>,
        top_k: usize,
    ) -> Vec<QueryMatch> {
        let candidates = match (lang_filter, project_filter) {
            (Some(lang), _) => self.by_language(lang),
            (_, Some(proj)) => self.by_project(proj),
            (None, None) => self.all_entries(),
        };

        let mut scored: Vec<QueryMatch> = candidates
            .into_iter()
            .filter_map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector)?;
                Some(QueryMatch { entry, score })
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }

    /// Zero-copy Hybrid Search (Semantic cosine + Keyword/Symbol/Path boost)
    /// Không clone toàn bộ VectorEntry khi chấm điểm; chỉ clone đúng `top_k` kết quả tốt nhất.
    pub fn search_hybrid(
        &self,
        query_vector: &[f32],
        query_text: &str,
        lang_filter: Option<&str>,
        project_filter: Option<&str>,
        top_k: usize,
        model_loaded: bool,
    ) -> (Vec<QueryMatch>, usize) {
        let query_lower = query_text.to_lowercase();
        let keywords: Vec<String> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| t.len() >= 3)
            .map(|t| t.to_string())
            .collect();

        let candidate_ids: Vec<String> = match (lang_filter, project_filter) {
            (Some(lang), Some(proj)) => {
                let li = self.lang_index.read();
                let pi = self.project_index.read();
                let proj_set: std::collections::HashSet<&str> = pi
                    .get(proj)
                    .map(|v| v.iter().map(|s| s.as_str()).collect())
                    .unwrap_or_default();
                li.get(lang)
                    .map(|v| {
                        v.iter()
                            .filter(|id| proj_set.contains(id.as_str()))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default()
            }
            (Some(lang), None) => {
                let li = self.lang_index.read();
                let mut ids = li.get(lang).cloned().unwrap_or_default();
                if let Some(common) = li.get("common") {
                    ids.extend(common.iter().cloned());
                }
                ids
            }
            (None, Some(proj)) => self
                .project_index
                .read()
                .get(proj)
                .cloned()
                .unwrap_or_default(),
            (None, None) => self.entries.read().keys().cloned().collect(),
        };

        let entries = self.entries.read();
        let mut candidates_count = 0usize;
        let mut scored_refs: Vec<(&VectorEntry, f32)> = Vec::with_capacity(candidate_ids.len().min(256));

        for id in &candidate_ids {
            let Some(entry) = entries.get(id) else {
                continue;
            };
            candidates_count += 1;
            let Some(mut score) = cosine_similarity(query_vector, &entry.vector) else {
                continue;
            };

            if !keywords.is_empty() {
                let fn_lower = entry.func_name.to_lowercase();
                let path_lower = entry.file_path.to_lowercase();
                let sig_lower = entry.signature.to_lowercase();
                let mut hits = 0usize;
                for kw in &keywords {
                    if fn_lower.contains(kw) {
                        hits += 2;
                    } else if path_lower.contains(kw) || sig_lower.contains(kw) {
                        hits += 1;
                    }
                }
                if hits > 0 {
                    let boost = if model_loaded {
                        (hits as f32 * 0.06).min(0.24)
                    } else {
                        (hits as f32 * 0.2).min(0.8) + 0.3
                    };
                    score += boost;
                }
            }

            scored_refs.push((entry, score));
        }

        scored_refs.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored_refs.truncate(top_k);

        let matches = scored_refs
            .into_iter()
            .map(|(entry, score)| QueryMatch {
                entry: entry.clone(),
                score,
            })
            .collect();

        (matches, candidates_count)
    }

    fn cache_file_for_project(&self, project_id: &str) -> PathBuf {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(project_id.as_bytes());
        let hash_hex = hex::encode(hasher.finalize());
        self.db_path.join(format!("proj_{}.rag.gz", &hash_hex[..16]))
    }

    /// Lưu cache index + mtimes của project xuống disk (nén gzip) để lần mở sau không cần re-embed
    pub fn save_project_cache(
        &self,
        project_id: &str,
        file_mtimes: &std::collections::HashMap<String, (u64, u64)>,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct ProjectCacheRef<'a> {
            project_id: &'a str,
            file_mtimes: &'a std::collections::HashMap<String, (u64, u64)>,
            entries: Vec<&'a VectorEntry>,
        }

        if let Err(e) = std::fs::create_dir_all(&self.db_path) {
            return Err(format!("Cannot create cache dir: {}", e));
        }

        let ids: Vec<String> = self
            .project_index
            .read()
            .get(project_id)
            .cloned()
            .unwrap_or_default();
        let entries_guard = self.entries.read();
        let proj_entries: Vec<&VectorEntry> =
            ids.iter().filter_map(|id| entries_guard.get(id)).collect();

        let payload = ProjectCacheRef {
            project_id,
            file_mtimes,
            entries: proj_entries,
        };

        let cache_path = self.cache_file_for_project(project_id);
        let file = std::fs::File::create(&cache_path)
            .map_err(|e| format!("Failed to create cache file: {}", e))?;
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        serde_json::to_writer(&mut encoder, &payload)
            .map_err(|e| format!("Failed to serialize cache: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Failed to finish gzip cache: {}", e))?;
        Ok(())
    }

    /// Nạp cache index + mtimes của project từ disk nếu có
    pub fn load_project_cache(
        &self,
        project_id: &str,
    ) -> Option<std::collections::HashMap<String, (u64, u64)>> {
        #[derive(Deserialize)]
        struct ProjectCacheOwned {
            project_id: String,
            file_mtimes: std::collections::HashMap<String, (u64, u64)>,
            entries: Vec<VectorEntry>,
        }

        let cache_path = self.cache_file_for_project(project_id);
        if !cache_path.exists() {
            return None;
        }
        let file = std::fs::File::open(&cache_path).ok()?;
        let decoder = flate2::read::GzDecoder::new(file);
        let cached: ProjectCacheOwned = serde_json::from_reader(decoder).ok()?;
        if cached.project_id != project_id {
            return None;
        }

        self.upsert_batch(cached.entries);
        Some(cached.file_mtimes)
    }

    /// Tổng số entries
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Cosine similarity giữa 2 vector
fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() {
        return None;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }

    Some(dot / (norm_a * norm_b))
}

/// Convert FunctionEntry → VectorEntry
impl From<(FunctionEntry, Vec<f32>)> for VectorEntry {
    fn from((entry, vector): (FunctionEntry, Vec<f32>)) -> Self {
        VectorEntry {
            id: entry.id,
            vector,
            normalized_text: entry.normalized_text,
            lang_id: entry.lang_id,
            project_id: entry.project_id,
            func_name: entry.func_name,
            file_path: entry.file_path,
            line_start: entry.line_start,
            line_end: entry.line_end,
            signature: entry.signature,
            docstring: entry.docstring,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, lang: &str, proj: &str, name: &str) -> VectorEntry {
        VectorEntry {
            id: id.to_string(),
            vector: vec![0.1, 0.2, 0.3],
            normalized_text: format!("[{}] {}", lang.to_uppercase(), name),
            lang_id: lang.to_string(),
            project_id: proj.to_string(),
            func_name: name.to_string(),
            file_path: format!("src/{}.rs", name),
            line_start: 0,
            line_end: 5,
            signature: format!("fn {}()", name),
            docstring: None,
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let db = VectorDb::new(PathBuf::from("/tmp/test_db"));
        let entry = make_entry("p1:main.rs:hello", "rust", "p1", "hello");
        db.upsert(entry.clone());
        assert_eq!(db.len(), 1);
        assert!(db.get("p1:main.rs:hello").is_some());
    }

    #[test]
    fn test_search_by_language() {
        let db = VectorDb::new(PathBuf::from("/tmp/test_db"));
        db.upsert(make_entry("1", "rust", "p1", "hello"));
        db.upsert(make_entry("2", "python", "p1", "world"));
        db.upsert(make_entry("3", "rust", "p2", "foo"));

        let rust_entries = db.by_language("rust");
        assert_eq!(rust_entries.len(), 2);
    }

    #[test]
    fn test_delete_project() {
        let db = VectorDb::new(PathBuf::from("/tmp/test_db"));
        db.upsert(make_entry("1", "rust", "p1", "hello"));
        db.upsert(make_entry("2", "python", "p1", "world"));
        db.upsert(make_entry("3", "rust", "p2", "foo"));

        db.delete_project("p1");
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let c = vec![1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b).unwrap(), 0.0);
        assert!((cosine_similarity(&a, &c).unwrap() - 1.0).abs() < 1e-6);
    }
}
