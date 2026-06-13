/// List supported programming languages for code RAG
pub fn code_rag_supported_languages() -> Vec<String> {
    core_engine::code_rag::all_languages()
        .into_iter()
        .map(|l| l.display_name)
        .collect()
}

/// Get file extension to language mapping
pub fn code_rag_extension_map() -> std::collections::HashMap<String, String> {
    core_engine::code_rag::extension_map()
}

/// Check code RAG system health
pub fn code_rag_health() -> String {
    format!(
        "Code RAG active. Languages: {}",
        code_rag_supported_languages().join(", ")
    )
}
