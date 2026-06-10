use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use crate::code_rag::db::{QueryMatch, VectorDb, VectorEntry, VectorEntryMeta};
use crate::code_rag::EmbeddingModel;

/// Kết quả query trả về cho frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Kết quả tìm được
    pub matches: Vec<QueryMatch>,
    /// Thời gian thực hiện (ms)
    pub elapsed_ms: u64,
    /// Số lượng candidates đã search
    pub candidates_count: usize,
    /// Có lỗi không
    pub error: Option<String>,
}

/// Query Engine: real-time tìm kiếm function
///
/// Luồng:
///   1. Nhận query text + language_id (từ editor context)
///   2. Embedding query → vector
///   3. Filtered ANN Search trên Vector DB
///   4. Fetch snippet → trả về
pub struct QueryEngine {
    db: Arc<VectorDb>,
    /// Embedding model (BGE-small-en-v1.5 ONNX)
    model: Arc<RwLock<Option<EmbeddingModel>>>,
}

impl QueryEngine {
    pub fn new(db: Arc<VectorDb>) -> Self {
        Self {
            db,
            model: Arc::new(RwLock::new(None)),
        }
    }

    /// Khởi tạo embedding model từ thư mục chứa model.onnx + tokenizer.json
    ///
    /// # Arguments
    /// * `model_dir` - Thư mục chứa model files (model.onnx, tokenizer.json)
    pub fn load_model(&self, model_dir: &Path) -> Result<(), String> {
        let model = EmbeddingModel::load(model_dir)?;
        eprintln!("[CodeRAG] Embedding model loaded successfully (384-dim)");
        *self.model.write() = Some(model);
        Ok(())
    }

    /// Kiểm tra model đã được load chưa
    pub fn is_model_loaded(&self) -> bool {
        self.model.read().is_some()
    }

    /// Query chính: gọi từ frontend
    ///
    /// # Arguments
    /// * `query` - Text query (tên function hoặc description)
    /// * `lang_filter` - Optional: chỉ search trong ngôn ngữ này
    /// * `project_filter` - Optional: chỉ search trong project này
    /// * `top_k` - Số kết quả tối đa
    pub fn query(
        &self,
        query: &str,
        lang_filter: Option<&str>,
        project_filter: Option<&str>,
        top_k: usize,
    ) -> QueryResult {
        let start = std::time::Instant::now();

        // Embedding query text → vector
        let query_vector = match self.embed(query) {
            Ok(v) => v,
            Err(e) => {
                return QueryResult {
                    matches: vec![],
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    candidates_count: 0,
                    error: Some(e),
                };
            }
        };

        // Search trong DB
        let candidates = match (lang_filter, project_filter) {
            (Some(lang), _) => self.db.by_language(lang),
            (_, Some(proj)) => self.db.by_project(proj),
            (None, None) => self.db.all_entries(),
        };

        let candidates_count = candidates.len();

        // Score từng candidate
        let mut scored: Vec<QueryMatch> = candidates
            .into_iter()
            .filter_map(|entry| {
                let score = cosine_similarity(&query_vector, &entry.vector)?;
                Some(QueryMatch { entry, score })
            })
            .collect();

        // Sort theo score giảm dần
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);

        let elapsed_ms = start.elapsed().as_millis() as u64;

        QueryResult {
            matches: scored,
            elapsed_ms,
            candidates_count,
            error: None,
        }
    }

    /// Query nhanh bằng function name — trả về VectorEntry (bao gồm vector)
    /// Dùng cho semantic search. Không dùng cho autocomplete gõ phím.
    pub fn query_by_name(
        &self,
        name: &str,
        lang_filter: Option<&str>,
        project_filter: Option<&str>,
        top_k: usize,
    ) -> Vec<VectorEntry> {
        let candidates = match (lang_filter, project_filter) {
            (Some(lang), _) => self.db.by_language(lang),
            (_, Some(proj)) => self.db.by_project(proj),
            (None, None) => self.db.all_entries(),
        };

        let name_lower = name.to_lowercase();
        let mut matched: Vec<VectorEntry> = candidates
            .into_iter()
            .filter(|e| e.func_name.to_lowercase().contains(&name_lower))
            .collect();

        matched.sort_by(|a, b| a.func_name.len().cmp(&b.func_name.len()));
        matched.truncate(top_k);
        matched
    }

    /// Query NHANH bằng function name — trả về VectorEntryMeta (KHÔNG có vector)
    /// Dùng cho autocomplete real-time. NHẸ hơn ~90% so với query_by_name.
    pub fn query_by_name_meta(
        &self,
        name: &str,
        lang_filter: Option<&str>,
        project_filter: Option<&str>,
        top_k: usize,
    ) -> Vec<VectorEntryMeta> {
        // Dùng meta methods — không clone vector 384-dim
        let candidates = match (lang_filter, project_filter) {
            (Some(lang), _) => self.db.by_language_meta(lang),
            (_, Some(proj)) => self.db.by_project_meta(proj),
            (None, None) => self.db.all_entries_meta(),
        };

        let name_lower = name.to_lowercase();
        let mut matched: Vec<VectorEntryMeta> = candidates
            .into_iter()
            .filter(|e| e.func_name.to_lowercase().contains(&name_lower))
            .collect();

        matched.sort_by(|a, b| a.func_name.len().cmp(&b.func_name.len()));
        matched.truncate(top_k);
        matched
    }

    /// Embedding text → vector
    /// Dùng BGE-small-en-v1.5 ONNX nếu model đã load, fallback về dummy vector
    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let model = self.model.read();
        if let Some(ref model) = *model {
            model.embed(text)
        } else {
            // Fallback: dummy vector (khi chưa download model)
            eprintln!(
                "[CodeRAG] WARNING: No embedding model loaded. Using fallback hash embedding."
            );
            eprintln!("[CodeRAG] Run: bash scripts/download_embedding_model.sh");
            Ok(dummy_embedding(text))
        }
    }
}

/// Dummy embedding: hash-based vector cho POC / fallback
/// Public vì được dùng trong indexer và các module khác
pub fn dummy_embedding(text: &str) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();

    // Tạo vector 384 chiều từ hash
    let mut vec = Vec::with_capacity(384);
    for i in 0..384 {
        let val = ((hash.wrapping_add(i as u64 * 7)) % 1000) as f32 / 1000.0;
        vec.push(val);
    }

    // Normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}

/// Cosine similarity
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup_db() -> Arc<VectorDb> {
        let db = Arc::new(VectorDb::new(PathBuf::from("/tmp/test_query")));
        use crate::code_rag::db::VectorEntry;

        db.upsert(VectorEntry {
            id: "1".into(),
            vector: dummy_embedding("calculate sum"),
            normalized_text: "[PYTHON] def calculate_sum(a, b) -> int".into(),
            lang_id: "python".into(),
            project_id: "p1".into(),
            func_name: "calculate_sum".into(),
            file_path: "math.py".into(),
            line_start: 0,
            line_end: 3,
            signature: "def calculate_sum(a, b) -> int".into(),
            docstring: Some("Sum two numbers".into()),
        });
        db.upsert(VectorEntry {
            id: "2".into(),
            vector: dummy_embedding("greet user"),
            normalized_text: "[RUST] fn greet(name: &str) -> String".into(),
            lang_id: "rust".into(),
            project_id: "p1".into(),
            func_name: "greet".into(),
            file_path: "lib.rs".into(),
            line_start: 5,
            line_end: 8,
            signature: "fn greet(name: &str) -> String".into(),
            docstring: None,
        });
        db
    }

    #[test]
    fn test_query_by_name() {
        let db = setup_db();
        let engine = QueryEngine::new(db);
        let results = engine.query_by_name("calculate", Some("python"), None, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].func_name, "calculate_sum");
    }

    #[test]
    fn test_query_by_name_no_filter() {
        let db = setup_db();
        let engine = QueryEngine::new(db);
        let results = engine.query_by_name("greet", None, None, 5);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_dummy_embedding() {
        let v1 = dummy_embedding("hello world");
        let v2 = dummy_embedding("hello world");
        let v3 = dummy_embedding("different text");
        assert_eq!(v1.len(), 384);
        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }
}
