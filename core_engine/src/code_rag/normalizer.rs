use crate::code_rag::tier::{LanguageConfig, Tier};

/// Kết quả extract function (đã chuẩn hóa)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionEntry {
    /// ID duy nhất: project_id:file_path:func_name
    pub id: String,
    /// Mã ngôn ngữ
    pub lang_id: String,
    /// Tên function
    pub func_name: String,
    /// Signature đầy đủ (ví dụ: "def foo(a, b) -> int")
    pub signature: String,
    /// Docstring (chỉ Tier 1)
    pub docstring: Option<String>,
    /// Đường dẫn file
    pub file_path: String,
    /// Dòng bắt đầu (0-indexed)
    pub line_start: usize,
    /// Dòng kết thúc
    pub line_end: usize,
    /// Project ID
    pub project_id: String,
    /// Text đã chuẩn hóa để embedding
    pub normalized_text: String,
}

/// Chuẩn hóa dữ liệu để đưa vào vector DB.
///
/// Format:
/// - Tier 1: `[LANG_ID] SIGNATURE | DOCSTRING`
/// - Tier 2 & 3: `[LANG_ID] SIGNATURE`
///
/// Model Embedding chỉ học 1 pattern duy nhất này.
pub fn normalize_entry(
    lang_cfg: &LanguageConfig,
    signature: &str,
    docstring: Option<&str>,
) -> String {
    let lang_tag = format!("[{}]", lang_cfg.lang_id.to_uppercase());

    match lang_cfg.tier {
        Tier::Tier1 => {
            let doc = docstring.unwrap_or("");
            let clean_doc = clean_docstring(doc, lang_cfg.comment_prefix.as_deref());
            if clean_doc.is_empty() {
                format!("{} {}", lang_tag, signature.trim())
            } else {
                format!("{} {} | {}", lang_tag, signature.trim(), clean_doc)
            }
        }
        Tier::Tier2 | Tier::Tier3 => {
            format!("{} {}", lang_tag, signature.trim())
        }
    }
}

/// Làm sạch docstring: bỏ comment prefix, trim
fn clean_docstring(doc: &str, comment_prefix: Option<&str>) -> String {
    let prefix = comment_prefix.unwrap_or("");
    let lines: Vec<&str> = doc
        .lines()
        .map(|l| {
            let trimmed = l.trim();
            if !prefix.is_empty() && trimmed.starts_with(prefix) {
                trimmed[prefix.len()..].trim()
            } else {
                trimmed
            }
        })
        .filter(|l| !l.is_empty())
        .collect();

    // Giới hạn docstring ở 200 ký tự (tránh noise)
    let joined = lines.join(" ");
    if joined.len() > 200 {
        format!("{}...", &joined[..197])
    } else {
        joined
    }
}

/// Tạo ID duy nhất cho FunctionEntry
pub fn make_entry_id(project_id: &str, file_path: &str, func_name: &str) -> String {
    format!("{}:{}:{}", project_id, file_path, func_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_rag::tier::{LanguageConfig, Tier};

    fn make_cfg(lang_id: &str, tier: Tier) -> LanguageConfig {
        LanguageConfig {
            lang_id: lang_id.into(),
            display_name: lang_id.into(),
            tier,
            extensions: vec![],
            tree_sitter_grammar: None,
            fallback_regex: None,
            comment_prefix: None,
            project_config_file: None,
        }
    }

    #[test]
    fn test_normalize_tier1_with_docstring() {
        let cfg = make_cfg("python", Tier::Tier1);
        let result = normalize_entry(
            &cfg,
            "def foo(a, b) -> int",
            Some("Calculate sum of two numbers"),
        );
        assert!(result.contains("[PYTHON]"));
        assert!(result.contains("def foo(a, b) -> int"));
        assert!(result.contains("Calculate sum of two numbers"));
        assert!(result.contains("|"));
    }

    #[test]
    fn test_normalize_tier1_no_docstring() {
        let cfg = make_cfg("rust", Tier::Tier1);
        let result = normalize_entry(&cfg, "fn foo(a: i32) -> i32", None);
        assert!(result.contains("[RUST]"));
        assert!(result.contains("fn foo(a: i32) -> i32"));
        assert!(!result.contains("|"));
    }

    #[test]
    fn test_normalize_tier2_signature_only() {
        let cfg = make_cfg("lua", Tier::Tier2);
        let result = normalize_entry(&cfg, "function foo(a, b)", Some("ignore docstring"));
        assert!(result.contains("[LUA]"));
        assert!(result.contains("function foo(a, b)"));
        assert!(!result.contains("|"));
        assert!(!result.contains("ignore"));
    }

    #[test]
    fn test_make_entry_id() {
        let id = make_entry_id("proj1", "src/main.py", "calculate");
        assert_eq!(id, "proj1:src/main.py:calculate");
    }
}
