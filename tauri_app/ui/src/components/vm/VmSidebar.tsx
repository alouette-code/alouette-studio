import { useState } from "react";
import { Plus } from "lucide-react";
import { VmItem } from "./types";

interface VmSidebarProps {
  vms: VmItem[];
  activeView: "create" | "manage";
  selectedVmId: string | null;
  onCreateClick: () => void;
  onSelectVm: (id: string) => void;
  onRenameVm: (id: string, newName: string) => Promise<void>;
}

export function VmSidebar({
  vms,
  activeView,
  selectedVmId,
  onCreateClick,
  onSelectVm,
  onRenameVm,
}: VmSidebarProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");

  const commitRename = async (vm: VmItem) => {
    const trimmed = editingName.trim();
    setEditingId(null);
    if (!trimmed || trimmed === vm.name) return;
    await onRenameVm(vm.id, trimmed);
  };

  return (
    <div className="vm-sidebar">
      <div style={{ padding: "8px" }}>
        <button
          onClick={onCreateClick}
          className="vm-btn"
          style={{ width: "100%", justifyContent: "center", backgroundColor: "var(--bg-primary)" }}
        >
          <Plus size={14} /> Create VM
        </button>
      </div>
      <div className="vm-sidebar-header">VIRTUAL MACHINES ({vms.length})</div>
      <div style={{ flex: 1, overflowY: "auto" }}>
        {vms.map((vm) => {
          const isSelected = activeView === "manage" && selectedVmId === vm.id;
          const isEditing = editingId === vm.id;
          return (
            <div
              key={vm.id}
              onClick={() => {
                if (!isEditing) onSelectVm(vm.id);
              }}
              onDoubleClick={() => {
                setEditingId(vm.id);
                setEditingName(vm.name);
              }}
              title="Click to select, double-click to rename"
              className={`vm-sidebar-item ${isSelected ? "active" : ""}`}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "8px", overflow: "hidden", flex: 1 }}>
                <div className={`vm-status-dot ${vm.status === "running" ? "running" : ""}`} />
                {isEditing ? (
                  <input
                    type="text"
                    value={editingName}
                    autoFocus
                    onFocus={(e) => e.target.select()}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) => setEditingName(e.target.value)}
                    onBlur={() => commitRename(vm)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitRename(vm);
                      if (e.key === "Escape") setEditingId(null);
                    }}
                    className="vm-input"
                    style={{ padding: "2px 4px", fontSize: "12px" }}
                  />
                ) : (
                  <span
                    style={{
                      fontSize: "12px",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {vm.name}
                  </span>
                )}
              </div>
              <span style={{ fontSize: "10px", color: "var(--text-muted)", flexShrink: 0 }}>
                {vm.config.cpu_cores}C/{Math.round(vm.config.ram_size_mb)}M
              </span>
            </div>
          );
        })}
        {vms.length === 0 && (
          <div style={{ padding: "14px", fontSize: "11px", color: "var(--text-muted)", textAlign: "center" }}>
            No VMs found.
          </div>
        )}
      </div>
    </div>
  );
}
