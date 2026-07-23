import React, { useState, useEffect, useRef, useCallback } from "react";
import {
  Save,
  FileCode,
  Check,
  AlertCircle,
  RefreshCw,
  FilePlus,
  Sparkles,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import Editor from "@monaco-editor/react";
import CodeMirror, { oneDark, gutter, GutterMarker } from "@uiw/react-codemirror";
import { linter, lintGutter, Diagnostic } from "@codemirror/lint";
import { showMinimap } from "@replit/codemirror-minimap";
import { loadLanguage } from "@uiw/codemirror-extensions-langs";
import { useEditorEngine } from "../hooks/useEditorEngine";
import { syntaxTree } from "@codemirror/language";
import { globalErrorStore } from "../services/errorStore";
import { editorStateStore } from "../services/editorStateStore";

class GitGutterMarker extends GutterMarker {
  type: string;
  constructor(type: string) {
    super();
    this.type = type;
  }
  toDOM() {
    const el = document.createElement("div");
    el.className = `cm-git-marker cm-git-marker-${this.type}`;
    return el;
  }
}

const gitAddedMarker = new GitGutterMarker("added");
const gitModifiedMarker = new GitGutterMarker("modified");
const gitDeletedMarker = new GitGutterMarker("deleted");
const gitUnsavedAddedMarker = new GitGutterMarker("unsaved_added");
const gitUnsavedModifiedMarker = new GitGutterMarker("unsaved_modified");

const getCodeMirrorLanguageExtension = (path: string | null) => {
  if (!path) return null;
  const fileName = path.split(/[\\/]/).pop()?.toLowerCase() || "";
  if (fileName.startsWith(".env")) return loadLanguage("ini");
  const ext = fileName.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "js":
      return loadLanguage("js");
    case "jsx":
      return loadLanguage("jsx");
    case "ts":
      return loadLanguage("ts");
    case "tsx":
      return loadLanguage("tsx");
    case "html":
    case "htm":
      return loadLanguage("html");
    case "css":
      return loadLanguage("css");
    case "json":
      return loadLanguage("json");
    case "md":
    case "markdown":
      return loadLanguage("markdown");
    case "rs":
    case "rust":
      return loadLanguage("rust" as any);
    case "py":
    case "python":
      return loadLanguage("python");
    case "go":
      return loadLanguage("go");
    case "sh":
    case "bash":
      return loadLanguage("bash");
    case "sql":
      return loadLanguage("sql");
    case "c":
    case "cpp":
    case "h":
    case "hpp":
      return loadLanguage("cpp");
    case "java":
      return loadLanguage("java");
    case "yaml":
    case "yml":
      return loadLanguage("yaml");
    case "toml":
      return loadLanguage("toml");
    default:
      return null;
  }
};
import { useGitDiff } from "../hooks/useGitDiff";
import CodeRagSearchWidget from "./CodeRagSearchWidget";
import {
  extractCodeContext,
  fetchAiCodeCompletion,
} from "../services/aiCompletionService";


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
  if (original === current) return [];
  const origLines = original.split("\n");
  const currLines = current.split("\n");
  const changes: UnsavedChange[] = [];

  // Strip common prefix
  let start = 0;
  while (
    start < origLines.length &&
    start < currLines.length &&
    origLines[start] === currLines[start]
  ) {
    start++;
  }

  // Strip common suffix
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

  const N = origEnd - start + 1;
  const M = currEnd - start + 1;

  if (N === 0) {
    for (let i = start; i <= currEnd; i++) {
      changes.push({ line: i + 1, type: "added", count: 0 });
    }
    return changes;
  }
  if (M === 0) {
    const lineNum = Math.min(start + 1, currLines.length);
    changes.push({ line: lineNum, type: "deleted_context", count: N });
    return changes;
  }

  // Fallback to bulk modified if the changed block is too large for fast DP
  if (N * M > 1000000) {
    for (let i = start; i <= currEnd; i++) {
      changes.push({ line: i + 1, type: "modified", count: 0 });
    }
    return changes;
  }

  // O(N*M) DP LCS
  const dp: number[][] = Array(N + 1).fill(0).map(() => Array(M + 1).fill(0));
  for (let i = 1; i <= N; i++) {
    for (let j = 1; j <= M; j++) {
      if (origLines[start + i - 1] === currLines[start + j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  let i = N;
  let j = M;
  const added = new Set<number>();
  const deletedBefore = new Map<number, number>();

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && origLines[start + i - 1] === currLines[start + j - 1]) {
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      added.add(start + j);
      j--;
    } else if (i > 0 && (j === 0 || dp[i][j - 1] < dp[i - 1][j])) {
      const nextLine = start + j + 1;
      deletedBefore.set(nextLine, (deletedBefore.get(nextLine) || 0) + 1);
      i--;
    }
  }

  for (let idx = start + 1; idx <= currEnd + 2; idx++) {
    if (added.has(idx)) {
      if (deletedBefore.has(idx)) {
        changes.push({ line: idx, type: "modified", count: 0 });
        deletedBefore.delete(idx);
      } else {
        changes.push({ line: idx, type: "added", count: 0 });
      }
    } else if (deletedBefore.has(idx)) {
      changes.push({ line: Math.min(idx, currLines.length), type: "deleted_context", count: deletedBefore.get(idx)! });
    }
  }

  return changes;
}

const editorOptions = {
  fontSize: 12,
  fontFamily: "'JetBrains Mono', Consolas, 'Courier New', monospace",
  minimap: { enabled: true, renderCharacters: false, scale: 0.5, maxColumn: 80, showSlider: "mouseover" as const },
  automaticLayout: true,
  scrollBeyondLastLine: false,
  cursorBlinking: "smooth" as const,
  lineNumbers: "on" as const,
  lineNumbersMinChars: 5,
  tabSize: 4,
  insertSpaces: true,
  wordWrap: "on" as const,
  renderLineHighlight: "all" as const,
  renderValidationDecorations: "editable" as const,
  glyphMargin: true,
  quickSuggestions: { other: true, comments: false, strings: false },
  suggestOnTriggerCharacters: true,
  quickSuggestionsDelay: 300,
  acceptSuggestionOnCommitCharacter: false,
  unicodeHighlight: { ambiguousCharacters: false },
  wordBasedSuggestions: "currentDocument" as const,
  inlineSuggest: {
    enabled: true,
    showToolbar: "onHover" as const,
    mode: "subword" as const,
  },
  suggest: {
    preview: false,
    showMethods: true,
    showFunctions: true,
    showConstructors: true,
    showFields: true,
    showVariables: true,
    showKeywords: true,
    showWords: true,
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
  const cmViewRef = useRef<any>(null);
  const lastPathRef = useRef<string | null>(null);
  const decorationIdsRef = useRef<string[]>([]);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Ref cho content mới nhất từ parent
  const initialContentRef = useRef<string | null>(initialContent);
  initialContentRef.current = initialContent;
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

  // ── AI Code Completion state ──
  const [aiCompletionEnabled, setAiCompletionEnabled] = useState<boolean>(
    () => localStorage.getItem("ai_completion_enabled") !== "false"
  );
  const aiCompletionEnabledRef = useRef(aiCompletionEnabled);
  useEffect(() => {
    aiCompletionEnabledRef.current = aiCompletionEnabled;
    localStorage.setItem("ai_completion_enabled", String(aiCompletionEnabled));
  }, [aiCompletionEnabled]);

  // ── Editor Engine (Monaco vs CodeMirror) via useEditorEngine hook ──
  const { editorEngine } = useEditorEngine();

  // ── Git diff decorations from backend (HEAD vs disk) ──
  const { diffLines, isUntracked } = useGitDiff({
    filePath,
    cwd,
    revision: saveRevision,
  });

  // ── Compute Git Diff Status map for CodeMirror (Synchronous 0ms delay for 60fps) ──
  const gitStatusMapRef = useRef<{ [line: number]: string }>({});
  const isUntrackedRef = useRef(isUntracked);
  isUntrackedRef.current = isUntracked;

  // Compute map synchronously during render pass so CodeMirror receives updates instantly (0ms lag)
  const synchronousGitMap: { [line: number]: string } = {};
  if (!isUntracked) {
    for (const d of diffLines) {
      if (d.change_type === "added") synchronousGitMap[d.line_number] = "added";
      else if (d.change_type === "modified") synchronousGitMap[d.line_number] = "modified";
      else if (d.change_type === "deleted_context") synchronousGitMap[d.line_number] = "deleted";
    }
  }
  for (const u of unsavedChanges) {
    if (u.type === "added") synchronousGitMap[u.line] = "unsaved_added";
    else if (u.type === "modified") synchronousGitMap[u.line] = "unsaved_modified";
    else if (u.type === "deleted_context") synchronousGitMap[u.line] = "deleted";
  }
  gitStatusMapRef.current = synchronousGitMap;

  // ── Native Monaco Error Count State ──
  const [monacoErrorCount, setMonacoErrorCount] = useState<number>(0);

  const filePathRef = useRef(filePath);
  useEffect(() => {
    filePathRef.current = filePath;
  }, [filePath]);

  // HTML & Inline Script Syntax Validator
  const validateHtmlModel = useCallback((model: any, monacoApi: any) => {
    if (!model || !monacoApi) return;

    const content = model.getValue();
    const markers: any[] = [];
    const lines = content.split("\n");

    const VOID_HTML_TAGS = new Set([
      "area", "base", "br", "col", "embed", "hr", "img", "input",
      "link", "meta", "param", "source", "track", "wbr", "!doctype"
    ]);

    const tagStack: { name: string; line: number; col: number }[] = [];
    let inScript = false;
    let scriptStartLine = -1;
    let scriptContentLines: string[] = [];

    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
      const lineNum = lineIdx + 1;
      const lineText = lines[lineIdx];

      // Collect & validate inline <script> JS code blocks
      if (inScript) {
        if (lineText.includes("</script>")) {
          inScript = false;
          const scriptCode = scriptContentLines.join("\n");
          if (scriptCode.trim()) {
            try {
              new Function(scriptCode);
            } catch (err: any) {
              let errLine = scriptStartLine;
              markers.push({
                severity: monacoApi.MarkerSeverity.Error,
                message: `JS Syntax Error inside <script>: ${err.message || err}`,
                startLineNumber: errLine,
                startColumn: 1,
                endLineNumber: Math.min(lineNum, lines.length),
                endColumn: lines[Math.min(lineNum, lines.length) - 1]?.length + 1 || 80,
              });
            }
          }
          scriptContentLines = [];
        } else {
          scriptContentLines.push(lineText);
        }
      } else if (lineText.includes("<script") && !lineText.includes("</script>")) {
        inScript = true;
        scriptStartLine = lineNum + 1;
        scriptContentLines = [];
      } else if (lineText.includes("<script") && lineText.includes("</script>")) {
        const scriptInner = lineText.substring(
          lineText.indexOf(">") + 1,
          lineText.lastIndexOf("</script>")
        );
        if (scriptInner.trim()) {
          try {
            new Function(scriptInner);
          } catch (err: any) {
            markers.push({
              severity: monacoApi.MarkerSeverity.Error,
              message: `JS Syntax Error in <script>: ${err.message || err}`,
              startLineNumber: lineNum,
              startColumn: Math.max(1, lineText.indexOf(">") + 1),
              endLineNumber: lineNum,
              endColumn: lineText.lastIndexOf("</script>") + 1,
            });
          }
        }
      }

      // HTML tag matcher regex
      const tagRegex = /<\/?([a-zA-Z0-9!-]+)(?:\s+[^>]*?)?(\/?)>/g;
      let match: RegExpExecArray | null;

      while ((match = tagRegex.exec(lineText)) !== null) {
        const fullTag = match[0];
        const rawTagName = match[1].toLowerCase();
        const isSelfClosing = match[2] === "/" || VOID_HTML_TAGS.has(rawTagName) || rawTagName.startsWith("!");
        const isClosing = fullTag.startsWith("</");
        const matchCol = match.index + 1;

        if (isClosing) {
          if (tagStack.length === 0) {
            markers.push({
              severity: monacoApi.MarkerSeverity.Error,
              message: `Unmatched closing tag </${rawTagName}>`,
              startLineNumber: lineNum,
              startColumn: matchCol,
              endLineNumber: lineNum,
              endColumn: matchCol + fullTag.length,
            });
          } else {
            const topTag = tagStack[tagStack.length - 1];
            if (topTag.name === rawTagName) {
              tagStack.pop();
            } else {
              const matchingIndex = tagStack.findLastIndex((t) => t.name === rawTagName);
              if (matchingIndex !== -1) {
                for (let i = tagStack.length - 1; i > matchingIndex; i--) {
                  const unclosed = tagStack[i];
                  markers.push({
                    severity: monacoApi.MarkerSeverity.Error,
                    message: `Unclosed HTML tag <${unclosed.name}>`,
                    startLineNumber: unclosed.line,
                    startColumn: unclosed.col,
                    endLineNumber: unclosed.line,
                    endColumn: unclosed.col + unclosed.name.length + 2,
                  });
                }
                tagStack.length = matchingIndex;
              } else {
                markers.push({
                  severity: monacoApi.MarkerSeverity.Error,
                  message: `Unexpected closing tag </${rawTagName}> (expected </${topTag.name}>)`,
                  startLineNumber: lineNum,
                  startColumn: matchCol,
                  endLineNumber: lineNum,
                  endColumn: matchCol + fullTag.length,
                });
              }
            }
          }
        } else if (!isSelfClosing) {
          tagStack.push({ name: rawTagName, line: lineNum, col: matchCol });
        }
      }
    }

    for (const unclosed of tagStack) {
      if (!VOID_HTML_TAGS.has(unclosed.name)) {
        markers.push({
          severity: monacoApi.MarkerSeverity.Error,
          message: `Unclosed HTML tag <${unclosed.name}>`,
          startLineNumber: unclosed.line,
          startColumn: unclosed.col,
          endLineNumber: unclosed.line,
          endColumn: unclosed.col + unclosed.name.length + 2,
        });
      }
    }

    monacoApi.editor.setModelMarkers(model, "html-custom-validator", markers);
  }, []);

  // Sync all Monaco models markers across all open files into globalErrorStore
  const syncAllMonacoMarkers = useCallback(() => {
    const monacoInstance = monacoRef.current;
    if (!monacoInstance) return;

    const models = monacoInstance.editor.getModels();
    models.forEach((model: any) => {
      const langId = model.getLanguageId() || "";
      const uriPath = model.uri?.path || "";

      if (langId === "html" || uriPath.endsWith(".html") || uriPath.endsWith(".htm")) {
        validateHtmlModel(model, monacoInstance);
      }

      const markers = monacoInstance.editor.getModelMarkers({ resource: model.uri });
      const errors = markers.filter((m: any) => m.severity === monacoInstance.MarkerSeverity.Error);

      let targetPath = model.uri.fsPath || model.uri.path || "";
      if (!targetPath && filePathRef.current) {
        targetPath = filePathRef.current;
      }

      if (targetPath) {
        globalErrorStore.setFileError(targetPath, errors.length);
      }
    });

    if (editorRef.current) {
      const activeModel = editorRef.current.getModel();
      if (activeModel) {
        const activeMarkers = monacoInstance.editor.getModelMarkers({ resource: activeModel.uri });
        const activeErrors = activeMarkers.filter((m: any) => m.severity === monacoInstance.MarkerSeverity.Error);
        setMonacoErrorCount(activeErrors.length);
        if (filePathRef.current) {
          globalErrorStore.setFileError(filePathRef.current, activeErrors.length);
        }
      }
    }
  }, [validateHtmlModel]);

  // Re-sync markers when active file or content changes
  useEffect(() => {
    syncAllMonacoMarkers();
    const timer1 = setTimeout(syncAllMonacoMarkers, 500);
    const timer2 = setTimeout(syncAllMonacoMarkers, 1500);
    const timer3 = setTimeout(syncAllMonacoMarkers, 3000);
    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
      clearTimeout(timer3);
    };
  }, [filePath, content, syncAllMonacoMarkers]);

  const restoreMonacoState = useCallback((targetPath: string) => {
    const editor = editorRef.current;
    if (!editor || !targetPath) return;

    const saved = editorStateStore.getFileState(targetPath);
    if (saved) {
      if (saved.viewState) {
        try {
          editor.restoreViewState(saved.viewState);
        } catch (e) {}
      }
      if (saved.lineNumber && saved.column) {
        editor.setPosition({ lineNumber: saved.lineNumber, column: saved.column });
        editor.revealPositionInCenter({ lineNumber: saved.lineNumber, column: saved.column });
      }
      if (typeof saved.scrollTop === "number") {
        try {
          editor.setScrollTop(saved.scrollTop);
        } catch (e) {}
      }
    }

    try {
      editor.focus();
    } catch (e) {}
  }, []);

  const restoreCodeMirrorState = useCallback((targetPath: string) => {
    const view = cmViewRef.current;
    if (!view || !targetPath) return;

    const saved = editorStateStore.getFileState(targetPath);
    if (!saved) {
      try {
        view.focus();
      } catch (e) {}
      return;
    }

    try {
      const doc = view.state.doc;
      if (!doc || doc.length === 0) {
        view.focus();
        return;
      }

      const lineNum = Math.min(Math.max(1, saved.lineNumber), doc.lines);
      const line = doc.line(lineNum);
      const col = Math.min(Math.max(1, saved.column), line.length + 1);
      const pos = line.from + col - 1;

      view.dispatch({
        selection: { anchor: pos, head: pos },
        scrollIntoView: true,
      });

      if (typeof saved.scrollTop === "number" && view.scrollDOM) {
        view.scrollDOM.scrollTop = saved.scrollTop;
      }
      if (typeof saved.scrollLeft === "number" && view.scrollDOM) {
        view.scrollDOM.scrollLeft = saved.scrollLeft;
      }

      view.focus();
    } catch (e) {
      console.warn("Error restoring CodeMirror position:", e);
    }
  }, []);

  const restoreFileState = useCallback((targetPath: string) => {
    if (!targetPath) return;

    const doRestore = () => {
      if (editorEngine === "codemirror") {
        restoreCodeMirrorState(targetPath);
      } else {
        restoreMonacoState(targetPath);
      }
    };

    doRestore();
    setTimeout(doRestore, 30);
    setTimeout(doRestore, 100);
  }, [editorEngine, restoreCodeMirrorState, restoreMonacoState]);

  // Restore cursor & view state whenever active filePath or content changes
  useEffect(() => {
    if (filePath) {
      restoreFileState(filePath);
    }
  }, [filePath, content, restoreFileState]);

  // ── Notify App & File Explorer when error status changes for active file ──
  useEffect(() => {
    if (filePath) {
      window.dispatchEvent(
        new CustomEvent("file-syntax-error-change", {
          detail: {
            filePath,
            hasError: monacoErrorCount > 0,
          },
        })
      );
    }
  }, [monacoErrorCount, filePath]);

  const cmExtensions = React.useMemo(() => {
    const exts: any[] = [];

    // Git Diff Gutter Marker
    const gitGutterExt = gutter({
      class: "cm-git-gutter",
      lineMarker(view, line) {
        if (isUntrackedRef.current) return gitAddedMarker;
        const lineNumber = view.state.doc.lineAt(line.from).number;
        const status = gitStatusMapRef.current[lineNumber];
        if (status === "added") return gitAddedMarker;
        if (status === "modified") return gitModifiedMarker;
        if (status === "deleted") return gitDeletedMarker;
        if (status === "unsaved_added") return gitUnsavedAddedMarker;
        if (status === "unsaved_modified") return gitUnsavedModifiedMarker;
        return null;
      },
      initialSpacer: () => gitAddedMarker,
    });
    exts.push(gitGutterExt);

    // Code Minimap Extension (Dynamic Git gutters from Ref without extension re-creation)
    const minimapExt = showMinimap.compute([], () => {
      const minimapGuttersMap: { [line: number]: string } = {};
      if (isUntrackedRef.current) {
        minimapGuttersMap[1] = "#2da44e";
      } else {
        for (const [lineStr, status] of Object.entries(gitStatusMapRef.current)) {
          const line = parseInt(lineStr, 10);
          if (status === "added") minimapGuttersMap[line] = "#2da44e";
          else if (status === "modified") minimapGuttersMap[line] = "#d29922";
          else if (status === "deleted") minimapGuttersMap[line] = "#da3633";
          else if (status === "unsaved_added") minimapGuttersMap[line] = "#58a6ff";
          else if (status === "unsaved_modified") minimapGuttersMap[line] = "#79c0ff";
        }
      }
      return {
        create: () => {
          const dom = document.createElement("div");
          dom.className = "cm-minimap-wrapper";
          return { dom };
        },
        displayText: "characters",
        showOverlay: "always",
        gutters: [minimapGuttersMap],
      };
    });
    exts.push(minimapExt);

    // CodeMirror Lint Gutter & Native Lezer AST Syntax Linter
    exts.push(lintGutter());

    const cmLezerLinter = linter((view) => {
      const diagnostics: Diagnostic[] = [];
      const tree = syntaxTree(view.state);

      tree.iterate({
        enter(node) {
          if (node.type.isError || node.name === "⚠" || node.name === "Error") {
            const from = Math.max(0, node.from);
            const to = Math.min(
              view.state.doc.length,
              node.to > node.from ? node.to : node.from + 1
            );
            diagnostics.push({
              from,
              to,
              severity: "error",
              message: "Cú pháp không hợp lệ (Syntax Error)",
            });
          }
        },
      });

      if (filePathRef.current) {
        globalErrorStore.setFileError(filePathRef.current, diagnostics.length);
      }
      setMonacoErrorCount(diagnostics.length);
      return diagnostics;
    });
    exts.push(cmLezerLinter);

    // Syntax Highlighting
    const langExt = getCodeMirrorLanguageExtension(filePath);
    if (langExt) exts.push(langExt);

    return exts;
  }, [filePath]);

  
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
            minimap: { color: "#2ea043", position: monacoApi.editor.MinimapPosition.Gutter },
            overviewRuler: { color: "#2ea043", position: monacoApi.editor.OverviewRulerLane.Left }
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
                minimap: { color: "#2ea043", position: monacoApi.editor.MinimapPosition.Gutter },
                overviewRuler: { color: "#2ea043", position: monacoApi.editor.OverviewRulerLane.Left }
              },
            });
            break;
          case "modified":
            decorations.push({
              range: new monacoApi.Range(lineNum, 1, lineNum, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: "git-glyph-modified",
                minimap: { color: "#007acc", position: monacoApi.editor.MinimapPosition.Gutter },
                overviewRuler: { color: "#007acc", position: monacoApi.editor.OverviewRulerLane.Left }
              },
            });
            break;
          case "deleted_context":
            decorations.push({
              range: new monacoApi.Range(lineNum, 1, lineNum, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: "git-glyph-deleted",
                minimap: { color: "#f85149", position: monacoApi.editor.MinimapPosition.Gutter },
                overviewRuler: { color: "#f85149", position: monacoApi.editor.OverviewRulerLane.Left }
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
              minimap: { color: "#2ea043", position: monacoApi.editor.MinimapPosition.Gutter },
              overviewRuler: { color: "#2ea043", position: monacoApi.editor.OverviewRulerLane.Left }
            },
          });
          break;
        case "modified":
          decorations.push({
            range: new monacoApi.Range(lineNum, 1, lineNum, 1),
            options: {
              isWholeLine: true,
              glyphMarginClassName: "git-glyph-unsaved-modified",
              minimap: { color: "#007acc", position: monacoApi.editor.MinimapPosition.Gutter },
              overviewRuler: { color: "#007acc", position: monacoApi.editor.OverviewRulerLane.Left }
            },
          });
          break;
        case "deleted_context":
          decorations.push({
            range: new monacoApi.Range(lineNum, 1, lineNum, 1),
            options: {
              isWholeLine: true,
              glyphMarginClassName: "git-glyph-unsaved-deleted",
              minimap: { color: "#f85149", position: monacoApi.editor.MinimapPosition.Gutter },
              overviewRuler: { color: "#f85149", position: monacoApi.editor.OverviewRulerLane.Left }
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
        const pos = editor.getPosition();
        if (pos) {
          const viewState = editor.saveViewState();
          editorStateStore.saveFileState(filePath, pos.lineNumber, pos.column, viewState);
        }
      }
    };
  }, [filePath]);

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
          setTimeout(() => {
            const saved = editorStateStore.getFileState(filePath);
            if (saved) {
              if (saved.viewState) {
                try {
                  editor.restoreViewState(saved.viewState);
                } catch (e) {}
              }
              if (saved.lineNumber && saved.column) {
                editor.setPosition({ lineNumber: saved.lineNumber, column: saved.column });
                editor.revealPositionInCenter({ lineNumber: saved.lineNumber, column: saved.column });
              }
            }
          }, 50);
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
    // Enable syntax validation & diagnostics for Monaco languages
    if (monaco.languages?.typescript) {
      monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: false,
        noSyntaxValidation: false,
      });
      monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: false,
        noSyntaxValidation: false,
      });
    }
    if (monaco.languages?.json) {
      monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
        validate: true,
        allowComments: true,
      });
    }
    if (monaco.languages?.html) {
      if (monaco.languages.html.htmlDefaults?.setOptions) {
        try {
          monaco.languages.html.htmlDefaults.setOptions({
            format: { tabSize: 2, insertSpaces: true },
            suggest: { html5: true },
            options: { validate: true }
          });
        } catch (e) {}
      }
    }

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
      html: [
        "div", "span", "p", "a", "button", "input", "form", "h1", "h2", "h3", "h4",
        "header", "footer", "main", "nav", "section", "article", "aside", "style",
        "script", "link", "meta", "title", "head", "body", "html", "img", "ul", "li",
        "ol", "table", "tr", "td", "th", "select", "option", "textarea", "label"
      ],
      css: [
        "display", "position", "flex", "grid", "margin", "padding", "width", "height",
        "color", "background", "border", "border-radius", "box-shadow", "font-size",
        "font-family", "font-weight", "align-items", "justify-content", "flex-direction",
        "overflow", "cursor", "transition", "transform", "opacity", "z-index", "absolute",
        "relative", "fixed", "sticky", "none", "block", "inline-block", "center", "hidden"
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

    // ── AI Inline Code Completion Provider (localhost:3001) ──
    let inlineDebounceTimer: any = null;

    try {
      monaco.languages.registerInlineCompletionsProvider("*", {
        provideInlineCompletions: async (model: any, position: any, _context: any, token: any) => {
          if (!aiCompletionEnabledRef.current) {
            return { items: [] };
          }

          const isManual = _context?.triggerKind === 1;

          // Nếu là tự động khi gõ: Đợi dừng gõ đúng 2 giây (2000ms) mới phát request tiết kiệm token
          if (!isManual) {
            await new Promise<void>((resolve) => {
              if (inlineDebounceTimer) clearTimeout(inlineDebounceTimer);
              inlineDebounceTimer = setTimeout(() => {
                resolve();
              }, 2000);
            });
          }

          if (token.isCancellationRequested) {
            return { items: [] };
          }

          // 1,000 token sliding window (800 prefix / 200 suffix)
          const { prefix, suffix } = extractCodeContext(model, position);
          if (!prefix.trim() && !suffix.trim()) {
            return { items: [] };
          }

          const langId = model.getLanguageId() || "plaintext";

          const completionText = await fetchAiCodeCompletion({
            prefix,
            suffix,
            language: langId,
            isManualTrigger: isManual,
          });

          if (!completionText || token.isCancellationRequested) {
            return { items: [] };
          }

          return {
            items: [
              {
                insertText: completionText,
                range: new monaco.Range(
                  position.lineNumber,
                  position.column,
                  position.lineNumber,
                  position.column
                ),
              },
            ],
          };
        },
        freeInlineCompletions: () => {},
      });

    } catch (e) {
      console.warn("Inline completion provider registration error:", e);
    }
  };


  const handleEditorDidMount = (editor: any, monacoInstance: any) => {
    editorRef.current = editor;
    monacoRef.current = monacoInstance;

    // Sync Monaco Native Marker Errors for Header Badge & File Explorer / Tabs
    const markerListener = monacoInstance.editor.onDidChangeMarkers(() => {
      syncAllMonacoMarkers();
    });
    setTimeout(syncAllMonacoMarkers, 500);
    setTimeout(syncAllMonacoMarkers, 1500);

    // Restore saved cursor position and view state for initial file
    if (filePathRef.current) {
      restoreFileState(filePathRef.current);
    }

    // Restore saved cursor position whenever Monaco swaps models (tab switch)
    const modelChangeListener = editor.onDidChangeModel(() => {
      if (filePathRef.current) {
        restoreFileState(filePathRef.current);
      }
    });

    // Save cursor position and view state on position change (user typing or mouse click only)
    const posListener = editor.onDidChangeCursorPosition((e: any) => {
      if (filePathRef.current) {
        if (e.source === "keyboard" || e.source === "mouse" || e.source === "api.command") {
          const viewState = editor.saveViewState();
          editorStateStore.saveFileState(
            filePathRef.current,
            e.position.lineNumber,
            e.position.column,
            viewState
          );
        }
      }
    });

    // Save view state on scroll change
    const scrollListener = editor.onDidScrollChange((e: any) => {
      if (filePathRef.current && e.isExplicit) {
        const pos = editor.getPosition();
        if (pos) {
          const viewState = editor.saveViewState();
          editorStateStore.saveFileState(
            filePathRef.current,
            pos.lineNumber,
            pos.column,
            viewState
          );
        }
      }
    });

    editor.onDidDispose(() => {
      if (markerListener) markerListener.dispose();
      if (posListener) posListener.dispose();
      if (scrollListener) scrollListener.dispose();
      if (modelChangeListener) modelChangeListener.dispose();
    });

    // ── Add context menu action: Sparkles AI Code Suggestion (Alt + \) ──
    editor.addAction({
      id: "ai-code-completion-trigger",
      label: "✨ Gợi ý Code bằng AI (Alt+\\)",
      keybindings: [monacoInstance.KeyMod.Alt | monacoInstance.KeyCode.Backslash],
      contextMenuGroupId: "navigation",
      contextMenuOrder: 1.4,
      run: (ed: any) => {
        ed.trigger("ai-completion", "editor.action.inlineSuggest.trigger", {});
      },
    });

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

    const currentInitialContent = initialContentRef.current;
    if (currentInitialContent !== null && editor.getValue() !== currentInitialContent) {
      editor.setValue(currentInitialContent);
    }
    setTimeout(() => {
      const domNode = editor.getDomNode();
      if (domNode) {
        const textArea = domNode.querySelector('textarea');
        if (textArea) {
          textArea.setAttribute('autocomplete', 'off');
          textArea.setAttribute('autocorrect', 'off');
          textArea.setAttribute('autocapitalize', 'off');
          textArea.setAttribute('spellcheck', 'false');
        }
      }
    }, 200);

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
          <FileCode size={14} className={`file-icon ${monacoErrorCount > 0 ? "has-error" : ""}`} />
          <span className={`file-name ${monacoErrorCount > 0 ? "has-error" : ""}`}>{fileName}</span>
          {isDirty && <span className="dirty-dot" title="Unsaved changes" />}
          {isUntracked && (
            <span className="untracked-badge" title="File chưa được commit">
              <FilePlus size={10} />
            </span>
          )}
          <span className="file-path">{filePath}</span>
        </div>
        <div className="editor-actions">
          <button
            className={`btn-ai-completion ${aiCompletionEnabled ? "active" : ""}`}
            onClick={() => setAiCompletionEnabled((prev) => !prev)}
            title="Gợi ý code bằng AI (server http://localhost:3001 - 800 prefix / 200 suffix token)"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "4px",
              padding: "3px 8px",
              fontSize: "11px",
              borderRadius: "4px",
              border: "1px solid var(--border-color, rgba(255,255,255,0.15))",
              background: aiCompletionEnabled ? "rgba(99, 102, 241, 0.15)" : "transparent",
              color: aiCompletionEnabled ? "#818cf8" : "var(--text-muted, #888)",
              cursor: "pointer",
              marginRight: "6px",
              transition: "all 0.15s ease",
            }}
          >
            <Sparkles size={12} style={{ color: aiCompletionEnabled ? "#818cf8" : "#888" }} />
            <span>AI Suggest {aiCompletionEnabled ? "ON" : "OFF"}</span>
          </button>
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
          ) : editorEngine === "codemirror" ? (
            <CodeMirror
              value={content}
              height="100%"
              width="100%"
              theme={theme === "light" ? "light" : oneDark}
              extensions={cmExtensions}
              onCreateEditor={(view) => {
                cmViewRef.current = view;
                if (filePathRef.current) {
                  restoreFileState(filePathRef.current);
                }
              }}
              onUpdate={(update) => {
                if (filePathRef.current && update.state) {
                  const isUserInteraction = update.transactions.some(
                    (tr: any) =>
                      tr.isUserEvent?.("select") ||
                      tr.isUserEvent?.("input") ||
                      tr.isUserEvent?.("delete") ||
                      tr.isUserEvent?.("move") ||
                      tr.isUserEvent?.("undo") ||
                      tr.isUserEvent?.("redo")
                  );

                  if (isUserInteraction) {
                    try {
                      const pos = update.state.selection.main.head;
                      const line = update.state.doc.lineAt(pos);
                      const lineNumber = line.number;
                      const column = pos - line.from + 1;
                      const scrollTop = update.view?.scrollDOM?.scrollTop || 0;
                      const scrollLeft = update.view?.scrollDOM?.scrollLeft || 0;
                      editorStateStore.saveFileState(
                        filePathRef.current,
                        lineNumber,
                        column,
                        null,
                        scrollTop,
                        scrollLeft
                      );
                    } catch (e) {}
                  }
                }
              }}
              onChange={(val) => handleEditorChange(val)}
              style={{ fontSize: "13px", height: "100%", width: "100%", flex: 1 }}
            />
          ) : (
            <Editor
              height="100%"
              width="100%"
              path={filePath || undefined}
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
