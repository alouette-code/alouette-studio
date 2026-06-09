import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Search,
  FileCode,
  Code2,
  Brain,
  Loader2,
  X,
  BookOpen,
  ArrowRight,
  Languages,
} from "lucide-react";

type CodeRagFunction = {
  id: string;
  func_name: string;
  signature: string;
  docstring: string | null;
  file_path: string;
  lang_id: string;
  project_id: string;
  line_start: number;
  line_end: number;
  normalized_text: string;
};

type CodeRagLanguage = {
  lang_id: string;
  display_name: string;
  tier: string;
  extensions: string[];
};

type CodeRagStats = {
  total_files_indexed: number;
  total_functions_extracted: number;
  total_errors: number;
  total_entries: number;
};

interface CodeRagPanelProps {
  theme?: "dark" | "light";
  currentFilePath?: string | null;
  activeProjectId?: string | null;
  onOpenFile: (path: string, line?: number) => void;
}

export default function CodeRagPanel({
  theme: _theme,
  currentFilePath,
  activeProjectId,
  onOpenFile,
}: CodeRagPanelProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<
    Array<{ entry: CodeRagFunction; score: number }>
  >([]);
  const [loading, setLoading] = useState(false);
  const [stats, setStats] = useState<CodeRagStats | null>(null);
  const [languages, setLanguages] = useState<CodeRagLanguage[]>([]);
  const [selectedLang, setSelectedLang] = useState<string>("");
  const [mode, setMode] = useState<"semantic" | "name">("name");
  const [elapsed, setElapsed] = useState<number | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<CodeRagLanguage[]>("code_rag_supported_languages")
      .then((langs) => setLanguages(langs))
      .catch(() => {});
    invoke<CodeRagStats>("code_rag_stats")
      .then((s) => setStats(s))
      .catch(() => {});
  }, []);

  // Auto-detect language từ file đang mở
  useEffect(() => {
    if (!currentFilePath) return;
    const ext = currentFilePath.split(".").pop()?.toLowerCase() || "";
    const extToLang: Record<string, string> = {
      py: "python",
      rs: "rust",
      js: "javascript",
      ts: "typescript",
      tsx: "typescript",
      jsx: "javascript",
      java: "java",
      c: "c",
      cpp: "cpp",
      h: "c",
      hpp: "cpp",
      cs: "csharp",
      go: "go",
      php: "php",
      rb: "ruby",
      swift: "swift",
      kt: "kotlin",
      kts: "kotlin",
      lua: "lua",
      r: "r",
      pl: "perl",
      scala: "scala",
      dart: "dart",
      ex: "elixir",
      clj: "clojure",
      jl: "julia",
      hs: "haskell",
      zig: "zig",
      sql: "sql",
    };
    if (extToLang[ext]) {
      setSelectedLang(extToLang[ext]);
    }
  }, [currentFilePath]);

  // Debounce search
  const doSearch = useCallback(
    (q: string, lang: string) => {
      if (!q.trim()) {
        setResults([]);
        setElapsed(null);
        return;
      }

      setLoading(true);
      const start = performance.now();

      if (mode === "name") {
        invoke<CodeRagFunction[]>("code_rag_query_by_name", {
          name: q.trim(),
          langId: lang || null,
          projectId: activeProjectId || null,
          topK: 15,
        })
          .then((matches) => {
            const elapsedMs = performance.now() - start;
            setResults(matches.map((entry) => ({ entry, score: 1.0 })));
            setElapsed(elapsedMs);
          })
          .catch(() => {})
          .finally(() => setLoading(false));
      } else {
        invoke<{
          matches: Array<{ entry: CodeRagFunction; score: number }>;
          elapsed_ms: number;
        }>("code_rag_query", {
          query: q.trim(),
          langId: lang || null,
          projectId: activeProjectId || null,
          topK: 15,
        })
          .then((data) => {
            setResults(data.matches);
            setElapsed(data.elapsed_ms);
          })
          .catch(() => {})
          .finally(() => setLoading(false));
      }
    },
    [mode, activeProjectId],
  );

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(query, selectedLang), 250);
  }, [query, selectedLang, doSearch]);

  return (
    <div
      className="code-rag-panel"
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: "var(--bg-secondary, #1a1b26)",
        color: "var(--text-primary, #c9d1d9)",
        fontSize: "12px",
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-primary, #2e2e3e)",
          display: "flex",
          alignItems: "center",
          gap: "6px",
          fontWeight: 600,
          fontSize: "11px",
          textTransform: "uppercase",
          letterSpacing: "0.5px",
        }}
      >
        <Brain size={13} style={{ color: "var(--color-accent, #6366f1)" }} />
        <span>Code RAG</span>
        {stats && (
          <span
            style={{
              marginLeft: "auto",
              fontSize: "10px",
              color: "var(--text-muted, #6b7280)",
              fontWeight: 400,
              textTransform: "none",
            }}
          >
            {stats.total_entries} functions
          </span>
        )}
      </div>

      {/* Search Bar */}
      <div
        style={{
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-primary, #2e2e3e)",
        }}
      >
        <div
          style={{
            display: "flex",
            gap: "4px",
            alignItems: "center",
            backgroundColor: "var(--bg-primary, #0f0f1a)",
            borderRadius: "4px",
            padding: "2px",
            marginBottom: "6px",
          }}
        >
          <button
            onClick={() => setMode("name")}
            style={{
              flex: 1,
              padding: "3px 6px",
              border: "none",
              borderRadius: "3px",
              background:
                mode === "name"
                  ? "var(--color-accent, #6366f1)"
                  : "transparent",
              color: mode === "name" ? "#fff" : "var(--text-muted)",
              cursor: "pointer",
              fontSize: "10px",
              fontWeight: 500,
            }}
          >
            <Code2
              size={10}
              style={{ marginRight: "3px", verticalAlign: "middle" }}
            />
            Name
          </button>
          <button
            onClick={() => setMode("semantic")}
            style={{
              flex: 1,
              padding: "3px 6px",
              border: "none",
              borderRadius: "3px",
              background:
                mode === "semantic"
                  ? "var(--color-accent, #6366f1)"
                  : "transparent",
              color: mode === "semantic" ? "#fff" : "var(--text-muted)",
              cursor: "pointer",
              fontSize: "10px",
              fontWeight: 500,
            }}
          >
            <Brain
              size={10}
              style={{ marginRight: "3px", verticalAlign: "middle" }}
            />
            Semantic
          </button>
        </div>

        <div
          style={{
            display: "flex",
            gap: "4px",
            alignItems: "center",
            backgroundColor: "var(--bg-primary, #0f0f1a)",
            borderRadius: "4px",
            padding: "4px 8px",
          }}
        >
          <Search
            size={12}
            style={{ color: "var(--text-muted)", flexShrink: 0 }}
          />
          <input
            autoFocus
            placeholder={
              mode === "name"
                ? "Search function name..."
                : "Describe what you need..."
            }
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{
              flex: 1,
              border: "none",
              background: "transparent",
              color: "var(--text-primary)",
              fontSize: "11px",
              outline: "none",
              minWidth: 0,
            }}
          />
          {loading && (
            <Loader2
              size={12}
              className="spin-animation"
              style={{ flexShrink: 0 }}
            />
          )}
          {query && !loading && (
            <X
              size={12}
              style={{
                cursor: "pointer",
                flexShrink: 0,
                color: "var(--text-muted)",
              }}
              onClick={() => setQuery("")}
            />
          )}
        </div>

        {/* Language filter */}
        <div
          style={{
            marginTop: "6px",
            display: "flex",
            gap: "4px",
            alignItems: "center",
          }}
        >
          <Languages
            size={10}
            style={{ color: "var(--text-muted)", flexShrink: 0 }}
          />
          <select
            value={selectedLang}
            onChange={(e) => setSelectedLang(e.target.value)}
            style={{
              flex: 1,
              border: "1px solid var(--border-primary, #2e2e3e)",
              background: "var(--bg-primary, #0f0f1a)",
              color: "var(--text-primary)",
              fontSize: "10px",
              padding: "2px 4px",
              borderRadius: "3px",
              outline: "none",
            }}
          >
            <option value="">All languages</option>
            {languages.map((lang) => (
              <option key={lang.lang_id} value={lang.lang_id}>
                {lang.display_name} ({lang.tier})
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Results */}
      <div
        style={{
          flex: 1,
          overflowY: "auto",
          overflowX: "hidden",
          padding: "4px 0",
        }}
      >
        {results.length === 0 && !loading && (
          <div
            style={{
              padding: "20px 16px",
              textAlign: "center",
              color: "var(--text-muted)",
              fontSize: "11px",
              lineHeight: 1.6,
            }}
          >
            {query
              ? "No results found."
              : "Type a function name or description to search across your codebase."}
          </div>
        )}

        {results.map((match) => (
          <div
            key={match.entry.id}
            className="code-rag-result-item"
            onClick={() =>
              onOpenFile(match.entry.file_path, match.entry.line_start + 1)
            }
            style={{
              padding: "6px 10px",
              cursor: "pointer",
              borderBottom: "1px solid var(--border-primary, #2e2e3e)",
              transition: "background 0.1s",
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background =
                "var(--bg-hover, #1e2030)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "transparent";
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "4px",
                marginBottom: "2px",
              }}
            >
              <span
                style={{
                  fontSize: "9px",
                  fontWeight: 700,
                  color: "var(--color-accent, #6366f1)",
                  background: "var(--bg-primary)",
                  padding: "0 4px",
                  borderRadius: "2px",
                  letterSpacing: "0.3px",
                }}
              >
                {match.entry.lang_id.toUpperCase()}
              </span>
              <span
                style={{
                  fontWeight: 600,
                  fontSize: "11px",
                  color: "var(--color-accent, #6366f1)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {match.entry.func_name}
              </span>
              {match.score < 1.0 && (
                <span
                  style={{
                    marginLeft: "auto",
                    fontSize: "9px",
                    color: "var(--text-muted)",
                  }}
                >
                  {(match.score * 100).toFixed(0)}%
                </span>
              )}
            </div>

            <div
              style={{
                color: "var(--text-secondary, #8b949e)",
                fontSize: "10px",
                fontFamily: "monospace",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                marginBottom: "2px",
              }}
            >
              {match.entry.signature}
            </div>

            {match.entry.docstring && (
              <div
                style={{
                  color: "var(--text-muted)",
                  fontSize: "9px",
                  display: "flex",
                  gap: "3px",
                  alignItems: "flex-start",
                  marginTop: "1px",
                }}
              >
                <BookOpen
                  size={8}
                  style={{ marginTop: "1px", flexShrink: 0 }}
                />
                <span
                  style={{
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {match.entry.docstring}
                </span>
              </div>
            )}

            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "4px",
                marginTop: "2px",
                fontSize: "9px",
                color: "var(--text-muted)",
              }}
            >
              <FileCode size={8} />
              <span
                style={{
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {match.entry.file_path}
              </span>
              <ArrowRight size={8} />
              <span>line {match.entry.line_start + 1}</span>
            </div>
          </div>
        ))}
      </div>

      {/* Footer */}
      {elapsed !== null && (
        <div
          style={{
            padding: "4px 10px",
            borderTop: "1px solid var(--border-primary, #2e2e3e)",
            fontSize: "9px",
            color: "var(--text-muted)",
            display: "flex",
            justifyContent: "space-between",
          }}
        >
          <span>{results.length} results</span>
          <span>{elapsed.toFixed(0)}ms</span>
        </div>
      )}
    </div>
  );
}
