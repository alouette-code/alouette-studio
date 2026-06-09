import React, { useState, useEffect, useRef, useCallback } from "react";
import { Save, FileCode, Check, AlertCircle, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import Editor from "@monaco-editor/react";
import { useGitDiff } from "../hooks/useGitDiff";

interface CodeEditorProps {
  theme?: "dark" | "light";
  filePath: string | null;
  content: string | null;
  isLoading: boolean;
  error: string | null;
  /** Git working directory for diff decorations */
  cwd?: string;
  onFileSaved?: () => void;
  onChange?: (val: string) => void;
  onSave?: (val: string) => void;
  scrollPositionsRef: React.MutableRefObject<{ [path: string]: number }>;
  cursorPositionsRef: React.MutableRefObject<{
    [path: string]: { start: number; end: number };
  }>;
}

const getLanguageFromPath = (path: string | null): string => {
  if (!path) return "plaintext";
  const fileName = path.split(/[\\/]/).pop()?.toLowerCase() || "";

  if (fileName.startsWith(".env")) return "ini";

  const ext = fileName.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "js":
    case "jsx":
      return "javascript";
    case "ts":
    case "tsx":
      return "typescript";
    case "html":
      return "html";
    case "css":
      return "css";
    case "json":
      return "json";
    case "md":
      return "markdown";
    case "rs":
      return "rust";
    case "toml":
      return "toml";
    case "yaml":
    case "yml":
      return "yaml";
    case "py":
      return "python";
    case "go":
      return "go";
    case "sh":
    case "bash":
    case "zsh":
      return "shell";
    case "sql":
      return "sql";
    case "xml":
      return "xml";
    case "c":
    case "cpp":
    case "h":
    case "hpp":
      return "cpp";
    case "java":
      return "java";
    case "cs":
      return "csharp";
    default:
      return "plaintext";
  }
};

export default React.memo(function CodeEditor({
  theme = "dark",
  filePath,
  content: initialContent,
  isLoading,
  error,
  cwd,
  onFileSaved,
  onChange,
  onSave,
  scrollPositionsRef,
  cursorPositionsRef,
}: CodeEditorProps) {
  const [content, setContent] = useState<string>("");
  const [originalContent, setOriginalContent] = useState<string>("");
  const [saveStatus, setSaveStatus] = useState<
    "idle" | "saving" | "success" | "error"
  >("idle");
  const [saveRevision, setSaveRevision] = useState(0);
  const editorRef = useRef<any>(null);
  const monacoRef = useRef<any>(null);
  const lastPathRef = useRef<string | null>(null);
  const decorationIdsRef = useRef<string[]>([]);

  // ── Git diff decorations ──
  const { diffLines, isUntracked } = useGitDiff({
    filePath,
    cwd,
    revision: saveRevision,
  });

  // ── Apply / update git diff decorations ──
  const applyDiffDecorations = useCallback(() => {
    const editor = editorRef.current;
    const monacoApi = monacoRef.current;
    if (!editor || !monacoApi) {
      return;
    }

    const model = editor.getModel();
    if (!model) return;

    const fullLineCount = model.getLineCount();
    const decorations: any[] = [];

    if (isUntracked) {
      for (let line = 1; line <= fullLineCount; line++) {
        decorations.push({
          range: new monacoApi.Range(line, 1, line, 1),
          options: {
            isWholeLine: true,
            glyphMarginClassName: "git-glyph-added",
            linesDecorationsClassName: "git-line-added",
          },
        });
      }
    } else {
      for (const d of diffLines) {
        let lineNum = d.line_number;

        // Handle trailing deletions (sentinel value from backend)
        if (
          d.change_type === "deleted_context" &&
          lineNum === Number.MAX_SAFE_INTEGER
        ) {
          lineNum = fullLineCount;
        }

        if (lineNum < 1 || lineNum > fullLineCount) continue;

        switch (d.change_type) {
          case "added":
            decorations.push({
              range: new monacoApi.Range(lineNum, 1, lineNum, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: "git-glyph-added",
                linesDecorationsClassName: "git-line-added",
              },
            });
            break;

          case "modified":
            decorations.push({
              range: new monacoApi.Range(lineNum, 1, lineNum, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: "git-glyph-modified",
                linesDecorationsClassName: "git-line-modified",
              },
            });
            break;

          case "deleted_context":
            decorations.push({
              range: new monacoApi.Range(lineNum, 1, lineNum, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: "git-glyph-deleted",
                glyphMarginHoverMessage: {
                  value: `${d.deleted_count} dòng đã bị xóa`,
                },
              },
            });
            break;
        }
      }
    }

    const oldIds = decorationIdsRef.current;
    const newIds = editor.deltaDecorations(oldIds, decorations);
    decorationIdsRef.current = newIds;
  }, [diffLines, isUntracked]);

  // ── Re-apply decorations when diff data changes ──
  useEffect(() => {
    applyDiffDecorations();
  }, [applyDiffDecorations]);

  // ── Clear decorations when filePath changes, then re-apply ──
  useEffect(() => {
    if (editorRef.current) {
      // Clear old decorations first
      const oldIds = decorationIdsRef.current;
      if (oldIds.length > 0) {
        decorationIdsRef.current = editorRef.current.deltaDecorations(
          oldIds,
          [],
        );
      }
      // Apply new decorations if diff data is available
      if (monacoRef.current) {
        applyDiffDecorations();
      }
    }
  }, [filePath]);

  // Save position on unmount or file path change
  useEffect(() => {
    return () => {
      if (filePath && editorRef.current) {
        const editor = editorRef.current;
        const model = editor.getModel();
        if (model) {
          scrollPositionsRef.current[filePath] = editor.getScrollTop();
          const selection = editor.getSelection();
          if (selection) {
            cursorPositionsRef.current[filePath] = {
              start: model.getOffsetAt({
                lineNumber: selection.startLineNumber,
                column: selection.startColumn,
              }),
              end: model.getOffsetAt({
                lineNumber: selection.endLineNumber,
                column: selection.endColumn,
              }),
            };
          }
        }
      }
    };
  }, [filePath, scrollPositionsRef, cursorPositionsRef]);

  // Sync internal content with parent's decoded content without cursor jumping
  useEffect(() => {
    if (
      filePath !== lastPathRef.current ||
      (initialContent !== null && originalContent === "")
    ) {
      if (initialContent !== null) {
        setContent(initialContent);
        setOriginalContent(initialContent);
        lastPathRef.current = filePath;

        if (filePath && editorRef.current) {
          const editor = editorRef.current;
          const model = editor.getModel();
          if (model) {
            const savedScroll = scrollPositionsRef.current[filePath] || 0;
            const savedCursor = cursorPositionsRef.current[filePath];

            setTimeout(() => {
              if (savedCursor) {
                const startPos = model.getPositionAt(savedCursor.start);
                const endPos = model.getPositionAt(savedCursor.end);
                editor.setSelection({
                  startLineNumber: startPos.lineNumber,
                  startColumn: startPos.column,
                  endLineNumber: endPos.lineNumber,
                  endColumn: endPos.column,
                });
              }
              editor.setScrollTop(savedScroll);
            }, 0);
          }
        }
      } else {
        setContent("");
        setOriginalContent("");
        lastPathRef.current = null;
      }
    }
    setSaveStatus("idle");
  }, [initialContent, filePath]);

  const handleEditorDidMount = (editor: any, monacoInstance: any) => {
    editorRef.current = editor;
    monacoRef.current = monacoInstance;

    // Restore scroll and cursor
    if (filePath) {
      const model = editor.getModel();
      if (model) {
        const savedScroll = scrollPositionsRef.current[filePath] || 0;
        const savedCursor = cursorPositionsRef.current[filePath];

        if (savedCursor) {
          const startPos = model.getPositionAt(savedCursor.start);
          const endPos = model.getPositionAt(savedCursor.end);
          editor.setSelection({
            startLineNumber: startPos.lineNumber,
            startColumn: startPos.column,
            endLineNumber: endPos.lineNumber,
            endColumn: endPos.column,
          });
        }
        editor.setScrollTop(savedScroll);
      }
    }

    // ── Apply git diff decorations IMMEDIATELY after mount ──
    applyDiffDecorations();
  };

  const handleSave = async () => {
    if (!filePath || saveStatus === "saving") return;
    setSaveStatus("saving");
    try {
      const latestContent = editorRef.current
        ? editorRef.current.getValue()
        : content;
      await invoke("write_file_content", {
        path: filePath,
        content: latestContent,
      });
      setOriginalContent(latestContent);
      setSaveStatus("success");
      // Bump revision to refresh git diff after save
      setSaveRevision((r) => r + 1);
      if (onFileSaved) onFileSaved();
      if (onSave) onSave(latestContent);
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err: any) {
      console.error("Error writing file:", err);
      setSaveStatus("error");
      alert(`Save failed: ${err.toString()}`);
    }
  };

  // Keyboard shortcut Ctrl+S
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [content, filePath, saveStatus]);

  const isDirty = content !== originalContent;
  const fileName = filePath ? filePath.split(/[\\/]/).pop() : "";

  if (!filePath) {
    return (
      <div className="code-editor-empty">
        <FileCode size={32} className="empty-icon" />
        <h3>No File Selected</h3>
        <p>
          Click on any file in the Project Explorer to read and edit it directly
          here.
        </p>
      </div>
    );
  }

  return (
    <div className="code-editor-container">
      <div className="code-editor-header">
        <div className="file-info">
          <FileCode size={14} className="file-icon" />
          <span className="file-name">{fileName}</span>
          {isDirty && <span className="dirty-dot" title="Unsaved changes" />}
          {isUntracked && (
            <span className="untracked-badge" title="File chưa được commit">
              U
            </span>
          )}
          <span className="file-path">{filePath}</span>
        </div>
        <div className="editor-actions">
          {saveStatus === "saving" && (
            <span className="save-status-indicator text-muted">
              <RefreshCw size={11} className="spin-animation" /> Saving...
            </span>
          )}
          {saveStatus === "success" && (
            <span className="save-status-indicator text-success">
              <Check size={11} /> Saved
            </span>
          )}
          <button
            className={`btn-save-file ${isDirty ? "dirty" : ""}`}
            onClick={handleSave}
            disabled={!isDirty || saveStatus === "saving"}
            title="Save file (Ctrl + S)"
          >
            <Save size={12} />
            <span>Save</span>
          </button>
        </div>
      </div>

      {isLoading ? (
        <div className="editor-loading">
          <RefreshCw size={24} className="spin-animation" />
          <span>Reading file contents...</span>
        </div>
      ) : error ? (
        <div className="editor-error">
          <AlertCircle size={24} />
          <span>{error}</span>
        </div>
      ) : (
        <div className="editor-body" style={{ minHeight: 0, flex: 1 }}>
          <Editor
            height="100%"
            width="100%"
            language={getLanguageFromPath(filePath)}
            theme={theme === "light" ? "light" : "vs-dark"}
            value={content}
            onChange={(val) => {
              const newVal = val || "";
              setContent(newVal);
              if (onChange) onChange(newVal);
            }}
            onMount={handleEditorDidMount}
            options={{
              fontSize: 12,
              fontFamily:
                "'JetBrains Mono', Consolas, 'Courier New', monospace",
              minimap: { enabled: false },
              automaticLayout: true,
              scrollBeyondLastLine: false,
              cursorBlinking: "smooth",
              lineNumbers: "on",
              lineNumbersMinChars: 5,
              tabSize: 4,
              insertSpaces: true,
              wordWrap: "on",
              renderLineHighlight: "all",
              glyphMargin: true,
              scrollbar: {
                vertical: "visible",
                horizontal: "visible",
                verticalScrollbarSize: 10,
                horizontalScrollbarSize: 10,
              },
            }}
          />
        </div>
      )}
    </div>
  );
});
