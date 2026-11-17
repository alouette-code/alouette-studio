import { useState, useEffect } from "react";
import { 
  GitBranch, RefreshCw, ArrowDown, ArrowUp, Plus, Minus, RotateCcw, 
  Check, FolderOpen, History, Loader2, ChevronDown, ChevronRight,
  Cloud, CloudOff, Edit3, X, FileCode, Upload
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
  full_hash: string;
  author: string;
  date: string;
  subject: string;
  is_pushed: boolean;
}

interface CommitFileStat {
  path: string;
  status: string;
  insertions: number;
  deletions: number;
}

interface CommitDetails {
  hash: string;
  total_files: number;
  added_files: number;
  modified_files: number;
  deleted_files: number;
  total_insertions: number;
  total_deletions: number;
  files: CommitFileStat[];
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
  const [expandedCommits, setExpandedCommits] = useState<{[hash: string]: CommitDetails}>({});
  const [loadingCommits, setLoadingCommits] = useState<{[hash: string]: boolean}>({});

  // Dynamic commit editing & selective push states
  const [editingCommitHash, setEditingCommitHash] = useState<string | null>(null);
  const [editingCommitMsg, setEditingCommitMsg] = useState("");
  const [diffModal, setDiffModal] = useState<{
    hash: string;
    file: string;
    diff: string;
    loading: boolean;
  } | null>(null);

  const cwd = activeProject?.cwd;

  const handleToggleCommit = async (hash: string) => {
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
      const details: CommitDetails = await invoke("git_get_commit_details", { cwd, hash });
      setExpandedCommits(prev => ({ ...prev, [hash]: details }));
    } catch (e) {
      triggerToast(`Không thể tải chi tiết commit: ${e}`, "error");
    } finally {
      setLoadingCommits(prev => ({ ...prev, [hash]: false }));
    }
  };

  const handleViewFileDiff = async (hash: string, file: string) => {
    if (!cwd) return;
    setDiffModal({ hash, file, diff: "", loading: true });
    try {
      const diff: string = await invoke("git_get_commit_file_diff", { cwd, hash, file });
      setDiffModal({ hash, file, diff: diff || "(Không có thay đổi nội dung văn bản / file nhị phân)", loading: false });
    } catch (e) {
      triggerToast(`Không thể tải diff file: ${e}`, "error");
      setDiffModal(null);
    }
  };

  const handlePushSingleCommit = async (commit: CommitInfo) => {
    if (!cwd) return;
    const ok = window.confirm(
      `Bạn muốn push riêng commit [${commit.hash}] "${commit.subject}" lên remote?\n\nNếu commit này nằm giữa các commit chưa push khác, hệ thống sẽ tự động sắp xếp lại để chỉ đẩy mình commit này lên trước.`
    );
    if (!ok) return;

    setIsActionLoading(true);
    try {
      const msg: string = await invoke("git_push_single_commit", { cwd, hash: commit.full_hash || commit.hash });
      triggerToast(msg, "success");
      setExpandedCommits({});
      await fetchGitData(true);
    } catch (err: any) {
      triggerToast(`Push riêng commit thất bại: ${err}`, "error");
    } finally {
      setIsActionLoading(false);
    }
  };

  const handleSaveReword = async (commit: CommitInfo) => {
    if (!cwd) return;
    if (!editingCommitMsg.trim()) {
      triggerToast("Nội dung commit không được để trống", "error");
      return;
    }
    setIsActionLoading(true);
    try {
      await invoke("git_reword_commit", {
        cwd,
        hash: commit.full_hash || commit.hash,
        newMessage: editingCommitMsg.trim(),
      });
      triggerToast(`Đã cập nhật message commit ${commit.hash}`, "success");
      setEditingCommitHash(null);
      setEditingCommitMsg("");
      await fetchGitData(true);
    } catch (err: any) {
      triggerToast(`Sửa commit thất bại: ${err}`, "error");
    } finally {
      setIsActionLoading(false);
    }
  };

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

  const renderContent = () => {
    if (!activeProject) {
      return (
        <div className="git-panel-empty-state">
          <GitBranch size={32} className="empty-icon-monochromatic" />
          <h3>Chưa Chọn Dự Án</h3>
          <p>
            Vui lòng mở một dự án đang hoạt động trong Workspace để hiển thị trạng thái Git.
          </p>
        </div>
      );
    }

    if (!cwd) {
      return (
        <div className="git-panel-empty-state">
          <FolderOpen size={32} className="empty-icon-monochromatic" />
          <h3>Thư Mục Không Hợp Lệ</h3>
          <p>
            Dự án hiện tại chưa được định cấu hình đường dẫn thư mục làm việc (CWD).
          </p>
        </div>
      );
    }

    if (isLoading) {
      return (
        <div className="git-panel-loading-state">
          <Loader2 size={16} className="spin-animation loader-monochromatic" />
          <span className="loading-text">Đang quét kho lưu trữ Git...</span>
        </div>
      );
    }

    // Not a Git repo
    if (!gitStatus) {
      return (
        <div className="git-panel-empty-state">
          <GitBranch size={32} className="empty-icon-monochromatic" />
          <div className="warning-badge-monochromatic">⚠️ Non-Git</div>
          <h3>Chưa Khởi Tạo Git</h3>
          <p>
            Thư mục này hiện tại chưa được khởi tạo dưới dạng một kho lưu trữ Git.
          </p>
          <button 
            className="btn-init-git-mono"
            onClick={() => fetchGitData()}
          >
            Tải lại trạng thái
          </button>
        </div>
      );
    }

    return (
      <>
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
                    commits.map((c) => {
                      const details = expandedCommits[c.hash];
                      const isExpanded = Boolean(details);
                      const isEditing = editingCommitHash === c.hash;

                      return (
                        <div 
                          key={c.hash} 
                          className={`git-timeline-item ${isExpanded ? "is-expanded" : ""}`}
                          onClick={() => handleToggleCommit(c.hash)}
                          style={{ cursor: "pointer" }}
                          title="Nhấp chuột trái để xem chi tiết file & thống kê dòng sửa"
                        >
                          <div className="git-timeline-dot-wrapper">
                            <div
                              className={`git-timeline-node ${c.is_pushed ? "node-pushed" : "node-local"}`}
                              title={c.is_pushed ? "Đã đẩy lên GitHub / Remote" : "Chỉ mới commit ở Local (chưa push)"}
                            />
                          </div>
                          <div className="git-timeline-body">
                            <div className="git-commit-meta" style={{ justifyContent: "space-between" }}>
                              <div style={{ display: "flex", alignItems: "center", gap: "5px", minWidth: 0 }}>
                                <span className="git-commit-hash">{c.hash}</span>
                                <span className={`git-sync-badge ${c.is_pushed ? "pushed" : "local"}`}>
                                  {c.is_pushed ? (
                                    <>
                                      <Cloud size={9} />
                                      <span>Đã push</span>
                                    </>
                                  ) : (
                                    <>
                                      <CloudOff size={9} />
                                      <span>Chưa push</span>
                                    </>
                                  )}
                                </span>
                              </div>

                              <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                                {!c.is_pushed && (
                                  <div className="git-commit-actions" onClick={(e) => e.stopPropagation()}>
                                    <button
                                      className="git-commit-action-btn"
                                      title="Sửa nội dung commit (chưa push)"
                                      disabled={isActionLoading}
                                      onClick={() => {
                                        setEditingCommitHash(c.hash);
                                        setEditingCommitMsg(c.subject);
                                      }}
                                    >
                                      <Edit3 size={10} />
                                    </button>
                                    <button
                                      className="git-commit-action-btn push-single"
                                      title="Push riêng commit này lên remote"
                                      disabled={isActionLoading}
                                      onClick={() => handlePushSingleCommit(c)}
                                    >
                                      <Upload size={10} />
                                      <span>Push</span>
                                    </button>
                                  </div>
                                )}
                                <span className="git-commit-date">{c.date}</span>
                              </div>
                            </div>

                            {isEditing ? (
                              <div
                                style={{ display: "flex", gap: "4px", marginTop: "4px" }}
                                onClick={(e) => e.stopPropagation()}
                              >
                                <input
                                  type="text"
                                  className="git-reword-input"
                                  value={editingCommitMsg}
                                  onChange={(e) => setEditingCommitMsg(e.target.value)}
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter") handleSaveReword(c);
                                    if (e.key === "Escape") setEditingCommitHash(null);
                                  }}
                                  autoFocus
                                />
                                <button
                                  className="git-reword-btn save"
                                  onClick={() => handleSaveReword(c)}
                                  disabled={isActionLoading}
                                  title="Lưu"
                                >
                                  <Check size={11} />
                                </button>
                                <button
                                  className="git-reword-btn"
                                  onClick={() => setEditingCommitHash(null)}
                                  title="Hủy"
                                >
                                  <X size={11} />
                                </button>
                              </div>
                            ) : (
                              <div style={{ display: "flex", alignItems: "baseline", gap: "6px" }}>
                                <span className="git-commit-author">{c.author}</span>
                                <span className="git-commit-subject">{c.subject}</span>
                              </div>
                            )}
                            
                            {loadingCommits[c.hash] && (
                              <div style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "10px", color: "var(--text-muted)", marginTop: "6px" }}>
                                <Loader2 size={10} className="spin-animation" />
                                <span>Đang phân tích thay đổi...</span>
                              </div>
                            )}

                            {details && (
                              <div
                                className="git-commit-details-box"
                                onClick={(e) => e.stopPropagation()}
                              >
                                <div className="git-commit-summary-bar">
                                  <span>{details.total_files} file</span>
                                  <span className="stat-ins">+{details.total_insertions}</span>
                                  <span className="stat-del">-{details.total_deletions}</span>
                                  {details.added_files > 0 && <span className="stat-tag added">{details.added_files} thêm</span>}
                                  {details.modified_files > 0 && <span className="stat-tag modified">{details.modified_files} sửa</span>}
                                  {details.deleted_files > 0 && <span className="stat-tag deleted">{details.deleted_files} xóa</span>}
                                </div>

                                {details.files.length === 0 ? (
                                  <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>Không có file thay đổi.</span>
                                ) : (
                                  <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
                                    {details.files.map((file) => (
                                      <div
                                        key={`${c.hash}-${file.path}`}
                                        className="git-commit-file-row"
                                        onClick={() => handleViewFileDiff(c.hash, file.path)}
                                        title={`Click để xem diff chi tiết: ${file.path}`}
                                      >
                                        <div style={{ display: "flex", alignItems: "center", gap: "6px", minWidth: 0, flex: 1 }}>
                                          <span
                                            className="git-file-badge"
                                            style={{
                                              backgroundColor: file.status === "added" ? "rgba(16, 185, 129, 0.15)" : file.status === "deleted" ? "rgba(239, 68, 68, 0.15)" : "rgba(245, 158, 11, 0.15)",
                                              color: file.status === "added" ? "#10b981" : file.status === "deleted" ? "#ef4444" : "#f59e0b"
                                            }}
                                          >
                                            {file.status[0]}
                                          </span>
                                          <span className="git-commit-file-name">{file.path}</span>
                                        </div>
                                        <div style={{ display: "flex", alignItems: "center", gap: "5px", flexShrink: 0, fontSize: "9.5px", fontFamily: "var(--font-mono, monospace)" }}>
                                          {file.insertions > 0 && <span style={{ color: "#10b981" }}>+{file.insertions}</span>}
                                          {file.deletions > 0 && <span style={{ color: "#ef4444" }}>-{file.deletions}</span>}
                                        </div>
                                      </div>
                                    ))}
                                  </div>
                                )}
                              </div>
                            )}
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              )}
            </div>

          </div>

        {diffModal && (
          <div className="git-diff-modal-overlay" onClick={() => setDiffModal(null)}>
            <div className="git-diff-modal" onClick={(e) => e.stopPropagation()}>
              <div className="git-diff-modal-header">
                <div style={{ display: "flex", alignItems: "center", gap: "8px", minWidth: 0 }}>
                  <FileCode size={14} style={{ color: "#6366f1", flexShrink: 0 }} />
                  <span className="git-commit-hash">{diffModal.hash}</span>
                  <span style={{ fontSize: "11.5px", fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={diffModal.file}>
                    {diffModal.file}
                  </span>
                </div>
                <button className="git-circle-action-btn" onClick={() => setDiffModal(null)} title="Đóng">
                  <X size={13} />
                </button>
              </div>
              <div className="git-diff-modal-body">
                {diffModal.loading ? (
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "8px", padding: "32px", color: "var(--text-muted)", fontSize: "12px" }}>
                    <Loader2 size={16} className="spin-animation" />
                    <span>Đang tải diff...</span>
                  </div>
                ) : (
                  <pre className="git-diff-code">
                    {diffModal.diff.split("\n").map((line, idx) => {
                      let cls = "diff-line";
                      if (line.startsWith("+") && !line.startsWith("+++")) cls += " diff-add";
                      else if (line.startsWith("-") && !line.startsWith("---")) cls += " diff-del";
                      else if (line.startsWith("@@")) cls += " diff-hunk";
                      else if (line.startsWith("diff ") || line.startsWith("index ") || line.startsWith("---") || line.startsWith("+++")) cls += " diff-meta";
                      return (
                        <div key={idx} className={cls}>
                          {line || " "}
                        </div>
                      );
                    })}
                  </pre>
                )}
              </div>
            </div>
          </div>
        )}
      </>
    );
  };

  return (
    <div className="premium-git-panel">
      <style>{`
        .premium-git-panel {

          display: flex;
          flex-direction: column;
          height: 100%;
          background: var(--bg-secondary);
          color: var(--text-primary);
          font-family: var(--font-sans);
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
          padding: 32px 24px;
          text-align: center;
          background: transparent;
        }

        .git-panel-empty-state h3 {
          font-size: 12.5px;
          font-weight: 600;
          color: var(--text-primary);
          margin: 12px 0 6px 0;
        }

        .git-panel-empty-state p {
          font-size: 11px;
          color: var(--text-muted);
          line-height: 1.5;
          margin: 0;
          max-width: 240px;
        }

        .empty-icon-monochromatic {
          color: var(--text-muted);
          opacity: 0.6;
        }

        .warning-badge-monochromatic {
          margin-top: 8px;
          font-size: 9px;
          font-weight: 600;
          color: var(--text-muted);
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          padding: 2px 8px;
          border-radius: 3px;
        }

        .btn-init-git-mono {
          margin-top: 14px;
          background: var(--bg-tertiary);
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          padding: 5px 12px;
          border-radius: 4px;
          font-size: 11px;
          cursor: pointer;
          transition: background-color var(--transition-fast), color var(--transition-fast);
        }

        .btn-init-git-mono:hover {
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

        .loader-monochromatic {
          color: var(--text-muted);
        }

        .loading-text {
          font-size: 11px;
          color: var(--text-muted);
        }

        /* Enhanced commit status, actions, details & diff modal */
        .git-timeline-node.node-pushed {
          border-color: #10b981;
          background: rgba(16, 185, 129, 0.25);
        }
        .git-timeline-node.node-local {
          border-color: #f59e0b;
          background: rgba(245, 158, 11, 0.3);
        }
        .git-timeline-item.is-expanded .git-timeline-body {
          background: var(--bg-tertiary);
          border-color: var(--border-primary);
        }
        .git-sync-badge {
          display: inline-flex;
          align-items: center;
          gap: 3px;
          font-size: 9px;
          font-weight: 600;
          padding: 1px 5px;
          border-radius: 3px;
          white-space: nowrap;
        }
        .git-sync-badge.pushed {
          background: rgba(16, 185, 129, 0.12);
          color: #10b981;
          border: 1px solid rgba(16, 185, 129, 0.25);
        }
        .git-sync-badge.local {
          background: rgba(245, 158, 11, 0.12);
          color: #f59e0b;
          border: 1px solid rgba(245, 158, 11, 0.3);
        }
        .git-commit-date {
          font-size: 9px;
          color: var(--text-muted);
          white-space: nowrap;
        }
        .git-commit-actions {
          display: flex;
          align-items: center;
          gap: 3px;
        }
        .git-commit-action-btn {
          display: inline-flex;
          align-items: center;
          gap: 3px;
          font-size: 9.5px;
          font-weight: 600;
          padding: 1px 5px;
          border-radius: 3px;
          background: var(--bg-secondary);
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          cursor: pointer;
          transition: all 0.12s ease;
        }
        .git-commit-action-btn:hover:not(:disabled) {
          color: var(--text-primary);
          border-color: var(--color-accent, #6366f1);
        }
        .git-commit-action-btn.push-single {
          color: #f59e0b;
          border-color: rgba(245, 158, 11, 0.35);
          background: rgba(245, 158, 11, 0.08);
        }
        .git-commit-action-btn.push-single:hover:not(:disabled) {
          background: rgba(245, 158, 11, 0.2);
          color: #fbbf24;
        }
        .git-commit-action-btn:disabled {
          opacity: 0.45;
          cursor: not-allowed;
        }
        .git-reword-input {
          flex: 1;
          min-width: 0;
          font-size: 11px;
          padding: 3px 6px;
          border-radius: 3px;
          border: 1px solid var(--color-accent, #6366f1);
          background: var(--bg-primary);
          color: var(--text-primary);
          outline: none;
        }
        .git-reword-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 22px;
          height: 22px;
          border-radius: 3px;
          border: 1px solid var(--border-primary);
          background: var(--bg-secondary);
          color: var(--text-secondary);
          cursor: pointer;
        }
        .git-reword-btn.save {
          color: #10b981;
          border-color: rgba(16, 185, 129, 0.35);
        }
        .git-commit-details-box {
          margin-top: 6px;
          border-top: 1px solid var(--border-primary);
          padding-top: 6px;
          display: flex;
          flex-direction: column;
          gap: 5px;
        }
        .git-commit-summary-bar {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
          gap: 6px;
          font-size: 9.5px;
          color: var(--text-secondary);
          padding: 3px 6px;
          border-radius: 3px;
          background: var(--bg-secondary);
          border: 1px solid var(--border-primary);
        }
        .git-commit-summary-bar .stat-ins {
          color: #10b981;
          font-weight: 600;
          font-family: var(--font-mono, monospace);
        }
        .git-commit-summary-bar .stat-del {
          color: #ef4444;
          font-weight: 600;
          font-family: var(--font-mono, monospace);
        }
        .git-commit-summary-bar .stat-tag {
          padding: 0 4px;
          border-radius: 2px;
          font-size: 9px;
          font-weight: 600;
        }
        .git-commit-summary-bar .stat-tag.added {
          background: rgba(16, 185, 129, 0.12);
          color: #10b981;
        }
        .git-commit-summary-bar .stat-tag.modified {
          background: rgba(245, 158, 11, 0.12);
          color: #f59e0b;
        }
        .git-commit-summary-bar .stat-tag.deleted {
          background: rgba(239, 68, 68, 0.12);
          color: #ef4444;
        }
        .git-commit-file-row {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 6px;
          font-size: 10px;
          padding: 2px 4px;
          border-radius: 3px;
          cursor: pointer;
          transition: background 0.1s ease;
        }
        .git-commit-file-row:hover {
          background: var(--bg-secondary);
        }
        .git-file-badge {
          font-size: 8.5px;
          font-weight: 700;
          padding: 1px 4px;
          border-radius: 2px;
          text-transform: uppercase;
          flex-shrink: 0;
        }
        .git-commit-file-name {
          color: var(--text-secondary);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .git-commit-file-row:hover .git-commit-file-name {
          color: var(--text-primary);
          text-decoration: underline;
        }

        /* Diff Viewer Modal */
        .git-diff-modal-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.65);
          z-index: 9999;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 20px;
        }
        .git-diff-modal {
          width: min(860px, 92vw);
          max-height: 82vh;
          background: var(--bg-secondary);
          border: 1px solid var(--border-primary);
          border-radius: 6px;
          display: flex;
          flex-direction: column;
          overflow: hidden;
          box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
        }
        .git-diff-modal-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 10px 14px;
          border-bottom: 1px solid var(--border-primary);
          background: var(--bg-tertiary);
        }
        .git-diff-modal-body {
          flex: 1;
          overflow: auto;
          background: var(--bg-primary);
        }
        .git-diff-code {
          margin: 0;
          padding: 8px 0;
          font-family: var(--font-mono, monospace);
          font-size: 11px;
          line-height: 1.45;
          tab-size: 4;
        }
        .diff-line {
          padding: 0 12px;
          white-space: pre-wrap;
          word-break: break-all;
          color: var(--text-secondary);
        }
        .diff-line.diff-add {
          background: rgba(16, 185, 129, 0.14);
          color: #6ee7b7;
        }
        .diff-line.diff-del {
          background: rgba(239, 68, 68, 0.14);
          color: #fca5a5;
        }
        .diff-line.diff-hunk {
          background: rgba(99, 102, 241, 0.14);
          color: #a5b4fc;
          font-weight: 600;
          margin-top: 4px;
        }
        .diff-line.diff-meta {
          color: var(--text-muted);
          font-size: 10px;
        }
      `}</style>

      {renderContent()}
    </div>
  );
}
