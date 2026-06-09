use crate::code_rag::languages::{all_languages, extension_map, language_map};
use crate::code_rag::tier::LanguageConfig;
use std::path::Path;

/// Smart Language Resolver: config → shebang → extension → fallback
pub struct LanguageResolver;

impl LanguageResolver {
    /// Giải quyết ngôn ngữ cho một file path.
    /// Luồng:
    ///   1. Kiểm tra project config file (.clangd, Cargo.toml, package.json,...)
    ///   2. Đọc shebang từ nội dung file
    ///   3. Fallback theo extension
    pub fn resolve(file_path: &Path, content: &str) -> Option<LanguageConfig> {
        // 1. Extension-based resolution (nhanh nhất)
        if let Some(cfg) = Self::from_extension(file_path) {
            let path_str = file_path.to_string_lossy();
            if path_str.ends_with(".rs") {
                eprintln!(
                    "[CodeRAG] resolve: .rs file resolved to lang={}",
                    cfg.lang_id
                );
            }
            return Some(cfg);
        }

        // Debug: log khi extension resolve thất bại
        eprintln!("[CodeRAG] resolve: extension failed for {:?}", file_path);

        // 2. Shebang check (file không có extension hoặc extension generic)
        if let Some(cfg) = Self::from_shebang(content) {
            return Some(cfg);
        }

        // 3. Check Dockerfile (filename = Dockerfile)
        if let Some(file_name) = file_path.file_name().and_then(|s| s.to_str()) {
            if file_name == "Dockerfile" || file_name.starts_with("Dockerfile.") {
                if let Some(cfg) = language_map().get("dockerfile") {
                    return Some(cfg.clone());
                }
            }
            if file_name == "Makefile" || file_name == "makefile" {
                if let Some(cfg) = language_map().get("makefile") {
                    return Some(cfg.clone());
                }
            }
        }

        eprintln!(
            "[CodeRAG] resolve: could not resolve language for {:?}",
            file_path
        );
        None
    }

    /// Resolve theo extension
    pub fn from_extension(file_path: &Path) -> Option<LanguageConfig> {
        let ext = file_path.extension()?.to_str()?.to_lowercase();
        let map = extension_map();
        let lang_id = map.get(&ext)?;
        let lang_map = language_map();
        let cfg = lang_map.get(lang_id).cloned();
        if cfg.is_none() {
            eprintln!(
                "[CodeRAG] from_extension: ext={:?} lang_id={:?} but config not found",
                ext, lang_id
            );
        }
        cfg
    }

    /// Resolve theo shebang (`#!/usr/bin/env python`)
    pub fn from_shebang(content: &str) -> Option<LanguageConfig> {
        let first_line = content.lines().next()?.trim();
        if !first_line.starts_with("#!") {
            return None;
        }

        let shebang = first_line[2..].trim().to_lowercase();
        // Map shebang → lang_id
        let shebang_map = [
            ("python", "python"),
            ("python3", "python"),
            ("node", "javascript"),
            ("nodejs", "javascript"),
            ("deno", "javascript"),
            ("bun", "javascript"),
            ("ruby", "ruby"),
            ("perl", "perl"),
            ("lua", "lua"),
            ("bash", "shell"),
            ("sh", "shell"),
            ("zsh", "shell"),
            ("fish", "shell"),
            ("racket", "racket"),
            ("guile", "scheme"),
            ("swift", "swift"),
            ("ocaml", "ocaml"),
            ("tclsh", "tcl"),
            ("wish", "tcl"),
            ("sbcl", "lisp"),
            ("clisp", "lisp"),
        ];

        for (keyword, lang_id) in &shebang_map {
            if shebang.contains(keyword) {
                let lang_map = language_map();
                return lang_map.get(*lang_id).cloned();
            }
        }

        None
    }

    /// Kiểm tra file có phải là file nhị phân không (bỏ qua)
    pub fn is_binary(content: &[u8]) -> bool {
        // Kiểm tra null bytes - dấu hiệu của file binary
        if content.len() > 1024 {
            let sample = &content[..1024];
            sample.contains(&0x00)
        } else {
            content.contains(&0x00)
        }
    }

    /// Kiểm tra file có phải là file quá lớn (>1MB)
    pub fn is_too_large(content: &[u8], max_bytes: usize) -> bool {
        content.len() > max_bytes
    }

    /// Lấy tất cả ngôn ngữ hỗ trợ
    pub fn supported_languages() -> Vec<LanguageConfig> {
        all_languages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_py_extension() {
        let cfg = LanguageResolver::resolve(Path::new("main.py"), "").unwrap();
        assert_eq!(cfg.lang_id, "python");
    }

    #[test]
    fn test_resolve_rs_extension() {
        let cfg = LanguageResolver::resolve(Path::new("lib.rs"), "").unwrap();
        assert_eq!(cfg.lang_id, "rust");
    }

    #[test]
    fn test_resolve_from_shebang() {
        let content = "#!/usr/bin/env python3\nprint('hello')";
        let cfg = LanguageResolver::resolve(Path::new("script"), content).unwrap();
        assert_eq!(cfg.lang_id, "python");
    }

    #[test]
    fn test_resolve_dockerfile() {
        let cfg = LanguageResolver::resolve(Path::new("Dockerfile"), "").unwrap();
        assert_eq!(cfg.lang_id, "dockerfile");
    }

    #[test]
    fn test_is_binary() {
        let binary = vec![0x00, 0x01, 0x02];
        assert!(LanguageResolver::is_binary(&binary));
        let text = b"hello world\n";
        assert!(!LanguageResolver::is_binary(text));
    }

    #[test]
    fn test_is_too_large() {
        let small = vec![0u8; 100];
        assert!(!LanguageResolver::is_too_large(&small, 1024));
        let large = vec![0u8; 2048];
        assert!(LanguageResolver::is_too_large(&large, 1024));
    }
}
