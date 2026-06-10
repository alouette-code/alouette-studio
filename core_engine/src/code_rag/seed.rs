use crate::code_rag::db::VectorEntry;
use crate::code_rag::query::dummy_embedding;
use crate::code_rag::tier::{LanguageConfig, Tier};
use crate::code_rag::{normalize_entry, VectorDb};
use std::path::Path;
use std::sync::Arc;

/// Seed the RAG database from a pre-built code snippet library JSON file.
///
/// Reads `seed_library.json` from disk, creates vector entries,
/// and upserts them into the VectorDb.
///
/// # Arguments
/// * `db` - The VectorDb to seed
/// * `json_path` - Path to `seed_library.json`
pub fn seed_code_library(db: &Arc<VectorDb>, json_path: &Path) -> usize {
    if !json_path.exists() {
        eprintln!(
            "[CodeRAG] ⚠️ Seed library not found at {:?}. Skipping.",
            json_path
        );
        return 0;
    }

    // Đọc file JSON
    let content = match std::fs::read_to_string(json_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[CodeRAG] ⚠️ Failed to read seed library: {}", e);
            return 0;
        }
    };

    let data: Vec<serde_json::Value> = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[CodeRAG] ⚠️ Failed to parse seed JSON: {}", e);
            return 0;
        }
    };

    if data.is_empty() {
        eprintln!("[CodeRAG] ⚠️ Seed library is empty!");
        return 0;
    }

    let mut count = 0;
    let mut seen = std::collections::HashSet::new();

    for entry in &data {
        let lang_id = entry["lang_id"].as_str().unwrap_or("common");
        let func_name = entry["func_name"].as_str().unwrap_or("unknown");
        let signature = entry["snippet"]
            .as_str()
            .or_else(|| entry["signature"].as_str())
            .unwrap_or("");
        let docstring = entry["docstring"].as_str();
        let file_path = entry["file_path"].as_str().unwrap_or("");

        // Skip entries without valid content
        if func_name == "unknown" || signature.len() < 5 {
            continue;
        }

        // Dedup by function name + language
        let dedup_key = format!("{}:{}", lang_id, func_name);
        if seen.contains(&dedup_key) {
            continue;
        }
        seen.insert(dedup_key);

        // Use first line of snippet as signature if available
        let sig = signature.lines().next().unwrap_or(signature);

        let id = format!("seed:{}:{}", lang_id, func_name);
        let project_id = "_seed_";
        let cfg = LanguageConfig {
            lang_id: lang_id.to_string(),
            display_name: lang_id.to_string(),
            tier: Tier::Tier1,
            extensions: vec![],
            tree_sitter_grammar: None,
            fallback_regex: None,
            comment_prefix: None,
            project_config_file: None,
        };

        let normalized_text = normalize_entry(&cfg, sig, docstring);
        let vector = dummy_embedding(&normalized_text);

        let vec_entry = VectorEntry {
            id,
            vector,
            normalized_text,
            lang_id: lang_id.to_string(),
            project_id: project_id.to_string(),
            func_name: func_name.to_string(),
            file_path: file_path.to_string(),
            line_start: 0,
            line_end: 0,
            signature: sig.to_string(),
            docstring: docstring.map(|s| s.to_string()),
        };
        db.upsert(vec_entry);
        count += 1;
    }

    eprintln!(
        "[CodeRAG] 🌱 Seeded {} code snippets into library (from {} languages)",
        count,
        count_languages(&data)
    );
    count
}

fn count_languages(data: &[serde_json::Value]) -> usize {
    let mut langs = std::collections::HashSet::new();
    for entry in data {
        if let Some(l) = entry["lang_id"].as_str() {
            langs.insert(l);
        }
    }
    langs.len()
}
