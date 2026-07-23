import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Folder,
  FolderOpen,
  File,
  ChevronRight,
  ChevronDown,
  Code,
  Braces,
  CircleDot,
  FilePlus,
  FolderPlus,
  RotateCw,
  Database,
} from "lucide-react";
import { globalErrorStore } from "../services/errorStore";

interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileNode[];
}

interface FileExplorerProps {
  activeCwd: string | undefined;
  onFileSelect: (filePath: string) => void;
  onFileSelectSide?: (filePath: string) => void;
}

// Helper to get parent path
const getParentPath = (path: string, isDir: boolean) => {
  if (isDir) return path;
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (lastSlash === -1) return "";
  return path.substring(0, lastSlash);
};

// Helper to get relative path
const getRelativePath = (absolutePath: string, baseCwd: string | undefined) => {
  if (!baseCwd) return absolutePath;
  const absNormalized = absolutePath.replace(/\\/g, "/");
  const baseNormalized = baseCwd.replace(/\\/g, "/");

  if (absNormalized.startsWith(baseNormalized)) {
    let rel = absNormalized.substring(baseNormalized.length);
    if (rel.startsWith("/")) {
      rel = rel.substring(1);
    }
    return rel;
  }
  return absolutePath;
};

// Sub-component to render directory tree nodes recursively
function TreeNode({
  node,
  onFileSelect,
  onNodeContextMenu,
  onNodeChildrenLoaded,
  gitFileStatuses,
  activeCwd,
  inlineRenameTarget,
  onRenameSubmit,
  onRenameCancel,
}: {
  node: FileNode;
  onFileSelect: (filePath: string) => void;
  onNodeContextMenu: (
    x: number,
    y: number,
    path: string,
    isDir: boolean,
  ) => void;
  onNodeChildrenLoaded: (path: string, children: FileNode[]) => void;
  gitFileStatuses: { [relPath: string]: string };
  activeCwd: string | undefined;
  inlineRenameTarget: string | null;
  onRenameSubmit: (oldPath: string, newName: string, isDir: boolean) => void;
  onRenameCancel: () => void;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const [isLazyLoading, setIsLazyLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (inlineRenameTarget === node.path && inputRef.current) {
      inputRef.current.focus();
      if (!node.is_dir) {
        const lastDot = node.name.lastIndexOf(".");
        if (lastDot > 0) {
          inputRef.current.setSelectionRange(0, lastDot);
        } else {
          inputRef.current.select();
        }
      } else {
        inputRef.current.select();
      }
    }
  }, [inlineRenameTarget, node.path, node.name, node.is_dir]);

  const getFileIcon = (fileName: string, isDir: boolean) => {
    if (isDir) {
      return isOpen ? (
        <FolderOpen size={13} className="tree-node-icon folder" />
      ) : (
        <Folder size={13} className="tree-node-icon folder" />
      );
    }

    const lowerName = fileName.toLowerCase();
    if (lowerName.endsWith(".json")) {
      return <Braces size={13} style={{ color: "#f59e0b" }} />;
    }
    if (
      lowerName.endsWith(".js") ||
      lowerName.endsWith(".ts") ||
      lowerName.endsWith(".tsx") ||
      lowerName.endsWith(".jsx") ||
      lowerName.endsWith(".rs")
    ) {
      return <Code size={13} style={{ color: "#3a86ff" }} />;
    }
    return <File size={13} className="tree-node-icon file" />;
  };

  const handleRowClick = async () => {
    if (node.is_dir) {
      if (!isOpen && (!node.children || node.children.length === 0)) {
        setIsLazyLoading(true);
        try {
          const data = await invoke<FileNode[]>("get_directory_contents", {
            dirPath: node.path,
          });
          onNodeChildrenLoaded(node.path, data);
        } catch (e) {
          console.error("Failed to load lazy directory contents:", e);
        } finally {
          setIsLazyLoading(false);
        }
      }
      setIsOpen(!isOpen);
    } else {
      onFileSelect(node.path);
    }
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onNodeContextMenu(e.clientX, e.clientY, node.path, node.is_dir);
  };

  const relPath = getRelativePath(node.path, activeCwd).replace(/\\/g, "/");

  // For directories: check if any descendant file has git status
  let gitStatus: string | undefined;
  if (node.is_dir) {
    const prefix = relPath ? relPath + "/" : "";
    const hasModified = Object.keys(gitFileStatuses).some(
      (k) => k.startsWith(prefix) && gitFileStatuses[k] === "modified",
    );
    const hasUntracked = Object.keys(gitFileStatuses).some(
      (k) =>
        k.startsWith(prefix) &&
        (gitFileStatuses[k] === "untracked" || gitFileStatuses[k] === "added"),
    );
    if (hasModified) gitStatus = "modified";
    else if (hasUntracked) gitStatus = "untracked";
  } else {
    gitStatus = gitFileStatuses[relPath];
  }

  return (
    <div
      style={{ display: "flex", flexDirection: "column" }}
      onContextMenu={handleContextMenu}
    >
      <div
        className="tree-node-row"
        onClick={handleRowClick}
        style={{ display: "flex", alignItems: "center", width: "100%" }}
      >
        {node.is_dir ? (
          <span style={{ display: "inline-flex", marginRight: "2px" }}>
            {isOpen ? <ChevronDown size={11} /> : <ChevronRight size={11} />}
          </span>
        ) : (
          <span style={{ width: "13px", display: "inline-block" }} />
        )}
        {getFileIcon(node.name, node.is_dir)}
        {inlineRenameTarget === node.path ? (
          <input
            ref={inputRef}
            type="text"
            defaultValue={node.name}
            style={{
              background: "var(--bg-primary)",
              border: "1px solid var(--color-primary)",
              color: "var(--text-primary)",
              padding: "0 2px",
              marginLeft: "4px",
              fontSize: "inherit",
              outline: "none",
              width: "100%",
              borderRadius: "2px",
            }}
            onClick={(e) => e.stopPropagation()}
            onBlur={(e) => {
              const val = e.target.value.trim();
              if (val && val !== node.name) {
                onRenameSubmit(node.path, val, node.is_dir);
              } else {
                onRenameCancel();
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                const target = e.target as HTMLInputElement;
                const val = target.value.trim();
                if (val && val !== node.name) {
                  onRenameSubmit(node.path, val, node.is_dir);
                } else {
                  onRenameCancel();
                }
              } else if (e.key === "Escape") {
                onRenameCancel();
              }
            }}
          />
        ) : (() => {
          const errCount = globalErrorStore.getErrorCount(node.path, node.is_dir);
          const hasSyntaxError = errCount > 0;

          return (
            <span
              className={`tree-node-name ${hasSyntaxError ? "has-error" : ""}`}
              style={{
                color: hasSyntaxError
                  ? "#ef4444"
                  : gitStatus === "modified"
                  ? "var(--git-modified, #eab308)"
                  : gitStatus === "untracked" || gitStatus === "added"
                    ? "var(--git-added, #10b981)"
                    : "inherit",
                fontWeight: hasSyntaxError ? 600 : "normal",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {node.name}
            </span>
          );
        })()}

        {(() => {
          const errCount = globalErrorStore.getErrorCount(node.path, node.is_dir);
          const hasSyntaxError = errCount > 0;

          if (!hasSyntaxError && !gitStatus) return null;

          return (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                marginLeft: "auto",
                marginRight: "6px",
                gap: "4px",
              }}
            >
              {hasSyntaxError && (
                <span
                  style={{
                    fontSize: "11px",
                    fontWeight: 600,
                    color: "#ef4444",
                    lineHeight: 1,
                  }}
                  title={`${errCount} lỗi code`}
                >
                  {errCount}
                </span>
              )}
              {gitStatus && (
                <span
                  style={{
                    display: "inline-flex",
                    opacity: 0.8,
                  }}
                  title={gitStatus === "modified" ? "Modified" : "Untracked"}
                >
                  {gitStatus === "modified" ? (
                    <CircleDot
                      size={10}
                      style={{ color: "var(--git-modified, #eab308)" }}
                    />
                  ) : (
                    <CircleDot
                      size={10}
                      style={{ color: "var(--git-added, #10b981)" }}
                    />
                  )}
                </span>
              )}
            </div>
          );
        })()}
      </div>

      {node.is_dir && isOpen && (
        <div
          style={{
            paddingLeft: "12px",
            borderLeft: "1px solid var(--border-primary)",
            marginLeft: "18px",
          }}
        >
          {isLazyLoading && (
            <div
              className="explorer-empty"
              style={{ padding: "4px 8px", textAlign: "left" }}
            >
              Loading...
            </div>
          )}
          {!isLazyLoading &&
            node.children &&
            node.children.map((child, idx) => (
              <TreeNode
                key={idx}
                node={child}
                onFileSelect={onFileSelect}
                onNodeContextMenu={onNodeContextMenu}
                onNodeChildrenLoaded={onNodeChildrenLoaded}
                gitFileStatuses={gitFileStatuses}
                activeCwd={activeCwd}
                inlineRenameTarget={inlineRenameTarget}
                onRenameSubmit={onRenameSubmit}
                onRenameCancel={onRenameCancel}
              />
            ))}
          {!isLazyLoading && (!node.children || node.children.length === 0) && (
            <div
              className="explorer-empty"
              style={{ padding: "4px 8px", textAlign: "left" }}
            >
              Empty
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// Tree Merging Helper
const mergeTrees = (newNodes: FileNode[], oldNodes: FileNode[]): FileNode[] => {
  return newNodes.map((newNode) => {
    const oldNode = oldNodes.find((o) => o.path === newNode.path);
    if (oldNode && oldNode.is_dir) {
      const mergedChildren =
        newNode.children && newNode.children.length > 0
          ? mergeTrees(newNode.children, oldNode.children || [])
          : oldNode.children || [];
      return { ...newNode, children: mergedChildren };
    }
    return newNode;
  });
};

export default function FileExplorer({
  activeCwd,
  onFileSelect,
  onFileSelectSide,
}: FileExplorerProps) {
  const [files, setFiles] = useState<FileNode[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [gitFileStatuses, setGitFileStatuses] = useState<{
    [relPath: string]: string;
  }>({});
  const [, forceRender] = useState(0);

  useEffect(() => {
    const unsubscribe = globalErrorStore.subscribe(() => {
      forceRender((n) => n + 1);
    });
    return unsubscribe;
  }, []);

  // Context Menu State
  const [contextMenu, setContextMenu] = useState<{
    visible: boolean;
    x: number;
    y: number;
    targetPath: string | null;
    targetIsDir: boolean;
  }>({
    visible: false,
    x: 0,
    y: 0,
    targetPath: null,
    targetIsDir: true,
  });

  // Naming Prompt Modal State
  const [showPrompt, setShowPrompt] = useState(false);
  const [promptType, setPromptType] = useState<"file" | "folder">("file");
  const [newItemName, setNewItemName] = useState("");

  // Rename & Clipboard State
  const [inlineRenameTarget, setInlineRenameTarget] = useState<string | null>(null);
  const [clipboard, setClipboard] = useState<{ action: "cut" | "copy", path: string } | null>(null);

  const fetchGitStatus = async () => {
    if (!activeCwd) return;
    try {
      const status: any = await invoke("git_get_status", { cwd: activeCwd });
      const statusMap: { [relPath: string]: string } = {};
      status.staged.forEach((f: any) => {
        statusMap[f.path.replace(/\\/g, "/")] = f.status;
      });
      status.unstaged.forEach((f: any) => {
        statusMap[f.path.replace(/\\/g, "/")] = f.status;
      });
      setGitFileStatuses(statusMap);
    } catch (e) {
      setGitFileStatuses({});
    }
  };

  const fetchFiles = async (silent = false) => {
    if (!silent && files.length === 0) setLoading(true);
    setError(null);
    try {
      const data = await invoke<FileNode[]>("get_project_files", {
        dirPath: activeCwd || null,
      });
      setFiles((prev) => mergeTrees(data, prev));
      await fetchGitStatus();
    } catch (e: any) {
      console.error("Failed to load project files tree:", e);
      setError(e.toString());
    } finally {
      if (!silent) setLoading(false);
    }
  };

  useEffect(() => {
    fetchFiles(false);

    // Thay thế polling bằng event-driven: lắng nghe file-system-changed từ Rust notify
    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      unlisten = await listen("file-system-changed", () => {
        fetchFiles(true);
      });
    };
    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [activeCwd]);

  // Check if we are inside a Tauri window to load files automatically
  useEffect(() => {
    const interval = setInterval(() => {
      fetchGitStatus();
    }, 10000);
    return () => clearInterval(interval);
  }, [activeCwd]);

  useEffect(() => {
    const closeMenu = () =>
      setContextMenu((prev) => ({ ...prev, visible: false }));
    window.addEventListener("click", closeMenu);
    return () => window.removeEventListener("click", closeMenu);
  }, []);

  const handleNodeContextMenu = (
    x: number,
    y: number,
    path: string,
    isDir: boolean,
  ) => {
    const menuHeight = 360; // approximate max height of the context menu
    let adjustedY = y;
    if (y + menuHeight > window.innerHeight) {
      adjustedY = Math.max(0, window.innerHeight - menuHeight - 10);
    }

    setContextMenu({
      visible: true,
      x,
      y: adjustedY,
      targetPath: path,
      targetIsDir: isDir,
    });
  };

  const handleContainerContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const menuHeight = 360;
    let adjustedY = e.clientY;
    if (adjustedY + menuHeight > window.innerHeight) {
      adjustedY = Math.max(0, window.innerHeight - menuHeight - 10);
    }

    setContextMenu({
      visible: true,
      x: e.clientX,
      y: adjustedY,
      targetPath: activeCwd || null,
      targetIsDir: true,
    });
  };

  const handleNodeChildrenLoaded = (path: string, children: FileNode[]) => {
    const updateTree = (nodes: FileNode[]): FileNode[] => {
      return nodes.map((n) => {
        if (n.path === path) {
          return { ...n, children };
        }
        if (n.is_dir && n.children) {
          return { ...n, children: updateTree(n.children) };
        }
        return n;
      });
    };
    setFiles((prev) => updateTree(prev));
  };

  const triggerCreateItem = (type: "file" | "folder") => {
    setPromptType(type);
    setNewItemName("");
    setShowPrompt(true);
  };

  const submitCreation = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newItemName.trim()) return;

    let targetDir = activeCwd || "";
    if (contextMenu.targetPath) {
      targetDir = getParentPath(
        contextMenu.targetPath,
        contextMenu.targetIsDir,
      );
    }

    const separator = targetDir.includes("\\") ? "\\" : "/";
    const fullPath = targetDir
      ? `${targetDir}${separator}${newItemName.trim()}`
      : newItemName.trim();

    try {
      if (promptType === "file") {
        await invoke("create_file", { path: fullPath });
      } else {
        await invoke("create_folder", { path: fullPath });
      }
      setShowPrompt(false);
      setNewItemName("");
      // Refresh immediately
      fetchFiles(true);
    } catch (err: any) {
      alert(`Error: ${err.toString()}`);
    }
  };

  const handleCopyPath = () => {
    if (contextMenu.targetPath) {
      navigator.clipboard.writeText(contextMenu.targetPath);
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  };

  const handleCopyRelativePath = () => {
    if (contextMenu.targetPath && activeCwd) {
      navigator.clipboard.writeText(getRelativePath(contextMenu.targetPath, activeCwd));
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  };

  const handleCopyFile = () => {
    if (contextMenu.targetPath) {
      setClipboard({ action: "copy", path: contextMenu.targetPath });
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  };

  const handleCutFile = () => {
    if (contextMenu.targetPath) {
      setClipboard({ action: "cut", path: contextMenu.targetPath });
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  };

  const handlePasteFile = async () => {
    if (!clipboard || !contextMenu.targetPath) return;
    
    let targetDir = contextMenu.targetPath;
    if (!contextMenu.targetIsDir) {
      targetDir = getParentPath(contextMenu.targetPath, false);
    }
    
    const sep = clipboard.path.includes("\\") ? "\\" : "/";
    const fileName = clipboard.path.split(sep).pop();
    const destPath = targetDir + sep + fileName;

    try {
      if (clipboard.action === "copy") {
        await invoke("copy_item", { source_path: clipboard.path, dest_path: destPath });
      } else {
        await invoke("rename_item", { old_path: clipboard.path, new_path: destPath });
        setClipboard(null);
      }
      fetchFiles(true);
    } catch (err: any) {
      alert(`Error pasting: ${err.toString()}`);
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  };

  const triggerRename = () => {
    if (contextMenu.targetPath) {
      setInlineRenameTarget(contextMenu.targetPath);
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  };

  const handleRenameSubmit = async (oldPath: string, newName: string, isDir: boolean) => {
    let targetDir = getParentPath(oldPath, isDir);
    if (isDir) {
      const lastSlash = Math.max(oldPath.lastIndexOf("/"), oldPath.lastIndexOf("\\"));
      targetDir = lastSlash !== -1 ? oldPath.substring(0, lastSlash) : "";
    }
    const sep = oldPath.includes("\\") ? "\\" : "/";
    const newPath = targetDir ? `${targetDir}${sep}${newName}` : newName;

    try {
      await invoke("rename_item", { old_path: oldPath, new_path: newPath });
      setInlineRenameTarget(null);
      fetchFiles(true);
    } catch (err: any) {
      alert(`Error renaming: ${err.toString()}`);
      setInlineRenameTarget(null);
    }
  };

  const handleRenameCancel = () => {
    setInlineRenameTarget(null);
  };
  
  const deleteItem = async () => {
    if (!contextMenu.targetPath) return;
    const confirmMsg = `Are you sure you want to delete ${
      contextMenu.targetIsDir ? "folder" : "file"
    } '${contextMenu.targetPath}'?`;
    if (!confirm(confirmMsg)) {
      setContextMenu((prev) => ({ ...prev, visible: false }));
      return;
    }

    try {
      await invoke("delete_item", { path: contextMenu.targetPath });
      setContextMenu((prev) => ({ ...prev, visible: false }));
      fetchFiles(true);
    } catch (err: any) {
      alert(`Error deleting item: ${err.toString()}`);
    }
  };

  return (
    <div
      className="explorer-container"
      onContextMenu={handleContainerContextMenu}
    >
      {/* Dynamic Styling block for action buttons & context menu */}
      <style>{`
        .explorer-action-btn {
          background: none;
          border: none;
          color: var(--text-muted);
          cursor: pointer;
          padding: 2px;
          display: flex;
          align-items: center;
          justify-content: center;
          transition: color var(--transition-fast);
          outline: none;
        }
        .explorer-action-btn:hover {
          color: var(--text-primary);
        }
        .explorer-context-menu {
          background-color: var(--bg-secondary);
          border: 1px solid var(--border-primary);
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
          padding: 4px 0;
          min-width: 140px;
          display: flex;
          flex-direction: column;
          border-radius: 4px;
        }
        .context-menu-item {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 6px 12px;
          font-size: 11.5px;
          color: var(--text-secondary);
          cursor: pointer;
          transition: background-color var(--transition-fast), color var(--transition-fast);
        }
        .context-menu-item:hover {
          background-color: var(--bg-tertiary);
          color: var(--text-primary);
        }
        .context-menu-separator {
          height: 1px;
          background-color: var(--border-primary);
          margin: 4px 0;
        }
        .context-menu-item-content {
          display: flex;
          justify-content: space-between;
          align-items: center;
          width: 100%;
        }
        .context-menu-item-left {
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .context-menu-item-right {
          font-size: 10px;
          color: var(--text-muted);
        }
      `}</style>

      <header
        className="explorer-header"
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <Folder size={11} />
          <span>PROJECT EXPLORER</span>
        </div>
        <div
          style={{ display: "flex", gap: "6px" }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            className="explorer-action-btn"
            title="New File"
            onClick={() => {
              setContextMenu((prev) => ({
                ...prev,
                targetPath: activeCwd || null,
                targetIsDir: true,
              }));
              triggerCreateItem("file");
            }}
          >
            <FilePlus size={12} />
          </button>
          <button
            className="explorer-action-btn"
            title="New Folder"
            onClick={() => {
              setContextMenu((prev) => ({
                ...prev,
                targetPath: activeCwd || null,
                targetIsDir: true,
              }));
              triggerCreateItem("folder");
            }}
          >
            <FolderPlus size={12} />
          </button>
          <button
            className="explorer-action-btn"
            title="Reload Explorer"
            onClick={() => fetchFiles(false)}
          >
            <RotateCw size={12} />
          </button>
          <button
            onClick={() => onFileSelect("db://new_connection")}
            className="explorer-action-btn"
            title="Connect Database"
            style={{ color: 'var(--color-accent)' }}
          >
            <Database size={12} />
          </button>
        </div>
      </header>



      <div className="explorer-scroll-viewport">
        {loading && <div className="explorer-empty">Traversing files...</div>}
        {!loading && error && (
          <div
            className="explorer-empty"
            style={{ color: "var(--color-danger)" }}
          >
            Error listing directory
          </div>
        )}
        {!loading && !error && files.length === 0 && (
          <div className="explorer-empty">Folder is empty</div>
        )}
        {!loading && !error && files.length > 0 && (
          <div>
            {files.map((file, idx) => (
              <TreeNode
                key={idx}
                node={file}
                onFileSelect={onFileSelect}
                onNodeContextMenu={handleNodeContextMenu}
                onNodeChildrenLoaded={handleNodeChildrenLoaded}
                gitFileStatuses={gitFileStatuses}
                activeCwd={activeCwd}
                inlineRenameTarget={inlineRenameTarget}
                onRenameSubmit={handleRenameSubmit}
                onRenameCancel={handleRenameCancel}
              />
            ))}
          </div>
        )}
      </div>

      {/* Sleek context menu */}
      {contextMenu.visible && (
        <div
          className="explorer-context-menu animate-fade-in"
          style={{
            position: "fixed",
            top: contextMenu.y,
            left: contextMenu.x,
            zIndex: 1000,
            minWidth: "220px",
          }}
          onClick={(e) => e.stopPropagation()}
        >
          {/* Create Items */}
          <div className="context-menu-item" onClick={() => triggerCreateItem("file")}>
            <div className="context-menu-item-content">
              <div className="context-menu-item-left"><span>New File</span></div>
            </div>
          </div>
          <div className="context-menu-item" onClick={() => triggerCreateItem("folder")}>
            <div className="context-menu-item-content">
              <div className="context-menu-item-left"><span>New Folder</span></div>
            </div>
          </div>
          <div className="context-menu-separator"></div>

          {/* Open Items */}
          {contextMenu.targetPath && !contextMenu.targetIsDir && (
            <div className="context-menu-item" onClick={() => {
              // Open to side if supported, fallback to open
              if (onFileSelectSide) onFileSelectSide(contextMenu.targetPath!);
              else onFileSelect(contextMenu.targetPath!);
              setContextMenu((prev) => ({ ...prev, visible: false }));
            }}>
              <div className="context-menu-item-content">
                <div className="context-menu-item-left"><span>Open to the Side</span></div>
                <span className="context-menu-item-right">Ctrl+Enter</span>
              </div>
            </div>
          )}
          {contextMenu.targetPath && (
            <div className="context-menu-item" onClick={() => {
              // Open Containing Folder could use shell
              setContextMenu((prev) => ({ ...prev, visible: false }));
            }}>
              <div className="context-menu-item-content">
                <div className="context-menu-item-left"><span>Open Containing Folder</span></div>
              </div>
            </div>
          )}
          {contextMenu.targetPath && (
            <div className="context-menu-item" onClick={() => {
              setContextMenu((prev) => ({ ...prev, visible: false }));
            }}>
              <div className="context-menu-item-content">
                <div className="context-menu-item-left"><span>Open in Integrated Terminal</span></div>
              </div>
            </div>
          )}
          
          <div className="context-menu-separator"></div>

          {/* Clipboard Ops */}
          {contextMenu.targetPath && (
            <>
              <div className="context-menu-item" onClick={handleCutFile}>
                <div className="context-menu-item-content">
                  <div className="context-menu-item-left"><span>Cut</span></div>
                  <span className="context-menu-item-right">Ctrl+X</span>
                </div>
              </div>
              <div className="context-menu-item" onClick={handleCopyFile}>
                <div className="context-menu-item-content">
                  <div className="context-menu-item-left"><span>Copy</span></div>
                  <span className="context-menu-item-right">Ctrl+C</span>
                </div>
              </div>
            </>
          )}
          {clipboard && contextMenu.targetIsDir && (
             <div className="context-menu-item" onClick={handlePasteFile}>
               <div className="context-menu-item-content">
                 <div className="context-menu-item-left"><span>Paste</span></div>
                 <span className="context-menu-item-right">Ctrl+V</span>
               </div>
             </div>
          )}

          <div className="context-menu-separator"></div>

          {/* Path Ops */}
          {contextMenu.targetPath && (
            <>
              <div className="context-menu-item" onClick={handleCopyPath}>
                <div className="context-menu-item-content">
                  <div className="context-menu-item-left"><span>Copy Path</span></div>
                  <span className="context-menu-item-right">Shift+Alt+C</span>
                </div>
              </div>
              <div className="context-menu-item" onClick={handleCopyRelativePath}>
                <div className="context-menu-item-content">
                  <div className="context-menu-item-left"><span>Copy Relative Path</span></div>
                  <span className="context-menu-item-right">Ctrl+Shift+C</span>
                </div>
              </div>
            </>
          )}

          <div className="context-menu-separator"></div>

          {/* Destructive Ops */}
          {contextMenu.targetPath && contextMenu.targetPath !== activeCwd && (
            <>
              <div className="context-menu-item" onClick={triggerRename}>
                <div className="context-menu-item-content">
                  <div className="context-menu-item-left"><span>Rename...</span></div>
                  <span className="context-menu-item-right">F2</span>
                </div>
              </div>
              <div className="context-menu-item" onClick={deleteItem}>
                <div className="context-menu-item-content">
                  <div className="context-menu-item-left"><span>Delete</span></div>
                  <span className="context-menu-item-right">Del</span>
                </div>
              </div>
            </>
          )}
        </div>
      )}

      {/* Styled Inline Prompt Modal */}
      {showPrompt && (
        <div
          className="modal-overlay animate-fade-in"
          style={{ zIndex: 1001 }}
          onClick={() => setShowPrompt(false)}
        >
          <div
            className="modal-content"
            style={{ width: "320px", borderRadius: "4px" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <span className="modal-title">
                Create New {promptType === "file" ? "File" : "Folder"}
              </span>
              <button
                className="terminal-action-btn"
                onClick={() => setShowPrompt(false)}
              >
                ✕
              </button>
            </div>
            <form onSubmit={submitCreation}>
              <div className="modal-body">
                <input
                  autoFocus
                  type="text"
                  className="admin-input"
                  placeholder={
                    promptType === "file" ? "filename.txt" : "Folder Name"
                  }
                  value={newItemName}
                  onChange={(e) => setNewItemName(e.target.value)}
                  style={{
                    width: "100%",
                    background: "var(--bg-primary)",
                    border: "1px solid var(--border-primary)",
                    color: "var(--text-primary)",
                    padding: "6px 10px",
                    borderRadius: "4px",
                    outline: "none",
                  }}
                />
              </div>
              <div
                className="modal-footer"
                style={{
                  padding: "8px 12px",
                  display: "flex",
                  justifyContent: "flex-end",
                  gap: "8px",
                }}
              >
                <button
                  type="button"
                  className="btn btn-secondary btn-xs"
                  onClick={() => setShowPrompt(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary btn-xs">
                  Create
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
