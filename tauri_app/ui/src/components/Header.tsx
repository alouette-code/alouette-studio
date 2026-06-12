import brandIcon from "./logo_alouette.png";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import {
  FileText,
  Settings,
  Search,
  Sun,
  Moon,
  Minus,
  Square,
  X,
  Play,
  Database,
  Sparkles,
  ExternalLink,
} from "lucide-react";

// Search Engine
import { isAgentHistorySearch } from "../lib/search";

interface HeaderProps {
  theme: "dark" | "light";
  setTheme: (t: "dark" | "light") => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  fileMenuOpen: boolean;
  setFileMenuOpen: (o: boolean) => void;
  settingMenuOpen: boolean;
  setSettingMenuOpen: (o: boolean) => void;
  handleFileAction: (action: string, payload?: any) => void;
  activeProject?: any;
  activeState?: any;
  handleStart?: () => void;
  handleStop?: () => void;
  triggerConfirm: (message: string, onConfirm: () => void) => void;
  triggerToast: (message: string, type: "success" | "error" | "info") => void;
  onOpenResources: () => void;
  onToggleTunnel?: () => void;
  agentHistoryList?: any[];
  onLoadAgentSession?: (sessionId: string) => void;
}

function CloudflareIcon({
  size = 16,
  active = false,
}: {
  size?: number;
  active?: boolean;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={active ? "#F38020" : "currentColor"}
      style={{
        opacity: active ? 1 : 0.55,
        color: active ? "#F38020" : "inherit",
        transition: "all 0.2s ease",
      }}
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M22.025 14.156c-.035-.3-.081-.606-.151-.9-.372-1.63-1.488-2.975-3.003-3.615a5.556 5.556 0 0 0 .151-1.28c0-2.825-2.22-5.115-4.965-5.115-.99 0-1.921.3-2.715.82a6.38 6.38 0 0 0-6.195-2.05A6.091 6.091 0 0 0 1.005 8.163a6.837 6.837 0 0 0-.93 3.385c0 3.515 2.76 6.376 6.165 6.376h12.39c3.003-.001 5.395-2.316 5.395-5.127 0-.226-.012-.446-.035-.66l-.01-.081z" />
    </svg>
  );
}

export default function Header({
  theme,
  setTheme,
  searchQuery,
  setSearchQuery,
  fileMenuOpen,
  setFileMenuOpen,
  settingMenuOpen,
  setSettingMenuOpen,
  handleFileAction,
  activeProject,
  activeState,
  handleStart,
  handleStop,
  triggerToast,
  onOpenResources,
  onToggleTunnel,
  agentHistoryList,
  onLoadAgentSession,
}: HeaderProps) {
  const appWindow = getCurrentWindow();

  const handleMinimize = async () => {
    try {
      await appWindow.minimize();
    } catch (e) {
      console.error("Minimize error:", e);
    }
  };

  const handleMaximize = async () => {
    try {
      await appWindow.toggleMaximize();
    } catch (e) {
      console.error("Maximize error:", e);
    }
  };

  const handleClose = async () => {
    try {
      await invoke("hide_or_close_window");
    } catch (e) {
      console.error("Close error:", e);
      try {
        await appWindow.close();
      } catch (err) {
        console.error("Fallback close error:", err);
      }
    }
  };

  return (
    <header className="global-header" data-tauri-drag-region>
      <div className="header-left">
        <div className="brand" data-tauri-drag-region>
          <img src={brandIcon} className="brand-icon" alt="App Icon" />
        </div>

        <div className="top-red-boxes">
          <div className="dropdown-container">
            <button
              className={`btn-top-box ${fileMenuOpen ? "active" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                setFileMenuOpen(!fileMenuOpen);
                setSettingMenuOpen(false);
              }}
            >
              <FileText size={13} />
              <span>File</span>
            </button>
            {fileMenuOpen && (() => {
              const recentFiles: string[] = JSON.parse(localStorage.getItem("recent_files") || "[]");
              const recentFolders: string[] = JSON.parse(localStorage.getItem("recent_folders") || "[]");
              const autoSaveEnabled = localStorage.getItem("auto_save_enabled") === "true";
              const getBaseName = (p: string) => {
                const normalized = p.replace(/\\/g, "/");
                const lastSlash = normalized.lastIndexOf("/");
                return lastSlash !== -1 ? normalized.substring(lastSlash + 1) : p;
              };

              return (
                <div className="dropdown-menu" style={{ minWidth: "240px", padding: "4px 0" }}>
                  <style>{`
                    .dropdown-item {
                      display: flex;
                      justify-content: space-between;
                      align-items: center;
                      padding: 6px 12px;
                      width: 100%;
                      background: none;
                      border: none;
                      color: var(--text-primary);
                      font-size: 11.5px;
                      cursor: pointer;
                      text-align: left;
                      position: relative;
                      transition: background-color var(--transition-fast);
                    }
                    .dropdown-item:hover {
                      background-color: var(--bg-tertiary);
                    }
                    .dropdown-item-label {
                      display: flex;
                      align-items: center;
                      gap: 8px;
                    }
                    .dropdown-item-shortcut {
                      font-size: 10px;
                      color: var(--text-muted);
                      opacity: 0.7;
                      margin-left: 20px;
                      white-space: nowrap;
                    }
                    .dropdown-item-arrow {
                      font-size: 9px;
                      color: var(--text-muted);
                      margin-left: auto;
                    }
                    .has-submenu:hover {
                      background-color: var(--bg-tertiary);
                    }
                    .submenu {
                      display: none;
                      position: absolute;
                      top: -4px;
                      left: 100%;
                      background-color: var(--bg-secondary);
                      border: 1px solid var(--border-primary);
                      box-shadow: 4px 4px 12px rgba(0, 0, 0, 0.4);
                      padding: 4px 0;
                      min-width: 200px;
                      border-radius: 4px;
                      z-index: 120;
                    }
                    .has-submenu:hover > .submenu {
                      display: block;
                    }
                    .dropdown-item-check {
                      display: inline-block;
                      width: 12px;
                      font-size: 12px;
                      font-weight: bold;
                      color: var(--accent-purple, #a78bfa);
                      margin-right: 6px;
                      text-align: center;
                    }
                  `}</style>
                  
                  {/* Section 1 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("new-text-file"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      New Text File
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+N</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("new-file"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      New File...
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+Alt+Super+N</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("new-window"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      New Window
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+Shift+N</span>
                  </button>
                  <div className="dropdown-item has-submenu">
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      New Window with Profile
                    </span>
                    <span className="dropdown-item-arrow">▶</span>
                    <div className="submenu">
                      <button className="dropdown-item" onClick={() => { handleFileAction("new-window-profile", "Default"); setFileMenuOpen(false); }}>
                        Default
                      </button>
                      <button className="dropdown-item" onClick={() => { handleFileAction("new-window-profile", "Development"); setFileMenuOpen(false); }}>
                        Development
                      </button>
                      <button className="dropdown-item" onClick={() => { handleFileAction("new-window-profile", "Minimal"); setFileMenuOpen(false); }}>
                        Minimal
                      </button>
                    </div>
                  </div>
                  <button className="dropdown-item" onClick={() => { handleFileAction("new-project"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      New Project...
                    </span>
                  </button>

                  <div className="dropdown-divider"></div>

                  {/* Section 2 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("open-file"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Open File...
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+O</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("open-folder"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Open Folder...
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+K Ctrl+O</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("open-workspace"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Open Workspace from File...
                    </span>
                  </button>
                  <div className="dropdown-item has-submenu">
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Open Recent
                    </span>
                    <span className="dropdown-item-arrow">▶</span>
                    <div className="submenu">
                      {recentFolders.length === 0 && recentFiles.length === 0 ? (
                        <div className="dropdown-item disabled" style={{ opacity: 0.5, cursor: "default" }}>
                          No Recent Items
                        </div>
                      ) : (
                        <>
                          {recentFolders.map((folder, idx) => (
                            <button
                              key={`f-${idx}`}
                              className="dropdown-item"
                              onClick={() => { handleFileAction("open-folder-path", folder); setFileMenuOpen(false); }}
                              title={folder}
                            >
                              📁 {getBaseName(folder)}
                            </button>
                          ))}
                          {recentFiles.map((file, idx) => (
                            <button
                              key={`fl-${idx}`}
                              className="dropdown-item"
                              onClick={() => { handleFileAction("open-file-path", file); setFileMenuOpen(false); }}
                              title={file}
                            >
                              📄 {getBaseName(file)}
                            </button>
                          ))}
                        </>
                      )}
                    </div>
                  </div>

                  <div className="dropdown-divider"></div>

                  {/* Section 3 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("add-folder"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Add Folder to Workspace...
                    </span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("save-workspace"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Save Workspace As...
                    </span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("duplicate-workspace"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Duplicate Workspace
                    </span>
                  </button>

                  <div className="dropdown-divider"></div>

                  {/* Section 4 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("save"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Save
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+S</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("save-as"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Save As...
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+Shift+S</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("save-all"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Save All
                    </span>
                  </button>

                  <div className="dropdown-divider"></div>

                  {/* Section 5 */}
                  <div className="dropdown-item has-submenu">
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Share
                    </span>
                    <span className="dropdown-item-arrow">▶</span>
                    <div className="submenu">
                      <button className="dropdown-item" onClick={() => {
                        if (localStorage.getItem("openFilePath")) {
                          navigator.clipboard.writeText(localStorage.getItem("openFilePath") || "");
                          triggerToast("Copied file path to clipboard", "success");
                        } else {
                          triggerToast("No active file open", "info");
                        }
                        setFileMenuOpen(false);
                      }}>
                        Copy File Path
                      </button>
                      <button className="dropdown-item" onClick={() => { triggerToast("Sharing features are coming soon!", "info"); setFileMenuOpen(false); }}>
                        Share via Email
                      </button>
                      <button className="dropdown-item" onClick={() => { triggerToast("Gist export features are coming soon!", "info"); setFileMenuOpen(false); }}>
                        Export to Gist
                      </button>
                    </div>
                  </div>

                  <div className="dropdown-divider"></div>

                  {/* Section 6 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("toggle-auto-save"); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check">{autoSaveEnabled ? "✓" : ""}</span>
                      Auto Save
                    </span>
                  </button>
                  <div className="dropdown-item has-submenu">
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Preferences
                    </span>
                    <span className="dropdown-item-arrow">▶</span>
                    <div className="submenu">
                      <button className="dropdown-item" onClick={() => { handleFileAction("toggle-theme"); setFileMenuOpen(false); setTheme(theme === "dark" ? "light" : "dark"); }}>
                        Toggle Theme ({theme === "dark" ? "Light" : "Dark"})
                      </button>
                      <button className="dropdown-item" onClick={() => { triggerToast("Config cap: 2000 lines", "info"); setFileMenuOpen(false); }}>
                        Log Buffer Settings
                      </button>
                      <button className="dropdown-item" onClick={() => { localStorage.clear(); triggerToast("App cache cleared!", "success"); setFileMenuOpen(false); }}>
                        Clear App Cache
                      </button>
                    </div>
                  </div>

                  <div className="dropdown-divider"></div>

                  {/* Section 7 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("revert"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Revert File
                    </span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("close-editor"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Close Editor
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+W</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("close-folder"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Close Folder
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+K F</span>
                  </button>
                  <button className="dropdown-item" onClick={() => { handleFileAction("close-window"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Close Window
                    </span>
                    <span className="dropdown-item-shortcut">Alt+F4</span>
                  </button>

                  <div className="dropdown-divider"></div>

                  {/* Section 8 */}
                  <button className="dropdown-item" onClick={() => { handleFileAction("exit"); setFileMenuOpen(false); }}>
                    <span className="dropdown-item-label">
                      <span className="dropdown-item-check"></span>
                      Exit
                    </span>
                    <span className="dropdown-item-shortcut">Ctrl+Q</span>
                  </button>
                </div>
              );
            })()}
          </div>

          <div className="dropdown-container">
            <button
              className={`btn-top-box ${settingMenuOpen ? "active" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                setSettingMenuOpen(!settingMenuOpen);
                setFileMenuOpen(false);
              }}
            >
              <Settings size={13} />
              <span>Setting</span>
            </button>
            {settingMenuOpen && (
              <div className="dropdown-menu">
                <div className="dropdown-header">Buffer Settings</div>
                <button
                  className="dropdown-item"
                  onClick={() => {
                    triggerToast("Log buffer capped at 2000 lines.", "info");
                    setSettingMenuOpen(false);
                  }}
                >
                  Capped 2000 lines
                </button>
                <div className="dropdown-divider"></div>
                <div className="dropdown-header">System Style</div>
                <button
                  className="dropdown-item"
                  onClick={() => {
                    setTheme(theme === "dark" ? "light" : "dark");
                    setSettingMenuOpen(false);
                  }}
                >
                  Toggle Theme ({theme.toUpperCase()})
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Center Search (Big box in the middle) */}
      <div className="header-center">
        <div className="search-bar-wrapper">
          <Search size={13} className="search-icon" />
          <input
            type="text"
            className="search-input"
            placeholder="Search processes, logs, commands..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          {searchQuery && (
            <button
              className="search-clear-btn"
              onClick={() => setSearchQuery("")}
            >
              ✕
            </button>
          )}

          {/* Floating Agent History Dropdown (sử dụng search engine) */}
          {isAgentHistorySearch(searchQuery) && agentHistoryList && (
            <div className="agent-search-dropdown">
              <div className="agent-search-dropdown-header">
                <span className="agent-search-dropdown-header-left">
                  <Sparkles
                    size={12}
                    style={{ color: "var(--accent-purple, #a78bfa)" }}
                  />
                  LỊCH SỬ AGENT ({agentHistoryList.length})
                </span>
                <span className="agent-search-dropdown-header-right">
                  Mở tab <ExternalLink size={10} />
                </span>
              </div>
              <div className="agent-search-dropdown-list">
                {agentHistoryList.length === 0 ? (
                  <div className="agent-search-dropdown-empty">
                    <Search size={20} style={{ opacity: 0.4 }} />
                    <span>Không tìm thấy lịch sử phù hợp</span>
                    <span style={{ fontSize: "10px", opacity: 0.6 }}>
                      Thử gõ từ khóa khác để tìm kiếm mờ (fuzzy)
                    </span>
                  </div>
                ) : (
                  agentHistoryList.map((item, index) => (
                    <button
                      key={item.session_id}
                      className="agent-search-dropdown-item"
                      onClick={() =>
                        onLoadAgentSession &&
                        onLoadAgentSession(item.session_id)
                      }
                    >
                      <div className="agent-search-dropdown-item-top">
                        <span className="agent-search-dropdown-item-index">
                          #{index + 1}
                        </span>
                        <span
                          className="agent-search-dropdown-item-title"
                          title={item.title}
                        >
                          {item.title}
                        </span>
                        <span
                          className={`agent-search-dropdown-badge mode-${item.mode}`}
                        >
                          {item.mode === "autonomous"
                            ? "Tự động"
                            : "Tiêu chuẩn"}
                        </span>
                      </div>
                      <div className="agent-search-dropdown-item-meta">
                        <span className="agent-search-dropdown-badge">
                          {item.model}
                        </span>
                        <span className="agent-search-dropdown-date">
                          {new Date(item.created_at * 1000).toLocaleString(
                            "vi-VN",
                            {
                              hour: "2-digit",
                              minute: "2-digit",
                              day: "2-digit",
                              month: "2-digit",
                            },
                          )}
                        </span>
                        <span className="agent-search-dropdown-open-icon">
                          <ExternalLink size={11} />
                        </span>
                      </div>
                    </button>
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="header-right">
        <button
          className="btn-header-resources"
          onClick={() => {
            onOpenResources();
          }}
          title="Tài nguyên"
        >
          <Database size={13} />
        </button>

        {activeProject && (
          <div className="header-process-controls">
            <button
              style={{
                background: "transparent",
                border: "none",
                padding: "0 4px",
                marginRight: "4px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                cursor: "pointer",
                color: "inherit",
              }}
              onClick={onToggleTunnel}
              title={
                activeProject.enable_tunnel
                  ? "Cloudflare Tunnel Enabled (Click to Disable)"
                  : "Cloudflare Tunnel Disabled (Click to Enable)"
              }
            >
              <CloudflareIcon
                active={!!activeProject.enable_tunnel}
                size={15}
              />
            </button>
            <span className="header-process-name" title={activeProject.name}>
              {activeProject.name}
            </span>
            <div className="header-meta-capsule">
              <span className="label">PID</span>
              <span className="value mono">
                {activeState?.type === "Running"
                  ? typeof activeState.data === "object"
                    ? activeState.data?.pid
                    : activeState.data
                  : "N/A"}
              </span>
            </div>
            {activeProject.port && (
              <div className="header-meta-capsule">
                <span className="label">PORT</span>
                <span className="value mono text-success">
                  {activeProject.port}
                </span>
              </div>
            )}
            {activeState?.type === "Running" ||
            activeState?.type === "Setup" ? (
              <button
                className="btn-header-stop"
                onClick={handleStop}
                title="Stop process"
              >
                <Square size={10} fill="currentColor" />
                <span>Stop</span>
              </button>
            ) : (
              <button
                className="btn-header-start"
                onClick={handleStart}
                title="Start process"
              >
                <Play size={10} fill="currentColor" />
                <span>Start</span>
              </button>
            )}
          </div>
        )}

        <button
          className="btn-theme-toggle"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          title="Toggle theme"
        >
          {theme === "dark" ? <Sun size={13} /> : <Moon size={13} />}
        </button>

        {/* Custom Window Action Controls */}
        <div className="window-controls-container">
          <button
            className="window-control-btn minimize"
            onClick={handleMinimize}
            title="Minimize"
          >
            <Minus size={13} />
          </button>
          <button
            className="window-control-btn maximize"
            onClick={handleMaximize}
            title="Maximize"
          >
            <Square size={10} />
          </button>
          <button
            className="window-control-btn close"
            onClick={handleClose}
            title="Close"
          >
            <X size={13} />
          </button>
        </div>
      </div>
    </header>
  );
}
