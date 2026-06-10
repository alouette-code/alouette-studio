import { useState, useEffect, useRef, useCallback } from "react";
import { Search, Loader2, FileCode, ArrowRight } from "lucide-react";
import { queryCodeRag, getCodeRagHealth } from "../lib/code_rag";

const CACHE_TTL = 60_000;
const cache = new Map<string, { data: any[]; ts: number }>();

interface CodeRagSearchWidgetProps {
  editorContainer: HTMLElement | null;
  editor: any;
  monaco: any;
  lineNumber: number;
  column: number;
  languageId: string;
  projectId?: string | null;
  projectPath?: string | null;
  onOpenFile: (path: string, line?: number) => void;
  onClose: () => void;
}

type ResultItem = {
  id: string;
  func_name: string;
  signature: string;
  file_path: string;
  lang_id: string;
  line_start: number;
  score: number;
};

export default function CodeRagSearchWidget({
  editorContainer,
  editor,
  monaco: _monaco,
  lineNumber,
  column,
  languageId: _languageId,
  projectId: _projectId,
  projectPath: _projectPath,
  onOpenFile,
  onClose,
}: CodeRagSearchWidgetProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ResultItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [totalEntries, setTotalEntries] = useState<number | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const widgetRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const fetchingRef = useRef(false);
  const lastQueryRef = useRef("");

  // ── 1. Position ──
  useEffect(() => {
    if (!editor || !editorContainer) return;
    const pos = editor.getScrolledVisiblePosition({ lineNumber, column });
    if (!pos) return;
    const editorRect = editorContainer.getBoundingClientRect();
    let top = pos.top + 22;
    let left = pos.left;
    const maxTop = editorRect.height - 50;
    if (top > maxTop) top = maxTop - 40;
    if (top < 4) top = 4;
    setPosition({ top: Math.round(top), left: Math.round(Math.max(6, left)) });
  }, [editor, editorContainer, lineNumber, column]);

  // ── 2. Get total entries (info only) ──
  useEffect(() => {
    getCodeRagHealth()
      .then((h) => setTotalEntries(h.total_entries))
      .catch(() => {});
  }, []);

  // ── 3. Focus ──
  useEffect(() => {
    const t = setTimeout(() => inputRef.current?.focus(), 80);
    return () => clearTimeout(t);
  }, []);

  // ── 4. Click outside ──
  useEffect(() => {
    const t = setTimeout(() => {
      const handler = (e: MouseEvent) => {
        if (
          widgetRef.current &&
          !widgetRef.current.contains(e.target as Node)
        ) {
          onClose();
        }
      };
      document.addEventListener("mousedown", handler, true);
      return () => document.removeEventListener("mousedown", handler, true);
    }, 0);
    return () => clearTimeout(t);
  }, [onClose]);

  // ── 5. Search ──
  const doSearch = useCallback(async (q: string) => {
    const trimmed = q.trim();
    if (!trimmed || trimmed.length < 2) {
      setResults([]);
      setSelectedIndex(-1);
      return;
    }

    const cacheKey = trimmed.toLowerCase();
    const cached = cache.get(cacheKey);
    if (cached && Date.now() - cached.ts < CACHE_TTL) {
      setResults(cached.data);
      setSelectedIndex(-1);
      return;
    }
    if (fetchingRef.current) return;

    fetchingRef.current = true;
    setLoading(true);
    try {
      const data = await queryCodeRag(trimmed, undefined, undefined, 20);
      const mapped: ResultItem[] = (data?.matches || []).map((m: any) => ({
        id: m.entry.id,
        func_name: m.entry.func_name,
        signature: m.entry.signature,
        file_path: m.entry.file_path,
        lang_id: m.entry.lang_id,
        line_start: m.entry.line_start,
        score: m.score,
      }));
      cache.set(cacheKey, { data: mapped, ts: Date.now() });
      setResults(mapped);
      setSelectedIndex(-1);
    } catch {
      setResults([]);
    } finally {
      fetchingRef.current = false;
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    lastQueryRef.current = query;
    const timer = setTimeout(() => {
      if (lastQueryRef.current === query) doSearch(query);
    }, 400);
    return () => clearTimeout(timer);
  }, [query, doSearch]);

  // ── 6. Keyboard ──
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((p) => (p < results.length - 1 ? p + 1 : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((p) => (p > 0 ? p - 1 : results.length - 1));
    } else if (
      e.key === "Enter" &&
      selectedIndex >= 0 &&
      results[selectedIndex]
    ) {
      e.preventDefault();
      const item = results[selectedIndex];
      onOpenFile(item.file_path, item.line_start + 1);
      onClose();
    }
  };

  return (
    <div
      ref={widgetRef}
      style={{
        position: "absolute",
        top: position.top,
        left: position.left,
        width: "360px",
        maxHeight: "340px",
        zIndex: 9999,
        background: "var(--bg-secondary, #111115)",
        border: "1px solid var(--border-primary, #22222a)",
        borderRadius: "3px",
        boxShadow: "0 6px 24px rgba(0,0,0,0.5)",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        fontSize: "11px",
      }}
    >
      {/* ── Search input ── */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "4px",
          padding: "5px 8px",
          borderBottom: "1px solid var(--border-primary, #22222a)",
          background: "var(--bg-primary, #0a0a0c)",
        }}
      >
        {loading ? (
          <Loader2
            size={12}
            className="spin-animation"
            style={{ flexShrink: 0, color: "var(--color-accent)" }}
          />
        ) : (
          <Search
            size={12}
            style={{ flexShrink: 0, color: "var(--text-muted)" }}
          />
        )}
        <input
          ref={inputRef}
          autoFocus
          placeholder="Tìm cấu trúc code..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          style={{
            flex: 1,
            border: "none",
            background: "transparent",
            color: "var(--text-primary, #e6e6eb)",
            fontSize: "11px",
            outline: "none",
            fontFamily: "inherit",
          }}
        />
        <span
          style={{
            fontSize: "9px",
            color: "var(--text-muted)",
            flexShrink: 0,
          }}
        >
          ESC
        </span>
      </div>

      {/* ── Content ── */}
      <div style={{ flex: 1, overflowY: "auto", maxHeight: "260px" }}>
        {/* Empty state (no query yet) */}
        {results.length === 0 && !loading && !query.trim() && (
          <div
            style={{
              padding: "12px",
              textAlign: "center",
              color: "var(--text-muted)",
              fontSize: "10px",
              lineHeight: 1.6,
            }}
          >
            {totalEntries !== null ? (
              <>
                <div
                  style={{ marginBottom: 6, color: "var(--text-secondary)" }}
                >
                  📚 {totalEntries.toLocaleString()} snippets đã index
                </div>
                <div>
                  Gõ mô tả để tìm, vd:{" "}
                  <em style={{ color: "var(--text-secondary)" }}>
                    connect, parse, validate, sort, render
                  </em>
                </div>
              </>
            ) : (
              <div>Đang tải dữ liệu...</div>
            )}
          </div>
        )}

        {/* No results */}
        {results.length === 0 && !loading && query.trim() && (
          <div
            style={{
              padding: "12px",
              textAlign: "center",
              color: "var(--text-muted)",
              fontSize: "10px",
            }}
          >
            Không tìm thấy kết quả cho "{query}"
          </div>
        )}

        {/* Results */}
        {results.map((item, index) => (
          <div
            key={item.id}
            onClick={() => {
              onOpenFile(item.file_path, item.line_start + 1);
              onClose();
            }}
            onMouseEnter={() => setSelectedIndex(index)}
            style={{
              padding: "4px 8px",
              cursor: "pointer",
              borderBottom:
                index < results.length - 1
                  ? "1px solid var(--border-primary, #22222a)"
                  : "none",
              background:
                selectedIndex === index
                  ? "var(--bg-hover, rgba(58, 134, 255, 0.08))"
                  : "transparent",
              transition: "background 0.06s",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
              <span
                style={{
                  fontSize: "8px",
                  fontWeight: 700,
                  color: "var(--color-accent, #3a86ff)",
                  background: "var(--bg-primary)",
                  padding: "0 3px",
                  letterSpacing: "0.3px",
                }}
              >
                {item.lang_id.toUpperCase()}
              </span>
              <span
                style={{
                  fontWeight: 600,
                  fontSize: "10px",
                  color: "var(--color-accent, #3a86ff)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {item.func_name}
              </span>
              <span
                style={{
                  marginLeft: "auto",
                  fontSize: "8px",
                  color: "var(--text-muted)",
                }}
              >
                {(item.score * 100).toFixed(0)}%
              </span>
            </div>
            <div
              style={{
                color: "var(--text-secondary, #92929e)",
                fontSize: "9px",
                fontFamily: "var(--font-mono, monospace)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {item.signature}
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "3px",
                marginTop: "1px",
                fontSize: "8px",
                color: "var(--text-muted)",
              }}
            >
              <FileCode size={7} />
              <span
                style={{
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {item.file_path}
              </span>
              <ArrowRight size={7} />
              <span>L{item.line_start + 1}</span>
            </div>
          </div>
        ))}
      </div>

      {/* ── Footer ── */}
      {results.length > 0 && (
        <div
          style={{
            padding: "2px 8px",
            borderTop: "1px solid var(--border-primary, #22222a)",
            fontSize: "8px",
            color: "var(--text-muted)",
            display: "flex",
            justifyContent: "space-between",
          }}
        >
          <span>{results.length} results</span>
          <span>↑↓ Enter</span>
        </div>
      )}
    </div>
  );
}
