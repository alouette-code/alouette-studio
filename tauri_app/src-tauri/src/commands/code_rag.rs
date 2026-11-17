use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

use core_engine::code_rag::{
    all_languages, embedding::EmbeddingModel, extension_map, extract_functions,
    language_resolver::LanguageResolver, seed_code_library, IndexEvent, Indexer, IndexerConfig,
    QueryEngine, QueryResult, VectorDb,
};

/// Kết quả kiểm tra nhanh DB
#[derive(serde::Serialize)]
pub struct CodeRagHealth {
    pub has_data: bool,
    pub total_entries: usize,
    pub model_loaded: bool,
}

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

    // 🌱 Seed thư viện code mẫu từ file seed_library.json (nằm ở core_engine/app_data/db_RAG/)
    // app_data_dir hiện tại trỏ vào tauri_app/app_data/, cần lùi về project root rồi sang core_engine
    let project_root = app_data_dir.parent().unwrap_or(app_data_dir);
    let seed_path = project_root
        .join("core_engine")
        .join("app_data")
        .join("db_RAG")
        .join("seed_library.json");
    seed_code_library(&db, &seed_path);

    let config = IndexerConfig::default();
    let indexer = Arc::new(Indexer::new(db.clone(), config));

    let query_engine = Arc::new(QueryEngine::new(db.clone()));

    // Start background worker (indexer) — chạy ngay
    let idx = indexer.clone();
    tauri::async_runtime::spawn(async move {
        idx.run().await;
    });

    // Load embedding model MỘT LẦN DUY NHẤT trong BACKGROUND và chia sẻ Arc weights
    // giữa QueryEngine và Indexer (giảm 50% RAM ~150MB & 50% thời gian khởi động).
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

        eprintln!("[CodeRAG] [Background] Loading shared embedding model...");
        let loaded = tokio::task::spawn_blocking(move || EmbeddingModel::load(&model_dir)).await;
        match loaded {
            Ok(Ok(embed)) => {
                qe.set_model(embed.clone());
                idx2.set_embedding_model(embed);
                eprintln!("[CodeRAG] ✅ [Background] Single shared embedding model loaded for QueryEngine + Indexer!");
            }
            Ok(Err(e)) => {
                eprintln!("[CodeRAG] ⚠️ [Background] Failed to load model: {}", e);
                eprintln!("[CodeRAG] Code search will use fallback hash embedding.");
            }
            Err(e) => {
                eprintln!("[CodeRAG] ⚠️ [Background] Model load task panicked: {}", e);
            }
        }
    });

    CodeRagState {
        db,
        indexer,
        query_engine,
    }
}

/// Tạo ngữ cảnh dự án kiểu Cursor (Incremental Index + Hybrid Semantic RAG + Repo Map)
/// để bơm nhẹ vào system context của OpenCode mà không xung đột với tool built-in.
pub fn build_cursor_project_context(
    code_rag: &Arc<CodeRagState>,
    workspace_dir: &str,
    user_query: &str,
) -> (String, String) {
    let base_path = Path::new(workspace_dir);
    if !base_path.exists() || !base_path.is_dir() {
        return (String::new(), String::new());
    }

    // 1. Đồng bộ tăng dần cực nhanh (tối đa 1200ms nếu là lần đầu mở project)
    let sync = code_rag
        .indexer
        .sync_project_incremental(base_path, workspace_dir, 1500, 1200);

    // Nếu project lần đầu quá lớn vượt budget 1.2s, chạy nốt phần còn lại ở background
    if sync.partial_remaining > 0 {
        let idx_bg = code_rag.indexer.clone();
        let dir_bg = workspace_dir.to_string();
        std::thread::spawn(move || {
            idx_bg.sync_project_incremental(Path::new(&dir_bg), &dir_bg, 2500, 15_000);
        });
    }

    // 2. Hybrid Semantic + Keyword Query
    let query_res = code_rag
        .query_engine
        .query_hybrid(user_query, None, Some(workspace_dir), 8);

    // 3. Repo Map tinh gọn
    let repo_map = code_rag.indexer.build_repo_map(base_path, 45);

    if repo_map.is_empty() && query_res.matches.is_empty() {
        return (String::new(), String::new());
    }

    let mut ctx = String::with_capacity(2048);
    ctx.push_str("<alouette_project_context>\n");
    ctx.push_str(&format!("Workspace root: {}\n", workspace_dir));

    if !repo_map.is_empty() {
        ctx.push_str("Key workspace files (Repo Map):\n");
        for rel in &repo_map {
            ctx.push_str("- ");
            ctx.push_str(rel);
            ctx.push('\n');
        }
    }

    let relevant: Vec<_> = query_res
        .matches
        .iter()
        .filter(|m| m.score >= 0.20)
        .take(8)
        .collect();

    if !relevant.is_empty() {
        ctx.push_str("\nTop semantically relevant symbols for the user's request:\n");
        for m in &relevant {
            let rel_path = Path::new(&m.entry.file_path)
                .strip_prefix(base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| m.entry.file_path.clone());
            let sig = m.entry.signature.lines().next().unwrap_or(&m.entry.func_name);
            let sig_short: String = sig.chars().take(120).collect();
            ctx.push_str(&format!(
                "- {}:{}-{} | {} (score {:.2})\n",
                rel_path, m.entry.line_start, m.entry.line_end, sig_short, m.score
            ));
        }
    }

    ctx.push_str(
        "\nGuidance: Use these pre-indexed paths and line ranges to jump directly with `read` instead of scanning the entire repository. Always verify exact file contents before editing.\n",
    );
    ctx.push_str("</alouette_project_context>");

    let log_summary = format!(
        "[CONTEXT] Đồng bộ {} files (cache hit: {}, mới/sửa: {}) trong {}ms | Gợi ý {} vị trí liên quan ({}ms)",
        sync.total_scanned_files,
        sync.cache_hits,
        sync.reindexed_files,
        sync.elapsed_ms,
        relevant.len(),
        query_res.elapsed_ms
    );

    (ctx, log_summary)
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

/// Kiểm tra nhanh tình trạng DB (có data không? model đã load chưa?)
/// KHÔNG block — chỉ đọc vài atomic/refcount.
#[tauri::command]
pub async fn code_rag_health(
    state: State<'_, Arc<CodeRagState>>,
) -> Result<CodeRagHealth, String> {
    Ok(CodeRagHealth {
        has_data: state.db.len() > 0,
        total_entries: state.db.len(),
        model_loaded: state.query_engine.is_model_loaded(),
    })
}

/// Query function definitions bằng text search (semantic)
#[tauri::command]
pub async fn code_rag_query(
    state: State<'_, Arc<CodeRagState>>,
    query: String,
    lang_id: Option<String>,
    project_id: Option<String>,
    top_k: Option<usize>,
) -> Result<QueryResult, String> {
    let top_k = top_k.unwrap_or(10);
    let qe = state.query_engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        qe.query_hybrid(&query, lang_id.as_deref(), project_id.as_deref(), top_k)
    })
    .await
    .map_err(|e| format!("Query task failed: {}", e))?;

    Ok(result)
}

/// Query function definitions bằng tên function — SIÊU NHANH
#[tauri::command]
pub fn code_rag_query_by_name(
    state: State<'_, Arc<CodeRagState>>,
    name: String,
    lang_id: Option<String>,
    project_id: Option<String>,
    top_k: Option<usize>,
) -> Vec<serde_json::Value> {
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
    state: State<'_, Arc<CodeRagState>>,
    path: String,
    project_id: String,
) -> Result<(), String> {
    state.indexer.push_event(IndexEvent::FileChanged {
        path: PathBuf::from(&path),
        project_id,
    });
    Ok(())
}

/// Trigger re-index toàn bộ project
#[tauri::command]
pub fn code_rag_rescan_project(
    state: State<'_, Arc<CodeRagState>>,
    project_id: String,
    base_path: String,
) -> Result<(), String> {
    state.indexer.push_event(IndexEvent::ProjectRescanned {
        project_id,
        base_path: PathBuf::from(&base_path),
    });
    Ok(())
}

/// Scan và index đồng bộ một thư mục project.
#[tauri::command]
pub fn code_rag_scan_directory(
    state: State<'_, Arc<CodeRagState>>,
    project_id: String,
    base_path: String,
) -> Result<u64, String> {
    eprintln!(
        "[CodeRAG] Incremental scan directory: {:?} for project {}",
        base_path, project_id
    );
    let sync = state
        .indexer
        .sync_project_incremental(Path::new(&base_path), &project_id, 3000, 30_000);
    let total = state.db.count_project(&project_id) as u64;
    eprintln!(
        "[CodeRAG] Scan done in {}ms: {} hits, {} reindexed, {} total functions",
        sync.elapsed_ms, sync.cache_hits, sync.reindexed_files, total
    );
    Ok(total)
}

/// Xóa index của một project
#[tauri::command]
pub fn code_rag_delete_project(
    state: State<'_, Arc<CodeRagState>>,
    project_id: String,
) -> Result<(), String> {
    state.db.delete_project(&project_id);
    Ok(())
}

/// Lấy thống kê indexer
#[tauri::command]
pub fn code_rag_stats(state: State<'_, Arc<CodeRagState>>) -> serde_json::Value {
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
pub fn code_rag_debug(state: State<'_, Arc<CodeRagState>>) -> serde_json::Value {
    use std::collections::HashMap;
    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    for entry in state.db.all_entries() {
        *lang_counts.entry(entry.lang_id.clone()).or_default() += 1;
    }

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
