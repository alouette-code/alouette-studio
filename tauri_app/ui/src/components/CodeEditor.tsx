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
import CodeRagSearchWidget from "./CodeRagSearchWidget";

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
  /** Mở file từ kết quả RAG search */
  onOpenFile?: (path: string, line?: number) => void;
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

  let start = 0;
  while (
    start < origLines.length &&
    start < currLines.length &&
    origLines[start] === currLines[start]
  ) {
    start++;
  }

  let origEnd = origLines.length - 1;
  let currEnd = currLines.length - 1;
  while (
    origEnd >= start &&
    currEnd >= start &&
    origLines[origEnd] === currLines[currEnd]
  ) {
    origEnd--;
    currEnd--;
  }

  for (let i = start; i <= currEnd; i++) {
    changes.push({ line: i + 1, type: "modified", count: 0 });
  }

  if (start > currEnd && start <= origEnd) {
     const deletedCount = origEnd - start + 1;
     const lineNum = Math.min(start + 1, currLines.length);
     changes.push({ line: lineNum, type: "deleted_context", count: deletedCount });
  }

  return changes;
}

const editorOptions = {
  fontSize: 12,
  fontFamily: "'JetBrains Mono', Consolas, 'Courier New', monospace",
  minimap: { enabled: false },
  automaticLayout: true,
  scrollBeyondLastLine: false,
  cursorBlinking: "smooth" as const,
  lineNumbers: "on" as const,
  lineNumbersMinChars: 5,
  tabSize: 4,
  insertSpaces: true,
  wordWrap: "on" as const,
  renderLineHighlight: "all" as const,
  glyphMargin: true,
  quickSuggestions: true,
  suggestOnTriggerCharacters: true,
  quickSuggestionsDelay: 100,
  wordBasedSuggestions: "currentDocument" as const,
  suggest: {
    showMethods: true,
    showFunctions: true,
    showConstructors: false,
    showFields: false,
    showVariables: false,
    showKeywords: false,
  },
  scrollbar: {
    vertical: "visible" as const,
    horizontal: "visible" as const,
    verticalScrollbarSize: 10,
    horizontalScrollbarSize: 10,
  },
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
  onOpenFile,
  activeProjectId,
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
  // Ref cho content mới nhất — tránh closure stale trong save handler
  const latestContentRef = useRef<string>("");
  // RAF ref cho batching onChange — giảm re-render khi gõ nhanh
  const rafRef = useRef<number | null>(null);
  // Ref cho editor container để tính vị trí overlay
  const editorContainerRef = useRef<HTMLDivElement | null>(null);

  // ── CodeRag Search Widget state ──
  const [ragSearchOpen, setRagSearchOpen] = useState(false);
  const [ragSearchLine, setRagSearchLine] = useState(1);
  const ragSearchEditorRef = useRef<any>(null);
  const ragSearchColumnRef = useRef<number>(1);
  
  // ── Memoize onChange prop to avoid changing internal onChange reference ──
  const onChangePropRef = useRef(onChange);
  useEffect(() => {
    onChangePropRef.current = onChange;
  }, [onChange]);

  const handleEditorChange = useCallback((val: string | undefined) => {
    const newVal = val || "";
    latestContentRef.current = newVal;
    if (rafRef.current === null) {
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        const latest = latestContentRef.current;
        setContent((prev) => (prev === latest ? prev : latest));
      });
    }
    if (onChangePropRef.current) onChangePropRef.current(newVal);
  }, []);

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
    }, 1000); // 1000ms debounce to avoid interrupting IME composition
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
              },
            });
            break;
          case "modified":
            decorations.push({
              range: new monacoApi.Range(lineNum, 1, lineNum, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: "git-glyph-modified",
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
            },
          });
          break;
        case "modified":
          decorations.push({
            range: new monacoApi.Range(lineNum, 1, lineNum, 1),
            options: {
              isWholeLine: true,
              glyphMarginClassName: "git-glyph-unsaved-modified",
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
    if (filePath !== lastPathRef.current) {
      if (initialContent !== null) {
        setContent(initialContent);
        setOriginalContent(initialContent);
        setUnsavedChanges([]);
        lastPathRef.current = filePath;

        if (editorRef.current && editorRef.current.getValue() !== initialContent) {
          editorRef.current.setValue(initialContent);
        }

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

  // ── Simple keyword completion để Monaco hiện suggest popup ──
  // Không cần backend, không scan, chạy hoàn toàn trên frontend
  const handleBeforeMount = (monaco: any) => {
    const keywords: { [lang: string]: string[] } = {
      typescript: [
        "function",
        "const",
        "let",
        "var",
        "if",
        "else",
        "return",
        "async",
        "await",
        "import",
        "export",
        "from",
        "class",
        "interface",
        "type",
        "extends",
        "implements",
        "new",
        "throw",
        "try",
        "catch",
        "finally",
        "switch",
        "case",
        "break",
        "continue",
        "for",
        "while",
        "do",
        "in",
        "of",
        "typeof",
        "instanceof",
        "keyof",
        "readonly",
        "public",
        "private",
        "protected",
        "static",
        "abstract",
        "enum",
      ],
      javascript: [
        "function",
        "const",
        "let",
        "var",
        "if",
        "else",
        "return",
        "async",
        "await",
        "import",
        "export",
        "from",
        "class",
        "new",
        "throw",
        "try",
        "catch",
        "finally",
        "switch",
        "case",
        "break",
        "continue",
        "for",
        "while",
        "do",
        "typeof",
        "instanceof",
        "this",
        "super",
        "yield",
        "delete",
        "void",
        "debugger",
      ],
      python: [
        "def",
        "class",
        "return",
        "if",
        "elif",
        "else",
        "for",
        "while",
        "try",
        "except",
        "finally",
        "with",
        "as",
        "import",
        "from",
        "async",
        "await",
        "yield",
        "lambda",
        "pass",
        "break",
        "continue",
        "raise",
        "assert",
        "del",
        "global",
        "nonlocal",
        "True",
        "False",
        "None",
        "self",
        "in",
        "not",
        "and",
        "or",
        "is",
      ],
      rust: [
        "fn",
        "let",
        "mut",
        "const",
        "if",
        "else",
        "match",
        "return",
        "for",
        "while",
        "loop",
        "impl",
        "struct",
        "enum",
        "trait",
        "pub",
        "use",
        "mod",
        "crate",
        "self",
        "super",
        "where",
        "async",
        "await",
        "move",
        "ref",
        "static",
        "unsafe",
        "type",
        "dyn",
        "in",
        "as",
      ],
      go: [
        "func",
        "var",
        "const",
        "if",
        "else",
        "for",
        "range",
        "return",
        "switch",
        "case",
        "break",
        "continue",
        "go",
        "defer",
        "select",
        "chan",
        "map",
        "struct",
        "interface",
        "type",
        "package",
        "import",
        "nil",
        "true",
        "false",
        "make",
        "new",
        "append",
        "len",
        "cap",
      ],
      java: [
        "public",
        "private",
        "protected",
        "static",
        "void",
        "class",
        "interface",
        "extends",
        "implements",
        "new",
        "return",
        "if",
        "else",
        "for",
        "while",
        "do",
        "switch",
        "case",
        "break",
        "continue",
        "try",
        "catch",
        "finally",
        "throw",
        "throws",
        "import",
        "package",
        "final",
        "abstract",
        "synchronized",
        "volatile",
        "transient",
        "this",
        "super",
        "null",
        "true",
        "false",
      ],
    };

    const allLanguages = Object.keys(keywords);
    monaco.languages.registerCompletionItemProvider(allLanguages, {
      triggerCharacters: [".", " ", "("],
      provideCompletionItems: (model: any, position: any) => {
        const word = model.getWordUntilPosition(position);
        const currentWord = word ? word.word : "";
        const langId = model.getLanguageId() || "plaintext";

        if (currentWord.length < 1) return { suggestions: [] };

        const langKeywords = keywords[langId] || [];
        const wordLower = currentWord.toLowerCase();

        const suggestions = langKeywords
          .filter((kw) => kw.startsWith(wordLower))
          .map((kw) => ({
            label: kw,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: kw,
            range: {
              startLineNumber: position.lineNumber,
              startColumn:
                word?.startColumn || position.column - currentWord.length,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
            sortText: `0${kw}`,
          }));

        return { suggestions };
      },
    });
  };

  const handleEditorDidMount = (editor: any, monacoInstance: any) => {
    editorRef.current = editor;
    monacoRef.current = monacoInstance;

    // ── Add context menu action: Tìm kiếm cấu trúc code (RAG) ──
    editor.addAction({
      id: "code-rag-search",
      label: "🔍 Tìm cấu trúc code...",
      contextMenuGroupId: "navigation",
      contextMenuOrder: 1.5,
      run: (ed: any) => {
        const position = ed.getPosition();
        const lineNumber = position?.lineNumber || 1;
        const column = position?.column || 1;
        ragSearchEditorRef.current = ed;
        ragSearchColumnRef.current = column;
        setRagSearchLine(lineNumber);
        setRagSearchOpen(true);
      },
    });

    // Lắng nghe sự kiện rag-go-to-line để scroll đến dòng
    // (khi mở file từ RAG search ở panel khác)
    const goToLineHandler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.line) {
        const lineNumber = detail.line;
        const position = { lineNumber, column: 1 };
        editor.setPosition(position);
        editor.revealPositionInCenter(position);
        editor.focus();
        // Highlight tạm thời
        const decorations = editor.createDecorationsCollection([
          {
            range: new monacoInstance.Range(lineNumber, 1, lineNumber, 1),
            options: {
              isWholeLine: true,
              className: "rag-highlight-line",
              linesDecorationsClassName: "rag-highlight-gutter",
            },
          },
        ]);
        setTimeout(() => decorations.clear(), 2000);
      }
    };
    window.addEventListener("rag-go-to-line", goToLineHandler);
    // Cleanup event listener khi editor unmount
    const cleanupListener = () => {
      window.removeEventListener("rag-go-to-line", goToLineHandler);
    };
    // Lưu cleanup function
    editor.onDidDispose(cleanupListener);

    if (initialContent !== null && editor.getValue() !== initialContent) {
      editor.setValue(initialContent);
    }

    // --- HACK ĐỂ CHẶN HOÀN TOÀN BỘ GÕ (IME) THEO YÊU CẦU ---
    // Trên Linux Tauri, Monaco bị lỗi lặp chữ với IME.
    // Dùng readOnly toggle thay vì blur/focus để tránh lỗi mất dòng/nhảy con trỏ.
    setTimeout(() => {
      const domNode = editor.getDomNode();
      if (domNode) {
        const textArea = domNode.querySelector('textarea');
        if (textArea) {
          textArea.addEventListener('compositionstart', (_e: Event) => {
            // Ép huỷ quá trình gõ tiếng Việt bằng cách chớp readOnly
            textArea.readOnly = true;
            setTimeout(() => {
              textArea.readOnly = false;
            }, 0);
          });
        }
      }
    }, 500);
    // -------------------------------------------------------

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

  const handleSaveAs = async () => {
    if (!filePath) return;
    try {
      const defaultName = filePath.substring(filePath.lastIndexOf("/") + 1);
      const selectedPath: string | null = await invoke("save_file_dialog", { defaultName });
      if (selectedPath) {
        const latestContent = editorRef.current
          ? editorRef.current.getValue()
          : content;
        await invoke("write_file_content", {
          path: selectedPath,
          content: latestContent,
        });
        
        window.dispatchEvent(new CustomEvent("open-saved-as-file", { detail: { path: selectedPath } }));
      }
    } catch (err: any) {
      console.error("Save As error:", err);
      alert(`Save As failed: ${err.toString()}`);
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

  // Window Custom Event Listeners
  useEffect(() => {
    const handleTriggerSave = () => {
      handleSave();
    };
    const handleTriggerSaveAs = () => {
      handleSaveAs();
    };
    const handleTriggerRevert = () => {
      if (window.confirm("Discard all unsaved changes for this file?")) {
        const latest = originalContent;
        setContent(latest);
        if (editorRef.current) {
          editorRef.current.setValue(latest);
        }
        latestContentRef.current = latest;
        setUnsavedChanges([]);
      }
    };
    const handleFilesSavedAll = () => {
      setOriginalContent(content);
      setUnsavedChanges([]);
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    };

    window.addEventListener("trigger-save-active-file", handleTriggerSave);
    window.addEventListener("trigger-save-as-active-file", handleTriggerSaveAs);
    window.addEventListener("trigger-revert-active-file", handleTriggerRevert);
    window.addEventListener("files-saved-all", handleFilesSavedAll);

    return () => {
      window.removeEventListener("trigger-save-active-file", handleTriggerSave);
      window.removeEventListener("trigger-save-as-active-file", handleTriggerSaveAs);
      window.removeEventListener("trigger-revert-active-file", handleTriggerRevert);
      window.removeEventListener("files-saved-all", handleFilesSavedAll);
    };
  }, [content, filePath, saveStatus, originalContent]);

  // Auto Save Effect
  useEffect(() => {
    const isAutoSave = localStorage.getItem("auto_save_enabled") === "true";
    if (!isAutoSave || !filePath || content === originalContent || originalContent === "") return;

    const timer = setTimeout(() => {
      handleSave();
    }, 1500); // Save after 1.5 seconds of inactivity

    return () => clearTimeout(timer);
  }, [content, originalContent, filePath]);


  // Cleanup RAF khi unmount
  useEffect(() => {
    return () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, []);

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

      {error ? (
        <div className="editor-error">
          <AlertCircle size={24} />
          <span>{error}</span>
        </div>
      ) : (
        <div
          className="editor-body"
          ref={(el) => {
            editorContainerRef.current = el;
          }}
          style={{ minHeight: 0, flex: 1, position: "relative" }}
        >
          {/* CodeRag Search Widget overlay */}
          {ragSearchOpen && editorContainerRef.current && (
            <CodeRagSearchWidget
              editorContainer={editorContainerRef.current}
              lineNumber={ragSearchLine}
              column={ragSearchColumnRef.current}
              editor={ragSearchEditorRef.current}
              monaco={monacoRef.current}
              languageId={getLanguageFromPath(filePath)}
              projectId={activeProjectId}
              projectPath={cwd || null}
              onOpenFile={(path, line) => {
                if (onOpenFile) {
                  onOpenFile(path, line);
                } else {
                  console.warn("[CodeRAG] onOpenFile not provided");
                }
              }}
              onClose={() => setRagSearchOpen(false)}
            />
          )}
          
          {/* Loading overlay */}
          {isLoading && (
            <div 
              style={{
                position: 'absolute',
                top: 0, left: 0, right: 0, bottom: 0,
                backgroundColor: 'var(--bg-primary)',
                opacity: 0.7,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                zIndex: 50
              }}
            >
              <RefreshCw size={24} className="spin-animation" />
              <span style={{marginLeft: '8px'}}>Reading...</span>
            </div>
          )}

          {content && content.startsWith("data:image/") ? (
            <div style={{
              width: "100%",
              height: "100%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "var(--bg-secondary)",
              overflow: "auto",
              padding: "20px",
              boxSizing: "border-box"
            }}>
              <img 
                src={content} 
                alt={filePath || "Image"} 
                style={{
                  maxWidth: "100%",
                  maxHeight: "100%",
                  objectFit: "contain",
                  boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
                  backgroundColor: "transparent" /* To allow checking transparency */
                }} 
              />
            </div>
          ) : (
            <Editor
            height="100%"
            width="100%"
            language={getLanguageFromPath(filePath)}
            theme={theme === "light" ? "light" : "vs-dark"}
            onChange={handleEditorChange}
            beforeMount={handleBeforeMount}
            onMount={handleEditorDidMount}
            options={editorOptions}
          />
          )}
        </div>
      )}
    </div>
  );
});
