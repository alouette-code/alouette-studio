use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use core_engine::code_rag::{
    all_languages, embedding::EmbeddingModel, extension_map, extract_functions,
    language_resolver::LanguageResolver, IndexEvent, Indexer, IndexerConfig, QueryEngine,
    QueryResult, VectorDb,
};

/// AppState mở rộng cho Code RAG
pub struct CodeRagState {
    pub db: Arc<VectorDb>,
    pub indexer: Arc<Indexer>,
    pub query_engine: Arc<QueryEngine>,
}

/// Khởi tạo Code RAG system
///
/// QUAN TRỌNG: Không block UI. Model embedding được load sau 1 giây ở background.
/// App vào được ngay, model xuất hiện sau ~2-3 giây.
pub fn init_code_rag(app_data_dir: &PathBuf) -> CodeRagState {
    let db_path = app_data_dir.join("code_rag_db");
    let db = Arc::new(VectorDb::new(db_path));

    let config = IndexerConfig::default();
    let indexer = Arc::new(Indexer::new(db.clone(), config));

    let query_engine = Arc::new(QueryEngine::new(db.clone()));

    // Start background worker (indexer) — chạy ngay
    let idx = indexer.clone();
    tauri::async_runtime::spawn(async move {
        idx.run().await;
    });

    // Load embedding model trong BACKGROUND — không block app startup
    let model_dir = app_data_dir.join("model_embedding/bge-small-en-v1.5");
    let qe = query_engine.clone();
    let idx2 = indexer.clone();
    tauri::async_runtime::spawn(async move {
        // Đợi 1.5 giây để app kịp render UI trước
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        if !model_dir.join("model.onnx").exists() {
            eprintln!("[CodeRAG] ⚠️ Model not found at {:?}. Run: bash scripts/download_embedding_model.sh", model_dir);
            eprintln!("[CodeRAG] Code search will use fallback hash embedding.");
            return;
        }

        eprintln!("[CodeRAG] [Background] Loading embedding model...");
        match qe.load_model(&model_dir) {
            Ok(()) => {
                eprintln!("[CodeRAG] ✅ [Background] Model loaded for QueryEngine!");
                // Load riêng cho Indexer (EmbeddingModel không Clone được)
                match EmbeddingModel::load(&model_dir) {
                    Ok(embed) => {
                        idx2.set_embedding_model(embed);
                        eprintln!("[CodeRAG] ✅ [Background] Model shared with Indexer!");
                    }
                    Err(e) => {
                        eprintln!("[CodeRAG] ⚠️ [Background] Indexer model failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[CodeRAG] ⚠️ [Background] Failed to load model: {}", e);
                eprintln!("[CodeRAG] Code search will use fallback hash embedding.");
            }
        }
    });

    CodeRagState {
        db,
        indexer,
        query_engine,
    }
}

/// Lấy danh sách ngôn ngữ được hỗ trợ
#[tauri::command]
pub fn code_rag_supported_languages() -> Vec<serde_json::Value> {
    all_languages()
        .into_iter()
        .map(|cfg| {
            serde_json::json!({
                "lang_id": cfg.lang_id,
                "display_name": cfg.display_name,
                "tier": format!("{:?}", cfg.tier),
                "extensions": cfg.extensions,
            })
        })
        .collect()
}

/// Lấy mapping extension → language
#[tauri::command]
pub fn code_rag_extension_map() -> std::collections::HashMap<String, String> {
    extension_map()
}

/// Query function definitions bằng text search (semantic)
#[tauri::command]
pub fn code_rag_query(
    state: State<'_, Mutex<CodeRagState>>,
    query: String,
    lang_id: Option<String>,
    project_id: Option<String>,
    top_k: Option<usize>,
) -> QueryResult {
    let state = state.blocking_lock();
    let top_k = top_k.unwrap_or(10);
    state
        .query_engine
        .query(&query, lang_id.as_deref(), project_id.as_deref(), top_k)
}

/// Query function definitions bằng tên function — SIÊU NHANH
/// Dùng VectorEntryMeta (không clone vector 384-dim) cho autocomplete real-time
/// NHẸ hơn ~90% so với phiên bản cũ (không copy 1.5KB vector mỗi entry)
#[tauri::command]
pub fn code_rag_query_by_name(
    state: State<'_, Mutex<CodeRagState>>,
    name: String,
    lang_id: Option<String>,
    project_id: Option<String>,
    top_k: Option<usize>,
) -> Vec<serde_json::Value> {
    let state = state.blocking_lock();
    let top_k = top_k.unwrap_or(10);

    state
        .query_engine
        .query_by_name_meta(&name, lang_id.as_deref(), project_id.as_deref(), top_k)
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "func_name": e.func_name,
                "signature": e.signature,
                "docstring": e.docstring,
                "file_path": e.file_path,
                "lang_id": e.lang_id,
                "project_id": e.project_id,
                "line_start": e.line_start,
                "line_end": e.line_end,
                "normalized_text": e.normalized_text,
            })
        })
        .collect()
}

/// Trigger index một file
#[tauri::command]
pub fn code_rag_index_file(
    state: State<'_, Mutex<CodeRagState>>,
    path: String,
    project_id: String,
) -> Result<(), String> {
    let state = state.blocking_lock();
    state.indexer.push_event(IndexEvent::FileChanged {
        path: PathBuf::from(&path),
        project_id,
    });
    Ok(())
}

/// Trigger re-index toàn bộ project
#[tauri::command]
pub fn code_rag_rescan_project(
    state: State<'_, Mutex<CodeRagState>>,
    project_id: String,
    base_path: String,
) -> Result<(), String> {
    let state = state.blocking_lock();
    state.indexer.push_event(IndexEvent::ProjectRescanned {
        project_id,
        base_path: PathBuf::from(&base_path),
    });
    Ok(())
}

/// Scan và index đồng bộ một thư mục project.
/// Gọi từ frontend khi user mở project hoặc bấm nút "Scan".
#[tauri::command]
pub fn code_rag_scan_directory(
    state: State<'_, Mutex<CodeRagState>>,
    project_id: String,
    base_path: String,
) -> Result<u64, String> {
    let state = state.blocking_lock();
    eprintln!(
        "[CodeRAG] Scanning directory: {:?} for project {}",
        base_path, project_id
    );
    let count = state
        .indexer
        .scan_and_index_project(&PathBuf::from(&base_path), &project_id)?;
    eprintln!("[CodeRAG] Scan done: {} functions indexed", count);
    Ok(count)
}

/// Xóa index của một project
#[tauri::command]
pub fn code_rag_delete_project(
    state: State<'_, Mutex<CodeRagState>>,
    project_id: String,
) -> Result<(), String> {
    let state = state.blocking_lock();
    state.db.delete_project(&project_id);
    Ok(())
}

/// Lấy thống kê indexer
#[tauri::command]
pub fn code_rag_stats(state: State<'_, Mutex<CodeRagState>>) -> serde_json::Value {
    let state = state.blocking_lock();
    let stats = state.indexer.stats();
    serde_json::json!({
        "total_files_indexed": stats.total_files_indexed,
        "total_functions_extracted": stats.total_functions_extracted,
        "total_errors": stats.total_errors,
        "total_entries": state.db.len(),
    })
}

/// Debug: dump thông tin DB
#[tauri::command]
pub fn code_rag_debug(state: State<'_, Mutex<CodeRagState>>) -> serde_json::Value {
    let state = state.blocking_lock();

    // Đếm entries theo lang_id
    use std::collections::HashMap;
    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    for entry in state.db.all_entries() {
        *lang_counts.entry(entry.lang_id.clone()).or_default() += 1;
    }

    // Lấy 5 entries đầu tiên để kiểm tra
    let samples: Vec<serde_json::Value> = state
        .db
        .all_entries()
        .into_iter()
        .take(5)
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "func_name": e.func_name,
                "lang_id": e.lang_id,
                "file_path": e.file_path,
            })
        })
        .collect();

    serde_json::json!({
        "total_entries": state.db.len(),
        "by_language": lang_counts,
        "samples": samples,
    })
}

/// Resolve ngôn ngữ cho một file path
#[tauri::command]
pub fn code_rag_resolve_language(file_path: String, content: String) -> Option<serde_json::Value> {
    let cfg = LanguageResolver::resolve(std::path::Path::new(&file_path), &content)?;
    Some(serde_json::json!({
        "lang_id": cfg.lang_id,
        "display_name": cfg.display_name,
        "tier": format!("{:?}", cfg.tier),
    }))
}

/// Trích xuất function signatures từ code string
#[tauri::command]
pub fn code_rag_extract_functions(
    content: String,
    file_path: String,
    project_id: String,
    lang_id: String,
) -> Vec<serde_json::Value> {
    let lang_map = core_engine::code_rag::languages::language_map();
    let Some(cfg) = lang_map.get(&lang_id) else {
        return vec![];
    };

    let result = extract_functions(&content, &file_path, &project_id, cfg);
    result
        .entries
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "func_name": entry.func_name,
                "signature": entry.signature,
                "docstring": entry.docstring,
                "file_path": entry.file_path,
                "line_start": entry.line_start,
                "line_end": entry.line_end,
                "normalized_text": entry.normalized_text,
            })
        })
        .collect()
}
