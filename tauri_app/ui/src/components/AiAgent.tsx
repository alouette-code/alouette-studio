import React, { useState, useRef, useEffect, useMemo } from "react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import DOMPurify from "dompurify";
import {
  Plus,
  RefreshCw,
  Layers,
  History,
  ArrowLeft,
  Shield,
  Terminal,
  Database,
  Globe,
  Cpu,
  Activity,
  Hammer,
  Lock,
  Unlock,
  Zap,
  GitBranch,
  X,
  Copy,
  Check,
  ChevronDown,
  ChevronRight,
  AlertCircle,
  Pencil,
  Wrench,
  Box,
  CornerDownLeft,
  Sparkles,
  FileText,
  Search,
  FolderOpen,
  Folder,
  File,
  Tag,
  Save,
  Brain,
  Plug,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ToolItem {
  name: string;
  args: string;
  pending_id: string;
  status?:
    | "waiting"
    | "approved"
    | "rejected"
    | "running"
    | "success"
    | "failed";
}

interface ChatItem {
  id: string;
  type:
    | "text"
    | "tool_request"
    | "tool_batch_request"
    | "agent_activity"
    | "alouette_error"
    | "skill_call";
  sender: "user" | "agent";
  text?: string;
  toolName?: string;
  args?: string;
  tools?: ToolItem[];
  toolStatus?:
    | "waiting"
    | "approved"
    | "rejected"
    | "running"
    | "success"
    | "failed";
  toolResult?: string;
  timestamp: string;
  projectName?: string;
  errorText?: string;
}

interface AiAgentProps {
  onBack?: () => void;
  activeProjectCwd?: string;
  activeProjectId?: string;
  initialSessionData?: any;
  onClearInitialSessionData?: () => void;
  onLoadSession?: (sessionId: string, title: string) => void;
  initialMessage?: string;
  onClearInitialMessage?: () => void;
  variant?: "sidebar" | "full";
}

// ─── Tool Card Item (compact, expandable) ───────────────────────────────

interface ToolCardItemProps {
  tool: ToolItem;
  index: number;
  onApprove: (toolIndex: number) => void;
  onReject: (toolIndex: number) => void;
}

function ToolCardItem({ tool, index, onApprove, onReject }: ToolCardItemProps) {
  const [expanded, setExpanded] = useState(false);
  const [approved, setApproved] = useState(tool.status === "approved");
  const [rejected, setRejected] = useState(tool.status === "rejected");

  if (approved || rejected) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "6px",
          padding: "3px 6px",
          borderRadius: "4px",
          background: approved
            ? "rgba(34, 197, 94, 0.06)"
            : "rgba(239, 68, 68, 0.06)",
          opacity: 0.7,
          fontSize: "10px",
          flexShrink: 0,
        }}
      >
        <span
          style={{ color: approved ? "#22c55e" : "#ef4444", fontSize: "9px" }}
        >
          {approved ? "✓" : "✕"}
        </span>
        <span style={{ color: "var(--text-muted)" }}>
          {getFriendlyToolNameStatic(tool.name, tool.args)}
        </span>
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        border: "1px solid var(--border-primary)",
        borderRadius: "4px",
        overflow: "hidden",
        flexShrink: 0,
      }}
    >
      {/* Mini header */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "3px 6px",
          cursor: "pointer",
          userSelect: "none",
          background: expanded ? "rgba(255,255,255,0.02)" : "transparent",
          transition: "background 0.15s",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
          {expanded ? (
            <ChevronDown
              size={9}
              style={{ color: "var(--text-muted)", flexShrink: 0 }}
            />
          ) : (
            <ChevronRight
              size={9}
              style={{ color: "var(--text-muted)", flexShrink: 0 }}
            />
          )}
          {getToolIconComponent(tool.name)}
          <span
            style={{
              fontSize: "9.5px",
              fontWeight: 600,
              color: "var(--text-primary)",
            }}
          >
            {getFriendlyToolNameStatic(tool.name, tool.args)}
          </span>
        </div>

        {/* Approve/Reject buttons (always visible) */}
        <div style={{ display: "flex", gap: "3px" }}>
          <button
            onClick={(e) => {
              e.stopPropagation();
              setRejected(true);
              onReject(index);
            }}
            title="Từ chối"
            style={{
              padding: "2px",
              background: "transparent",
              border: "1px solid var(--border-primary)",
              color: "var(--text-secondary)",
              borderRadius: "3px",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              lineHeight: 1,
            }}
          >
            <X size={9} />
          </button>
          <button
            onClick={(e) => {
              e.stopPropagation();
              setApproved(true);
              onApprove(index);
            }}
            title="Đồng ý chạy"
            style={{
              padding: "2px",
              background: "var(--border-strong, #374151)",
              border: "none",
              color: "#fff",
              borderRadius: "3px",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              lineHeight: 1,
            }}
          >
            <Check size={9} />
          </button>
        </div>
      </div>

      {/* Expanded args */}
      {expanded && tool.args && tool.args !== "{}" && (
        <div
          style={{
            borderTop: "1px solid var(--border-primary)",
            background: "var(--bg-primary)",
            padding: "4px 6px",
          }}
        >
          <pre
            style={{
              margin: 0,
              fontFamily: "var(--font-mono)",
              fontSize: "9px",
              color: "var(--text-secondary)",
              whiteSpace: "pre-wrap",
              wordBreak: "break-all",
              lineHeight: "1.4",
              maxHeight: "100px",
              overflowY: "auto",
            }}
          >
            {tool.args}
          </pre>
        </div>
      )}
    </div>
  );
}

function getToolIconComponent(name: string): React.ReactNode {
  const size = 11;
  const style = { flexShrink: 0, color: "var(--text-secondary)" };
  switch (name) {
    case "read_file":
    case "read_file_range":
      return <FileText size={size} style={style} />;
    case "write_file":
      return <Pencil size={size} style={style} />;
    case "execute_command":
      return <Terminal size={size} style={style} />;
    case "search_files":
      return <Search size={size} style={style} />;
    case "scan_directory_tree":
    case "scan_subdirectory":
      return <FolderOpen size={size} style={style} />;
    case "extract_symbol":
    case "search_symbol":
      return <Tag size={size} style={style} />;
    case "save_memory":
      return <Save size={size} style={style} />;
    case "search_memory":
      return <Brain size={size} style={style} />;
    case "check_port":
      return <Plug size={size} style={style} />;
    default:
      return <Wrench size={size} style={style} />;
  }
}

function getFriendlyToolNameStatic(name: string, argsStr?: string): string {
  if (name === "execute_command" && argsStr) {
    try {
      const parsed = JSON.parse(argsStr);
      if (parsed.command) {
        return `Chạy lệnh: ${parsed.command}`;
      }
    } catch (e) {
      // ignore
    }
  }
  switch (name) {
    case "read_file":
    case "read_file_range":
      return "Đọc tệp tin";
    case "write_file":
      return "Ghi tệp tin";
    case "execute_command":
      return "Chạy lệnh terminal";
    case "search_files":
      return "Tìm kiếm tệp";
    case "scan_directory_tree":
    case "scan_subdirectory":
      return "Quét thư mục";
    case "extract_symbol":
    case "search_symbol":
      return "Truy xuất mã nguồn";
    case "save_memory":
      return "Lưu ký ức";
    case "search_memory":
      return "Tìm kiếm ký ức";
    case "check_port":
      return "Kiểm tra cổng mạng";
    default:
      return name;
  }
}

// ─── Single Tool Request Card (compact, expandable) ────────────────────

interface SingleToolRequestCardProps {
  item: ChatItem;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
}

function SingleToolRequestCard({
  item,
  onApprove,
  onReject,
}: SingleToolRequestCardProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      className="message-container agent-fade-in"
      style={{
        borderRadius: "6px",
        border: "1px solid var(--border-primary)",
        background: "var(--bg-secondary)",
        padding: "6px 10px",
        display: "flex",
        flexDirection: "column",
        gap: "4px",
        flexShrink: 0,
      }}
    >
      {/* Compact header */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          cursor: "pointer",
          userSelect: "none",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "4px",
          }}
        >
          {expanded ? (
            <ChevronDown
              size={9}
              style={{ color: "var(--text-muted)", flexShrink: 0 }}
            />
          ) : (
            <ChevronRight
              size={9}
              style={{ color: "var(--text-muted)", flexShrink: 0 }}
            />
          )}
          {getToolIconComponent(item.toolName || "")}
          <span
            style={{
              fontSize: "9.5px",
              fontWeight: 600,
              color: "var(--text-primary)",
            }}
          >
            {getFriendlyToolNameStatic(item.toolName || "", item.args)}
          </span>
        </div>
        <span style={{ fontSize: "9px", color: "var(--text-muted)" }}>
          {item.timestamp}
        </span>
      </div>

      {/* Expanded args */}
      {expanded && item.args && item.args !== "{}" && (
        <pre
          style={{
            margin: 0,
            padding: "4px 6px",
            background: "var(--bg-primary)",
            border: "1px solid var(--border-primary)",
            borderRadius: "4px",
            fontFamily: "var(--font-mono)",
            fontSize: "9px",
            color: "var(--text-secondary)",
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
            lineHeight: "1.4",
            maxHeight: "120px",
            overflowY: "auto",
          }}
        >
          {item.args}
        </pre>
      )}

      {/* Actions */}
      {item.toolStatus === "waiting" && (
        <div
          style={{
            display: "flex",
            gap: "4px",
            justifyContent: "flex-end",
          }}
        >
          <button
            onClick={() => onReject(item.id)}
            style={{
              padding: "2px 8px",
              fontSize: "9px",
              background: "transparent",
              border: "1px solid var(--border-primary)",
              color: "var(--text-secondary)",
              cursor: "pointer",
              borderRadius: "3px",
              display: "inline-flex",
              alignItems: "center",
              gap: "3px",
              fontWeight: 500,
              lineHeight: "18px",
            }}
          >
            <X size={9} /> Từ chối
          </button>
          <button
            onClick={() => onApprove(item.id)}
            style={{
              padding: "2px 8px",
              fontSize: "9px",
              background: "var(--border-strong, #374151)",
              border: "none",
              color: "#fff",
              cursor: "pointer",
              borderRadius: "3px",
              display: "inline-flex",
              alignItems: "center",
              gap: "3px",
              fontWeight: 600,
              lineHeight: "18px",
            }}
          >
            <Check size={9} /> Đồng ý chạy
          </button>
        </div>
      )}

      {item.toolStatus === "approved" && (
        <div
          style={{
            fontSize: "9px",
            color: "#22c55e",
            textAlign: "right",
            display: "flex",
            alignItems: "center",
            justifyContent: "flex-end",
            gap: "3px",
          }}
        >
          <Check size={9} /> Đã chấp thuận
        </div>
      )}

      {item.toolStatus === "rejected" && (
        <div
          style={{
            fontSize: "9px",
            color: "var(--text-muted)",
            textAlign: "right",
            display: "flex",
            alignItems: "center",
            justifyContent: "flex-end",
            gap: "3px",
            fontStyle: "italic",
          }}
        >
          <X size={9} /> Đã từ chối
        </div>
      )}
    </div>
  );
}

export default function AiAgent({
  onBack,
  activeProjectCwd,
  activeProjectId,
  initialSessionData,
  onClearInitialSessionData,
  onLoadSession,
  initialMessage,
  onClearInitialMessage,
  variant = "sidebar",
}: AiAgentProps) {
  const [chatHistory, setChatHistory] = useState<ChatItem[]>([]);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [historyModalOpen, setHistoryModalOpen] = useState(false);
  const [historyItems, setHistoryItems] = useState<any[]>([]);

  const [activeTool, setActiveTool] = useState<{
    status: "executing" | "idle";
    tool_name?: string;
    args?: string;
  }>({ status: "idle" });
  const [alouetteError, setAlouetteError] = useState<{
    id: string;
    projectName: string;
    errorText: string;
  } | null>(null);
  const [loopIterations, setLoopIterations] = useState<number>(0);
  const [totalIterations, setTotalIterations] = useState<number>(25);
  const [activeThought, setActiveThought] = useState<string | null>(null);

  const [inputVal, setInputVal] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [selectedModel, setSelectedModel] = useState("gemini-3.5-flash");
  const [selectedMode, setSelectedMode] = useState("interactive");
  const [menuOpen, setMenuOpen] = useState(false);
  const [modelDropdownOpen, setModelDropdownOpen] = useState(false);
  const [modeDropdownOpen, setModeDropdownOpen] = useState(false);
  const [sessionTitle, setSessionTitle] = useState("New Chat");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [thinkingMode, setThinkingMode] = useState<"high" | "low">("low");
  const [showTokenTooltip, setShowTokenTooltip] = useState(false);
  const [totalSessionTokens, setTotalSessionTokens] = useState<number>(0);
  const [expandedSkills, setExpandedSkills] = useState<Record<string, boolean>>(
    {},
  );

  // ─── Token Estimation ────────────────────────────────────────────────
  const estimateTokens = (text: string): number => {
    if (!text) return 0;
    // Heuristic: ~4 ký tự ≈ 1 token (ước lượng)
    return Math.max(1, Math.ceil(text.length / 4));
  };

  const estimatedInputTokens = useMemo(
    () => estimateTokens(inputVal),
    [inputVal],
  );

  const tokenBreakdown = useMemo(() => {
    let total = 0;
    const perMsg: number[] = [];
    for (const msg of chatHistory) {
      const t = msg.text ? estimateTokens(msg.text) : 0;
      perMsg.push(t);
      total += t;
    }
    return { total, perMsg };
  }, [chatHistory]);

  // ─── Multi-Session: Switch Project ─────────────────────────────────
  const prevProjectIdRef = useRef<string | undefined>(undefined);
  const switchSeqRef = useRef<number>(0);
  const switchingRef = useRef<boolean>(false);

  // Badge: hiển thị trạng thái agent cũ
  const [agentBadge, setAgentBadge] = useState<{
    visible: boolean;
    text: string;
    type: "paused" | "running" | "idle";
  }>({ visible: false, text: "", type: "idle" });

  // Mentions (@) Autocomplete states
  const [allWorkspaceFiles, setAllWorkspaceFiles] = useState<
    Array<{ name: string; path: string; is_dir: boolean }>
  >([]);
  const [showMentions, setShowMentions] = useState(false);
  const [mentionQuery, setMentionQuery] = useState("");
  const [mentionTriggerIndex, setMentionTriggerIndex] = useState(-1);
  const [mentionSelectedIndex, setMentionSelectedIndex] = useState(0);
  const [selectedContextItems, setSelectedContextItems] = useState<
    Array<{ name: string; path: string; is_dir: boolean }>
  >([]);

  // Fetch all files and directories on mount or CWD change
  useEffect(() => {
    const loadFiles = async () => {
      try {
        const items = await invoke<any[]>("get_all_files_and_folders", {
          dirPath: activeProjectCwd || null,
        });
        setAllWorkspaceFiles(items);
      } catch (err) {
        console.error("Failed to load workspace files for mentions:", err);
      }
    };
    loadFiles();
  }, [activeProjectCwd]);

  // Fuzzy match scoring function
  const getFuzzyScore = (text: string, query: string): number => {
    const t = text.toLowerCase();
    const q = query.toLowerCase();
    if (t === q) return 1000;
    if (t.includes(q)) {
      return 500 - t.indexOf(q);
    }

    let qIdx = 0;
    let tIdx = 0;
    let score = 0;
    let lastMatchIdx = -1;

    while (tIdx < t.length && qIdx < q.length) {
      if (t[tIdx] === q[qIdx]) {
        if (lastMatchIdx !== -1) {
          const gap = tIdx - lastMatchIdx;
          if (gap === 1) score += 10;
          else if (gap <= 3) score += 5;
          else score += 1;
        } else {
          score += 5;
        }
        lastMatchIdx = tIdx;
        qIdx++;
      }
      tIdx++;
    }

    return qIdx === q.length ? score : 0;
  };

  // Memoized filter list of matching files and folders
  const filteredMentions = useMemo(() => {
    if (!showMentions) return [];
    if (!mentionQuery) {
      return allWorkspaceFiles.slice(0, 30);
    }
    return allWorkspaceFiles
      .map((item) => ({
        item,
        score: Math.max(
          getFuzzyScore(item.name, mentionQuery) * 1.5,
          getFuzzyScore(item.path, mentionQuery),
        ),
      }))
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((x) => x.item)
      .slice(0, 30);
  }, [showMentions, mentionQuery, allWorkspaceFiles]);

  const checkMentionTrigger = (text: string, selectionStart: number) => {
    let atIndex = -1;
    for (let i = selectionStart - 1; i >= 0; i--) {
      if (text[i] === "@") {
        if (i === 0 || text[i - 1] === " " || text[i - 1] === "\n") {
          atIndex = i;
          break;
        }
      }
      if (text[i] === " " || text[i] === "\n") {
        break;
      }
    }

    if (atIndex !== -1) {
      const query = text.substring(atIndex + 1, selectionStart);
      setShowMentions(true);
      setMentionQuery(query);
      setMentionTriggerIndex(atIndex);
      setMentionSelectedIndex((prev) => {
        // Keep selected index within bounds if size changed
        const listLength = filteredMentions.length || 30;
        return prev >= listLength ? 0 : prev;
      });
    } else {
      setShowMentions(false);
      setMentionQuery("");
      setMentionTriggerIndex(-1);
    }
  };

  const handleSelectMention = (item: {
    name: string;
    path: string;
    is_dir: boolean;
  }) => {
    if (!textareaRef.current) return;
    const text = inputVal;
    const start = mentionTriggerIndex;
    const end = textareaRef.current.selectionStart || start;

    const before = text.substring(0, start);
    const after = text.substring(end);
    const newVal = before + after;

    setInputVal(newVal);
    setShowMentions(false);

    setSelectedContextItems((prev) => {
      if (prev.some((x) => x.path === item.path)) return prev;
      return [...prev, item];
    });

    setTimeout(() => {
      if (textareaRef.current) {
        textareaRef.current.focus();
        textareaRef.current.setSelectionRange(start, start);
        textareaRef.current.style.height = "auto";
        textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
      }
    }, 0);
  };

  const SafeMarkdown = ({ content }: { content: string }) => {
    const strictSchema = {
      tagNames: [
        "p",
        "br",
        "b",
        "i",
        "strong",
        "em",
        "strike",
        "code",
        "pre",
        "del",
        "ul",
        "ol",
        "li",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "blockquote",
        "a",
        "img",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
      ],
      attributes: {
        a: ["href", "title", "target"],
        img: ["src", "alt", "title"],
        "*": ["className"],
      },
      protocols: {
        href: ["http", "https", "mailto"],
        src: ["http", "https"],
      },
    };

    const sanitizedContent = DOMPurify.sanitize(content);

    return (
      <ReactMarkdown
        rehypePlugins={[[rehypeSanitize, strictSchema]]}
        components={{
          code({ node, inline, className, children, ...props }: any) {
            const codeContent = String(children).replace(/\n$/, "");
            if (!inline) {
              return (
                <pre
                  className="code-block"
                  style={{
                    margin: 0,
                    padding: "10px",
                    background: "var(--bg-secondary)",
                    borderRadius: "4px",
                    overflowX: "auto",
                  }}
                >
                  <code
                    className={className}
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: "11.5px",
                      color: "var(--text-primary)",
                    }}
                  >
                    {codeContent}
                  </code>
                </pre>
              );
            }
            return (
              <code
                className={className}
                style={{
                  fontFamily: "var(--font-mono)",
                  background: "var(--bg-secondary)",
                  padding: "2px 4px",
                  borderRadius: "3px",
                  color: "var(--color-accent)",
                }}
                {...props}
              >
                {children}
              </code>
            );
          },
        }}
      >
        {sanitizedContent}
      </ReactMarkdown>
    );
  };

  const saveSession = async (
    history: ChatItem[],
    title: string,
    sessId: string,
  ) => {
    try {
      await invoke("save_agent_session", {
        sessionId: sessId,
        title: title,
        model: selectedModel,
        mode: selectedMode,
        activeCwd: activeProjectCwd || null,
        projectId: activeProjectId || null,
        frontendHistory: history,
      });
    } catch (err) {
      console.error("Failed to save session:", err);
    }
  };

  useEffect(() => {
    if (sessionId && chatHistory.length > 0) {
      saveSession(chatHistory, sessionTitle, sessionId);
    }
  }, [chatHistory, sessionTitle, sessionId]);

  useEffect(() => {
    if (initialSessionData) {
      setChatHistory(initialSessionData.frontend_history);
      setSessionTitle(initialSessionData.title);
      setSessionId(initialSessionData.session_id);
      setSelectedModel(initialSessionData.model);
      setSelectedMode(initialSessionData.mode);
      if (onClearInitialSessionData) {
        onClearInitialSessionData();
      }
    }
  }, [initialSessionData]);
  useEffect(() => {
    if (initialMessage && initialMessage.trim()) {
      triggerSendMessage(initialMessage);
      if (onClearInitialMessage) {
        onClearInitialMessage();
      }
    }
  }, [initialMessage]);
  // ─── Multi-Session: Watch Project Switch ────────────────────────────
  // Debounce 150ms + AbortController + Global Lock serialize
  useEffect(() => {
    if (!activeProjectId) return;
    const prev = prevProjectIdRef.current;
    if (prev === activeProjectId) return;

    const controller = new AbortController();
    const seq = ++switchSeqRef.current;

    const timer = setTimeout(async () => {
      // Serialize: bỏ qua nếu đang có switch khác
      if (switchingRef.current) return;
      switchingRef.current = true;

      try {
        if (controller.signal.aborted) return;

        // 1. Save session cũ (nếu có)
        if (sessionId && chatHistory.length > 0) {
          await saveSession(chatHistory, sessionTitle, sessionId);
        }

        if (controller.signal.aborted) return;

        // 2. Gọi backend switch project
        const info: any = await invoke("switch_agent_project", {
          newProjectId: activeProjectId,
          newProjectCwd: activeProjectCwd || "",
          seq: seq,
        });

        if (controller.signal.aborted) return;

        // 3. Load history lazy (trang 1)
        if (info.session_id) {
          setSessionId(info.session_id);
          try {
            const page: any = await invoke("load_history_page", {
              sessionId: info.session_id,
              page: 0,
              pageSize: 50,
            });
            if (!controller.signal.aborted && page?.items) {
              setChatHistory(page.items);
            }
          } catch {
            // No history yet — session mới
            if (!controller.signal.aborted) {
              setChatHistory([]);
            }
          }
        } else {
          if (!controller.signal.aborted) {
            setChatHistory([]);
            setSessionId(null);
            setSessionTitle("New Chat");
          }
        }

        // 4. Show badge nếu agent cũ bị pause
        if (info.old_status === "paused") {
          setAgentBadge({
            visible: true,
            text: "⏸️ Agent tạm dừng ở project trước",
            type: "paused",
          });
          setTimeout(() => {
            setAgentBadge((prev) => ({ ...prev, visible: false }));
          }, 5000);
        }

        // Reset active tool state
        setActiveTool({ status: "idle" });
        setActiveThought(null);
      } finally {
        switchingRef.current = false;
      }
    }, 150); // Debounce 150ms

    prevProjectIdRef.current = activeProjectId;
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [activeProjectId, activeProjectCwd]);

  const handleLoadSession = async (sessId: string, title?: string) => {
    if (onLoadSession) {
      onLoadSession(sessId, title || "Lịch sử Chat");
      setHistoryModalOpen(false);
      return;
    }
    try {
      const data: any = await invoke("load_agent_session", {
        sessionId: sessId,
      });
      setChatHistory(data.frontend_history);
      setSessionTitle(data.title);
      setSessionId(data.session_id);
      setSelectedModel(data.model);
      setSelectedMode(data.mode);
      setHistoryModalOpen(false);
    } catch (err: any) {
      alert(`Lỗi khi tải lịch sử: ${err?.message || err}`);
    }
  };

  const handleDeleteSession = async (sessId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Bạn có chắc chắn muốn xóa lịch sử chat này không?")) return;
    try {
      await invoke("agent_delete_session", { sessionId: sessId });
      setHistoryItems((prev) =>
        prev.filter((item) => item.session_id !== sessId),
      );
    } catch (err: any) {
      alert(`Lỗi khi xóa lịch sử: ${err?.message || err}`);
    }
  };

  const handleOpenHistoryModal = async () => {
    try {
      const list = await invoke<any[]>("agent_get_history", {
        projectId: activeProjectId || null,
      });
      setHistoryItems(list);
      setHistoryModalOpen(true);
      setMenuOpen(false);
    } catch (err: any) {
      alert(`Lỗi khi lấy lịch sử chat: ${err?.message || err}`);
    }
  };

  const handleCopy = (id: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => {
      setCopiedId(null);
    }, 2000);
  };

  const [availableModels, setAvailableModels] = useState<
    { id: string; name: string }[]
  >([]);

  const [capabilities, setCapabilities] = useState(() => {
    const saved = localStorage.getItem("alouette_capabilities");
    return saved
      ? JSON.parse(saved)
      : {
          sandbox: true,
          terminal: true,
          localData: true,
          internet: false,
          environment: true,
          logSystem: true,
          build: true,
          browser: false,
          interaction: "full",
          postMini: false,
          git: true,
        };
  });

  const toggleCapability = (key: string) => {
    setCapabilities((prev: any) => {
      const next = { ...prev };
      if (key === "interaction") {
        next.interaction = prev.interaction === "full" ? "readonly" : "full";
      } else {
        next[key] = !prev[key];
      }
      localStorage.setItem("alouette_capabilities", JSON.stringify(next));
      return next;
    });
  };

  const [capsOpen, setCapsOpen] = useState(false);

  const activeStreamMessageIdRef = useRef<string | null>(null);
  const isActiveSender = useRef(false);

  const createStreamPlaceholder = (initialText: string) => {
    const streamId = "stream_" + Date.now();
    activeStreamMessageIdRef.current = streamId;
    setActiveThought("");
    const streamMsg: ChatItem = {
      id: streamId,
      type: "text",
      sender: "agent",
      text: initialText,
      timestamp: new Date().toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      }),
    };
    setChatHistory((prev) => [...prev, streamMsg]);
    return streamId;
  };

  const removeStreamPlaceholder = () => {
    const streamId = activeStreamMessageIdRef.current;
    if (streamId) {
      activeStreamMessageIdRef.current = null;
      setChatHistory((prev) => prev.filter((msg) => msg.id !== streamId));
    }
  };

  const chatEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const modelDropdownRef = useRef<HTMLDivElement>(null);
  const modeDropdownRef = useRef<HTMLDivElement>(null);
  const processedIters = useRef<Set<string>>(new Set());
  const panelRef = useRef<HTMLDivElement>(null);
  const [isCompact, setIsCompact] = useState(false);

  // Track panel width and toggle compact mode
  useEffect(() => {
    if (!panelRef.current) return;
    const obs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setIsCompact(entry.contentRect.width < 320);
      }
    });
    obs.observe(panelRef.current);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    let unlistenText: any;
    let unlistenThought: any;
    let unlistenThoughtFinal: any;
    let unlistenStreamComplete: any;

    // Watchdog timer: nếu không có event nào trong 120s, tự động reset isTyping
    let watchdogTimer: ReturnType<typeof setTimeout> | null = null;
    const resetWatchdog = () => {
      if (watchdogTimer) clearTimeout(watchdogTimer);
      watchdogTimer = setTimeout(() => {
        console.warn(
          "[ALOUETTE UI WATCHDOG] Không nhận được event stream trong 120s, tự động reset.",
        );
        setIsTyping(false);
        setActiveThought(null);
        activeStreamMessageIdRef.current = null;
      }, 120_000);
    };

    const setupStreamListeners = async () => {
      unlistenText = await listen("agent-text-chunk", (event: any) => {
        resetWatchdog();
        // Chỉ xử lý nếu chính instance NÀY đang gửi message
        if (!isActiveSender.current) return;
        const chunk = event.payload;
        const streamId = activeStreamMessageIdRef.current;
        if (!streamId) {
          createStreamPlaceholder(chunk);
        } else {
          setChatHistory((prev) =>
            prev.map((msg) =>
              msg.id === streamId
                ? { ...msg, text: (msg.text || "") + chunk }
                : msg,
            ),
          );
        }
      });

      unlistenThought = await listen("agent-thought-chunk", (event: any) => {
        resetWatchdog();
        if (!isActiveSender.current) return;
        const chunk = event.payload;
        setActiveThought((prev) => (prev || "") + chunk);
      });

      unlistenThoughtFinal = await listen(
        "agent-thought-final",
        (event: any) => {
          resetWatchdog();
          if (!isActiveSender.current) return;
          const finalThought = event.payload;
          setActiveThought(finalThought);
        },
      );

      unlistenStreamComplete = await listen(
        "agent-stream-complete",
        (event: any) => {
          console.log(
            "[ALOUETTE UI] Received stream-complete event:",
            event.payload,
          );
          if (!isActiveSender.current) return;
          if (watchdogTimer) clearTimeout(watchdogTimer);
          // Do NOT set isTyping(false) here, because the agent might still be executing tools!
          // isTyping will be set to false when agent_send_message completely resolves.
        },
      );
    };

    setupStreamListeners();

    return () => {
      if (watchdogTimer) clearTimeout(watchdogTimer);
      Promise.resolve(unlistenText).then(fn => fn && fn());
      Promise.resolve(unlistenThought).then(fn => fn && fn());
      Promise.resolve(unlistenThoughtFinal).then(fn => fn && fn());
      Promise.resolve(unlistenStreamComplete).then(fn => fn && fn());
    };
  }, []);

  // ─── Sync totalSessionTokens ────────────────────────────────────────
  useEffect(() => {
    setTotalSessionTokens(tokenBreakdown.total);
  }, [tokenBreakdown.total]);

  useEffect(() => {
    const loadActiveModels = async () => {
      let activeIds: string[] = [];
      let activeModelBackend = "";
      let config: any = null;
      try {
        config = await invoke<any>("get_custom_ai_config");
        if (config) {
          activeModelBackend = config.active_model;
        }
      } catch (e) {
        console.error("Failed to fetch custom AI config:", e);
      }

      const savedActive = localStorage.getItem("alouette_active_models");
      if (savedActive) {
        try {
          activeIds = JSON.parse(savedActive);
        } catch (_) {}
      } else if (activeModelBackend) {
        activeIds = [activeModelBackend];
      }

      // If activeIds is still empty, fallback to all models so dropdown isn't empty
      if (activeIds.length === 0 && config && config.providers) {
        Object.values(config.providers).forEach((prov: any) => {
          if (prov && prov.models) {
            activeIds.push(...Object.keys(prov.models));
          }
        });
      }

      const savedCustom = localStorage.getItem("alouette_custom_models");
      const customs: any[] = savedCustom ? JSON.parse(savedCustom) : [];

      const list: { id: string; name: string }[] = [];

      // Dynamically load models from backend config
      if (config && config.providers) {
        Object.values(config.providers).forEach((provCfg: any) => {
          if (provCfg && provCfg.models) {
            Object.keys(provCfg.models).forEach((modelId) => {
              if (activeIds.includes(modelId)) {
                // Generate a name similar to AdminPanel
                let modelName = modelId
                  .split("-")
                  .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
                  .join(" ");
                if (modelId.startsWith("gpt-")) {
                  modelName = "GPT-" + modelId.substring(4).toUpperCase();
                } else if (modelId.startsWith("gemini-")) {
                  modelName = "Gemini " + modelId.substring(7).split("-").map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
                } else if (modelId.startsWith("claude-")) {
                  modelName = "Claude " + modelId.substring(7).split("-").map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
                } else if (modelId.startsWith("deepseek-")) {
                  modelName = "DeepSeek-" + modelId.substring(9).toUpperCase();
                } else if (modelId.startsWith("qwen-")) {
                  modelName = "Qwen " + modelId.substring(5).toUpperCase();
                }
                list.push({ id: modelId, name: modelName });
              }
            });
          }
        });
      }

      customs.forEach((c) => {
        if (activeIds.includes(c.id)) {
          list.push({ id: c.id, name: `${c.provider} - ${c.name}` });
        }
      });

      if (list.length > 0) {
        list.sort((a, b) => a.id.localeCompare(b.id));
        setAvailableModels(list);
        setSelectedModel((prev) => {
          if (list.some((m) => m.id === prev)) return prev;
          if (
            activeModelBackend &&
            list.some((m) => m.id === activeModelBackend)
          ) {
            return activeModelBackend;
          }
          return list[0].id;
        });
      }
    };

    loadActiveModels();

      window.addEventListener("storage", loadActiveModels);

      return () => {
        window.removeEventListener("storage", loadActiveModels);
      };
    }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatHistory, isTyping]);

  useEffect(() => {
    let unlistenFn: any;
    const setupListener = async () => {
      unlistenFn = await listen("agent-activity", (event: any) => {
        if (!isActiveSender.current) return;
        setActiveTool(event.payload as any);
      });
    };
    setupListener();
    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, []);

  useEffect(() => {
    let unlistenFn: any;
    const setupIterationListener = async () => {
      unlistenFn = await listen("agent-iteration", (event: any) => {
        if (!isActiveSender.current) return;
        const data = event.payload;

        const iterKey = `iter_${data.iteration}`;
        if (processedIters.current.has(iterKey + "_" + !!data.tool_result)) return;
        processedIters.current.add(iterKey + "_" + !!data.tool_result);
        
        // Clear stream placeholder so the next iteration starts fresh
        removeStreamPlaceholder();

        setActiveThought(data.thought || null);
        setLoopIterations(data.iteration || 0);

        if (data.tool_name) {
          const status = data.tool_result
            ? data.tool_success !== false
              ? "success"
              : "failed"
            : "running";
          const newSkill: ChatItem = {
            id: iterKey,
            type: "skill_call",
            sender: "agent",
            toolName: data.tool_name,
            args: data.tool_args || "",
            toolStatus: status,
            toolResult: data.tool_result || undefined,
            timestamp:
              data.timestamp ||
              new Date().toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              }),
          };

          setChatHistory((prev) => {
            const idx = prev.findIndex((item) => item.id === newSkill.id);
            if (idx >= 0) {
              const next = [...prev];
              next[idx] = newSkill;
              return next;
            } else {
              return [...prev, newSkill];
            }
          });
        }
      });
    };
    setupIterationListener();
    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, []);

  useEffect(() => {
    let unlistenFn: any;
    const setupErrorListener = async () => {
      unlistenFn = await listen("alouette-open-error", (event: any) => {
        const errorData = event.payload;

        const normalizePath = (p: string | undefined | null) => {
          if (!p) return "";
          let clean = p.replace(/\\/g, "/").toLowerCase();
          if (clean.startsWith("//?/")) clean = clean.substring(4);
          if (clean.startsWith("\\\\?\\")) clean = clean.substring(4);
          clean = clean.replace(/^\/\/\?\//, "");
          return clean;
        };

        const matchesCwd =
          activeProjectCwd &&
          errorData.cwd &&
          normalizePath(activeProjectCwd) === normalizePath(errorData.cwd);

        const matchesId =
          activeProjectId &&
          errorData.project_id &&
          activeProjectId.toLowerCase() === errorData.project_id.toLowerCase();

        if (matchesCwd || matchesId) {
          setAlouetteError({
            id: `err_${Date.now()}`,
            projectName: errorData.project_name,
            errorText: errorData.text,
          });
        }
      });
    };
    setupErrorListener();
    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [activeProjectCwd, activeProjectId]);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 180)}px`;
    }
  }, [inputVal]);

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(e.target as Node)
      ) {
        setMenuOpen(false);
      }
      if (
        modelDropdownRef.current &&
        !modelDropdownRef.current.contains(e.target as Node)
      ) {
        setModelDropdownOpen(false);
      }
      if (
        modeDropdownRef.current &&
        !modeDropdownRef.current.contains(e.target as Node)
      ) {
        setModeDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleOutsideClick);
    return () => document.removeEventListener("mousedown", handleOutsideClick);
  }, []);

  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isTyping) {
        handleCancel();
      }
    };
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => window.removeEventListener("keydown", handleGlobalKeyDown);
  }, [isTyping]);

  const triggerSendMessage = async (messageText: string) => {
    if (!messageText.trim()) return;

    const userMsg: ChatItem = {
      id: Date.now().toString(),
      type: "text",
      sender: "user",
      text: messageText,
      timestamp: new Date().toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      }),
    };

    let isFirstMessage = chatHistory.length === 0;
    let newTitle = sessionTitle;
    if (isFirstMessage) {
      const words = messageText.trim().split(/\s+/);
      newTitle =
        words.length <= 6
          ? messageText.trim()
          : words.slice(0, 6).join(" ") + "...";
      setSessionTitle(newTitle);
    }

    setChatHistory((prev) => [...prev, userMsg]);
    setIsTyping(true);
    // Đánh dấu instance này là sender đang active
    isActiveSender.current = true;
    activeStreamMessageIdRef.current = null;

    processedIters.current = new Set();
    setLoopIterations(0);

    let backendModelName = selectedModel;
    if (selectedModel.startsWith("custom-")) {
      const savedCustom = localStorage.getItem("alouette_custom_models");
      const customs: any[] = savedCustom ? JSON.parse(savedCustom) : [];
      const found = customs.find((c) => c.id === selectedModel);
      if (found) {
        backendModelName = found.name;
      }
    }

    try {
      const response: {
        session_id: string;
        reply_type: string;
        text?: string;
        tool_name?: string;
        args?: string;
        tools?: Array<{ name: string; args: string; pending_id: string }>;
        pending_id?: string;
        approved_tool_index?: number;
        loop_result?: {
          iterations: Array<{
            iteration: number;
            thought: string | null;
            tool_name: string | null;
            tool_args: string | null;
            tool_result: string | null;
            tool_success: boolean;
            timestamp: string;
          }>;
          final_text: string | null;
          total_iterations: number;
          tool_calls_made: number;
          stopped_early: boolean;
          stop_reason: string | null;
        };
        iteration?: number;
        total_iterations?: number;
      } = await invoke("agent_send_message", {
        message: messageText,
        model: backendModelName,
        mode: selectedMode,
        activeCwd: activeProjectCwd,
        thinkingMode: thinkingMode,
      });

      setIsTyping(false);
      setActiveThought(null);
      setLoopIterations(0);

      removeStreamPlaceholder();

      if (response.session_id) {
        setSessionId(response.session_id);
      }

      if (response.total_iterations) {
        setTotalIterations(response.total_iterations);
      }

      if (response.reply_type === "loop_result") {
        const loopResult = response.loop_result;
        if (loopResult) {
          if (loopResult.iterations) {
            setChatHistory((prev) => {
              let nextHistory = [...prev];
              for (const iter of loopResult.iterations) {
                if (iter.tool_name) {
                  const status = iter.tool_success ? "success" : "failed";
                  const skillItem: ChatItem = {
                    id: `iter_${iter.iteration}`,
                    type: "skill_call",
                    sender: "agent",
                    toolName: iter.tool_name,
                    args: iter.tool_args || "",
                    toolStatus: status,
                    toolResult: iter.tool_result || undefined,
                    timestamp: iter.timestamp,
                  };

                  const idx = nextHistory.findIndex(
                    (item) => item.id === skillItem.id,
                  );
                  if (idx >= 0) {
                    nextHistory[idx] = skillItem;
                  } else {
                    nextHistory.push(skillItem);
                  }
                }
              }
              return nextHistory;
            });
          }
          if (loopResult.final_text) {
            setChatHistory((prev) => [
              ...prev,
              {
                id: `final_${Date.now()}`,
                type: "text",
                sender: "agent",
                text: loopResult.final_text || undefined,
                timestamp: new Date().toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                }),
              },
            ]);
          }
          if (loopResult.stopped_early && loopResult.stop_reason) {
            setChatHistory((prev) => [
              ...prev,
              {
                id: `stop_${Date.now()}`,
                type: "text",
                sender: "agent",
                text: `⏸️ ${loopResult.stop_reason}`,
                timestamp: new Date().toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                }),
              },
            ]);
          }
        } else if (response.text) {
          setChatHistory((prev) => [
            ...prev,
            {
              id: Date.now().toString(),
              type: "text",
              sender: "agent",
              text: response.text!,
              timestamp: new Date().toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              }),
            },
          ]);
        }
      } else if (response.reply_type === "tool_request") {
        const toolMsg: ChatItem = {
          id: response.pending_id || Date.now().toString(),
          type: "tool_request",
          sender: "agent",
          toolName: response.tool_name,
          args: response.args,
          toolStatus: "waiting",
          timestamp: new Date().toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          }),
        };
        setChatHistory((prev) => [...prev, toolMsg]);
      } else if (response.reply_type === "tool_batch_request") {
        const responseAny = response as any;
        const tools: ToolItem[] = (responseAny.tools || []).map((t: any) => ({
          name: t.name,
          args: t.args,
          pending_id: t.pending_id,
          status: "waiting" as const,
        }));
        const batchMsg: ChatItem = {
          id: `batch_${Date.now()}`,
          type: "tool_batch_request",
          sender: "agent",
          tools,
          timestamp: new Date().toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          }),
        };
        setChatHistory((prev) => [...prev, batchMsg]);
      } else if (response.reply_type === "agent_activity") {
        const activityMsg: ChatItem = {
          id: Date.now().toString(),
          type: "agent_activity",
          sender: "agent",
          text: response.text,
          timestamp: new Date().toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          }),
        };
        setChatHistory((prev) => [...prev, activityMsg]);
      } else {
        const agentMsg: ChatItem = {
          id: Date.now().toString(),
          type: "text",
          sender: "agent",
          text: response.text || "Tôi đã ghi nhận yêu cầu.",
          timestamp: new Date().toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          }),
        };
        setChatHistory((prev) => [...prev, agentMsg]);
      }
    } catch (err: any) {
      setIsTyping(false);
      const errorMsg: ChatItem = {
        id: Date.now().toString(),
        type: "text",
        sender: "agent",
        text: `Lỗi kết nối Harness Backend: ${err?.message || err}`,
        timestamp: new Date().toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        }),
      };
      setChatHistory((prev) => [...prev, errorMsg]);
    } finally {
      // Dọn dẹp: instance này không còn active nữa
      isActiveSender.current = false;
    }
  };

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    const hasText = !!inputVal.trim();
    const hasContext = selectedContextItems.length > 0;
    if (!hasText && !hasContext) return;

    let text = inputVal;
    if (hasContext) {
      const contextPrefix = selectedContextItems
        .map((item) => `\`@${item.path}\``)
        .join(" ");
      text = contextPrefix + (text.trim() ? "\n\n" + text : "");
    }

    setInputVal("");
    setSelectedContextItems([]);
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
    await triggerSendMessage(text);
  };

  const handleStartAnalyze = async (errorLog: string) => {
    const promptText = `Tôi phát hiện lỗi hệ thống dưới đây. Hãy phân tích nguyên nhân và tìm cách sửa lỗi này giúp tôi:\n\n${errorLog}`;
    await triggerSendMessage(promptText);
  };

  const handleApproveTool = async (id: string, toolIndex?: number) => {
    // For batch items, don't set item-level status (individual tools track their own)
    const isBatch = toolIndex !== undefined;
    if (!isBatch) {
      setChatHistory((prev) =>
        prev.map((item) =>
          item.id === id ? { ...item, toolStatus: "approved" as const } : item,
        ),
      );
    }

    setIsTyping(true);
    isActiveSender.current = true;
    activeStreamMessageIdRef.current = null;

    let backendModelName = selectedModel;
    if (selectedModel.startsWith("custom-")) {
      const savedCustom = localStorage.getItem("alouette_custom_models");
      const customs: any[] = savedCustom ? JSON.parse(savedCustom) : [];
      const found = customs.find((c) => c.id === selectedModel);
      if (found) {
        backendModelName = found.name;
      }
    }

    try {
      const response: {
        session_id?: string;
        text?: string;
        reply_type?: string;
        tool_result?: string;
        tools?: Array<{ name: string; args: string; pending_id: string }>;
        approved_tool_index?: number;
        loop_result?: {
          iterations: Array<{
            iteration: number;
            thought: string | null;
            tool_name: string | null;
            tool_args: string | null;
            tool_result: string | null;
            tool_success: boolean;
            timestamp: string;
          }>;
          final_text: string | null;
          total_iterations: number;
          tool_calls_made: number;
          stopped_early: boolean;
          stop_reason: string | null;
        };
      } = await invoke("agent_approve_tool", {
        approved: true,
        model: backendModelName,
        activeCwd: activeProjectCwd,
        toolIndex: toolIndex ?? null,
        thinkingMode: thinkingMode,
      });

      setIsTyping(false);
      setActiveThought(null);
      removeStreamPlaceholder();

      if (response.session_id) {
        setSessionId(response.session_id);
      }

      // Batch response: cập nhật tool result và hiển thị tools còn lại
      if (response.reply_type === "tool_batch_request" && response.tools) {
        setChatHistory((prev) =>
          prev.map((item) => {
            if (item.id === id) {
              // Đánh dấu tool vừa được duyệt
              const updatedTools = response.tools!.map((t) => ({
                ...t,
                status: "waiting" as const,
              }));
              return {
                ...item,
                tools: updatedTools,
                toolResult: response.tool_result,
              };
            }
            return item;
          }),
        );
        return;
      }

      if (response.tool_result) {
        setChatHistory((prev) =>
          prev.map((item) => {
            if (item.id === id) {
              return {
                ...item,
                type: "skill_call",
                toolStatus: "success",
                toolResult: response.tool_result,
              };
            }
            return item;
          }),
        );
      } else if (response.reply_type === "tool_request") {
        const toolMsg: ChatItem = {
          id: (response as any).pending_id || Date.now().toString(),
          type: "tool_request",
          sender: "agent",
          toolName: (response as any).tool_name,
          args: (response as any).args,
          toolStatus: "waiting",
          timestamp: new Date().toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          }),
        };
        setChatHistory((prev) => [...prev, toolMsg]);
      }

      const loopResult = response.loop_result;
      if (
        loopResult &&
        loopResult.iterations &&
        loopResult.iterations.length > 1
      ) {
        setChatHistory((prev) => {
          let nextHistory = [...prev];
          for (const iter of loopResult.iterations) {
            if (iter.tool_name) {
              const status = iter.tool_success ? "success" : "failed";
              const skillItem: ChatItem = {
                id: `iter_${iter.iteration}`,
                type: "skill_call",
                sender: "agent",
                toolName: iter.tool_name,
                args: iter.tool_args || "",
                toolStatus: status,
                toolResult: iter.tool_result || undefined,
                timestamp: iter.timestamp,
              };

              const idx = nextHistory.findIndex(
                (item) => item.id === skillItem.id,
              );
              if (idx >= 0) {
                nextHistory[idx] = skillItem;
              } else {
                nextHistory.push(skillItem);
              }
            }
          }
          return nextHistory;
        });
        if (loopResult.final_text) {
          setChatHistory((prev) => [
            ...prev,
            {
              id: `final_${Date.now()}`,
              type: "text",
              sender: "agent",
              text: loopResult.final_text || undefined,
              timestamp: new Date().toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              }),
            },
          ]);
        }
      } else {
        if (response.text) {
          const successMsg: ChatItem = {
            id: Date.now().toString(),
            type: "text",
            sender: "agent",
            text: response.text,
            timestamp: new Date().toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            }),
          };
          setChatHistory((prev) => [...prev, successMsg]);
        }
      }
    } catch (err: any) {
      removeStreamPlaceholder();
      setIsTyping(false);
      const errorMsg: ChatItem = {
        id: Date.now().toString(),
        type: "text",
        sender: "agent",
        text: `Lỗi khi phê duyệt tool: ${err?.message || err}`,
        timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      };
      setChatHistory((prev) => [...prev, errorMsg]);
    } finally {
      isActiveSender.current = false;
    }
  };

  const handleRejectTool = async (id: string, toolIndex?: number) => {
    // For batch items, don't set item-level status (individual tools track their own)
    const isBatch = toolIndex !== undefined;
    if (!isBatch) {
      setChatHistory((prev) =>
        prev.map((item) =>
          item.id === id ? { ...item, toolStatus: "rejected" as const } : item,
        ),
      );
    }

    setIsTyping(true);
    isActiveSender.current = true;
    activeStreamMessageIdRef.current = null;

    let backendModelName = selectedModel;
    if (selectedModel.startsWith("custom-")) {
      const savedCustom = localStorage.getItem("alouette_custom_models");
      const customs: any[] = savedCustom ? JSON.parse(savedCustom) : [];
      const found = customs.find((c) => c.id === selectedModel);
      if (found) {
        backendModelName = found.name;
      }
    }

    try {
      const response: {
        session_id?: string;
        text?: string;
        reply_type?: string;
        tools?: Array<{ name: string; args: string; pending_id: string }>;
      } = await invoke("agent_approve_tool", {
        approved: false,
        model: backendModelName,
        activeCwd: activeProjectCwd,
        toolIndex: toolIndex ?? null,
        thinkingMode: thinkingMode,
      });

      setIsTyping(false);
      setActiveThought(null);
      removeStreamPlaceholder();

      if (response.session_id) {
        setSessionId(response.session_id);
      }

      // Batch response: cập nhật danh sách tools còn lại
      if (response.reply_type === "tool_batch_request" && response.tools) {
        setChatHistory((prev) =>
          prev.map((item) => {
            if (item.id === id) {
              const updatedTools = response.tools!.map((t) => ({
                ...t,
                status: "waiting" as const,
              }));
              return {
                ...item,
                tools: updatedTools,
              };
            }
            return item;
          }),
        );
        return;
      }

      const rejectMsg: ChatItem = {
        id: Date.now().toString(),
        type: "text",
        sender: "agent",
        text: `✕ ${response.text || "Từ chối thực thi công cụ."}`,
        timestamp: new Date().toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        }),
      };
      setChatHistory((prev) => [...prev, rejectMsg]);
    } catch (err: any) {
      removeStreamPlaceholder();
      setIsTyping(false);
      const errorMsg: ChatItem = {
        id: Date.now().toString(),
        type: "text",
        sender: "agent",
        text: `Lỗi khi từ chối tool: ${err?.message || err}`,
        timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      };
      setChatHistory((prev) => [...prev, errorMsg]);
    } finally {
      isActiveSender.current = false;
    }
  };

  const handleNewChat = async () => {
    try {
      await invoke("agent_reset_session");
      // Also update registry: switch to a fresh session
      if (activeProjectId) {
        const seq = ++switchSeqRef.current;
        await invoke("switch_agent_project", {
          newProjectId: activeProjectId,
          newProjectCwd: activeProjectCwd || "",
          seq: seq,
        });
      }
      setChatHistory([]);
      setSessionId(null);
      setSessionTitle("New Chat");
      setMenuOpen(false);
      setActiveTool({ status: "idle" });
      setActiveThought(null);
    } catch (err: any) {
      alert(`Lỗi khi reset session: ${err?.message || err}`);
    }
  };

  const handleCancel = async () => {
    setIsTyping(false);
    setActiveThought(null);
    activeStreamMessageIdRef.current = null;
    try {
      await invoke("agent_cancel");
    } catch (err: any) {
      console.error("Failed to cancel agent:", err);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (showMentions && filteredMentions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionSelectedIndex((prev) => (prev + 1) % filteredMentions.length);
        return;
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionSelectedIndex(
          (prev) =>
            (prev - 1 + filteredMentions.length) % filteredMentions.length,
        );
        return;
      } else if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        handleSelectMention(filteredMentions[mentionSelectedIndex]);
        return;
      } else if (e.key === "Escape") {
        e.preventDefault();
        setShowMentions(false);
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend(e);
    } else if (e.key === "Escape" && isTyping) {
      e.preventDefault();
      handleCancel();
    }
  };

  const capList = [
    {
      key: "sandbox",
      label: "Sandbox",
      icon: Shield,
      isActive: capabilities.sandbox,
    },
    {
      key: "terminal",
      label: "Terminal",
      icon: Terminal,
      isActive: capabilities.terminal,
    },
    {
      key: "localData",
      label: "Local Data",
      icon: Database,
      isActive: capabilities.localData,
    },
    {
      key: "internet",
      label: "Internet",
      icon: Globe,
      isActive: capabilities.internet,
    },
    {
      key: "environment",
      label: "Môi trường",
      icon: Cpu,
      isActive: capabilities.environment,
    },
    {
      key: "logSystem",
      label: "Log System",
      icon: Activity,
      isActive: capabilities.logSystem,
    },
    {
      key: "build",
      label: "Build",
      icon: Hammer,
      isActive: capabilities.build,
    },
    {
      key: "browser",
      label: "Browser",
      icon: Globe,
      isActive: capabilities.browser,
    },
    {
      key: "interaction",
      label: capabilities.interaction === "full" ? "Quyền: Ghi" : "Quyền: Đọc",
      icon: capabilities.interaction === "full" ? Unlock : Lock,
      isActive: capabilities.interaction === "full",
    },
    {
      key: "postMini",
      label: "Ping Zero Min",
      icon: Zap,
      isActive: capabilities.postMini,
    },
    { key: "git", label: "Git", icon: GitBranch, isActive: capabilities.git },
  ];


  return (
    <div
      ref={panelRef}
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        background: "var(--bg-primary)",
        color: "var(--text-primary)",
        overflow: "hidden",
        fontFamily: "var(--font-sans)",
      }}
    >
      <style>{`
        .message-container:hover .copy-button-hover {
          opacity: 1 !important;
        }
        @keyframes agentSlideUp {
          from { opacity: 0; transform: translateY(4px); }
          to { opacity: 1; transform: translateY(0); }
        }
        .agent-fade-in {
          animation: agentSlideUp 0.15s ease-out;
        }
        .agent-spin {
          animation: spin 1s linear infinite;
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        @keyframes agentTyping {
          0%, 100% { opacity: 0.4; transform: translateY(0); }
          50% { opacity: 1; transform: translateY(-1.5px); }
        }
        .agent-typing-dot:nth-child(1) { animation: agentTyping 1.2s ease-in-out 0s infinite; }
        .agent-typing-dot:nth-child(2) { animation: agentTyping 1.2s ease-in-out 0.2s infinite; }
        .agent-typing-dot:nth-child(3) { animation: agentTyping 1.2s ease-in-out 0.4s infinite; }
        .agent-capsule-btn {
          transition: all 0.1s ease;
        }
        .agent-capsule-btn:hover {
          background: var(--bg-secondary) !important;
        }
        .agent-select {
          appearance: none;
          background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
          background-repeat: no-repeat;
          background-position: right 6px center;
          padding-right: 20px !important;
        }
        .agent-scroll::-webkit-scrollbar {
          width: 4px;
        }
        .agent-scroll::-webkit-scrollbar-track {
          background: transparent;
        }
        .agent-scroll::-webkit-scrollbar-thumb {
          background: var(--border-primary);
          border-radius: 4px;
        }
        .agent-scroll::-webkit-scrollbar-thumb:hover {
          background: var(--text-muted);
        }
        .agent-dropdown-item {
          transition: background 0.1s ease;
        }
        .agent-dropdown-item:hover {
          background: var(--bg-tertiary) !important;
        }
        .agent-header-btn {
          transition: all 0.1s ease;
          cursor: pointer;
        }
        .agent-header-btn:hover {
          background: var(--bg-secondary) !important;
        }
      `}</style>

      {/* ===== HEADER ===== */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 12px",
          height: "36px",
          borderBottom: "1px solid var(--border-primary)",
          background: "var(--bg-primary)",
          flexShrink: 0,
          position: "relative",
        }}
      >
        {/* Left: Brand + Session */}
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0px" }}>
            <span
              style={{
                fontSize: "11.5px",
                fontWeight: 600,
                color: "var(--text-primary)",
                lineHeight: 1.2,
              }}
            >
              {sessionTitle}
            </span>
            {agentBadge.visible && (
              <span
                style={{
                  fontSize: "9px",
                  color:
                    agentBadge.type === "paused"
                      ? "var(--color-warning, #f59e0b)"
                      : "var(--color-success, #22c55e)",
                  lineHeight: 1,
                  marginTop: "1px",
                }}
              >
                {agentBadge.text}
              </span>
            )}
            <span
              style={{
                fontSize: "9px",
                color: "var(--text-muted)",
                lineHeight: 1,
                display: isCompact ? "none" : "block",
              }}
            >
              {chatHistory.length} tin nhắn · {selectedMode}
            </span>
          </div>
        </div>

        {/* Right: Actions */}
        <div
          ref={dropdownRef}
          style={{
            position: "relative",
            display: "flex",
            alignItems: "center",
          }}
        >
          <button
            onClick={() => setMenuOpen(!menuOpen)}
            className="agent-header-btn"
            style={{
              width: "28px",
              height: "28px",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: menuOpen ? "var(--bg-tertiary)" : "transparent",
              border: menuOpen
                ? "1px solid var(--border-primary)"
                : "1px solid transparent",
              color: "var(--text-secondary)",
              borderRadius: "6px",
              padding: 0,
            }}
          >
            <Plus size={15} />
          </button>

          {menuOpen && (
            <div
              style={{
                position: "absolute",
                top: "calc(100% + 6px)",
                right: 0,
                minWidth: "180px",
                background: "var(--bg-secondary)",
                border: "1px solid var(--border-primary)",
                borderRadius: "8px",
                boxShadow:
                  "0 12px 32px rgba(0,0,0,0.4), 0 2px 8px rgba(0,0,0,0.2)",
                padding: "4px",
                zIndex: 100,
              }}
            >
              <button
                onClick={handleNewChat}
                className="agent-dropdown-item"
                style={{
                  width: "100%",
                  padding: "7px 10px",
                  textAlign: "left",
                  background: "none",
                  border: "none",
                  color: "var(--text-primary)",
                  fontSize: "11.5px",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  borderRadius: "6px",
                }}
              >
                <Plus
                  size={13}
                  style={{ color: "var(--text-muted)", flexShrink: 0 }}
                />
                <span>New Chat</span>
              </button>

              <button
                onClick={() => {
                  alert("Chức năng thêm model đang được phát triển.");
                  setMenuOpen(false);
                }}
                className="agent-dropdown-item"
                style={{
                  width: "100%",
                  padding: "7px 10px",
                  textAlign: "left",
                  background: "none",
                  border: "none",
                  color: "var(--text-primary)",
                  fontSize: "11.5px",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  borderRadius: "6px",
                }}
              >
                <Layers
                  size={13}
                  style={{ color: "var(--text-muted)", flexShrink: 0 }}
                />
                <span>Thêm model</span>
              </button>

              <button
                onClick={handleOpenHistoryModal}
                className="agent-dropdown-item"
                style={{
                  width: "100%",
                  padding: "7px 10px",
                  textAlign: "left",
                  background: "none",
                  border: "none",
                  color: "var(--text-primary)",
                  fontSize: "11.5px",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  borderRadius: "6px",
                }}
              >
                <History
                  size={13}
                  style={{ color: "var(--text-muted)", flexShrink: 0 }}
                />
                <span>Lịch sử chat</span>
              </button>

              <div
                style={{
                  height: "1px",
                  background: "var(--border-primary)",
                  margin: "4px 8px",
                }}
              />

              <button
                onClick={() => {
                  onBack?.();
                  setMenuOpen(false);
                }}
                className="agent-dropdown-item"
                style={{
                  width: "100%",
                  padding: "7px 10px",
                  textAlign: "left",
                  background: "none",
                  border: "none",
                  color: "var(--text-muted)",
                  fontSize: "11.5px",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  borderRadius: "6px",
                }}
              >
                <ArrowLeft size={13} style={{ flexShrink: 0 }} />
                <span>Quay lại Quản lý</span>
              </button>
            </div>
          )}
        </div>
      </div>

      {/* ===== CHAT TIMELINE ===== */}
      <div
        className="agent-scroll"
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "16px",
          display: "flex",
          flexDirection: "column",
          gap: "10px",
        }}
      >
        {chatHistory.length === 0 && !isTyping && (
          <div
            style={{
              flex: 1,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: "8px",
              padding: "40px 20px",
              opacity: 0.7,
            }}
          >
            <div style={{ textAlign: "center" }}>
              <div
                style={{
                  fontSize: "14px",
                  fontWeight: 600,
                  color: "var(--text-primary)",
                  marginBottom: "4px",
                }}
              >
                Alouette AI Agent
              </div>
              <div
                style={{
                  fontSize: "11px",
                  color: "var(--text-muted)",
                  maxWidth: "280px",
                  lineHeight: "1.5",
                }}
              >
                Gửi tin nhắn để bắt đầu. Có thể giúp bạn đọc/ghi tệp, chạy
                terminal, tìm kiếm mã nguồn.
              </div>
            </div>
            <div
              style={{
                display: "flex",
                gap: "6px",
                marginTop: "8px",
                flexWrap: "wrap",
                justifyContent: "center",
              }}
            >
              {["DeepSeek-V4", "Claude Opus", "Gemini 3.5"].map((m) => (
                <span
                  key={m}
                  style={{
                    padding: "3px 8px",
                    fontSize: "9.5px",
                    background: "var(--bg-secondary)",
                    border: "1px solid var(--border-primary)",
                    color: "var(--text-muted)",
                    borderRadius: "4px",
                  }}
                >
                  {m}
                </span>
              ))}
            </div>
          </div>
        )}

        {chatHistory.map((item) => {
          const isUser = item.sender === "user";
          if (item.type === "agent_activity") {
            const isExpanded = !!expandedSkills[item.id];
            return (
              <div
                key={item.id}
                className="message-container agent-fade-in"
                style={{
                  display: "flex",
                  flexDirection: "column",
                  fontSize: "12px",
                  color: "var(--text-muted)",
                  fontFamily: "var(--font-mono)",
                  lineHeight: "1.6",
                  marginBottom: "2px",
                  paddingLeft: "8px",
                }}
              >
                <div
                  onClick={() => {
                    setExpandedSkills((prev) => ({
                      ...prev,
                      [item.id]: !prev[item.id],
                    }));
                  }}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "6px",
                    cursor: "pointer",
                    userSelect: "none",
                    width: "max-content",
                  }}
                >
                  <span>Thought</span>
                  {isExpanded ? <ChevronDown size={12} style={{ opacity: 0.6 }} /> : <ChevronRight size={12} style={{ opacity: 0.6 }} />}
                </div>
                {isExpanded && (
                  <div
                    style={{
                      paddingLeft: "12px",
                      marginTop: "4px",
                      whiteSpace: "pre-wrap",
                      color: "var(--text-secondary)",
                      borderLeft: "1px solid var(--border-primary)",
                      marginLeft: "4px",
                      fontSize: "11px",
                    }}
                  >
                    {item.text}
                  </div>
                )}
              </div>
            );
          }

          if (item.type === "skill_call") {
            const isExpanded = !!expandedSkills[item.id];
            
            let compactNode: React.ReactNode = null;
            let argsObj: any = {};
            try {
              if (item.args) {
                if (typeof item.args === "string") {
                  argsObj = JSON.parse(item.args);
                } else if (typeof item.args === "object") {
                  argsObj = item.args;
                }
              }
            } catch(e) {}
            
            const tName = item.toolName || "";
            const iconStyle = { marginRight: "4px", flexShrink: 0 };
            
            // Tìm giá trị đầu tiên là chuỗi (string) trong params để làm fallback nếu không khớp key
            const fallbackPath = Object.values(argsObj).find(v => typeof v === "string") || "";
            
            if (tName === "scan_directory_tree") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Analyzed</span><Folder size={12} style={iconStyle} /><span>Workspace</span></span>;
            } else if (tName === "scan_subdirectory") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Analyzed</span><Folder size={12} style={iconStyle} /><span>{argsObj.path || argsObj.dirPath || fallbackPath}</span></span>;
            } else if (tName === "read_file") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Analyzed</span><FileText size={12} style={iconStyle} /><span>{argsObj.path || argsObj.filePath || fallbackPath}</span></span>;
            } else if (tName === "read_file_range") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Analyzed</span><FileText size={12} style={iconStyle} /><span>{argsObj.file || argsObj.path || fallbackPath} (Lines {argsObj.start_line}-{argsObj.end_line})</span></span>;
            } else if (tName === "write_file") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Edited</span><FileText size={12} style={iconStyle} /><span>{argsObj.path || argsObj.targetFile || fallbackPath}</span></span>;
            } else if (tName === "replace_in_file") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Edited</span><FileText size={12} style={iconStyle} /><span>{argsObj.path || fallbackPath} (Lines {argsObj.start_line}-{argsObj.end_line})</span></span>;
            } else if (tName === "execute_command") {
              compactNode = <span>{item.toolStatus === "failed" ? "Failed Task" : item.toolStatus === "success" ? "Checked Task" : "Running Task"}: {argsObj.command || fallbackPath}</span>;
            } else if (tName === "search_files") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Searched</span><Folder size={12} style={iconStyle} /><span>{argsObj.pattern || argsObj.query || fallbackPath}</span></span>;
            } else if (tName === "extract_symbol") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Analyzed</span><FileText size={12} style={iconStyle} /><span>{argsObj.file || fallbackPath} (Symbol: {argsObj.symbol || ""})</span></span>;
            } else if (tName === "search_symbol") {
              compactNode = <span style={{display: "inline-flex", alignItems: "center"}}><span style={{marginRight: "4px"}}>Searched</span><Search size={12} style={iconStyle} /><span>Symbol: {argsObj.symbol || fallbackPath}</span></span>;
            } else {
              compactNode = <span>Used tool: {tName}</span>;
            }

            return (
              <div
                key={item.id}
                className="message-container agent-fade-in"
                style={{
                  display: "flex",
                  flexDirection: "column",
                  fontSize: "12px",
                  color: "var(--text-muted)",
                  fontFamily: "var(--font-mono)",
                  lineHeight: "1.6",
                  marginBottom: "2px",
                  paddingLeft: "8px",
                }}
              >
                <div
                  onClick={() => {
                    setExpandedSkills((prev) => ({
                      ...prev,
                      [item.id]: !prev[item.id],
                    }));
                  }}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "6px",
                    cursor: "pointer",
                    userSelect: "none",
                    width: "max-content",
                  }}
                >
                  <span>{compactNode}</span>
                  {isExpanded ? <ChevronDown size={12} style={{ opacity: 0.6 }} /> : <ChevronRight size={12} style={{ opacity: 0.6 }} />}
                  {item.toolStatus === "running" && <RefreshCw size={10} className="agent-spin" style={{ marginLeft: "4px" }} />}
                </div>

                {isExpanded && (
                  <div
                    style={{
                      paddingLeft: "12px",
                      marginTop: "4px",
                      display: "flex",
                      flexDirection: "column",
                      gap: "6px",
                      borderLeft: "1px solid var(--border-primary)",
                      marginLeft: "4px",
                    }}
                  >
                    {item.args && (
                      <div>
                        <div style={{ fontSize: "10px", color: "var(--text-muted)", marginBottom: "2px" }}>Tham số đầu vào:</div>
                        <pre
                          style={{
                            margin: 0,
                            padding: "4px 8px",
                            background: "transparent",
                            borderLeft: "1px solid var(--border-primary)",
                            fontFamily: "var(--font-mono)",
                            fontSize: "10px",
                            color: "var(--text-secondary)",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                          }}
                        >
                          {item.args}
                        </pre>
                      </div>
                    )}
                    {item.toolResult && (
                      <div>
                        <div style={{ fontSize: "10px", color: "var(--text-muted)", marginBottom: "2px" }}>Kết quả trả về:</div>
                        <pre
                          style={{
                            margin: 0,
                            padding: "4px 8px",
                            background: "transparent",
                            borderLeft: "1px solid var(--border-primary)",
                            fontFamily: "var(--font-mono)",
                            fontSize: "10px",
                            color: "var(--text-secondary)",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                            maxHeight: "150px",
                            overflowY: "auto",
                          }}
                        >
                          {item.toolResult}
                        </pre>
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          }

          // ─── TOOL BATCH REQUEST (nhiều tools cùng lúc) ───────────
          if (item.type === "tool_batch_request") {
            const tools = item.tools || [];
            return (
              <div
                key={item.id}
                className="message-container agent-fade-in"
                style={{
                  borderRadius: "6px",
                  border: "1px solid var(--border-primary)",
                  background: "var(--bg-secondary)",
                  padding: "8px 10px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                  flexShrink: 0,
                }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "4px",
                    }}
                  >
                    <Wrench
                      size={10}
                      style={{ color: "var(--text-secondary)" }}
                    />
                    <span
                      style={{
                        fontSize: "9px",
                        fontWeight: 700,
                        color: "var(--text-secondary)",
                        textTransform: "uppercase",
                        letterSpacing: "0.04em",
                      }}
                    >
                      Yêu cầu chạy {tools.length} công cụ
                    </span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                    <span style={{ fontSize: "9px", color: "var(--text-muted)" }}>
                      {item.timestamp}
                    </span>
                    <button
                      onClick={() => handleApproveTool(item.id)}
                      style={{
                        padding: "2px 8px",
                        background: "var(--border-strong, #374151)",
                        border: "none",
                        color: "#fff",
                        borderRadius: "4px",
                        fontSize: "9px",
                        fontWeight: 600,
                        cursor: "pointer",
                        display: "flex",
                        alignItems: "center",
                        gap: "4px"
                      }}
                    >
                      <Check size={9} /> Duyệt tất cả
                    </button>
                  </div>
                </div>

                <div
                  className="agent-scroll"
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "4px",
                    maxHeight: "300px",
                    overflowY: "auto",
                    paddingRight: "2px"
                  }}
                >
                  {tools.map((tool, idx) => (
                    <ToolCardItem
                      key={tool.pending_id}
                      tool={tool}
                      index={idx}
                      onApprove={(ti) => handleApproveTool(item.id, ti)}
                      onReject={(ti) => handleRejectTool(item.id, ti)}
                    />
                  ))}
                </div>
              </div>
            );
          }

          if (item.type === "tool_request") {
            return (
              <SingleToolRequestCard
                key={item.id}
                item={item}
                onApprove={(id) => handleApproveTool(id)}
                onReject={(id) => handleRejectTool(id)}
              />
            );
          }

          // Text messages
          if (item.sender === "agent" && item.text === "" && item.id.startsWith("stream_")) {
            return null;
          }

          return (
            <div
              key={item.id}
              className="message-container agent-fade-in"
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: isUser ? "flex-end" : "flex-start",
                maxWidth: "100%",
                position: "relative",
                flexShrink: 0,
              }}
            >
              {/* Sender label */}
              {!(item.sender === "agent" && item.text === "" && item.id.startsWith("stream_")) && (
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "4px",
                    marginBottom: "3px",
                    padding: isUser ? "0 4px" : "0 2px",
                    fontSize: "10px",
                    color: "var(--text-muted)",
                  }}
                >
                  <span
                    style={{
                      fontWeight: 500,
                      color: "var(--text-secondary)",
                    }}
                  >
                    {isUser ? "Bạn" : "Agent"}
                  </span>
                  <span style={{ fontSize: "9px", opacity: 0.5 }}>•</span>
                  <span style={{ fontSize: "9px" }}>{item.timestamp}</span>
                </div>
              )}

              {/* Message bubble */}
              <div
                style={{
                  maxWidth: "85%",
                  width: isUser ? "auto" : "100%",
                  background: isUser ? "var(--bg-secondary)" : "transparent",
                  color: "var(--text-primary)",
                  padding: isUser ? "8px 14px" : "0",
                  borderRadius: "6px",
                  border: isUser ? "1px solid var(--border-primary)" : "none",
                  fontSize: "12.5px",
                  lineHeight: "1.55",
                  whiteSpace: "pre-wrap",
                  position: "relative",
                  wordBreak: "break-word",
                }}
              >
                {item.sender === "agent" && item.text?.startsWith("Lỗi") ? (
                  <div style={{ color: "#ef4444" }}>
                    <SafeMarkdown content={item.text} />
                  </div>
                ) : (
                  <div>
                    {item.sender === "agent" && item.text === "" && item.id.startsWith("stream_") ? (
                      <div style={{ display: "flex", alignItems: "center", gap: "4px", padding: "6px 2px", height: "20px" }}>
                        <span className="agent-typing-dot" style={{ width: "5px", height: "5px", borderRadius: "50%", background: "var(--text-muted)", display: "inline-block" }} />
                        <span className="agent-typing-dot" style={{ width: "5px", height: "5px", borderRadius: "50%", background: "var(--text-muted)", display: "inline-block" }} />
                        <span className="agent-typing-dot" style={{ width: "5px", height: "5px", borderRadius: "50%", background: "var(--text-muted)", display: "inline-block" }} />
                      </div>
                    ) : (
                      <SafeMarkdown content={item.text || ""} />
                    )}
                  </div>
                )}

                {/* Copy button for agent messages */}
                {!isUser && item.text && (
                  <button
                    onClick={() => handleCopy(item.id, item.text || "")}
                    className="copy-button-hover"
                    title="Sao chép nội dung"
                    style={{
                      position: "absolute",
                      right: "0px",
                      top: "-20px",
                      background: "var(--bg-tertiary)",
                      border: "1px solid var(--border-primary)",
                      color: "var(--text-primary)",
                      borderRadius: "4px",
                      width: "22px",
                      height: "22px",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      cursor: "pointer",
                      opacity: 0,
                      transition: "opacity 0.15s, background 0.15s",
                    }}
                    onMouseEnter={(e) =>
                      (e.currentTarget.style.background =
                        "var(--border-primary)")
                    }
                    onMouseLeave={(e) =>
                      (e.currentTarget.style.background = "var(--bg-tertiary)")
                    }
                  >
                    {copiedId === item.id ? (
                      <Check size={10} style={{ color: "#22c55e" }} />
                    ) : (
                      <Copy size={10} />
                    )}
                  </button>
                )}
              </div>
            </div>
          );
        })}

        {/* Typing Indicator */}
        {isTyping && (
          <div
            className="agent-fade-in"
            style={{ display: "flex", flexDirection: "column", gap: "6px" }}
          >
            {activeThought && (
              <div
                style={{
                  fontSize: "10px",
                  color: "var(--text-muted)",
                  fontStyle: "italic",
                  padding: "2px 4px",
                  lineHeight: "1.4",
                }}
              >
                Suy nghĩ: {activeThought}
              </div>
            )}
            {activeTool.status === "executing" ? (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  padding: "8px 12px",
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border-primary)",
                  borderRadius: "6px",
                  fontSize: "11px",
                  color: "var(--text-primary)",
                }}
              >
                <span>
                  AI đang chạy công cụ:{" "}
                  <strong style={{ color: "var(--text-primary)" }}>
                    {activeTool.tool_name}
                  </strong>
                </span>
                {activeTool.args && (
                  <span
                    style={{
                      fontSize: "10px",
                      color: "var(--text-muted)",
                      wordBreak: "break-all",
                    }}
                  >
                    ({activeTool.args})
                  </span>
                )}
              </div>
            ) : (
              <div
                style={{
                  padding: "8px 4px",
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                }}
              >
                <div
                  style={{ display: "flex", alignItems: "center", gap: "2px" }}
                >
                  <span
                    className="agent-typing-dot"
                    style={{
                      width: "4px",
                      height: "4px",
                      borderRadius: "50%",
                      background: "var(--text-muted)",
                      display: "inline-block",
                    }}
                  />
                  <span
                    className="agent-typing-dot"
                    style={{
                      width: "4px",
                      height: "4px",
                      borderRadius: "50%",
                      background: "var(--text-muted)",
                      display: "inline-block",
                    }}
                  />
                  <span
                    className="agent-typing-dot"
                    style={{
                      width: "4px",
                      height: "4px",
                      borderRadius: "50%",
                      background: "var(--text-muted)",
                      display: "inline-block",
                    }}
                  />
                </div>
                <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>
                  Agent đang xử lý
                  {loopIterations > 0
                    ? ` (${loopIterations}/${totalIterations})`
                    : ""}
                </span>
              </div>
            )}
          </div>
        )}
        <div ref={chatEndRef} />
      </div>

      {/* ===== BOTTOM INPUT BAR ===== */}
      <div
        style={{
          borderTopLeftRadius: variant === "full" ? "24px" : "0px",
          borderTopRightRadius: variant === "full" ? "24px" : "0px",
          borderTop: variant === "full" ? "none" : "1px solid var(--border-primary)",
          background: "var(--bg-secondary)",
          padding: "0 16px 12px",
          display: "flex",
          flexDirection: "column",
          gap: "8px",
          flexShrink: 0,
          overflow: "hidden",
          boxShadow: variant === "full" ? "0 -4px 12px rgba(0,0,0,0.05)" : "none",
        }}
      >
        {/* Error banner */}
        {alouetteError && (
          <div
            className="agent-fade-in"
            style={{
              padding: "10px 14px",
              background: "rgba(239, 68, 68, 0.06)",
              border: "1px solid rgba(239, 68, 68, 0.2)",
              borderRadius: "8px",
              display: "flex",
              flexDirection: "column",
              gap: "8px",
              marginTop: "10px",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <span
                style={{
                  fontSize: "11px",
                  fontWeight: 600,
                  color: "var(--text-primary)",
                  display: "flex",
                  alignItems: "center",
                  gap: "6px",
                }}
              >
                <AlertCircle size={13} style={{ color: "#ef4444" }} />
                <span>
                  Lỗi ở{" "}
                  <strong style={{ color: "#ef4444" }}>
                    [{alouetteError.projectName}]
                  </strong>
                </span>
              </span>
              <button
                onClick={() => setAlouetteError(null)}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                  display: "flex",
                  padding: "2px",
                  borderRadius: "4px",
                  transition: "all 0.15s",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.color = "var(--text-primary)";
                  e.currentTarget.style.background = "rgba(255,255,255,0.05)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.color = "var(--text-muted)";
                  e.currentTarget.style.background = "transparent";
                }}
              >
                <X size={13} />
              </button>
            </div>

            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: "10.5px",
                color: "#fca5a5",
                background: "rgba(239, 68, 68, 0.04)",
                padding: "6px 10px",
                border: "1px solid rgba(239, 68, 68, 0.1)",
                borderRadius: "6px",
                maxHeight: "72px",
                overflowY: "auto",
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                lineHeight: "1.4",
              }}
            >
              {alouetteError.errorText}
            </div>

            <div
              style={{
                display: "flex",
                gap: "6px",
                justifyContent: "flex-end",
              }}
            >
              <button
                onClick={() => setAlouetteError(null)}
                className="agent-capsule-btn"
                style={{
                  padding: "4px 10px",
                  fontSize: "10.5px",
                  background: "transparent",
                  border: "1px solid var(--border-primary)",
                  color: "var(--text-secondary)",
                  cursor: "pointer",
                  borderRadius: "6px",
                }}
              >
                Bỏ qua
              </button>
              <button
                onClick={() => {
                  handleStartAnalyze(alouetteError.errorText);
                  setAlouetteError(null);
                }}
                className="agent-capsule-btn"
                style={{
                  padding: "4px 10px",
                  fontSize: "10.5px",
                  background: "var(--border-strong, #374151)",
                  border: "none",
                  color: "#fff",
                  fontWeight: 600,
                  cursor: "pointer",
                  borderRadius: "4px",
                  display: "flex",
                  alignItems: "center",
                  gap: "4px",
                }}
              >
                Bắt đầu tìm hiểu
              </button>
            </div>
          </div>
        )}

        {/* Input box styled dynamically */}
        <form
          onSubmit={handleSend}
          style={{
            display: "flex",
            flexDirection: "column",
            background: "transparent",
            border: "none",
            padding: variant === "full" ? "14px 0" : "12px 0",
            gap: "8px",
            position: "relative",
            boxShadow: "none",
            margin: variant === "full" ? "10px 0" : "0",
          }}
        >
          {/* Top Row: Icons on Left, "Local Config v" on Right */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              paddingBottom: "2px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
              <button
                type="button"
                onClick={() =>
                  setThinkingMode((prev) => (prev === "high" ? "low" : "high"))
                }
                style={{
                  background: "none",
                  border: "none",
                  padding: 0,
                  color:
                    thinkingMode === "high"
                      ? "#a855f7"
                      : "var(--text-muted)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color =
                    thinkingMode === "high"
                      ? "#c084fc"
                      : "var(--text-secondary)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color =
                    thinkingMode === "high"
                      ? "#a855f7"
                      : "var(--text-muted)")
                }
                title={
                  thinkingMode === "high"
                    ? "Thinking Mode: High (Force reasoning)"
                    : "Thinking Mode: Low (Automatic)"
                }
              >
                <Brain size={13} />
              </button>
              <button
                type="button"
                onClick={() => setCapsOpen(!capsOpen)}
                style={{
                  background: "none",
                  border: "none",
                  padding: 0,
                  color: capsOpen ? "#38bdf8" : ("var(--text-muted)"),
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color = capsOpen
                    ? "#38bdf8"
                    : "var(--text-secondary)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color = capsOpen
                    ? "#38bdf8"
                    : "var(--text-muted)")
                }
                title="Config permissions"
              >
                <Wrench size={13} />
              </button>
              <button
                type="button"
                onClick={() => setCapsOpen(!capsOpen)}
                style={{
                  background: "none",
                  border: "none",
                  padding: 0,
                  color: "var(--text-muted)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color = "var(--text-primary)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color = "var(--text-secondary)")
                }
                title="Workspace status"
              >
                <Box size={13} />
              </button>
            </div>

            <button
              type="button"
              onClick={() => setCapsOpen(!capsOpen)}
              style={{
                background: "none",
                border: "none",
                padding: "2px 6px",
                color: "var(--text-secondary)",
                fontSize: "12.5px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "4px",
                fontFamily: "var(--font-sans)",
                transition: "color 0.2s",
              }}
              onMouseEnter={(e) =>
                (e.currentTarget.style.color = "var(--text-primary)")
              }
              onMouseLeave={(e) =>
                (e.currentTarget.style.color = "var(--text-secondary)")
              }
            >
              <span style={{ display: isCompact ? "none" : "inline" }}>Local Config</span>
              <ChevronDown size={11} style={{ opacity: 0.8 }} />
            </button>
          </div>

          {/* Capabilities Panel inside the box when expanded */}
          {capsOpen && (
            <div
              className="agent-fade-in"
              style={{
                display: "flex",
                flexWrap: "wrap",
                gap: "4px",
                padding: "8px 0 4px",
                borderBottom: "1px solid var(--border-primary)",
                borderTop: "1px solid var(--border-primary)",
              }}
            >
              {capList.map((item) => {
                return (
                  <button
                    key={item.key}
                    type="button"
                    onClick={() => toggleCapability(item.key)}
                    className="agent-capsule-btn"
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "4px",
                      padding: "4px 8px",
                      fontSize: "10px",
                      borderRadius: "4px",
                      border: item.isActive
                        ? "1px solid var(--border-primary)"
                        : "1px solid var(--border-primary)",
                      background: item.isActive
                        ? "var(--bg-secondary)"
                        : "transparent",
                      color: item.isActive
                        ? "var(--text-primary)"
                        : "var(--text-muted)",
                      cursor: "pointer",
                      transition: "all 0.1s",
                      outline: "none",
                    }}
                  >
                    <span>{item.label}</span>
                  </button>
                );
              })}
            </div>
          )}

          {/* Mentions Autocomplete Dropdown */}
          {showMentions && filteredMentions.length > 0 && (
            <div
              style={{
                position: "absolute",
                bottom: "100%",
                left: "18px",
                right: "18px",
                marginBottom: "8px",
                background: "var(--bg-primary)",
                border: "1px solid var(--border-primary)",
                borderRadius: "8px",
                boxShadow:
                  "0 -8px 24px rgba(0, 0, 0, 0.5), 0 8px 24px rgba(0, 0, 0, 0.5)",
                zIndex: 1000,
                maxHeight: "220px",
                overflowY: "auto",
                padding: "6px",
                display: "flex",
                flexDirection: "column",
                gap: "2px",
              }}
            >
              {filteredMentions.map((item, idx) => {
                const isSelected = idx === mentionSelectedIndex;
                return (
                  <button
                    key={item.path}
                    type="button"
                    onClick={() => handleSelectMention(item)}
                    onMouseEnter={() => setMentionSelectedIndex(idx)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                      width: "100%",
                      padding: "6px 10px",
                      background: isSelected
                        ? "var(--bg-secondary)"
                        : "transparent",
                      border: "none",
                      borderRadius: "6px",
                      color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
                      fontSize: "12px",
                      cursor: "pointer",
                      textAlign: "left",
                      outline: "none",
                      transition: "background 0.15s, color 0.15s",
                    }}
                  >
                    {item.is_dir ? (
                      <Folder
                        size={13}
                        style={{
                          color: "var(--text-secondary)",
                          flexShrink: 0,
                        }}
                      />
                    ) : (
                      <File
                        size={13}
                        style={{
                          color: "var(--text-secondary)",
                          flexShrink: 0,
                        }}
                      />
                    )}
                    <span style={{ fontWeight: 500, flexShrink: 0 }}>
                      {item.name}
                    </span>
                    <span
                      style={{
                        color: "var(--text-muted)",
                        fontSize: "10.5px",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        marginLeft: "4px",
                      }}
                    >
                      {item.path}
                    </span>
                  </button>
                );
              })}
            </div>
          )}

          {/* Selected Mentions Pills */}
          {selectedContextItems.length > 0 && (
            <div
              style={{
                display: "flex",
                flexWrap: "wrap",
                gap: "6px",
                padding: "4px 0 8px 0",
              }}
            >
              {selectedContextItems.map((item) => (
                <div
                  key={item.path}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "6px",
                    background: "var(--bg-secondary)",
                    border: "1px solid var(--border-primary)",
                    borderRadius: "16px",
                    padding: "3px 10px",
                    fontSize: "11px",
                    color: "var(--text-primary)",
                    fontFamily: "var(--font-sans)",
                    userSelect: "none",
                  }}
                >
                  {item.is_dir ? (
                    <Folder
                      size={11}
                      style={{
                        color: "var(--text-secondary)",
                        flexShrink: 0,
                      }}
                    />
                  ) : (
                    <File
                      size={11}
                      style={{
                        color: "var(--text-secondary)",
                        flexShrink: 0,
                      }}
                    />
                  )}
                  <span style={{ fontWeight: 500 }}>{item.name}</span>
                  <button
                    type="button"
                    onClick={() =>
                      setSelectedContextItems((prev) =>
                        prev.filter((x) => x.path !== item.path),
                      )
                    }
                    style={{
                      background: "none",
                      border: "none",
                      color: "inherit",
                      cursor: "pointer",
                      display: "flex",
                      padding: 0,
                      opacity: 0.6,
                      alignItems: "center",
                      justifyContent: "center",
                      transition: "opacity 0.15s",
                    }}
                    onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
                    onMouseLeave={(e) =>
                      (e.currentTarget.style.opacity = "0.6")
                    }
                    title="Remove context"
                  >
                    <X size={10} />
                  </button>
                </div>
              ))}
            </div>
          )}

          {/* Text Area */}
          <textarea
            ref={textareaRef}
            rows={1}
            placeholder="Ask anything, '@' to add context"
            value={inputVal}
            onChange={(e) => {
              setInputVal(e.target.value);
              checkMentionTrigger(e.target.value, e.target.selectionStart);
            }}
            onSelect={(e) => {
              const target = e.target as HTMLTextAreaElement;
              checkMentionTrigger(target.value, target.selectionStart);
            }}
            onKeyDown={handleKeyDown}
            disabled={isTyping}
            style={{
              width: "100%",
              background: "transparent",
              border: "none",
              color: "var(--text-primary)",
              padding: variant === "full" ? "10px 0" : "6px 0",
              fontSize: variant === "full" ? "16px" : "15px",
              outline: "none",
              resize: "none",
              fontFamily: "var(--font-sans)",
              lineHeight: "1.5",
              maxHeight: "200px",
              minHeight: variant === "full" ? "48px" : "36px",
              overflowY: "auto",
            }}
          />

          {/* Bottom Row: Custom Dropdowns (Agent, Model, @) on Left, Blue Send Button on Right */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              marginTop: "2px",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "8px",
                position: "relative",
              }}
            >
              {/* Agent Mode Pill Dropdown */}
              <div ref={modeDropdownRef} style={{ position: "relative" }}>
                <button
                  type="button"
                  onClick={() => setModeDropdownOpen(!modeDropdownOpen)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "4px",
                    padding: "5px 12px",
                    background: "var(--bg-secondary)",
                    border: "1px solid var(--border-primary)",
                    borderRadius: "20px",
                    color: "var(--text-secondary)",
                    fontSize: "12px",
                    cursor: "pointer",
                    transition: "all 0.2s",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      "var(--bg-secondary)";
                    e.currentTarget.style.color = "var(--text-primary)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background =
                      "var(--bg-secondary)";
                    e.currentTarget.style.color = "var(--text-secondary)";
                  }}
                >
                  <Sparkles size={12} style={{ opacity: 0.8 }} />
                  <span>
                    {selectedMode === "interactive"
                      ? "Agent"
                      : selectedMode === "autonomous"
                        ? "Autonomous"
                        : "Copilot"}
                  </span>
                  <ChevronDown size={11} style={{ opacity: 0.6 }} />
                </button>

                {modeDropdownOpen && (
                  <div
                    style={{
                      position: "absolute",
                      bottom: "calc(100% + 6px)",
                      left: 0,
                      background: "var(--bg-primary)",
                      border: "1px solid var(--border-primary)",
                      borderRadius: "8px",
                      boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
                      zIndex: 1000,
                      padding: "4px",
                      minWidth: "120px",
                    }}
                  >
                    {[
                      { value: "interactive", label: "Agent (Interactive)" },
                      { value: "autonomous", label: "Autonomous" },
                      { value: "copilot", label: "Copilot" },
                    ].map((mode) => (
                      <button
                        key={mode.value}
                        type="button"
                        onClick={() => {
                          setSelectedMode(mode.value);
                          setModeDropdownOpen(false);
                        }}
                        style={{
                          width: "100%",
                          textAlign: "left",
                          padding: "6px 8px",
                          background:
                            selectedMode === mode.value
                              ? "var(--bg-secondary)"
                              : "transparent",
                          border: "none",
                          borderRadius: "4px",
                          color: "var(--text-primary)",
                          fontSize: "11px",
                          cursor: "pointer",
                        }}
                      >
                        {mode.label}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Model Dropdown */}
              <div ref={modelDropdownRef} style={{ position: "relative" }}>
                <button
                  type="button"
                  onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
                  style={{
                    display: isCompact ? "none" : "flex",
                    alignItems: "center",
                    gap: "4px",
                    padding: "4px 8px",
                    background: "transparent",
                    border: "none",
                    color: "var(--text-secondary)",
                    fontSize: "12px",
                    cursor: "pointer",
                    transition: "color 0.2s",
                  }}
                  onMouseEnter={(e) =>
                    (e.currentTarget.style.color = "var(--text-primary)")
                  }
                  onMouseLeave={(e) =>
                    (e.currentTarget.style.color = "var(--text-secondary)")
                  }
                >
                  <span>
                    {availableModels.find((m) => m.id === selectedModel)
                      ?.name || selectedModel}
                  </span>
                  <ChevronDown size={11} style={{ opacity: 0.6 }} />
                </button>

                {modelDropdownOpen && (
                  <div
                    style={{
                      position: "absolute",
                      bottom: "calc(100% + 6px)",
                      left: 0,
                      background: "var(--bg-primary)",
                      border: "1px solid var(--border-primary)",
                      borderRadius: "8px",
                      boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
                      zIndex: 1000,
                      padding: "4px",
                      minWidth: "160px",
                      maxHeight: "200px",
                      overflowY: "auto",
                    }}
                  >
                    {availableModels.map((m) => (
                      <button
                        key={m.id}
                        type="button"
                        onClick={() => {
                          setSelectedModel(m.id);
                          setModelDropdownOpen(false);
                        }}
                        style={{
                          width: "100%",
                          textAlign: "left",
                          padding: "6px 8px",
                          background:
                            selectedModel === m.id
                              ? "var(--bg-secondary)"
                              : "transparent",
                          border: "none",
                          borderRadius: "4px",
                          color: "var(--text-primary)",
                          fontSize: "11px",
                          cursor: "pointer",
                        }}
                      >
                        {m.name}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Mentions button: @ */}
              <button
                type="button"
                onClick={() => {
                  setInputVal((prev) => prev + "@");
                  textareaRef.current?.focus();
                }}
                style={{
                  background: "none",
                  border: "none",
                  padding: "4px 8px",
                  color: "var(--text-secondary)",
                  fontSize: "13px",
                  cursor: "pointer",
                  fontFamily: "var(--font-sans)",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color = "var(--text-primary)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color = "var(--text-secondary)")
                }
                title="Add context (@)"
              >
                @
              </button>
            </div>

            {/* Token counter (center) */}
            <span
              style={{
                position: "relative",
                display: isCompact ? "none" : "inline-flex",
                alignItems: "center",
                gap: "5px",
                cursor: "help",
                userSelect: "none",
              }}
              onMouseEnter={() => setShowTokenTooltip(true)}
              onMouseLeave={() => setShowTokenTooltip(false)}
            >
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                  width: "14px",
                  height: "14px",
                  borderRadius: "50%",
                  background: showTokenTooltip
                    ? "rgba(167, 139, 250, 0.15)"
                    : "var(--bg-secondary)",
                  border: showTokenTooltip
                    ? "1px solid rgba(167, 139, 250, 0.5)"
                    : "1px solid var(--border-primary)",
                  fontSize: "7px",
                  fontWeight: 700,
                  color: showTokenTooltip
                    ? "#a78bfa"
                    : "var(--text-muted)",
                  transition: "all 0.2s",
                  lineHeight: 1,
                  flexShrink: 0,
                }}
              >
                Σ
              </span>
              <span
                style={{
                  fontSize: "11px",
                  color: showTokenTooltip
                    ? "var(--text-secondary)"
                    : "var(--text-muted)",
                  transition: "color 0.2s",
                  fontFamily: "var(--font-mono)",
                }}
              >
                {inputVal.trim()
                  ? `~${estimatedInputTokens}`
                  : `~${totalSessionTokens.toLocaleString()}`}
              </span>

              {/* Tooltip (pops up above) */}
              {showTokenTooltip && (
                <div
                  style={{
                    position: "absolute",
                    bottom: "calc(100% + 10px)",
                    left: "50%",
                    transform: "translateX(-50%)",
                    background: "var(--bg-primary)",
                    border: "1px solid rgba(255,255,255,0.12)",
                    borderRadius: "8px",
                    padding: "8px 12px",
                    whiteSpace: "nowrap",
                    zIndex: 2000,
                    boxShadow: "0 8px 24px rgba(0,0,0,0.6)",
                    pointerEvents: "none",
                  }}
                >
                  <div
                    style={{
                      fontSize: "10px",
                      fontWeight: 700,
                      color: "var(--text-primary)",
                      marginBottom: "6px",
                      letterSpacing: "0.03em",
                    }}
                  >
                    Ước lượng Token
                  </div>
                  <div
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: "3px",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        gap: "16px",
                        fontSize: "10px",
                        color: "var(--text-secondary)",
                      }}
                    >
                      <span>Tin nhắn hiện tại</span>
                      <span
                        style={{
                          color: "#a78bfa",
                          fontWeight: 600,
                          fontFamily: "var(--font-mono)",
                        }}
                      >
                        ~{estimatedInputTokens}
                      </span>
                    </div>
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        gap: "16px",
                        fontSize: "10px",
                        color: "var(--text-secondary)",
                      }}
                    >
                      <span>Tổng session</span>
                      <span
                        style={{
                          color: "#38bdf8",
                          fontWeight: 600,
                          fontFamily: "var(--font-mono)",
                        }}
                      >
                        ~{totalSessionTokens.toLocaleString()}
                      </span>
                    </div>
                  </div>
                  <div
                    style={{
                      marginTop: "6px",
                      paddingTop: "5px",
                      borderTop: "1px solid rgba(255,255,255,0.06)",
                      fontSize: "9px",
                      color: "var(--text-muted)",
                      fontStyle: "italic",
                    }}
                  >
                    ~4 ký tự / token
                  </div>
                </div>
              )}
            </span>

            {/* Blue Send / Cancel Button */}
            {isTyping ? (
              <button
                type="button"
                onClick={handleCancel}
                style={{
                  background: "#ef4444",
                  border: "none",
                  color: "#fff",
                  width: "28px",
                  height: "28px",
                  borderRadius: "8px",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  flexShrink: 0,
                }}
                title="Dừng hoạt động"
              >
                <span
                  style={{
                    width: "8px",
                    height: "8px",
                    background: "#fff",
                    borderRadius: "1px",
                  }}
                />
              </button>
            ) : (
              <button
                type="submit"
                disabled={!inputVal.trim()}
                style={{
                  background: inputVal.trim()
                    ? "#0078d4"
                    : "var(--bg-secondary)",
                  border: inputVal.trim()
                    ? "none"
                    : "1px solid var(--border-primary)",
                  color: inputVal.trim() ? "var(--text-primary)" : "var(--text-muted)",
                  width: "28px",
                  height: "28px",
                  borderRadius: "8px",
                  cursor: inputVal.trim() ? "pointer" : "default",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  transition: "all 0.2s",
                  flexShrink: 0,
                }}
              >
                <CornerDownLeft size={13} strokeWidth={2.5} />
              </button>
            )}
          </div>
        </form>


      </div>

      {/* ===== HISTORY MODAL ===== */}
      {historyModalOpen && (
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0, 0, 0, 0.65)",
            backdropFilter: "blur(4px)",
            zIndex: 2000,
            display: "flex",
            flexDirection: "column",
            justifyContent: "center",
            padding: "20px",
          }}
          onClick={() => setHistoryModalOpen(false)}
        >
          <div
            style={{
              background: "var(--bg-secondary)",
              border: "1px solid var(--border-primary)",
              borderRadius: "8px",
              boxShadow: "0 16px 48px rgba(0, 0, 0, 0.6)",
              display: "flex",
              flexDirection: "column",
              maxHeight: "80%",
              width: "100%",
              overflow: "hidden",
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "12px 16px",
                borderBottom: "1px solid var(--border-primary)",
                background: "var(--bg-primary)",
              }}
            >
              <span
                style={{
                  fontSize: "13px",
                  fontWeight: 600,
                  color: "var(--text-primary)",
                }}
              >
                Lịch sử Chat Agent
              </span>
              <button
                onClick={() => setHistoryModalOpen(false)}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--text-secondary)",
                  cursor: "pointer",
                  padding: "4px",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <X size={15} />
              </button>
            </div>

            <div
              className="agent-scroll"
              style={{
                flex: 1,
                overflowY: "auto",
                padding: "8px",
                display: "flex",
                flexDirection: "column",
                gap: "6px",
              }}
            >
              {historyItems.length === 0 ? (
                <div
                  style={{
                    textAlign: "center",
                    padding: "30px",
                    color: "var(--text-muted)",
                    fontSize: "11.5px",
                  }}
                >
                  Chưa có lịch sử chat nào.
                </div>
              ) : (
                historyItems.map((item) => (
                  <div
                    key={item.session_id}
                    onClick={() =>
                      handleLoadSession(item.session_id, item.title)
                    }
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      padding: "8px 12px",
                      background: "var(--bg-primary)",
                      border: "1px solid var(--border-primary)",
                      borderRadius: "6px",
                      cursor: "pointer",
                      transition: "all var(--transition-fast)",
                      position: "relative",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.borderColor = "var(--border-focus)";
                      e.currentTarget.style.backgroundColor =
                        "var(--bg-tertiary)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.borderColor =
                        "var(--border-primary)";
                      e.currentTarget.style.backgroundColor =
                        "var(--bg-primary)";
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "flex-start",
                        gap: "10px",
                      }}
                    >
                      <span
                        style={{
                          fontSize: "11.5px",
                          fontWeight: 600,
                          color: "var(--text-primary)",
                          wordBreak: "break-all",
                        }}
                      >
                        {item.title}
                      </span>
                      <button
                        onClick={(e) => handleDeleteSession(item.session_id, e)}
                        style={{
                          background: "none",
                          border: "none",
                          color: "var(--color-danger)",
                          cursor: "pointer",
                          padding: "2px",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          opacity: 0.6,
                        }}
                        onMouseEnter={(e) =>
                          (e.currentTarget.style.opacity = "1")
                        }
                        onMouseLeave={(e) =>
                          (e.currentTarget.style.opacity = "0.6")
                        }
                        title="Xóa lịch sử"
                      >
                        <X size={12} />
                      </button>
                    </div>
                    <div
                      style={{
                        display: "flex",
                        flexWrap: "wrap",
                        gap: "6px",
                        marginTop: "6px",
                        fontSize: "9px",
                        color: "var(--text-muted)",
                      }}
                    >
                      <span
                        style={{
                          padding: "1px 4px",
                          background: "var(--bg-secondary)",
                          borderRadius: "3px",
                        }}
                      >
                        {item.model}
                      </span>
                      <span
                        style={{
                          padding: "1px 4px",
                          background: "var(--bg-secondary)",
                          borderRadius: "3px",
                          textTransform: "uppercase",
                        }}
                      >
                        {item.mode}
                      </span>
                      <span>
                        {new Date(item.created_at * 1000).toLocaleString([], {
                          month: "short",
                          day: "numeric",
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
