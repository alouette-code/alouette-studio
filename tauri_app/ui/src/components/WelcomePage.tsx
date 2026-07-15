import { useEffect, useState, KeyboardEvent, useRef } from "react";
import {
  Plus,
  FolderOpen,
  FileText,
  Play,
  Square,
  ExternalLink,
  Terminal,
  Settings,
  LayoutGrid,
  ArrowRight,
  ArrowLeftRight,
  MonitorCog,
  Bolt,
  Package,
  Wrench,
  Database,
  Clock,
} from "lucide-react";
import { Project, ProcessState } from "../types";
import { listen } from "@tauri-apps/api/event";

import ReactMarkdown from "react-markdown";

interface WelcomePageProps {
  projects: Project[];
  projectStates: { [id: string]: ProcessState };
  setActiveProjectId: (id: string) => void;
  handleFileAction: (action: string, payload?: any) => void;
  handleStartProject: (id: string) => Promise<void>;
  handleStopProject: (id: string) => Promise<void>;
  handleImportMockConfig: () => Promise<void>;
  triggerToast: (msg: string, type: "success" | "error" | "info") => void;
}

interface Message {
  role: "user" | "assistant";
  content: string;
}

export default function WelcomePage({
  projects,
  projectStates,
  setActiveProjectId,
  handleFileAction,
  handleStartProject,
  handleStopProject,
  handleImportMockConfig,
  triggerToast,
}: WelcomePageProps) {
  const [recentFolders, setRecentFolders] = useState<string[]>([]);
  const [recentFiles, setRecentFiles] = useState<string[]>([]);
  const [chatInput, setChatInput] = useState("");

  // Local Chat states
  const [isChatting, setIsChatting] = useState(false);
  const [messages, setMessages] = useState<Message[]>([]);
  const [isTyping, setIsTyping] = useState(false);
  const [status, setStatus] = useState<"idle" | "starting" | "running">("idle");
  const [statusMessage, setStatusMessage] = useState("Sẵn sàng");

  const chatEndRef = useRef<HTMLDivElement>(null);
  const activeStreamContentRef = useRef<string>("");

  useEffect(() => {
    const loadRecents = () => {
      try {
        const folders = JSON.parse(
          localStorage.getItem("recent_folders") || "[]",
        );
        const files = JSON.parse(localStorage.getItem("recent_files") || "[]");
        setRecentFolders(folders.slice(0, 5));
        setRecentFiles(files.slice(0, 5));
      } catch (e) {
        console.error("Failed to parse recent items:", e);
      }
    };
    loadRecents();
    window.addEventListener("storage", loadRecents);
    return () => window.removeEventListener("storage", loadRecents);
  }, []);

  // Set up listeners for the streaming events
  useEffect(() => {
    let unlistenChunk: any;
    let unlistenStatus: any;
    let unlistenComplete: any;

    const setupListeners = async () => {
      unlistenChunk = await listen<string>("local-chat-chunk", (event) => {
        const chunk = event.payload;
        activeStreamContentRef.current += chunk;

        setMessages((prev) => {
          if (prev.length === 0) return prev;
          const last = prev[prev.length - 1];
          if (last.role === "assistant") {
            const next = [...prev];
            next[next.length - 1] = {
              ...last,
              content: activeStreamContentRef.current,
            };
            return next;
          }
          return prev;
        });
      });

      unlistenStatus = await listen<string>("local-chat-status", (event) => {
        const statusVal = event.payload as "starting" | "running";
        setStatus(statusVal);
        if (statusVal === "starting") {
          setStatusMessage("Đang khởi động local server...");
        } else {
          setStatusMessage("Mô hình local đang chạy");
        }
      });

      unlistenComplete = await listen<string>("local-chat-complete", () => {
        setIsTyping(false);
        activeStreamContentRef.current = "";
      });
    };

    setupListeners();

    return () => {
      if (unlistenChunk) unlistenChunk();
      if (unlistenStatus) unlistenStatus();
      if (unlistenComplete) unlistenComplete();
    };
  }, []);

  const getBaseName = (p: string) => {
    const normalized = p.replace(/\\/g, "/");
    const lastSlash = normalized.lastIndexOf("/");
    return lastSlash !== -1 ? normalized.substring(lastSlash + 1) : p;
  };

  const handleOpenFolder = (path: string) => {
    handleFileAction("open-folder-path", path);
  };

  const handleOpenFile = (path: string) => {
    handleFileAction("open-file-path", path);
  };

  const abortControllerRef = useRef<AbortController | null>(null);

  const handleSendChat = async () => {
    const text = chatInput.trim();
    if (!text || isTyping) return;

    setIsChatting(true);
    const userMsg: Message = { role: "user", content: text };
    const historyForBackend = [...messages, userMsg];

    setMessages((prev) => [...prev, userMsg]);
    setChatInput("");
    setIsTyping(true);

    activeStreamContentRef.current = "";
    setMessages((prev) => [...prev, { role: "assistant", content: "" }]);
    setStatus("running");
    setStatusMessage("Đang gọi API Ollama...");

    try {
      abortControllerRef.current = new AbortController();
      const response = await fetch("http://localhost:11434/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: "llama3", // Mô hình mặc định (có thể thay đổi)
          messages: historyForBackend,
          stream: true,
        }),
        signal: abortControllerRef.current.signal,
      });

      if (!response.ok) {
        throw new Error(`API returned ${response.status}`);
      }

      const reader = response.body?.getReader();
      const decoder = new TextDecoder("utf-8");

      if (reader) {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value, { stream: true });
          const lines = chunk.split("\n").filter((line) => line.trim() !== "");

          for (const line of lines) {
            try {
              const data = JSON.parse(line);
              if (data.message && data.message.content) {
                activeStreamContentRef.current += data.message.content;
                setMessages((prev) => {
                  const next = [...prev];
                  next[next.length - 1] = {
                    role: "assistant",
                    content: activeStreamContentRef.current,
                  };
                  return next;
                });
              }
            } catch (e) {
              // Ignore parse errors on partial chunks
            }
          }
        }
      }
      setIsTyping(false);
      setStatus("idle");
    } catch (err: any) {
      if (err.name === "AbortError") {
        console.log("Chat stopped");
      } else {
        console.error(err);
        setStatusMessage("Lỗi kết nối Ollama");
        setMessages((prev) => {
          const next = [...prev];
          if (next.length > 0 && next[next.length - 1].role === "assistant") {
            next[next.length - 1] = {
              role: "assistant",
              content: `❌ Lỗi: Không thể kết nối đến Ollama API. Hãy chắc chắn Ollama đang chạy tại http://localhost:11434 và bạn đã tải mô hình 'llama3' (hoặc sửa model trong source).`,
            };
          }
          return next;
        });
      }
      setIsTyping(false);
      setStatus("idle");
    }
  };

  const handleStopChat = async () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    setIsTyping(false);
    triggerToast("Đã dừng phản hồi", "info");
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleSendChat();
    }
  };

  return (
    <div className="welcome-container welcome-monochrome">
      <div className="welcome-content-wrapper">
        {/* HERO: Alouette Studio — luôn hiển thị */}
        <div className="welcome-hero">
          <h1>Alouette Studio</h1>
        </div>

        {/* AI RESPONSE AREA — chỉ hiện khi có chat, nằm TRÊN khung input */}
        {isChatting && (
          <div className="welcome-chat-response-area">
            <div className="response-messages">
              {messages.length === 0 ? (
                <div className="response-empty">
                  <p className="response-empty-text">
                    <span className="response-status-indicator">
                      <span className={`response-status-dot ${status}`}></span>
                      {statusMessage}
                    </span>
                  </p>
                </div>
              ) : (
                messages.map((msg, index) => (
                  <div key={index} className={`response-msg ${msg.role}`}>
                    <div className="response-msg-bubble">
                      {msg.role === "assistant" && msg.content === "" ? (
                        <div className="thinking-indicator">
                          <div className="thinking-dots">
                            <span className="dot dot-1"></span>
                            <span className="dot dot-2"></span>
                            <span className="dot dot-3"></span>
                          </div>
                          <span className="thinking-text">Đang suy nghĩ</span>
                        </div>
                      ) : (
                        <ReactMarkdown>{msg.content}</ReactMarkdown>
                      )}
                    </div>
                  </div>
                ))
              )}
              <div ref={chatEndRef} />
            </div>
          </div>
        )}

        {/* CHAT INPUT — luôn hiển thị */}
        <div className="welcome-chat-section">
          <div className="welcome-chat-wrapper-outer">
            <div className="welcome-chat-box">
              <input
                type="text"
                className="welcome-chat-input"
                placeholder="Nhập câu hỏi hoặc yêu cầu gửi tới trợ lý AI..."
                value={chatInput}
                onChange={(e) => setChatInput(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={isTyping}
              />
            </div>
            <button
              className={`welcome-chat-send-btn ${isTyping ? "is-stopping" : ""}`}
              onClick={isTyping ? handleStopChat : handleSendChat}
              title={isTyping ? "Dừng phản hồi" : "Gửi yêu cầu"}
              disabled={!isTyping && !chatInput.trim()}
            >
              {isTyping ? (
                <Square size={14} fill="currentColor" />
              ) : (
                <ArrowRight size={18} />
              )}
            </button>
          </div>
        </div>

        {/* QUICK ACTIONS ROW */}
        <div className="welcome-quick-actions flat-actions">
          <button
            className="qa-flat-btn"
            onClick={() => handleFileAction("new-project")}
          >
            <Plus size={14} />
            <span>Dự án mới</span>
          </button>
          <button
            className="qa-flat-btn"
            onClick={() => handleFileAction("open-folder")}
          >
            <FolderOpen size={14} />
            <span>Mở thư mục</span>
          </button>
          <button
            className="qa-flat-btn"
            onClick={() => handleFileAction("open-file")}
          >
            <FileText size={14} />
            <span>Mở tệp</span>
          </button>
          <button className="qa-flat-btn" onClick={handleImportMockConfig}>
            <Database size={14} />
            <span>Dự án Mẫu</span>
          </button>
        </div>

        {/* MAIN BODY GRID */}
        <div className="welcome-main-grid flat-grid">
          {/* LEFT: PROJECTS */}
          <div className="welcome-section welcome-projects-section flat-section">
            <div className="welcome-section-header">
              <div className="title-group">
                <LayoutGrid size={14} />
                <h2>Dự án hoạt động</h2>
              </div>
              <span className="badge-count">{projects.length}</span>
            </div>
            {projects.length === 0 ? (
              <div className="empty-projects-card">
                <Terminal size={24} className="empty-icon" />
                <p>Chưa có dự án nào được đăng ký</p>
                <button
                  className="btn-welcome-secondary"
                  onClick={handleImportMockConfig}
                >
                  Tải dự án mẫu demo
                </button>
              </div>
            ) : (
              <div className="projects-list-scroll">
                {projects.map((proj) => {
                  const state: ProcessState = projectStates[proj.id] || {
                    type: "Stopped",
                  };
                  const isRunning =
                    state.type === "Running" || state.type === "Setup";
                  return (
                    <div key={proj.id} className="project-row-card flat-card">
                      <div
                        className="project-row-main"
                        onClick={() => setActiveProjectId(proj.id)}
                      >
                        <div className="project-status-dot-container">
                          <span
                            className={`status-dot ${state.type.toLowerCase()}`}
                          ></span>
                        </div>
                        <div className="project-row-info">
                          <div className="project-row-title-row">
                            <span className="project-row-name">
                              {proj.name}
                            </span>
                            {proj.port && (
                              <span className="project-row-port">
                                Port {proj.port}
                              </span>
                            )}
                            {proj.toolchain && (
                              <span className="project-row-toolchain">
                                {proj.toolchain}
                              </span>
                            )}
                          </div>
                          <span className="project-row-cwd" title={proj.cwd}>
                            {proj.cwd || "Không có cwd"}
                          </span>
                        </div>
                      </div>
                      <div className="project-row-actions">
                        {isRunning ? (
                          <button
                            className="btn-row-action stop"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleStopProject(proj.id);
                            }}
                          >
                            <Square size={10} fill="currentColor" />
                            <span>Dừng</span>
                          </button>
                        ) : (
                          <button
                            className="btn-row-action start"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleStartProject(proj.id);
                            }}
                          >
                            <Play size={10} fill="currentColor" />
                            <span>Chạy</span>
                          </button>
                        )}
                        <button
                          className="btn-row-action open"
                          onClick={() => setActiveProjectId(proj.id)}
                        >
                          <ExternalLink size={10} />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* RIGHT: RECENTS & TOOLS */}
          <div className="welcome-section welcome-sidebar-section flat-section">
            <div className="welcome-subsection">
              <div className="welcome-section-header">
                <div className="title-group">
                  <Clock size={14} />
                  <h2>Mục gần đây</h2>
                </div>
              </div>
              <div className="recents-container">
                <div className="recent-sub-section">
                  {recentFolders.length > 0 && (
                    <>
                      <h4>Thư mục</h4>
                      {recentFolders.map((folder, index) => (
                        <button
                          key={`folder-${index}`}
                          className="recent-item"
                          onClick={() => handleOpenFolder(folder)}
                        >
                          <span className="recent-item-name">
                            {getBaseName(folder)}
                          </span>
                        </button>
                      ))}
                    </>
                  )}
                  {recentFiles.length > 0 && (
                    <>
                      <h4 style={{ marginTop: "8px" }}>Tệp tin</h4>
                      {recentFiles.map((file, index) => (
                        <button
                          key={`file-${index}`}
                          className="recent-item"
                          onClick={() => handleOpenFile(file)}
                        >
                          <span className="recent-item-name">
                            {getBaseName(file)}
                          </span>
                        </button>
                      ))}
                    </>
                  )}
                  {recentFolders.length === 0 && recentFiles.length === 0 && (
                    <span className="recents-empty">
                      Không có lịch sử mở gần đây
                    </span>
                  )}
                </div>
              </div>
            </div>
            <div className="welcome-subsection" style={{ marginTop: "16px" }}>
              <div className="welcome-section-header">
                <div className="title-group">
                  <Bolt size={14} />
                  <h2>Công cụ khác</h2>
                </div>
              </div>
              <div className="tools-buttons-grid flat-grid">
                <button
                  className="tool-btn-welcome flat-tool-btn"
                  onClick={async () => {
                    try {
                      const { invoke } = await import("@tauri-apps/api/core");
                      await invoke("open_admin_window");
                    } catch (e) {
                      triggerToast("Lỗi mở Bảng quản trị", "error");
                    }
                  }}
                >
                  <Settings size={12} />
                  <span>Admin</span>
                </button>
                <button
                  className="tool-btn-welcome flat-tool-btn"
                  onClick={async () => {
                    try {
                      const { invoke } = await import("@tauri-apps/api/core");
                      await invoke("open_ping_window");
                    } catch (e) {
                      triggerToast("Lỗi mở PingZero", "error");
                    }
                  }}
                >
                  <ArrowLeftRight size={12} />
                  <span>PingZero API</span>
                </button>
                <button
                  className="tool-btn-welcome flat-tool-btn"
                  onClick={async () => {
                    try {
                      const { invoke } = await import("@tauri-apps/api/core");
                      await invoke("open_vm_window");
                    } catch (e) {
                      triggerToast("Lỗi mở Alouette VMM", "error");
                    }
                  }}
                >
                  <MonitorCog size={12} />
                  <span>Alouette VMM</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
