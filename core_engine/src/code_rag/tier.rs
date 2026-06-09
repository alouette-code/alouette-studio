use serde::{Deserialize, Serialize};

/// Tier hỗ trợ ngôn ngữ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Tier {
    /// Full: Signature + Docstring + Return Type (Tree-sitter queries riêng)
    Tier1,
    /// Signature Only: Tên hàm + Tham số (Tree-sitter queries cơ bản)
    Tier2,
    /// Fallback Regex: Bắt tên hàm top-level bằng regex
    Tier3,
}

/// Cấu hình cho một ngôn ngữ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Mã ngôn ngữ (ví dụ: "python", "rust")
    pub lang_id: String,
    /// Tên hiển thị
    pub display_name: String,
    /// Tier hỗ trợ
    pub tier: Tier,
    /// Danh sách extension (ví dụ: ["py", "pyw"])
    pub extensions: Vec<String>,
    /// Tên grammar Tree-sitter (nếu có)
    pub tree_sitter_grammar: Option<String>,
    /// Regex fallback cho Tier 3 (bắt tên hàm)
    pub fallback_regex: Option<String>,
    /// Ký tự comment (để lọc docstring)
    pub comment_prefix: Option<String>,
    /// File config đặc trưng của project
    pub project_config_file: Option<Vec<String>>,
}
