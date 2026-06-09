use regex::Regex;
use crate::code_rag::tier::{Tier, LanguageConfig};
use crate::code_rag::normalizer::{FunctionEntry, make_entry_id, normalize_entry};
use std::path::Path;

/// Kết quả extract từ 1 file
pub struct ExtractionResult {
    pub entries: Vec<FunctionEntry>,
    pub file_path: String,
    pub lang_id: String,
}

/// Trích xuất function signatures từ code.
///
/// Luồng:
/// 1. Nếu Tier 1 và feature tree-sitter → dùng Tree-sitter queries
/// 2. Nếu có fallback_regex → dùng Regex
/// 3. Không có gì → skip
pub fn extract_functions(
    content: &str,
    file_path: &str,
    project_id: &str,
    lang_cfg: &LanguageConfig,
) -> ExtractionResult {
    let entries = match lang_cfg.tier {
        Tier::Tier1 => {
            #[cfg(feature = "tree-sitter")]
            {
                extract_with_treesitter(content, file_path, project_id, lang_cfg)
            }
            #[cfg(not(feature = "tree-sitter"))]
            {
                extract_with_regex(content, file_path, project_id, lang_cfg)
            }
        }
        Tier::Tier2 => {
            #[cfg(feature = "tree-sitter")]
            {
                extract_signature_only_treesitter(content, file_path, project_id, lang_cfg)
            }
            #[cfg(not(feature = "tree-sitter"))]
            {
                extract_with_regex(content, file_path, project_id, lang_cfg)
            }
        }
        Tier::Tier3 => {
            extract_with_regex(content, file_path, project_id, lang_cfg)
        }
    };

    ExtractionResult {
        entries,
        file_path: file_path.to_string(),
        lang_id: lang_cfg.lang_id.clone(),
    }
}

/// Extract bằng Regex (dùng cho Tier 3 và fallback)
fn extract_with_regex(
    content: &str,
    file_path: &str,
    project_id: &str,
    lang_cfg: &LanguageConfig,
) -> Vec<FunctionEntry> {
    let Some(pattern) = &lang_cfg.fallback_regex else {
        return vec![];
    };

    let Ok(re) = Regex::new(pattern) else {
        return vec![];
    };

    let mut entries = Vec::new();
    // Ưu tiên các dòng không phải comment
    let comment_prefix = lang_cfg.comment_prefix.as_deref().unwrap_or("");

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Bỏ qua dòng comment
        if !comment_prefix.is_empty() && trimmed.starts_with(comment_prefix) {
            continue;
        }

        if let Some(caps) = re.captures(trimmed) {
            // Lấy group đầu tiên match được
            let func_name = (1..caps.len())
                .find_map(|i| caps.get(i).filter(|m| !m.as_str().is_empty()))
                .map(|m| m.as_str().to_string());

            if let Some(name) = func_name {
                let signature = trimmed.to_string();
                let normalized = normalize_entry(lang_cfg, &signature, None);

                entries.push(FunctionEntry {
                    id: make_entry_id(project_id, file_path, &name),
                    lang_id: lang_cfg.lang_id.clone(),
                    func_name: name,
                    signature,
                    docstring: None,
                    file_path: file_path.to_string(),
                    line_start: line_idx,
                    line_end: line_idx,
                    project_id: project_id.to_string(),
                    normalized_text: normalized,
                });
            }
        }
    }

    entries
}

/// Extract bằng Tree-sitter (Tier 1: full signature + docstring)
/// Chỉ compile khi feature "tree-sitter" được bật
#[cfg(feature = "tree-sitter")]
fn extract_with_treesitter(
    content: &str,
    file_path: &str,
    project_id: &str,
    lang_cfg: &LanguageConfig,
) -> Vec<FunctionEntry> {
    // Sử dụng crate code_rag_parser (module parser) để parse
    // Tạm thời fallback về regex cho tới khi parser module hoàn thiện
    extract_with_regex(content, file_path, project_id, lang_cfg)
}

/// Extract bằng Tree-sitter (Tier 2: signature only)
#[cfg(feature = "tree-sitter")]
fn extract_signature_only_treesitter(
    content: &str,
    file_path: &str,
    project_id: &str,
    lang_cfg: &LanguageConfig,
) -> Vec<FunctionEntry> {
    extract_with_regex(content, file_path, project_id, lang_cfg)
}

/// Lấy docstring phía trên function (phát hiện comment block)
/// Dùng cho Tier 1 khi cần docstring
#[cfg(feature = "tree-sitter")]
fn extract_docstring(content: &str, func_line: usize, comment_prefix: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if func_line == 0 {
        return None;
    }

    let mut doc_lines: Vec<&str> = Vec::new();
    // Đi ngược từ func_line - 1 để tìm comment
    for i in (0..func_line).rev() {
        let line = lines[i].trim();
        if line.is_empty() {
            // Dòng trống ngăn cách → dừng
            if !doc_lines.is_empty() {
                break;
            }
            continue;
        }
        if line.starts_with(comment_prefix) {
            doc_lines.push(line);
        } else {
            break;
        }
    }

    doc_lines.reverse();
    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

/// FileFilter: kiểm tra file có nên được index không
pub fn should_index_file(file_path: &Path, content: &[u8]) -> bool {
    use crate::code_rag::language_resolver::LanguageResolver;

    // Bỏ qua file binary
    if LanguageResolver::is_binary(content) {
        return false;
    }

    // Bỏ qua file quá lớn (>512KB)
    if LanguageResolver::is_too_large(content, 512 * 1024) {
        return false;
    }

    // Bỏ qua dotfiles và node_modules
    if let Some(file_name) = file_path.file_name().and_then(|s| s.to_str()) {
        if file_name.starts_with('.') && file_name != ".env" && file_name != ".gitignore" {
            return false;
        }
    }

    // Bỏ qua thư mục không cần index
    let path_str = file_path.to_string_lossy().to_lowercase();
    let skip_dirs = [
        "node_modules", ".git", "target", "build", "dist", ".next",
        "__pycache__", "vendor", ".venv", "venv", "env", ".tox",
        ".eggs", "*.egg-info", ".gradle", "idea", ".vscode", ".tauri",
    ];
    for dir in &skip_dirs {
        if path_str.contains(dir) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_rag::languages::language_map;

    #[test]
    fn test_extract_python_functions() {
        let code = r#"
def hello(name):
    print(f"Hello {name}")

def add(a, b) -> int:
    return a + b

class Foo:
    def bar(self):
        pass
"#;
        let cfg = language_map().get("python").unwrap().clone();
        let result = extract_functions(code, "test.py", "proj1", &cfg);
        assert!(result.entries.len() >= 2, "Expected >=2 functions, got {}", result.entries.len());
        assert_eq!(result.entries[0].func_name, "hello");
    }

    #[test]
    fn test_extract_rust_functions() {
        let code = r#"
pub fn greet(name: &str) -> String {
    format!("Hello {}", name)
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let cfg = language_map().get("rust").unwrap().clone();
        let result = extract_functions(code, "lib.rs", "proj1", &cfg);
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].func_name, "greet");
    }

    #[test]
    fn test_skip_comments() {
        let code = r#"
# this is a comment
# not a function
def real():
    pass
"#;
        let cfg = language_map().get("python").unwrap().clone();
        let result = extract_functions(code, "test.py", "p1", &cfg);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].func_name, "real");
    }

    #[test]
    fn test_tier3_fortran() {
        let code = r#"
      subroutine init_grid
      integer i
      end subroutine

      function compute(x)
      real :: x
      end function
"#;
        let cfg = language_map().get("fortran").unwrap().clone();
        let result = extract_functions(code, "test.f90", "p1", &cfg);
        assert_eq!(result.entries.len(), 2);
    }

    #[test]
    fn test_should_index_skip_binary() {
        let path = Path::new("test.bin");
        let binary = vec![0x00, 0x01, 0x02];
        assert!(!should_index_file(path, &binary));
    }

    #[test]
    fn test_should_index_skip_node_modules() {
        let path = Path::new("node_modules/express/index.js");
        let content = b"hello";
        assert!(!should_index_file(path, content));
    }

    #[test]
    fn test_should_index_normal_file() {
        let path = Path::new("src/main.rs");
        let content = b"fn main() {}";
        assert!(should_index_file(path, content));
    }
}
