import React, { useState, useRef, useEffect } from "react";
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
  Chrome,
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
        }}
      >
        <span
          style={{ color: approved ? "#22c55e" : "#ef4444", fontSize: "9px" }}
        >
          {approved ? "✓" : "✕"}
        </span>
        <span style={{ color: "var(--text-muted)" }}>
          {getFriendlyToolNameStatic(tool.name)}
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
            {getFriendlyToolNameStatic(tool.name)}
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

function getFriendlyToolNameStatic(name: string): string {
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
            {getFriendlyToolNameStatic(item.toolName || "")}
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
  const [expandedSkills, setExpandedSkills] = useState<Record<string, boolean>>(
    {},
  );

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
      const list = await invoke<any[]>("agent_get_history");
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
  >([
    { id: "gemini-3.5-flash", name: "Gemini 3.5 Flash" },
    { id: "claude-opus-4.7", name: "Claude Opus 4.7" },
    { id: "deepseek-v4-pro", name: "DeepSeek-V4 Pro" },
  ]);

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
          setIsTyping(false);
        },
      );
    };

    setupStreamListeners();

    return () => {
      if (watchdogTimer) clearTimeout(watchdogTimer);
      if (unlistenText) unlistenText();
      if (unlistenThought) unlistenThought();
      if (unlistenThoughtFinal) unlistenThoughtFinal();
      if (unlistenStreamComplete) unlistenStreamComplete();
    };
  }, []);

  useEffect(() => {
    const loadActiveModels = async () => {
      let activeIds: string[] = [
        "deepseek-v4-pro",
        "claude-opus-4.7",
        "gemini-3.5-flash",
      ];
      let activeModelBackend = "";
      try {
        const config = await invoke<any>("get_custom_ai_config");
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

      const savedCustom = localStorage.getItem("alouette_custom_models");
      const customs: any[] = savedCustom ? JSON.parse(savedCustom) : [];

      const list: { id: string; name: string }[] = [];

      const providerModelsMapping: {
        [providerId: string]: { id: string; name: string }[];
      } = {
        deepseek: [
          { id: "deepseek-v4-pro", name: "DeepSeek-V4 Pro" },
          { id: "deepseek-v4", name: "DeepSeek-V4" },
          { id: "deepseek-v4-flash", name: "DeepSeek-V4 Flash" },
          { id: "deepseek-r1", name: "DeepSeek-R1" },
        ],
        "gpt-chatgpt": [
          { id: "gpt-5.5", name: "GPT-5.5" },
          { id: "o1-pro", name: "o1-Pro (Reasoning)" },
          { id: "o3-mini", name: "o3-Mini (Coding)" },
          { id: "gpt-4o", name: "GPT-4o (Vision)" },
        ],
        gemini: [
          { id: "gemini-3.5-flash", name: "Gemini 3.5 Flash" },
          { id: "gemini-3.1-pro", name: "Gemini 3.1 Pro" },
          { id: "gemini-1.5-flash", name: "Gemini 1.5 Flash" },
          { id: "gemini-1.5-pro", name: "Gemini 1.5 Pro" },
        ],
        claude: [
          { id: "claude-opus-4.7", name: "Claude Opus 4.7" },
          { id: "claude-sonnet-5", name: "Claude Sonnet 5" },
        ],
        qwen: [{ id: "qwen-3.7-max", name: "Qwen 3.7 Max" }],
      };

      Object.keys(providerModelsMapping).forEach((provId) => {
        providerModelsMapping[provId].forEach((m) => {
          if (activeIds.includes(m.id)) {
            list.push(m);
          }
        });
      });

      customs.forEach((c) => {
        if (activeIds.includes(c.id)) {
          list.push({ id: c.id, name: `${c.provider} - ${c.name}` });
        }
      });

      if (list.length > 0) {
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
    const interval = setInterval(loadActiveModels, 2000);

    return () => {
      window.removeEventListener("storage", loadActiveModels);
      clearInterval(interval);
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
        if (processedIters.current.has(iterKey)) return;
        processedIters.current.add(iterKey);

        setActiveThought(data.thought || null);
        setLoopIterations(data.iteration || 0);

        if (data.tool_name) {
          const status = data.tool_result
            ? data.tool_success
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
    if (!inputVal.trim()) return;
    const text = inputVal;
    setInputVal("");
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
      alert(`Lỗi khi phê duyệt tool: ${err?.message || err}`);
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
      alert(`Lỗi khi từ chối tool: ${err?.message || err}`);
    } finally {
      isActiveSender.current = false;
    }
  };

  const handleNewChat = async () => {
    try {
      await invoke("agent_reset_session");
      setChatHistory([]);
      setSessionId(null);
      setSessionTitle("New Chat");
      setMenuOpen(false);
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
      icon: Chrome,
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
      label: "Post Mini",
      icon: Zap,
      isActive: capabilities.postMini,
    },
    { key: "git", label: "Git", icon: GitBranch, isActive: capabilities.git },
  ];

  const getToolStatusIcon = (status?: string) => {
    switch (status) {
      case "running":
        return <RefreshCw size={11} className="agent-spin" />;
      case "success":
        return <Check size={11} style={{ color: "#22c55e" }} />;
      case "failed":
        return <X size={11} style={{ color: "#ef4444" }} />;
      case "waiting":
        return <AlertCircle size={11} style={{ color: "#f59e0b" }} />;
      default:
        return null;
    }
  };

  const getToolStatusText = (status?: string) => {
    switch (status) {
      case "running":
        return "Đang chạy";
      case "success":
        return "Hoàn thành";
      case "failed":
        return "Thất bại";
      case "waiting":
        return "Chờ duyệt";
      case "approved":
        return "Đã duyệt";
      case "rejected":
        return "Đã từ chối";
      default:
        return "";
    }
  };

  const getFriendlyToolName = (name: string) => {
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
        return `Công cụ: ${name}`;
    }
  };

  return (
    <div
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
            <span
              style={{
                fontSize: "9px",
                color: "var(--text-muted)",
                lineHeight: 1,
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
            return (
              <div
                key={item.id}
                className="message-container agent-fade-in"
                style={{
                  padding: "8px 12px",
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border-primary)",
                  borderRadius: "6px",
                  fontSize: "11px",
                  fontFamily: "var(--font-mono)",
                  color: "var(--text-secondary)",
                  lineHeight: "1.5",
                  whiteSpace: "pre-wrap",
                  position: "relative",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    marginBottom: "4px",
                  }}
                >
                  <span
                    style={{
                      fontSize: "9px",
                      fontWeight: 600,
                      color: "var(--text-secondary)",
                      textTransform: "uppercase",
                      letterSpacing: "0.05em",
                    }}
                  >
                    Hoạt động
                  </span>
                  <span
                    style={{
                      fontSize: "9px",
                      color: "var(--text-muted)",
                      marginLeft: "auto",
                    }}
                  >
                    {item.timestamp}
                  </span>
                </div>
                <div style={{ color: "var(--text-primary)", fontSize: "11px" }}>
                  {item.text}
                </div>
              </div>
            );
          }

          if (item.type === "skill_call") {
            const isExpanded = !!expandedSkills[item.id];
            const statusColor =
              item.toolStatus === "success"
                ? "#22c55e"
                : item.toolStatus === "failed"
                  ? "#ef4444"
                  : item.toolStatus === "running"
                    ? "#6366f1"
                    : "var(--text-muted)";

            return (
              <div
                key={item.id}
                className="message-container agent-fade-in"
                style={{
                  border: "1px solid var(--border-primary)",
                  borderRadius: "8px",
                  background: "var(--bg-primary)",
                  overflow: "hidden",
                }}
              >
                {/* Header */}
                <div
                  onClick={() => {
                    setExpandedSkills((prev) => ({
                      ...prev,
                      [item.id]: !prev[item.id],
                    }));
                  }}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "8px 12px",
                    cursor: "pointer",
                    userSelect: "none",
                    background: isExpanded
                      ? "rgba(255,255,255,0.015)"
                      : "transparent",
                    transition: "background 0.15s",
                  }}
                  onMouseEnter={(e) => {
                    if (!isExpanded)
                      e.currentTarget.style.background =
                        "rgba(255,255,255,0.02)";
                  }}
                  onMouseLeave={(e) => {
                    if (!isExpanded)
                      e.currentTarget.style.background = "transparent";
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                    }}
                  >
                    {isExpanded ? (
                      <ChevronDown
                        size={11}
                        style={{ color: "var(--text-muted)", flexShrink: 0 }}
                      />
                    ) : (
                      <ChevronRight
                        size={11}
                        style={{ color: "var(--text-muted)", flexShrink: 0 }}
                      />
                    )}
                    {getToolIconComponent(item.toolName || "")}
                    <span
                      style={{
                        fontSize: "11px",
                        fontWeight: 600,
                        color: "var(--text-primary)",
                      }}
                    >
                      {getFriendlyToolName(item.toolName || "")}
                    </span>
                  </div>

                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                    }}
                  >
                    <span
                      style={{
                        fontSize: "9px",
                        color: statusColor,
                        display: "flex",
                        alignItems: "center",
                        gap: "3px",
                        fontWeight: 500,
                      }}
                    >
                      {getToolStatusIcon(item.toolStatus)}
                      {getToolStatusText(item.toolStatus)}
                    </span>
                    <span
                      style={{ fontSize: "9px", color: "var(--text-muted)" }}
                    >
                      {item.timestamp}
                    </span>
                  </div>
                </div>

                {/* Expanded Details */}
                {isExpanded && (
                  <div
                    style={{
                      borderTop: "1px solid var(--border-primary)",
                      background: "var(--bg-secondary)",
                      padding: "10px 12px",
                      display: "flex",
                      flexDirection: "column",
                      gap: "8px",
                    }}
                  >
                    {item.args && (
                      <div>
                        <div
                          style={{
                            fontSize: "9px",
                            color: "var(--text-muted)",
                            textTransform: "uppercase",
                            fontWeight: 700,
                            marginBottom: "4px",
                            letterSpacing: "0.04em",
                          }}
                        >
                          Tham số đầu vào
                        </div>
                        <pre
                          style={{
                            margin: 0,
                            padding: "6px 8px",
                            background: "var(--bg-primary)",
                            border: "1px solid var(--border-primary)",
                            borderRadius: "6px",
                            fontFamily: "var(--font-mono)",
                            fontSize: "10px",
                            color: "var(--text-primary)",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                            lineHeight: "1.4",
                          }}
                        >
                          {item.args}
                        </pre>
                      </div>
                    )}

                    {item.toolResult && (
                      <div>
                        <div
                          style={{
                            fontSize: "9px",
                            color: "var(--text-muted)",
                            textTransform: "uppercase",
                            fontWeight: 700,
                            marginBottom: "4px",
                            letterSpacing: "0.04em",
                          }}
                        >
                          Kết quả trả về
                        </div>
                        <pre
                          style={{
                            margin: 0,
                            padding: "6px 8px",
                            background: "var(--bg-primary)",
                            border: "1px solid var(--border-primary)",
                            borderRadius: "6px",
                            fontFamily: "var(--font-mono)",
                            fontSize: "10px",
                            color: "var(--text-secondary)",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                            maxHeight: "180px",
                            overflowY: "auto",
                            lineHeight: "1.4",
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
                  <span style={{ fontSize: "9px", color: "var(--text-muted)" }}>
                    {item.timestamp}
                  </span>
                </div>

                <div
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "4px",
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
              }}
            >
              {/* Sender label */}
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
                  <div style={{ color: "#ef4444" }}>{item.text}</div>
                ) : (
                  <div>{item.text}</div>
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
          borderTop: "1px solid var(--border-primary)",
          background: "var(--bg-secondary)",
          padding: "0 16px 12px",
          display: "flex",
          flexDirection: "column",
          gap: "8px",
          flexShrink: 0,
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

        {/* Input box styled exactly like the screenshot */}
        <form
          onSubmit={handleSend}
          style={{
            display: "flex",
            flexDirection: "column",
            background: "#18181b",
            border: "1px solid rgba(255, 255, 255, 0.08)",
            borderRadius: "24px",
            padding: "12px 18px",
            gap: "6px",
            position: "relative",
            boxShadow: "0 10px 30px -10px rgba(0, 0, 0, 0.7)",
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
                      : "rgba(255, 255, 255, 0.35)",
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
                      : "rgba(255, 255, 255, 0.75)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color =
                    thinkingMode === "high"
                      ? "#a855f7"
                      : "rgba(255, 255, 255, 0.35)")
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
                  color: capsOpen ? "#38bdf8" : "rgba(255, 255, 255, 0.35)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color = capsOpen
                    ? "#38bdf8"
                    : "rgba(255, 255, 255, 0.75)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color = capsOpen
                    ? "#38bdf8"
                    : "rgba(255, 255, 255, 0.35)")
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
                  color: "rgba(255, 255, 255, 0.35)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color = "rgba(255, 255, 255, 0.75)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color = "rgba(255, 255, 255, 0.35)")
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
                color: "rgba(255, 255, 255, 0.45)",
                fontSize: "12.5px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "4px",
                fontFamily: "var(--font-sans)",
                transition: "color 0.2s",
              }}
              onMouseEnter={(e) =>
                (e.currentTarget.style.color = "rgba(255, 255, 255, 0.8)")
              }
              onMouseLeave={(e) =>
                (e.currentTarget.style.color = "rgba(255, 255, 255, 0.45)")
              }
            >
              <span>Local Config</span>
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
                borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
                borderTop: "1px solid rgba(255, 255, 255, 0.08)",
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
                        ? "1px solid rgba(255, 255, 255, 0.2)"
                        : "1px solid rgba(255, 255, 255, 0.05)",
                      background: item.isActive
                        ? "rgba(255, 255, 255, 0.08)"
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

          {/* Text Area */}
          <textarea
            ref={textareaRef}
            rows={1}
            placeholder="Ask anything, '@' to add context"
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={isTyping}
            style={{
              width: "100%",
              background: "transparent",
              border: "none",
              color: "rgba(255, 255, 255, 0.9)",
              padding: "6px 0",
              fontSize: "15px",
              outline: "none",
              resize: "none",
              fontFamily: "var(--font-sans)",
              lineHeight: "1.5",
              maxHeight: "160px",
              minHeight: "36px",
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
                    background: "rgba(255, 255, 255, 0.05)",
                    border: "1px solid rgba(255, 255, 255, 0.08)",
                    borderRadius: "20px",
                    color: "rgba(255, 255, 255, 0.65)",
                    fontSize: "12px",
                    cursor: "pointer",
                    transition: "all 0.2s",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      "rgba(255, 255, 255, 0.08)";
                    e.currentTarget.style.color = "#fff";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background =
                      "rgba(255, 255, 255, 0.05)";
                    e.currentTarget.style.color = "rgba(255, 255, 255, 0.65)";
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
                      background: "#242424",
                      border: "1px solid rgba(255, 255, 255, 0.1)",
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
                              ? "rgba(255, 255, 255, 0.08)"
                              : "transparent",
                          border: "none",
                          borderRadius: "4px",
                          color: "#fff",
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
                    display: "flex",
                    alignItems: "center",
                    gap: "4px",
                    padding: "4px 8px",
                    background: "transparent",
                    border: "none",
                    color: "rgba(255, 255, 255, 0.45)",
                    fontSize: "12px",
                    cursor: "pointer",
                    transition: "color 0.2s",
                  }}
                  onMouseEnter={(e) =>
                    (e.currentTarget.style.color = "rgba(255, 255, 255, 0.75)")
                  }
                  onMouseLeave={(e) =>
                    (e.currentTarget.style.color = "rgba(255, 255, 255, 0.45)")
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
                      background: "#242424",
                      border: "1px solid rgba(255, 255, 255, 0.1)",
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
                              ? "rgba(255, 255, 255, 0.08)"
                              : "transparent",
                          border: "none",
                          borderRadius: "4px",
                          color: "#fff",
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
                  color: "rgba(255, 255, 255, 0.45)",
                  fontSize: "13px",
                  cursor: "pointer",
                  fontFamily: "var(--font-sans)",
                  transition: "color 0.2s",
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.color = "rgba(255, 255, 255, 0.75)")
                }
                onMouseLeave={(e) =>
                  (e.currentTarget.style.color = "rgba(255, 255, 255, 0.45)")
                }
                title="Add context (@)"
              >
                @
              </button>
            </div>

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
                    : "rgba(255, 255, 255, 0.05)",
                  border: inputVal.trim()
                    ? "none"
                    : "1px solid rgba(255, 255, 255, 0.08)",
                  color: inputVal.trim() ? "#fff" : "rgba(255, 255, 255, 0.25)",
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

        {/* Below the box: "Last Session" and Status */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "0 4px",
          }}
        >
          <button
            type="button"
            onClick={handleNewChat}
            style={{
              background: "none",
              border: "none",
              color: "rgba(255, 255, 255, 0.4)",
              fontSize: "12px",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: "4px",
              padding: 0,
              fontFamily: "var(--font-sans)",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.color = "rgba(255, 255, 255, 0.7)")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.color = "rgba(255, 255, 255, 0.4)")
            }
          >
            <span>← Last Session</span>
          </button>

          {/* Status indicator */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              fontSize: "11px",
              color: "rgba(255, 255, 255, 0.4)",
            }}
          >
            <span
              style={{
                width: "6px",
                height: "6px",
                borderRadius: "50%",
                background: isTyping ? "#f59e0b" : "#22c55e",
              }}
            />
            <span>{isTyping ? "Đang xử lý" : "Sẵn sàng"}</span>
          </div>
        </div>
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
