import { useEffect, useMemo, useState } from "react";
import { ArrowRight, ChevronsLeft, Folder, FolderOpen, Plus, Trash2, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { AlouetteCheckbox, AlouetteNumberInput } from "../AlouetteControls";
import { AlouetteSelect, SelectOption } from "../AlouetteSelect";
import { listAllOsOptions } from "./presets";
import { VmConfig, VmItem, VmNotice } from "./types";

interface VmDetailsPanelProps {
  vm: VmItem;
  draftConfig: VmConfig;
  onDraftChange: (next: VmConfig) => void;
  onNotice: (notice: VmNotice) => void;
  onHidePanel?: () => void;
}

export function VmDetailsPanel({
  vm,
  draftConfig,
  onDraftChange,
  onNotice,
  onHidePanel,
}: VmDetailsPanelProps) {
  const [snapshots, setSnapshots] = useState<string[]>([]);
  const [newSnapshotName, setNewSnapshotName] = useState("");
  const [confirmRestoreSnap, setConfirmRestoreSnap] = useState<string | null>(null);
  const [injectHostPath, setInjectHostPath] = useState("");
  const [injectGuestPath, setInjectGuestPath] = useState("");
  const [newHostPort, setNewHostPort] = useState("");
  const [newGuestPort, setNewGuestPort] = useState("");

  const refreshSnapshots = async (vmId: string) => {
    try {
      const list = await invoke<string[]>("list_vm_snapshots", { id: vmId });
      setSnapshots(list || []);
    } catch {
      setSnapshots([]);
    }
  };

  useEffect(() => {
    setConfirmRestoreSnap(null);
    refreshSnapshots(vm.id);
  }, [vm.id]);

  const browseFile = async (setter: (path: string) => void) => {
    try {
      const path = await invoke<string | null>("open_file_dialog");
      if (path) setter(path);
    } catch {
      // cancelled
    }
  };

  const handleCreateSnapshot = async () => {
    const name = newSnapshotName.trim();
    if (!name) return;
    try {
      await invoke("create_vm_snapshot", { id: vm.id, name });
      setNewSnapshotName("");
      await refreshSnapshots(vm.id);
      onNotice({ type: "success", message: `Snapshot "${name}" created.` });
    } catch (err) {
      onNotice({ type: "error", message: `Snapshot failed: ${err}` });
    }
  };

  const handleRestoreSnapshot = async (name: string) => {
    if (confirmRestoreSnap !== name) {
      setConfirmRestoreSnap(name);
      return;
    }
    setConfirmRestoreSnap(null);
    try {
      await invoke("restore_vm_snapshot", { id: vm.id, name });
      onNotice({ type: "success", message: `Snapshot "${name}" restored.` });
    } catch (err) {
      onNotice({ type: "error", message: `Restore failed: ${err}` });
    }
  };

  const handleDeleteSnapshot = async (name: string) => {
    try {
      await invoke("delete_vm_snapshot", { id: vm.id, name });
      await refreshSnapshots(vm.id);
      onNotice({ type: "info", message: `Snapshot "${name}" deleted.` });
    } catch (err) {
      onNotice({ type: "error", message: `Delete snapshot failed: ${err}` });
    }
  };

  const handleInjectFile = async () => {
    if (!injectHostPath.trim() || !injectGuestPath.trim()) {
      onNotice({ type: "error", message: "Please specify both Host Path and Guest Destination." });
      return;
    }
    try {
      await invoke("inject_guest_file", {
        id: vm.id,
        hostPath: injectHostPath.trim(),
        guestPath: injectGuestPath.trim(),
      });
      onNotice({ type: "success", message: `Injected file to ${injectGuestPath.trim()}.` });
      setInjectHostPath("");
      setInjectGuestPath("");
    } catch (err) {
      onNotice({ type: "error", message: `File injection failed: ${err}` });
    }
  };

  const handleAddPortForward = () => {
    const h = Number(newHostPort);
    const g = Number(newGuestPort);
    if (!Number.isInteger(h) || !Number.isInteger(g) || h < 1 || h > 65535 || g < 1 || g > 65535) {
      onNotice({ type: "error", message: "Ports must be integers between 1 and 65535." });
      return;
    }
    onDraftChange({
      ...draftConfig,
      advanced: {
        ...draftConfig.advanced,
        host_port_forwards: [...draftConfig.advanced.host_port_forwards, [h, g]],
      },
    });
    setNewHostPort("");
    setNewGuestPort("");
  };

  const handleRemovePortForward = (idx: number) => {
    onDraftChange({
      ...draftConfig,
      advanced: {
        ...draftConfig.advanced,
        host_port_forwards: draftConfig.advanced.host_port_forwards.filter((_, i) => i !== idx),
      },
    });
  };

  const osOptions: SelectOption[] = useMemo(
    () =>
      listAllOsOptions().map((opt) => ({
        value: opt.id,
        label: opt.label,
        group: opt.group,
      })),
    []
  );

  const archOptions: SelectOption[] = useMemo(
    () => [
      { value: "x86_64", label: "x86_64" },
      { value: "aarch64", label: "ARM64" },
    ],
    []
  );

  const firmwareOptions: SelectOption[] = useMemo(
    () => [
      { value: "bios", label: "BIOS" },
      { value: "uefi", label: "UEFI" },
    ],
    []
  );

  const networkOptions: SelectOption[] = useMemo(
    () => [
      { value: "nat", label: "NAT (User Mode)" },
      { value: "bridge", label: "Bridged Adapter (br0)" },
      { value: "host-only", label: "Host-Only / Isolated" },
    ],
    []
  );

  return (
    <div className="vm-details-panel">
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderBottom: "1px solid var(--border-primary)",
          paddingBottom: "4px",
          marginBottom: "10px",
        }}
      >
        <h3
          className="vm-section-title"
          style={{ borderBottom: "none", paddingBottom: 0, margin: 0 }}
        >
          Hardware &amp; Profile
        </h3>
        {onHidePanel && (
          <button
            type="button"
            onClick={onHidePanel}
            className="vm-btn vm-btn-icon"
            title="Collapse config panel"
            style={{ padding: "3px 6px" }}
          >
            <ChevronsLeft size={13} />
          </button>
        )}
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 110px", gap: "8px", marginBottom: "10px" }}>
        <div>
          <label className="vm-label">OS Profile</label>
          <AlouetteSelect
            value={draftConfig.os_type || "custom"}
            options={osOptions}
            onChange={(val) => onDraftChange({ ...draftConfig, os_type: val })}
            searchable
            searchPlaceholder="Filter OS..."
          />
        </div>
        <div>
          <label className="vm-label">Arch</label>
          <AlouetteSelect
            value={draftConfig.arch || "x86_64"}
            options={archOptions}
            onChange={(val) => onDraftChange({ ...draftConfig, arch: val })}
            searchable={false}
          />
        </div>
      </div>
      <div style={{ marginBottom: "10px" }}>
        <label className="vm-label">VM Directory (ID: {vm.id})</label>
        <input
          type="text"
          value={draftConfig.vm_dir}
          readOnly
          className="vm-input vm-input-readonly"
        />
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1.25fr 1fr", gap: "8px", marginBottom: "12px" }}>
        <div>
          <label className="vm-label">CPU Cores</label>
          <AlouetteNumberInput
            size="sm"
            min={1}
            max={64}
            value={draftConfig.cpu_cores}
            onChange={(val) => onDraftChange({ ...draftConfig, cpu_cores: val })}
          />
        </div>
        <div>
          <label className="vm-label">RAM (MB)</label>
          <AlouetteNumberInput
            size="sm"
            min={128}
            max={65536}
            step={256}
            value={draftConfig.ram_size_mb}
            onChange={(val) => onDraftChange({ ...draftConfig, ram_size_mb: val })}
          />
        </div>
        <div>
          <label className="vm-label">Firmware</label>
          <AlouetteSelect
            size="sm"
            value={draftConfig.firmware || "bios"}
            options={firmwareOptions}
            onChange={(val) => onDraftChange({ ...draftConfig, firmware: val })}
            searchable={false}
          />
        </div>
      </div>

      <h3 className="vm-section-title">Storage</h3>
      <div style={{ marginBottom: "10px" }}>
        <label className="vm-label">Disk Image (.qcow2)</label>
        <div style={{ display: "flex", gap: "4px" }}>
          <input
            type="text"
            value={draftConfig.disk_path || ""}
            onChange={(e) => onDraftChange({ ...draftConfig, disk_path: e.target.value || null })}
            className="vm-input"
            placeholder="Auto-created if empty"
          />
          <button
            type="button"
            onClick={() => browseFile((p) => onDraftChange({ ...draftConfig, disk_path: p }))}
            className="vm-btn"
            title="Browse Disk"
          >
            <FolderOpen size={12} />
          </button>
        </div>
      </div>
      <div style={{ marginBottom: "12px" }}>
        <label className="vm-label">ISO Image (CD-ROM)</label>
        <div style={{ display: "flex", gap: "4px" }}>
          <input
            type="text"
            value={draftConfig.iso_path || ""}
            onChange={(e) => onDraftChange({ ...draftConfig, iso_path: e.target.value || null })}
            className="vm-input"
            placeholder="Leave empty after OS installation"
          />
          <button
            type="button"
            onClick={() => browseFile((p) => onDraftChange({ ...draftConfig, iso_path: p }))}
            className="vm-btn"
            title="Browse ISO"
          >
            <FolderOpen size={12} />
          </button>
          {draftConfig.iso_path && (
            <button
              type="button"
              onClick={() => onDraftChange({ ...draftConfig, iso_path: null })}
              className="vm-btn"
              title="Eject ISO"
            >
              <X size={12} />
            </button>
          )}
        </div>
      </div>

      <h3 className="vm-section-title">Network &amp; Advanced</h3>
      <div style={{ marginBottom: "10px" }}>
        <label className="vm-label">Adapter Mode</label>
        <AlouetteSelect
          value={draftConfig.network_mode}
          options={networkOptions}
          onChange={(val) => onDraftChange({ ...draftConfig, network_mode: val })}
          searchable={false}
        />
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "6px", marginBottom: "10px" }}>
        <AlouetteCheckbox
          checked={draftConfig.advanced.tpm_enabled}
          onChange={(checked) =>
            onDraftChange({
              ...draftConfig,
              advanced: { ...draftConfig.advanced, tpm_enabled: checked },
            })
          }
          label="Enable TPM 2.0 (swtpm — Win11)"
        />
        <AlouetteCheckbox
          checked={draftConfig.advanced.audio_enabled}
          onChange={(checked) =>
            onDraftChange({
              ...draftConfig,
              advanced: { ...draftConfig.advanced, audio_enabled: checked },
            })
          }
          label="Enable Audio (PulseAudio / HDA)"
        />
      </div>

      <div style={{ marginBottom: "12px" }}>
        <label className="vm-label">TCP Port Forwarding (Host &rarr; Guest)</label>
        <div style={{ display: "flex", gap: "4px", marginBottom: "6px" }}>
          <input
            type="text"
            inputMode="numeric"
            placeholder="Host (e.g. 2222)"
            value={newHostPort}
            onChange={(e) => setNewHostPort(e.target.value.replace(/\D/g, ""))}
            className="vm-input"
          />
          <input
            type="text"
            inputMode="numeric"
            placeholder="Guest (e.g. 22)"
            value={newGuestPort}
            onChange={(e) => setNewGuestPort(e.target.value.replace(/\D/g, ""))}
            className="vm-input"
          />
          <button type="button" onClick={handleAddPortForward} className="vm-btn">
            <Plus size={12} />
          </button>
        </div>
        {draftConfig.advanced.host_port_forwards.map(([h, g], idx) => (
          <div
            key={`${h}-${g}-${idx}`}
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              backgroundColor: "var(--bg-tertiary)",
              padding: "3px 8px",
              border: "1px solid var(--border-primary)",
              marginBottom: "4px",
              fontSize: "11px",
              fontFamily: "var(--font-mono, monospace)",
            }}
          >
            <span>127.0.0.1:{h} &rarr; guest:{g}</span>
            <button
              type="button"
              onClick={() => handleRemovePortForward(idx)}
              style={{ background: "none", border: "none", color: "var(--color-danger)", cursor: "pointer", padding: 0 }}
            >
              <X size={12} />
            </button>
          </div>
        ))}
      </div>

      <h3 className="vm-section-title">Snapshots</h3>
      <div style={{ display: "flex", flexDirection: "column", gap: "6px", marginBottom: "12px" }}>
        <div style={{ display: "flex", gap: "4px" }}>
          <input
            type="text"
            value={newSnapshotName}
            onChange={(e) => setNewSnapshotName(e.target.value)}
            placeholder="Snapshot name (e.g. clean-boot)"
            className="vm-input"
          />
          <button type="button" onClick={handleCreateSnapshot} className="vm-btn">
            <Plus size={12} /> Create
          </button>
        </div>
        {snapshots.map((s) => (
          <div
            key={s}
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              backgroundColor: "var(--bg-tertiary)",
              padding: "4px 8px",
              border: "1px solid var(--border-primary)",
            }}
          >
            <span style={{ fontSize: "12px", fontWeight: 700 }}>{s}</span>
            <div style={{ display: "flex", gap: "4px" }}>
              <button
                type="button"
                onClick={() => handleRestoreSnapshot(s)}
                className="vm-btn vm-btn-success"
                style={{ padding: "2px 7px", fontSize: "11px" }}
              >
                {confirmRestoreSnap === s ? "Confirm?" : "Restore"}
              </button>
              <button
                type="button"
                onClick={() => handleDeleteSnapshot(s)}
                className="vm-btn vm-btn-danger"
                style={{ padding: "2px 6px" }}
                title="Delete snapshot"
              >
                <Trash2 size={12} />
              </button>
            </div>
          </div>
        ))}
        {snapshots.length === 0 && (
          <div className="vm-hint" style={{ fontStyle: "italic" }}>
            No snapshots exist for this VM.
          </div>
        )}
      </div>

      <h3 className="vm-section-title">Guest File Injection (QGA)</h3>
      <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
        <div style={{ display: "flex", gap: "4px" }}>
          <input
            type="text"
            value={injectHostPath}
            onChange={(e) => setInjectHostPath(e.target.value)}
            placeholder="Host file path"
            className="vm-input"
          />
          <button
            type="button"
            onClick={() => browseFile(setInjectHostPath)}
            className="vm-btn"
            title="Browse Host File"
          >
            <Folder size={12} />
          </button>
        </div>
        <input
          type="text"
          value={injectGuestPath}
          onChange={(e) => setInjectGuestPath(e.target.value)}
          placeholder="Guest destination (e.g. /root/file.txt)"
          className="vm-input"
        />
        <button
          type="button"
          onClick={handleInjectFile}
          disabled={vm.status !== "running"}
          className="vm-btn vm-btn-primary"
          style={{ justifyContent: "center" }}
        >
          <ArrowRight size={12} /> Inject File
        </button>
        <div className="vm-hint" style={{ fontStyle: "italic" }}>
          Requires <code>qemu-guest-agent</code> running inside the guest (max 10 MB).
        </div>
      </div>
    </div>
  );
}
