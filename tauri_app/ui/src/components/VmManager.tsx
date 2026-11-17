import { useCallback, useEffect, useState } from "react";
import { Pencil, Play, Power, Save, Square, Trash2, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import WindowResizer from "./WindowResizer";
import brandIcon from "./logo_alouette.png";
import { WindowControls } from "./WindowControls";
import { VmConsole } from "./vm/VmConsole";
import { VmCreateWizard } from "./vm/VmCreateWizard";
import { VmDetailsPanel } from "./vm/VmDetailsPanel";
import { VmSidebar } from "./vm/VmSidebar";
import { VmConfig, VmItem, VmNotice, VmRuntimeInfo } from "./vm/types";
import "./vm/VmManager.css";

export default function VmManager() {
  const [activeView, setActiveView] = useState<"create" | "manage">("create");
  const [selectedVmId, setSelectedVmId] = useState<string | null>(null);
  const [vms, setVms] = useState<VmItem[]>([]);
  const [draftConfig, setDraftConfig] = useState<VmConfig | null>(null);
  const [logs, setLogs] = useState<string>("");
  const [notice, setNotice] = useState<VmNotice | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);

  // Inline click-to-rename state in toolbar
  const [isRenamingToolbar, setIsRenamingToolbar] = useState(false);
  const [toolbarRenameValue, setToolbarRenameValue] = useState("");

  const showNotice = useCallback((next: VmNotice) => {
    setNotice(next);
  }, []);

  const refreshVms = useCallback(async () => {
    try {
      const list = await invoke<VmRuntimeInfo[]>("list_virtual_machines");
      setVms(
        list.map((item) => ({
          id: item.config.id,
          name: item.config.name,
          status: item.status,
          ws_port: item.ws_port,
          vnc_display: item.vnc_display,
          config: item.config,
        }))
      );
    } catch (err) {
      showNotice({ type: "error", message: `Failed to load VMs: ${err}` });
    }
  }, [showNotice]);

  useEffect(() => {
    refreshVms();
  }, [refreshVms]);

  const currentVm = vms.find((v) => v.id === selectedVmId) || null;

  useEffect(() => {
    if (currentVm) {
      setDraftConfig({
        ...currentVm.config,
        arch: currentVm.config.arch || "x86_64",
        advanced: {
          audio_enabled: currentVm.config.advanced?.audio_enabled ?? false,
          tpm_enabled: currentVm.config.advanced?.tpm_enabled ?? false,
          graceful_shutdown_timeout: currentVm.config.advanced?.graceful_shutdown_timeout ?? 30,
          host_port_forwards: currentVm.config.advanced?.host_port_forwards ?? [],
          additional_disks: currentVm.config.advanced?.additional_disks ?? [],
        },
      });
    } else {
      setDraftConfig(null);
    }
    setConfirmDeleteId(null);
    setIsRenamingToolbar(false);
  }, [currentVm?.id]);

  useEffect(() => {
    if (activeView !== "manage" || !selectedVmId) return;

    const fetchLogs = async () => {
      try {
        const text = await invoke<string>("get_virtual_machine_logs", { id: selectedVmId });
        setLogs(text);
      } catch {
        // ignore transient polling failures
      }
    };

    fetchLogs();

    if (currentVm?.status !== "running") return;

    const logTimer = setInterval(fetchLogs, 1500);
    const statusTimer = setInterval(refreshVms, 3000);
    return () => {
      clearInterval(logTimer);
      clearInterval(statusTimer);
    };
  }, [activeView, selectedVmId, currentVm?.status, refreshVms]);

  const handleRenameVm = async (id: string, rawName: string) => {
    const newName = rawName.trim();
    const target = vms.find((v) => v.id === id);
    if (!target || !newName || newName === target.name) return;

    try {
      const base = draftConfig && draftConfig.id === id ? draftConfig : target.config;
      const updated = await invoke<VmConfig>("save_virtual_machine", {
        config: { ...base, name: newName },
      });
      if (draftConfig && draftConfig.id === id) {
        setDraftConfig(updated);
      }
      await refreshVms();
      showNotice({ type: "success", message: `Renamed VM to "${updated.name}".` });
    } catch (err) {
      showNotice({ type: "error", message: `Failed to rename VM: ${err}` });
    }
  };

  const commitToolbarRename = async () => {
    const vm = currentVm;
    setIsRenamingToolbar(false);
    if (!vm) return;
    await handleRenameVm(vm.id, toolbarRenameValue);
  };

  const handleStartVm = async (id: string) => {
    setBusyAction("start");
    setLogs("--- Starting VM ---");
    try {
      await invoke("start_virtual_machine", { id });
      await refreshVms();
      showNotice({ type: "success", message: "VM started." });
    } catch (err) {
      showNotice({ type: "error", message: `Failed to start VM: ${err}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleShutdownVm = async (id: string) => {
    setBusyAction("shutdown");
    try {
      await invoke("shutdown_virtual_machine", { id });
      showNotice({ type: "info", message: "ACPI shutdown signal sent to guest." });
      setTimeout(refreshVms, 2000);
    } catch (err) {
      showNotice({ type: "error", message: `ACPI shutdown failed: ${err}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleStopVm = async (id: string) => {
    setBusyAction("stop");
    try {
      await invoke("stop_virtual_machine", { id });
      await refreshVms();
      showNotice({ type: "info", message: "VM process stopped." });
    } catch (err) {
      showNotice({ type: "error", message: `Failed to stop VM: ${err}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleApplyConfig = async () => {
    if (!draftConfig) return;
    setBusyAction("save");
    try {
      const saved = await invoke<VmConfig>("save_virtual_machine", { config: draftConfig });
      setDraftConfig(saved);
      await refreshVms();
      showNotice({
        type: "success",
        message:
          currentVm?.status === "running"
            ? "Configuration saved (restart VM to apply hardware changes)."
            : "Configuration saved.",
      });
    } catch (err) {
      showNotice({ type: "error", message: `Failed to save config: ${err}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleDeleteVm = async (id: string) => {
    if (confirmDeleteId !== id) {
      setConfirmDeleteId(id);
      return;
    }
    setBusyAction("delete");
    try {
      await invoke("delete_virtual_machine", { id });
      setSelectedVmId(null);
      setActiveView("create");
      setConfirmDeleteId(null);
      await refreshVms();
      showNotice({ type: "info", message: "VM deleted." });
    } catch (err) {
      showNotice({ type: "error", message: `Failed to delete VM: ${err}` });
    } finally {
      setBusyAction(null);
    }
  };

  return (
    <div className="vm-root">
      <WindowResizer />

      <div data-tauri-drag-region className="vm-titlebar">
        <div className="vm-titlebar-brand">
          <img src={brandIcon} alt="Logo" style={{ width: "14px", height: "14px" }} />
          <span>Alouette VMM</span>
        </div>
        <WindowControls />
      </div>

      <div className="vm-layout">
        <VmSidebar
          vms={vms}
          activeView={activeView}
          selectedVmId={selectedVmId}
          onCreateClick={() => {
            setActiveView("create");
            setSelectedVmId(null);
          }}
          onSelectVm={(id) => {
            setActiveView("manage");
            setSelectedVmId(id);
          }}
          onRenameVm={handleRenameVm}
        />

        <div className="vm-main">
          {notice && (
            <div className={`vm-notice ${notice.type}`}>
              <span>{notice.message}</span>
              <button
                type="button"
                onClick={() => setNotice(null)}
                style={{ background: "none", border: "none", color: "inherit", cursor: "pointer", padding: 0 }}
              >
                <X size={14} />
              </button>
            </div>
          )}

          {activeView === "manage" && currentVm && draftConfig ? (
            <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
              <div className="vm-toolbar">
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  {isRenamingToolbar ? (
                    <input
                      type="text"
                      value={toolbarRenameValue}
                      autoFocus
                      onFocus={(e) => e.target.select()}
                      onChange={(e) => setToolbarRenameValue(e.target.value)}
                      onBlur={commitToolbarRename}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitToolbarRename();
                        if (e.key === "Escape") setIsRenamingToolbar(false);
                      }}
                      className="vm-input"
                      style={{ width: "200px", fontSize: "13px", fontWeight: 700, padding: "3px 6px" }}
                    />
                  ) : (
                    <span
                      className="vm-rename-trigger"
                      title="Click to rename VM"
                      onClick={() => {
                        setToolbarRenameValue(currentVm.name);
                        setIsRenamingToolbar(true);
                      }}
                    >
                      <span>{currentVm.name}</span>
                      <Pencil size={12} className="vm-rename-icon" />
                    </span>
                  )}

                  {currentVm.status === "running" ? (
                    <>
                      <button
                        type="button"
                        disabled={busyAction !== null}
                        onClick={() => handleShutdownVm(currentVm.id)}
                        className="vm-btn vm-btn-warning"
                        title="Send graceful ACPI powerdown signal"
                      >
                        <Power size={12} /> Shutdown
                      </button>
                      <button
                        type="button"
                        disabled={busyAction !== null}
                        onClick={() => handleStopVm(currentVm.id)}
                        className="vm-btn vm-btn-danger"
                        title="Force kill QEMU process"
                      >
                        <Square size={12} fill="currentColor" /> Force Stop
                      </button>
                    </>
                  ) : (
                    <button
                      type="button"
                      disabled={busyAction !== null}
                      onClick={() => handleStartVm(currentVm.id)}
                      className="vm-btn vm-btn-success"
                    >
                      <Play size={12} fill="currentColor" /> Start VM
                    </button>
                  )}
                </div>

                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <button
                    type="button"
                    disabled={busyAction !== null}
                    onClick={handleApplyConfig}
                    className="vm-btn vm-btn-primary"
                  >
                    <Save size={12} /> Save Config
                  </button>
                  <button
                    type="button"
                    disabled={busyAction !== null}
                    onClick={() => handleDeleteVm(currentVm.id)}
                    className="vm-btn vm-btn-danger"
                  >
                    <Trash2 size={12} /> {confirmDeleteId === currentVm.id ? "Confirm Delete?" : "Delete"}
                  </button>
                </div>
              </div>

              <div className="vm-manage-body">
                <VmDetailsPanel
                  vm={currentVm}
                  draftConfig={draftConfig}
                  onDraftChange={setDraftConfig}
                  onNotice={showNotice}
                />
                <VmConsole vm={currentVm} logs={logs} onClearLogs={() => setLogs("")} />
              </div>
            </div>
          ) : (
            <VmCreateWizard
              onNotice={showNotice}
              onCreated={async (created) => {
                await refreshVms();
                setSelectedVmId(created.id);
                setActiveView("manage");
              }}
            />
          )}
        </div>
      </div>
    </div>
  );
}
