import { useState, useEffect } from "react";
import { 
  GitBranch, RefreshCw, ArrowDown, ArrowUp, Plus, Minus, RotateCcw, 
  Check, FolderOpen, History, Loader2, ChevronDown, ChevronRight
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface GitFile {
  path: string;
  status: string;
}

interface GitStatusData {
  branch: string;
  remote: string;
  staged: GitFile[];
  unstaged: GitFile[];
}

interface CommitInfo {
  hash: string;
  author: string;
  date: string;
  subject: string;
}

interface GitPanelProps {
  activeProject: {
    id: string;
    name: string;
    cwd?: string;
  } | null;
  triggerToast: (message: string, type: "success" | "error" | "info") => void;
}

export default function GitPanel({ activeProject, triggerToast }: GitPanelProps) {
  const [gitStatus, setGitStatus] = useState<GitStatusData | null>(null);
  const [commits, setCommits] = useState<CommitInfo[]>([]);
  const [commitMessage, setCommitMessage] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isActionLoading, setIsActionLoading] = useState(false);
  
  // Collapsible section states
  const [showStaged, setShowStaged] = useState(true);
  const [showUnstaged, setShowUnstaged] = useState(true);
  const [showHistory, setShowHistory] = useState(true);

  // Commit inspection states
  const [expandedCommits, setExpandedCommits] = useState<{[hash: string]: GitFile[]}>({});
  const [loadingCommits, setLoadingCommits] = useState<{[hash: string]: boolean}>({});

  const handleCommitDoubleClick = async (hash: string) => {
    if (!cwd) return;
    if (expandedCommits[hash]) {
      setExpandedCommits(prev => {
        const copy = { ...prev };
        delete copy[hash];
        return copy;
      });
      return;
    }

    setLoadingCommits(prev => ({ ...prev, [hash]: true }));
    try {
      const files: GitFile[] = await invoke("git_get_commit_files", { cwd, hash });
      setExpandedCommits(prev => ({ ...prev, [hash]: files }));
    } catch (e) {
      triggerToast(`Không thể tải chi tiết commit: ${e}`, "error");
    } finally {
      setLoadingCommits(prev => ({ ...prev, [hash]: false }));
    }
  };

  const cwd = activeProject?.cwd;

  const fetchGitData = async (silent = false) => {
    if (!cwd) return;
    if (!silent) setIsLoading(true);
    try {
      const status: GitStatusData = await invoke("git_get_status", { cwd });
      setGitStatus(status);
      
      const log: CommitInfo[] = await invoke("git_get_log", { cwd });
      setCommits(log);
    } catch (err: any) {
      console.error("Error fetching git data:", err);
      setGitStatus(null);
      setCommits([]);
    } finally {
      if (!silent) setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchGitData();
  }, [cwd]);

  const handleAction = async (actionName: string, promise: Promise<any>, successMsg: string) => {
    setIsActionLoading(true);
    try {
      await promise;
      triggerToast(successMsg, "success");
      await fetchGitData(true);
    } catch (err: any) {
      triggerToast(`${actionName} thất bại: ${err}`, "error");
    } finally {
      setIsActionLoading(false);
    }
  };

  const handleStageFile = (file: string) => {
    if (!cwd) return;
    handleAction("Stage file", invoke("git_stage_file", { cwd, file }), `Đã stage ${file}`);
  };

  const handleStageAll = () => {
    if (!cwd) return;
    handleAction("Stage all files", invoke("git_stage_all", { cwd }), "Đã stage tất cả thay đổi");
  };

  const handleUnstageFile = (file: string) => {
    if (!cwd) return;
    handleAction("Unstage file", invoke("git_unstage_file", { cwd, file }), `Đã unstage ${file}`);
  };

  const handleUnstageAll = () => {
    if (!cwd) return;
    handleAction("Unstage all files", invoke("git_unstage_all", { cwd }), "Đã unstage tất cả thay đổi");
  };

  const handleDiscardFile = (file: string, status: string) => {
    if (!cwd) return;
    const confirm = window.confirm(`Bạn có chắc chắn muốn hoàn tác tất cả thay đổi trong file ${file}? Thao tác này không thể khôi phục.`);
    if (confirm) {
      handleAction("Discard changes", invoke("git_discard_file", { cwd, file, status }), `Đã hủy các thay đổi ở ${file}`);
    }
  };

  const handleCommit = () => {
    if (!cwd) return;
    if (!commitMessage.trim()) {
      triggerToast("Vui lòng nhập nội dung commit message", "error");
      return;
    }
    handleAction(
      "Commit", 
      invoke("git_commit", { cwd, message: commitMessage }), 
      "Commit thành công!"
    );
    setCommitMessage("");
  };

  const handlePush = () => {
    if (!cwd) return;
    handleAction("Push", invoke("git_push", { cwd }), "Push thay đổi thành công!");
  };

  const handlePull = () => {
    if (!cwd) return;
    handleAction("Pull", invoke("git_pull", { cwd }), "Pull thay đổi thành công!");
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "modified": return "var(--git-modified, #f59e0b)";
      case "added": return "var(--git-added, #10b981)";
      case "deleted": return "var(--git-deleted, #ef4444)";
      case "untracked": return "var(--git-untracked, #10b981)";
      default: return "var(--text-muted)";
    }
  };

  const getStatusLetter = (status: string) => {
    switch (status) {
      case "modified": return "M";
      case "added": return "A";
      case "deleted": return "D";
      case "untracked": return "U";
      default: return "•";
    }
  };

  if (!activeProject) {
    return (
      <div className="git-panel-empty-state">
        <GitBranch size={40} className="empty-iconPulse" />
        <h3>Chưa Chọn Dự Án</h3>
        <p>Vui lòng mở một dự án hoạt động trong Workspace để hiển thị trạng thái Git.</p>
      </div>
    );
  }

  if (!cwd) {
    return (
      <div className="git-panel-empty-state">
        <FolderOpen size={40} className="empty-iconPulse" />
        <h3>Thư Mục Không Hợp Lệ</h3>
        <p>Dự án hiện tại chưa được định cấu hình đường dẫn thư mục làm việc (CWD).</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="git-panel-loading-state">
        <Loader2 size={24} className="spin-animation loader-accent" />
        <span>Đang quét kho lưu trữ Git...</span>
      </div>
    );
  }

  // Not a Git repo
  if (!gitStatus) {
    return (
      <div className="git-panel-empty-state">
        <div className="warning-badge">⚠️ Non-Git</div>
        <h3>Chưa Khởi Tạo Git</h3>
        <p>Thư mục này hiện tại chưa được khởi tạo dưới dạng một kho lưu trữ Git.</p>
        <button 
          className="btn-init-git"
          onClick={() => fetchGitData()}
        >
          Tải lại trạng thái
        </button>
      </div>
    );
  }

  return (
    <div className="premium-git-panel">
      <style>{`
        .premium-git-panel {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: var(--bg-secondary);
          color: var(--text-primary);
          font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
        }
        
        /* Header section styles */
        .git-header-card {
          padding: 12px 16px;
          background: var(--bg-secondary);
          border-bottom: 1px solid var(--border-primary);
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .git-header-info {
          display: flex;
          align-items: center;
          gap: 8px;
          min-width: 0;
        }
        .git-branch-badge {
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--text-primary);
          padding: 3px 8px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 600;
          display: inline-flex;
          align-items: center;
          gap: 4px;
        }
        .git-remote-txt {
          font-size: 10px;
          color: var(--text-muted);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          margin-top: 2px;
        }

        /* Toolbar styles */
        .git-header-toolbar {
          display: flex;
          justify-content: flex-end;
          gap: 4px;
        }
        .git-tool-btn {
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          padding: 5px 8px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 500;
          cursor: pointer;
          display: flex;
          align-items: center;
          gap: 4px;
          transition: all 0.15s ease;
        }
        .git-tool-btn:hover:not(:disabled) {
          background: var(--bg-primary);
          color: var(--text-primary);
        }
        .git-tool-btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        /* Commit Input Area */
        .git-commit-container {
          padding: 12px 16px;
          border-bottom: 1px solid var(--border-primary);
          background: var(--bg-secondary);
        }
        .git-textarea-wrapper {
          position: relative;
          border-radius: 6px;
          border: 1px solid var(--border-primary);
          background: var(--bg-tertiary);
          transition: all 0.15s ease;
          overflow: hidden;
        }
        .git-textarea-wrapper:focus-within {
          border-color: var(--color-accent, #6366f1);
        }
        .git-commit-textarea {
          width: 100%;
          height: 60px;
          background: transparent;
          border: none;
          color: var(--text-primary);
          padding: 8px;
          font-size: 12px;
          outline: none;
          resize: none;
          font-family: inherit;
        }
        .git-commit-shortcut {
          position: absolute;
          bottom: 4px;
          right: 6px;
          font-size: 9px;
          color: var(--text-muted);
        }
        .btn-git-commit-submit {
          width: 100%;
          height: 28px;
          margin-top: 8px;
          background: var(--color-accent, #6366f1);
          color: white;
          border: none;
          border-radius: 4px;
          font-size: 11.5px;
          font-weight: 600;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 4px;
          transition: all 0.15s ease;
        }
        .btn-git-commit-submit:hover:not(:disabled) {
          filter: brightness(1.08);
        }
        .btn-git-commit-submit:disabled {
          background: var(--bg-tertiary);
          color: var(--text-muted);
          border: 1px solid var(--border-primary);
          cursor: not-allowed;
        }

        /* Collapsible Section Header */
        .git-section-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 10px 16px;
          cursor: pointer;
          user-select: none;
        }
        .git-section-header:hover {
          background: var(--bg-tertiary);
        }
        .git-section-title {
          font-size: 10.5px;
          font-weight: 700;
          color: var(--text-secondary);
          letter-spacing: 0.05em;
          display: flex;
          align-items: center;
          gap: 6px;
        }
        .git-section-count {
          font-size: 9.5px;
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          padding: 0px 5px;
          border-radius: 8px;
          font-weight: 600;
        }
        .git-section-action-text {
          font-size: 10px;
          color: var(--color-accent, #6366f1);
          font-weight: 600;
          background: none;
          border: none;
          cursor: pointer;
        }
        .git-section-action-text:hover {
          text-decoration: underline;
        }

        /* File list row styles */
        .git-file-list {
          padding: 0 12px 10px 12px;
          display: flex;
          flex-direction: column;
          gap: 3px;
        }
        .git-file-card {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 6px 10px;
          border-radius: 4px;
          background: transparent;
          border: 1px solid transparent;
          transition: all 0.15s ease;
        }
        .git-file-card:hover {
          background: var(--bg-tertiary);
          border-color: var(--border-primary);
        }
        .file-info-col {
          display: flex;
          align-items: center;
          gap: 8px;
          min-width: 0;
        }
        .git-status-indicator {
          width: 14px;
          height: 14px;
          border-radius: 3px;
          font-size: 9px;
          font-weight: 700;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .git-file-name {
          font-size: 12px;
          color: var(--text-primary);
          font-weight: 500;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .git-file-path {
          font-size: 9.5px;
          color: var(--text-muted);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          max-width: 120px;
        }
        .git-row-actions {
          display: flex;
          gap: 2px;
          opacity: 0;
          transition: opacity 0.1s ease;
        }
        .git-file-card:hover .git-row-actions {
          opacity: 1;
        }
        .git-circle-action-btn {
          width: 20px;
          height: 20px;
          border-radius: 3px;
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
        }
        .git-circle-action-btn:hover {
          background: var(--bg-primary);
          color: var(--text-primary);
        }

        /* Commit Tree History Timeline */
        .git-timeline {
          margin-top: 4px;
          padding: 0 16px 16px 20px;
          display: flex;
          flex-direction: column;
          gap: 12px;
          position: relative;
        }
        .git-timeline-line {
          position: absolute;
          left: 25px;
          top: 6px;
          bottom: 16px;
          width: 1px;
          background: var(--border-primary);
        }
        .git-timeline-item {
          display: flex;
          gap: 10px;
          position: relative;
          z-index: 1;
        }
        .git-timeline-dot-wrapper {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: flex-start;
          padding-top: 4px;
        }
        .git-timeline-node {
          width: 10px;
          height: 10px;
          border-radius: 50%;
          background: var(--bg-secondary);
          border: 2px solid var(--border-primary);
          transition: all 0.15s ease;
        }
        .git-timeline-item:hover .git-timeline-node {
          border-color: var(--color-accent, #6366f1);
          background: var(--color-accent, #6366f1);
        }
        .git-timeline-body {
          flex: 1;
          min-width: 0;
          padding: 4px 8px;
          border-radius: 4px;
          border: 1px solid transparent;
          transition: all 0.15s ease;
        }
        .git-timeline-item:hover .git-timeline-body {
          background: var(--bg-tertiary);
          border-color: var(--border-primary);
        }
        .git-commit-meta {
          display: flex;
          align-items: center;
          gap: 6px;
          margin-bottom: 2px;
        }
        .git-commit-hash {
          font-family: var(--font-mono, monospace);
          font-size: 9.5px;
          font-weight: 600;
          background: var(--bg-tertiary);
          color: var(--text-secondary);
          padding: 0px 4px;
          border-radius: 3px;
          border: 1px solid var(--border-primary);
        }
        .git-commit-author {
          font-size: 10px;
          font-weight: 600;
          color: var(--text-secondary);
        }
        .git-commit-subject {
          font-size: 11px;
          color: var(--text-muted);
          line-height: 1.35;
          word-break: break-all;
        }
        .git-timeline-item:hover .git-commit-subject {
          color: var(--text-primary);
        }

        /* Empty states */
        .git-panel-empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          height: 100%;
          padding: 24px;
          text-align: center;
          color: var(--text-muted);
        }
        .git-panel-empty-state h3 {
          font-size: 13px;
          font-weight: 600;
          color: var(--text-primary);
          margin: 12px 0 6px 0;
        }
        .git-panel-empty-state p {
          font-size: 11px;
          color: var(--text-muted);
          line-height: 1.4;
          margin: 0;
        }
        .warning-badge {
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--color-danger);
          padding: 2px 6px;
          border-radius: 10px;
          font-size: 9.5px;
          font-weight: 700;
        }
        .btn-init-git {
          margin-top: 12px;
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          padding: 5px 12px;
          border-radius: 4px;
          font-size: 11px;
          cursor: pointer;
        }
        .btn-init-git:hover {
          color: var(--text-primary);
          background: var(--bg-primary);
        }

        /* Loading state */
        .git-panel-loading-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          height: 100%;
          color: var(--text-muted);
          gap: 8px;
        }
        .loader-accent {
          color: var(--color-accent, #6366f1);
        }
      `}</style>


      {/* Repository Header */}
      <div className="git-header-card">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", gap: "10px" }}>
          <div className="git-header-info">
            <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
              <div className="git-branch-badge">
                <GitBranch size={11} />
                <span>{gitStatus.branch}</span>
              </div>
              <span className="git-remote-txt">{gitStatus.remote}</span>
            </div>
          </div>
          
          <div className="git-header-toolbar">
            <button 
              className="git-tool-btn" 
              title="Pull changes" 
              onClick={handlePull}
              disabled={isActionLoading}
            >
              <ArrowDown size={12} />
              <span>Pull</span>
            </button>
            <button 
              className="git-tool-btn" 
              title="Push changes" 
              onClick={handlePush}
              disabled={isActionLoading}
            >
              <ArrowUp size={12} />
              <span>Push</span>
            </button>
            <button 
              className="git-tool-btn" 
              title="Refresh status" 
              onClick={() => fetchGitData()}
              disabled={isActionLoading}
              style={{ padding: "6px" }}
            >
              <RefreshCw size={12} className={isActionLoading ? "spin-animation" : ""} />
            </button>
          </div>
        </div>
      </div>

      {/* Main Body */}
      <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column" }}>
        
        {/* Commit Composer */}
        <div className="git-commit-container">
          <div className="git-textarea-wrapper">
            <textarea
              className="git-commit-textarea"
              placeholder="Nhập nội dung commit..."
              value={commitMessage}
              onChange={(e) => setCommitMessage(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  handleCommit();
                }
              }}
            />
            <span className="git-commit-shortcut">Ctrl+Enter</span>
          </div>
          <button 
            className="btn-git-commit-submit"
            onClick={handleCommit}
            disabled={isActionLoading || !commitMessage.trim()}
          >
            <Check size={14} />
            <span>Commit</span>
          </button>
        </div>

        {/* Staged Changes Section */}
        {gitStatus.staged.length > 0 && (
          <div>
            <div className="git-section-header" onClick={() => setShowStaged(!showStaged)}>
              <div className="git-section-title">
                {showStaged ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                <span>ĐÃ STAGE</span>
                <span className="git-section-count">{gitStatus.staged.length}</span>
              </div>
              {showStaged && (
                <button 
                  className="git-section-action-text"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleUnstageAll();
                  }}
                >
                  Unstage All
                </button>
              )}
            </div>

            {showStaged && (
              <div className="git-file-list">
                {gitStatus.staged.map((file) => (
                  <div key={`staged-${file.path}`} className="git-file-card">
                    <div className="file-info-col">
                      <div className="git-status-indicator" style={{
                        background: `${getStatusColor(file.status)}1e`,
                        border: `1px solid ${getStatusColor(file.status)}40`,
                        color: getStatusColor(file.status)
                      }}>
                        {getStatusLetter(file.status)}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span className="git-file-name" title={file.path}>
                          {file.path.split("/").pop()}
                        </span>
                        <span className="git-file-path" title={file.path} style={{ display: "block", color: "var(--text-muted)", fontSize: "9.5px", marginTop: "2px" }}>
                          {file.path}
                        </span>
                      </div>
                    </div>
                    
                    <div className="git-row-actions">
                      <button 
                        className="git-circle-action-btn"
                        title="Unstage changes"
                        onClick={() => handleUnstageFile(file.path)}
                      >
                        <Minus size={12} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Changes Section */}
        <div>
          <div className="git-section-header" onClick={() => setShowUnstaged(!showUnstaged)}>
            <div className="git-section-title">
              {showUnstaged ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              <span>THAY ĐỔI CHƯA COMMIT</span>
              <span className="git-section-count">{gitStatus.unstaged.length}</span>
            </div>
            {showUnstaged && gitStatus.unstaged.length > 0 && (
              <button 
                className="git-section-action-text"
                onClick={(e) => {
                  e.stopPropagation();
                  handleStageAll();
                }}
              >
                Stage All
              </button>
            )}
          </div>

          {showUnstaged && (
            <div className="git-file-list">
              {gitStatus.unstaged.length === 0 ? (
                <div style={{ fontSize: "11px", color: "#475569", padding: "8px 16px", fontStyle: "italic" }}>
                  Không có thay đổi nào chưa stage.
                </div>
              ) : (
                gitStatus.unstaged.map((file) => (
                  <div key={`unstaged-${file.path}`} className="git-file-card">
                    <div className="file-info-col">
                      <div className="git-status-indicator" style={{
                        background: `${getStatusColor(file.status)}1e`,
                        border: `1px solid ${getStatusColor(file.status)}40`,
                        color: getStatusColor(file.status)
                      }}>
                        {getStatusLetter(file.status)}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span className="git-file-name" title={file.path}>
                          {file.path.split("/").pop()}
                        </span>
                        <span className="git-file-path" title={file.path} style={{ display: "block", color: "var(--text-muted)", fontSize: "9.5px", marginTop: "2px" }}>
                          {file.path}
                        </span>
                      </div>
                    </div>
                    
                    <div className="git-row-actions">
                      <button 
                        className="git-circle-action-btn"
                        title="Hoàn tác thay đổi"
                        onClick={() => handleDiscardFile(file.path, file.status)}
                      >
                        <RotateCcw size={11} />
                      </button>
                      <button 
                        className="git-circle-action-btn"
                        title="Stage changes"
                        onClick={() => handleStageFile(file.path)}
                      >
                        <Plus size={11} />
                      </button>
                    </div>
                  </div>
                )))}
              </div>
            )}
          </div>

          {/* Git History Timeline */}
          <div>
            <div className="git-section-header" onClick={() => setShowHistory(!showHistory)}>
              <div className="git-section-title">
                {showHistory ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                <History size={12} style={{ color: "#6366f1" }} />
                <span>LỊCH SỬ COMMIT</span>
                <span className="git-section-count">{commits.length}</span>
              </div>
            </div>

            {showHistory && (
              <div className="git-timeline">
                <div className="git-timeline-line" />
                {commits.length === 0 ? (
                  <div style={{ fontSize: "11px", color: "#475569", fontStyle: "italic", paddingLeft: "12px" }}>
                    Chưa có commit cục bộ nào.
                  </div>
                ) : (
                  commits.map((c) => (
                    <div 
                      key={c.hash} 
                      className="git-timeline-item"
                      onDoubleClick={() => handleCommitDoubleClick(c.hash)}
                      style={{ cursor: "pointer" }}
                      title="Nhấp đúp để xem các file thay đổi"
                    >
                      <div className="git-timeline-dot-wrapper">
                        <div className="git-timeline-node" />
                      </div>
                      <div className="git-timeline-body">
                        <div className="git-commit-meta">
                          <span className="git-commit-hash">{c.hash}</span>
                          <span className="git-commit-author">{c.author}</span>
                        </div>
                        <div className="git-commit-subject">{c.subject}</div>
                        
                        {loadingCommits[c.hash] && (
                          <div style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "10px", color: "var(--text-muted)", marginTop: "4px" }}>
                            <Loader2 size={10} className="spin-animation" />
                            <span>Đang tải danh sách file...</span>
                          </div>
                        )}

                        {expandedCommits[c.hash] && (
                          <div style={{ 
                            marginTop: "6px", display: "flex", flexDirection: "column", gap: "4px", 
                            borderTop: "1px solid var(--border-primary)", paddingTop: "6px" 
                          }}>
                            {expandedCommits[c.hash].length === 0 ? (
                              <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>Không có file thay đổi.</span>
                            ) : (
                              expandedCommits[c.hash].map((file) => (
                                <div key={`${c.hash}-${file.path}`} style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "10px" }}>
                                  <span style={{
                                    fontSize: "8.5px", fontWeight: 700, padding: "1px 3px", borderRadius: "2px",
                                    textTransform: "uppercase",
                                    backgroundColor: file.status === "added" ? "rgba(16, 185, 129, 0.15)" : file.status === "deleted" ? "rgba(239, 68, 68, 0.15)" : "rgba(245, 158, 11, 0.15)",
                                    color: file.status === "added" ? "#10b981" : file.status === "deleted" ? "#ef4444" : "#f59e0b"
                                  }}>
                                    {file.status[0]}
                                  </span>
                                  <span style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={file.path}>
                                    {file.path}
                                  </span>
                                </div>
                              ))
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}
          </div>

        </div>
      </div>
    );
}
