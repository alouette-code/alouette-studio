import { useEffect, useState, KeyboardEvent } from "react";
import {
  Plus,
  FolderOpen,
  FileText,
  Play,
  Square,
  ExternalLink,
  Database,
  Server,
  Activity,
  Clock,
  Terminal,
  Settings,
  LayoutGrid,
  ArrowRight
} from "lucide-react";
import { Project, ProcessState } from "../types";

interface WelcomePageProps {
  projects: Project[];
  projectStates: { [id: string]: ProcessState };
  setActiveProjectId: (id: string) => void;
  handleFileAction: (action: string, payload?: any) => void;
  handleStartProject: (id: string) => Promise<void>;
  handleStopProject: (id: string) => Promise<void>;
  handleImportMockConfig: () => Promise<void>;
  triggerToast: (msg: string, type: "success" | "error" | "info") => void;
  onSubmitPrompt: (prompt: string) => void;
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
  onSubmitPrompt,
}: WelcomePageProps) {
  const [recentFolders, setRecentFolders] = useState<string[]>([]);
  const [recentFiles, setRecentFiles] = useState<string[]>([]);
  const [chatInput, setChatInput] = useState("");

  useEffect(() => {
    const loadRecents = () => {
      try {
        const folders = JSON.parse(localStorage.getItem("recent_folders") || "[]");
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

  const handleSendChat = () => {
    if (!chatInput.trim()) return;
    onSubmitPrompt(chatInput.trim());
    setChatInput("");
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleSendChat();
    }
  };

  return (
    <div className="welcome-container welcome-monochrome">
      <div className="welcome-content-wrapper">
        
        {/* HERO SECTION */}
        <div className="welcome-hero">
          <h1>Alouette Studio</h1>
        </div>

        {/* AI CHAT INPUT BOX (Rounded and centered) */}
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
              />
            </div>
            <button className="welcome-chat-send-btn" onClick={handleSendChat} title="Gửi yêu cầu">
              <ArrowRight size={18} />
            </button>
          </div>
        </div>

        {/* QUICK ACTIONS ROW */}
        <div className="welcome-quick-actions flat-actions">
          <button className="qa-flat-btn" onClick={() => handleFileAction("new-project")}>
            <Plus size={14} />
            <span>Dự án mới</span>
          </button>
          <button className="qa-flat-btn" onClick={() => handleFileAction("open-folder")}>
            <FolderOpen size={14} />
            <span>Mở thư mục</span>
          </button>
          <button className="qa-flat-btn" onClick={() => handleFileAction("open-file")}>
            <FileText size={14} />
            <span>Mở tệp</span>
          </button>
          <button className="qa-flat-btn" onClick={handleImportMockConfig}>
            <Database size={14} />
            <span>Dự án Mẫu</span>
          </button>
        </div>

        {/* MAIN BODY GRID (Projects list + Recents) */}
        <div className="welcome-main-grid flat-grid">
          
          {/* LEFT: PROJECTS LIST */}
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
                <button className="btn-welcome-secondary" onClick={handleImportMockConfig}>
                  Tải dự án mẫu demo
                </button>
              </div>
            ) : (
              <div className="projects-list-scroll">
                {projects.map((proj) => {
                  const state: ProcessState = projectStates[proj.id] || { type: "Stopped" };
                  const isRunning = state.type === "Running" || state.type === "Setup";
                  
                  return (
                    <div key={proj.id} className="project-row-card flat-card">
                      <div className="project-row-main" onClick={() => setActiveProjectId(proj.id)}>
                        <div className="project-status-dot-container">
                          <span className={`status-dot ${state.type.toLowerCase()}`}></span>
                        </div>
                        <div className="project-row-info">
                          <div className="project-row-title-row">
                            <span className="project-row-name">{proj.name}</span>
                            {proj.port && <span className="project-row-port">Port {proj.port}</span>}
                            {proj.toolchain && <span className="project-row-toolchain">{proj.toolchain}</span>}
                          </div>
                          <span className="project-row-cwd" title={proj.cwd}>{proj.cwd || "Không có cwd"}</span>
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
            
            {/* RECENT ITEMS */}
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
                        <button key={`folder-${index}`} className="recent-item" onClick={() => handleOpenFolder(folder)}>
                          <span className="recent-item-name">{getBaseName(folder)}</span>
                        </button>
                      ))}
                    </>
                  )}
                  
                  {recentFiles.length > 0 && (
                    <>
                      <h4 style={{ marginTop: "8px" }}>Tệp tin</h4>
                      {recentFiles.map((file, index) => (
                        <button key={`file-${index}`} className="recent-item" onClick={() => handleOpenFile(file)}>
                          <span className="recent-item-name">{getBaseName(file)}</span>
                        </button>
                      ))}
                    </>
                  )}

                  {recentFolders.length === 0 && recentFiles.length === 0 && (
                    <span className="recents-empty">Không có lịch sử mở gần đây</span>
                  )}
                </div>
              </div>
            </div>

            {/* QUICK TOOLS PANEL */}
            <div className="welcome-subsection" style={{ marginTop: "16px" }}>
              <div className="welcome-section-header">
                <div className="title-group">
                  <Server size={14} />
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
                      triggerToast("Lỗi mở Postman", "error");
                    }
                  }}
                >
                  <Activity size={12} />
                  <span>Postman API</span>
                </button>
              </div>
            </div>

          </div>

        </div>

      </div>
    </div>
  );
}
