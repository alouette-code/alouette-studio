import React, { useState, useEffect, useRef, useCallback } from "react";
import {
  Save,
  FileCode,
  Check,
  AlertCircle,
  RefreshCw,
  FilePlus,
} from "lucide-react";
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
  activeProjectId?: string | null;
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

// ── Simple line diff for unsaved changes (real-time, no IPC) ──
interface UnsavedChange {
  line: number;
  type: "added" | "modified" | "deleted_context";
  count: number;
}

function computeUnsavedDiff(
  original: string,
  current: string,
): UnsavedChange[] {
  const origLines = original.split("\n");
  const currLines = current.split("\n");
  const changes: UnsavedChange[] = [];

  // Track added/deleted lines using a simple approach
  // We use line-by-line comparison
  const maxLen = Math.max(origLines.length, currLines.length);
  let deletionBuffer = 0;

  for (let i = 0; i < maxLen; i++) {
    const origLine = i < origLines.length ? origLines[i] : undefined;
    const currLine = i < currLines.length ? currLines[i] : undefined;

    if (origLine === undefined && currLine !== undefined) {
      // Line added at end
      changes.push({ line: i + 1, type: "added", count: 0 });
    } else if (origLine !== undefined && currLine === undefined) {
      // Line deleted at end
      deletionBuffer++;
    } else if (
      origLine !== undefined &&
      currLine !== undefined &&
      origLine !== currLine
    ) {
      if (deletionBuffer > 0) {
        changes.push({ line: i + 1, type: "modified", count: deletionBuffer });
        deletionBuffer = 0;
      } else {
        changes.push({ line: i + 1, type: "modified", count: 0 });
      }
    } else if (currLine !== undefined && deletionBuffer > 0) {
      // Context line after deletions
      changes.push({
        line: i + 1,
        type: "deleted_context",
        count: deletionBuffer,
      });
      deletionBuffer = 0;
    }
  }

  if (deletionBuffer > 0) {
    changes.push({
      line: currLines.length,
      type: "deleted_context",
      count: deletionBuffer,
    });
  }

  return changes;
}

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
  const [unsavedChanges, setUnsavedChanges] = useState<UnsavedChange[]>([]);
  const editorRef = useRef<any>(null);
  const monacoRef = useRef<any>(null);
  const lastPathRef = useRef<string | null>(null);
  const decorationIdsRef = useRef<string[]>([]);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── Git diff decorations from backend (HEAD vs disk) ──
  const { diffLines, isUntracked } = useGitDiff({
    filePath,
    cwd,
    revision: saveRevision,
  });

  // ── Compute unsaved diff whenever content changes (debounced) ──
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      if (content !== originalContent && originalContent !== "") {
        const diff = computeUnsavedDiff(originalContent, content);
        setUnsavedChanges(diff);
      } else {
        setUnsavedChanges([]);
      }
    }, 50); // 50ms debounce for smooth real-time updates
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [content, originalContent]);

  // ── Apply / update git diff + unsaved decorations ──
  const applyDiffDecorations = useCallback(() => {
    const editor = editorRef.current;
    const monacoApi = monacoRef.current;
    if (!editor || !monacoApi) return;

    const model = editor.getModel();
    if (!model) return;

    const fullLineCount = model.getLineCount();
    const decorations: any[] = [];

    // ── Git diff decorations (HEAD vs disk) ──
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
              },
            });
            break;
        }
      }
    }

    // ── Unsaved decorations (real-time, overlay on top of git diff) ──
    // These use a different color (blue/cyan) so user can see what's not yet saved
    for (const u of unsavedChanges) {
      let lineNum = u.line;
      if (lineNum < 1 || lineNum > fullLineCount) continue;

      switch (u.type) {
        case "added":
          decorations.push({
            range: new monacoApi.Range(lineNum, 1, lineNum, 1),
            options: {
              isWholeLine: true,
              glyphMarginClassName: "git-glyph-unsaved-added",
              linesDecorationsClassName: "git-line-unsaved-added",
            },
          });
          break;
        case "modified":
          decorations.push({
            range: new monacoApi.Range(lineNum, 1, lineNum, 1),
            options: {
              isWholeLine: true,
              glyphMarginClassName: "git-glyph-unsaved-modified",
              linesDecorationsClassName: "git-line-unsaved-modified",
            },
          });
          break;
        case "deleted_context":
          decorations.push({
            range: new monacoApi.Range(lineNum, 1, lineNum, 1),
            options: {
              isWholeLine: true,
              glyphMarginClassName: "git-glyph-unsaved-deleted",
            },
          });
          break;
      }
    }

    const oldIds = decorationIdsRef.current;
    decorationIdsRef.current = editor.deltaDecorations(oldIds, decorations);
  }, [diffLines, isUntracked, unsavedChanges]);

  // Re-apply decorations when diff data or unsaved changes change
  useEffect(() => {
    applyDiffDecorations();
  }, [applyDiffDecorations]);

  // Clear decorations when filePath changes
  useEffect(() => {
    if (editorRef.current) {
      const oldIds = decorationIdsRef.current;
      if (oldIds.length > 0) {
        decorationIdsRef.current = editorRef.current.deltaDecorations(
          oldIds,
          [],
        );
      }
      if (monacoRef.current) {
        applyDiffDecorations();
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  // Sync internal content with parent's decoded content
  useEffect(() => {
    if (
      filePath !== lastPathRef.current ||
      (initialContent !== null && originalContent === "")
    ) {
      if (initialContent !== null) {
        setContent(initialContent);
        setOriginalContent(initialContent);
        setUnsavedChanges([]);
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
        setUnsavedChanges([]);
        lastPathRef.current = null;
      }
    }
    setSaveStatus("idle");
  }, [initialContent, filePath]);

  const handleEditorDidMount = (editor: any, monacoInstance: any) => {
    editorRef.current = editor;
    monacoRef.current = monacoInstance;

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

    // Apply decorations immediately after mount
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
      setUnsavedChanges([]);
      setSaveStatus("success");
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
              <FilePlus size={10} />
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
