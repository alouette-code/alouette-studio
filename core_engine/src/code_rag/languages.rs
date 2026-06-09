use crate::code_rag::tier::{LanguageConfig, Tier};
use std::collections::HashMap;

/// Danh sách tất cả ngôn ngữ được hỗ trợ (50+ languages)
pub fn all_languages() -> Vec<LanguageConfig> {
    let mut list = Vec::new();

    // ─────────── TIER 1 (13 languages) ───────────
    list.push(LanguageConfig {
        lang_id: "python".into(),
        display_name: "Python".into(),
        tier: Tier::Tier1,
        extensions: vec!["py", "pyw"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("python".into()),
        fallback_regex: Some(r"^\s*(?:async\s+)?def\s+(\w+)\s*\(".into()),
        comment_prefix: Some("#".into()),
        project_config_file: Some(
            vec!["setup.py", "pyproject.toml", "requirements.txt"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });
    list.push(LanguageConfig {
        lang_id: "javascript".into(),
        display_name: "JavaScript".into(),
        tier: Tier::Tier1,
        extensions: vec!["js", "mjs", "cjs", "jsx"]
            .into_iter()
            .map(String::from)
            .collect(),
        tree_sitter_grammar: Some("javascript".into()),
        fallback_regex: Some(r"(?:function\s+(\w+)|const\s+(\w+)\s*=\s*(?:async\s*)?\()".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(vec!["package.json"].into_iter().map(String::from).collect()),
    });
    list.push(LanguageConfig {
        lang_id: "typescript".into(),
        display_name: "TypeScript".into(),
        tier: Tier::Tier1,
        extensions: vec!["ts", "tsx", "mts", "cts"]
            .into_iter()
            .map(String::from)
            .collect(),
        tree_sitter_grammar: Some("typescript".into()),
        fallback_regex: Some(r"(?:function\s+(\w+)|const\s+(\w+)\s*=\s*(?:async\s*)?\()".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(
            vec!["tsconfig.json"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });
    list.push(LanguageConfig {
        lang_id: "java".into(),
        display_name: "Java".into(),
        tier: Tier::Tier1,
        extensions: vec!["java"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("java".into()),
        fallback_regex: Some(r"(?:public|private|protected|static|\s)\s+\w+\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(
            vec!["pom.xml", "build.gradle", "build.gradle.kts"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });
    list.push(LanguageConfig {
        lang_id: "c".into(),
        display_name: "C".into(),
        tier: Tier::Tier1,
        extensions: vec!["c", "h"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("c".into()),
        fallback_regex: Some(r"^\s*\w+\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(
            vec!["Makefile", "CMakeLists.txt"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });
    list.push(LanguageConfig {
        lang_id: "cpp".into(),
        display_name: "C++".into(),
        tier: Tier::Tier1,
        extensions: vec!["cpp", "cc", "cxx", "hpp", "hh", "hxx"]
            .into_iter()
            .map(String::from)
            .collect(),
        tree_sitter_grammar: Some("cpp".into()),
        fallback_regex: Some(r"^\s*\w+\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(
            vec!["CMakeLists.txt"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });
    list.push(LanguageConfig {
        lang_id: "csharp".into(), display_name: "C#".into(), tier: Tier::Tier1,
        extensions: vec!["cs"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("csharp".into()),
        fallback_regex: Some(r"(?:public|private|protected|internal|static|virtual|override|async)\s+\w+\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(vec![".csproj", "*.sln"].into_iter().map(String::from).collect()),
    });
    list.push(LanguageConfig {
        lang_id: "go".into(),
        display_name: "Go".into(),
        tier: Tier::Tier1,
        extensions: vec!["go"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("go".into()),
        fallback_regex: Some(r"^\s*func\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(vec!["go.mod"].into_iter().map(String::from).collect()),
    });
    list.push(LanguageConfig {
        lang_id: "rust".into(),
        display_name: "Rust".into(),
        tier: Tier::Tier1,
        extensions: vec!["rs"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("rust".into()),
        fallback_regex: Some(r"^\s*(?:pub\s+(?:unsafe\s+)?)?fn\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(vec!["Cargo.toml"].into_iter().map(String::from).collect()),
    });
    list.push(LanguageConfig {
        lang_id: "php".into(),
        display_name: "PHP".into(),
        tier: Tier::Tier1,
        extensions: vec!["php"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("php".into()),
        fallback_regex: Some(
            r"(?:function\s+(\w+)|(?:public|private|protected)\s+function\s+(\w+))".into(),
        ),
        comment_prefix: Some("//".into()),
        project_config_file: Some(
            vec!["composer.json"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });
    list.push(LanguageConfig {
        lang_id: "ruby".into(),
        display_name: "Ruby".into(),
        tier: Tier::Tier1,
        extensions: vec!["rb"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("ruby".into()),
        fallback_regex: Some(r"^\s*(?:def\s+(?:self\.)?(\w+)|def\s+(\w+))".into()),
        comment_prefix: Some("#".into()),
        project_config_file: Some(vec!["Gemfile"].into_iter().map(String::from).collect()),
    });
    list.push(LanguageConfig {
        lang_id: "swift".into(), display_name: "Swift".into(), tier: Tier::Tier1,
        extensions: vec!["swift"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("swift".into()),
        fallback_regex: Some(r"^\s*(?:public|private|internal|fileprivate|static|class|struct|enum|func)\s+(\w+)\s*\(".into()),
        comment_prefix: Some("//".into()),
        project_config_file: Some(vec!["Package.swift"].into_iter().map(String::from).collect()),
    });
    list.push(LanguageConfig {
        lang_id: "kotlin".into(),
        display_name: "Kotlin".into(),
        tier: Tier::Tier1,
        extensions: vec!["kt", "kts"].into_iter().map(String::from).collect(),
        tree_sitter_grammar: Some("kotlin".into()),
        fallback_regex: Some(
            r"^\s*(?:fun\s+(\w+)|(?:public|private|internal)\s+fun\s+(\w+))".into(),
        ),
        comment_prefix: Some("//".into()),
        project_config_file: Some(
            vec!["build.gradle.kts"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    });

    // ─────────── TIER 2 (13 languages) ───────────
    for cfg in tier2_languages() {
        list.push(cfg);
    }

    // ─────────── TIER 3 (25+ languages) ───────────
    for cfg in tier3_languages() {
        list.push(cfg);
    }

    list
}

fn tier2_languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig {
            lang_id: "lua".into(),
            display_name: "Lua".into(),
            tier: Tier::Tier2,
            extensions: vec!["lua"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("lua".into()),
            fallback_regex: Some(r"^\s*function\s+(\w+(?:\.\w+)*)\s*\(".into()),
            comment_prefix: Some("--".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "r".into(),
            display_name: "R".into(),
            tier: Tier::Tier2,
            extensions: vec!["r", "R"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("r".into()),
            fallback_regex: Some(r"^\s*(\w+)\s*<-\s*function\s*\(".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "perl".into(),
            display_name: "Perl".into(),
            tier: Tier::Tier2,
            extensions: vec!["pl", "pm"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("perl".into()),
            fallback_regex: Some(r"^\s*sub\s+(\w+)".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "scala".into(),
            display_name: "Scala".into(),
            tier: Tier::Tier2,
            extensions: vec!["scala", "sc"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("scala".into()),
            fallback_regex: Some(r"^\s*def\s+(\w+)\s*\(".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "dart".into(),
            display_name: "Dart".into(),
            tier: Tier::Tier2,
            extensions: vec!["dart"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("dart".into()),
            fallback_regex: Some(r"^\s*(?:\w+\s+)?(\w+)\s*\(".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "elixir".into(),
            display_name: "Elixir".into(),
            tier: Tier::Tier2,
            extensions: vec!["ex", "exs"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("elixir".into()),
            fallback_regex: Some(r"^\s*def(?:p|macro)?\s+(\w+)\s*\(".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "erlang".into(),
            display_name: "Erlang".into(),
            tier: Tier::Tier2,
            extensions: vec!["erl", "hrl"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("erlang".into()),
            fallback_regex: Some(r"^\s*(\w+)\s*\([^)]*\)\s*->".into()),
            comment_prefix: Some("%".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "clojure".into(),
            display_name: "Clojure".into(),
            tier: Tier::Tier2,
            extensions: vec!["clj", "cljs", "cljc", "edn"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: Some("clojure".into()),
            fallback_regex: Some(r"^\s*\(defn\s+(\w+)".into()),
            comment_prefix: Some(";".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "julia".into(),
            display_name: "Julia".into(),
            tier: Tier::Tier2,
            extensions: vec!["jl"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("julia".into()),
            fallback_regex: Some(r"^\s*function\s+(\w+)\s*\(".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "haskell".into(),
            display_name: "Haskell".into(),
            tier: Tier::Tier2,
            extensions: vec!["hs", "lhs"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("haskell".into()),
            fallback_regex: Some(r"^\s*(\w+)\s*::".into()),
            comment_prefix: Some("--".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "ocaml".into(),
            display_name: "OCaml".into(),
            tier: Tier::Tier2,
            extensions: vec!["ml", "mli"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("ocaml".into()),
            fallback_regex: Some(r"^\s*let\s+(\w+)\s".into()),
            comment_prefix: Some("(*".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "sql".into(),
            display_name: "SQL".into(),
            tier: Tier::Tier2,
            extensions: vec!["sql"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("sql".into()),
            fallback_regex: Some(
                r"(?:CREATE\s+(?:OR\s+REPLACE\s+)?(?:FUNCTION|PROCEDURE)\s+(\w+))".into(),
            ),
            comment_prefix: Some("--".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "zig".into(),
            display_name: "Zig".into(),
            tier: Tier::Tier2,
            extensions: vec!["zig"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: Some("zig".into()),
            fallback_regex: Some(r"^\s*(?:pub\s+)?fn\s+(\w+)\s*\(".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
    ]
}

fn tier3_languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig {
            lang_id: "assembly".into(),
            display_name: "Assembly".into(),
            tier: Tier::Tier3,
            extensions: vec!["asm", "s", "S"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^(\w+):".into()),
            comment_prefix: Some(";".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "cobol".into(),
            display_name: "COBOL".into(),
            tier: Tier::Tier3,
            extensions: vec!["cob", "cbl"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*\d+\s+(\w+)\s+SECTION".into()),
            comment_prefix: Some("*".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "fortran".into(),
            display_name: "Fortran".into(),
            tier: Tier::Tier3,
            extensions: vec!["f", "for", "f90", "f95"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*(?:subroutine|function)\s+(\w+)".into()),
            comment_prefix: Some("!".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "prolog".into(),
            display_name: "Prolog".into(),
            tier: Tier::Tier3,
            extensions: vec!["pl", "pro", "P"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^(\w+)\s*\(".into()),
            comment_prefix: Some("%".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "ada".into(),
            display_name: "Ada".into(),
            tier: Tier::Tier3,
            extensions: vec!["ada", "ads", "adb"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*(?:procedure|function)\s+(\w+)".into()),
            comment_prefix: Some("--".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "lisp".into(),
            display_name: "Lisp".into(),
            tier: Tier::Tier3,
            extensions: vec!["lisp", "lsp", "cl"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*\(defun\s+(\w+)".into()),
            comment_prefix: Some(";".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "scheme".into(),
            display_name: "Scheme".into(),
            tier: Tier::Tier3,
            extensions: vec!["scm", "ss"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*\(define\s+\((\w+)".into()),
            comment_prefix: Some(";".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "groovy".into(),
            display_name: "Groovy".into(),
            tier: Tier::Tier3,
            extensions: vec!["groovy", "gvy"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*(?:def|void|int|String|boolean)\s+(\w+)\s*\(".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "matlab".into(),
            display_name: "MATLAB".into(),
            tier: Tier::Tier3,
            extensions: vec!["m"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*function\s+(?:\w+\s*=\s*)?(\w+)\s*\(".into()),
            comment_prefix: Some("%".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "crystal".into(),
            display_name: "Crystal".into(),
            tier: Tier::Tier3,
            extensions: vec!["cr"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*(?:def|macro|fun)\s+(\w+)".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "nim".into(),
            display_name: "Nim".into(),
            tier: Tier::Tier3,
            extensions: vec!["nim"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*(?:proc|func|method|template|macro)\s+(\w+)".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "racket".into(),
            display_name: "Racket".into(),
            tier: Tier::Tier3,
            extensions: vec!["rkt"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*\(define\s+\((\w+)".into()),
            comment_prefix: Some(";".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "tcl".into(),
            display_name: "Tcl".into(),
            tier: Tier::Tier3,
            extensions: vec!["tcl"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*proc\s+(\w+)\s".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "vhdl".into(),
            display_name: "VHDL".into(),
            tier: Tier::Tier3,
            extensions: vec!["vhd", "vhdl"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"ENTITY\s+(\w+)".into()),
            comment_prefix: Some("--".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "verilog".into(),
            display_name: "Verilog".into(),
            tier: Tier::Tier3,
            extensions: vec!["v", "vh", "sv"]
                .into_iter()
                .map(String::from)
                .collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*module\s+(\w+)".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "powershell".into(),
            display_name: "PowerShell".into(),
            tier: Tier::Tier3,
            extensions: vec!["ps1", "psm1"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*function\s+(\w+(?:-\w+)*)".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "batch".into(),
            display_name: "Batch".into(),
            tier: Tier::Tier3,
            extensions: vec!["bat", "cmd"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r":(\w+)".into()),
            comment_prefix: Some("REM".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "fsharp".into(),
            display_name: "F#".into(),
            tier: Tier::Tier3,
            extensions: vec!["fs", "fsx"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*let\s+(\w+)\s".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "objectivec".into(),
            display_name: "Objective-C".into(),
            tier: Tier::Tier3,
            extensions: vec!["m", "mm"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^[+-]\s*\([\w\s*]+\)\s*(\w+)".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "solidity".into(),
            display_name: "Solidity".into(),
            tier: Tier::Tier3,
            extensions: vec!["sol"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*function\s+(\w+)\s*\(".into()),
            comment_prefix: Some("//".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "yaml".into(),
            display_name: "YAML".into(),
            tier: Tier::Tier3,
            extensions: vec!["yml", "yaml"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: None,
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "toml".into(),
            display_name: "TOML".into(),
            tier: Tier::Tier3,
            extensions: vec!["toml"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: None,
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "dockerfile".into(),
            display_name: "Dockerfile".into(),
            tier: Tier::Tier3,
            extensions: vec!["dockerfile"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(
                r"(?:FROM|RUN|CMD|ENTRYPOINT|COPY|ADD|ENV|WORKDIR)\s+(\S+)".into(),
            ),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "cmake".into(),
            display_name: "CMake".into(),
            tier: Tier::Tier3,
            extensions: vec!["cmake"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^\s*(?:function|macro)\s+\(?\s*(\w+)".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
        LanguageConfig {
            lang_id: "makefile".into(),
            display_name: "Makefile".into(),
            tier: Tier::Tier3,
            extensions: vec!["mk"].into_iter().map(String::from).collect(),
            tree_sitter_grammar: None,
            fallback_regex: Some(r"^(\w+):".into()),
            comment_prefix: Some("#".into()),
            project_config_file: None,
        },
    ]
}

/// HashMap<lang_id, LanguageConfig>
pub fn language_map() -> HashMap<String, LanguageConfig> {
    all_languages()
        .into_iter()
        .map(|cfg| (cfg.lang_id.clone(), cfg))
        .collect()
}

/// HashMap<extension, lang_id>
pub fn extension_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for cfg in all_languages() {
        for ext in cfg.extensions {
            map.insert(ext.to_lowercase(), cfg.lang_id.clone());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier1_count() {
        let n = all_languages()
            .iter()
            .filter(|l| l.tier == Tier::Tier1)
            .count();
        assert!(n >= 13, "Tier 1: expected >=13, got {n}");
    }

    #[test]
    fn test_total_count() {
        let n = all_languages().len();
        assert!(n >= 50, "Total: expected >=50, got {n}");
    }

    #[test]
    fn test_no_dup_extension() {
        let mut seen = std::collections::HashSet::new();
        for (ext, _) in extension_map() {
            assert!(seen.insert(ext.clone()), "Duplicate extension: {ext}");
        }
    }

    #[test]
    fn test_no_dup_lang_id() {
        let mut seen = std::collections::HashSet::new();
        for lang in all_languages() {
            assert!(
                seen.insert(lang.lang_id.clone()),
                "Duplicate: {}",
                lang.lang_id
            );
        }
    }
}
