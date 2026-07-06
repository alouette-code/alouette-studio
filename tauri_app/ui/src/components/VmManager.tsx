import React, { useState, useEffect, useRef } from "react";
import { Play, Square, FolderOpen, Plus, Save, Terminal, Trash2, Monitor, Folder, ArrowRight, Check } from "lucide-react";
// @ts-ignore
import RFB from '@novnc/novnc';
import { invoke } from "@tauri-apps/api/core";
import WindowResizer from "./WindowResizer";
import brandIcon from "./logo_alouette.png";
import { WindowControls } from "./WindowControls";

interface VM {
  id: string;
  name: string;
  status: string;
  config: {
    id: string;
    name: string;
    cpu_cores: number;
    ram_size_mb: number;
    vm_dir: string;
    iso_path: string | null;
    disk_path: string | null;
    network_mode: string;
  };
}

const OS_FAMILIES = {
  windows: {
    label: "Windows",
    versions: [
      { id: "win11", label: "Windows 11 (x64/ARM64)", desc: "UEFI, 4 Cores, 4GB RAM, 60GB Disk", conf: { cpu: 4, ram: 4, disk: 60, fw: "uefi" } },
      { id: "win10", label: "Windows 10 (x64)", desc: "BIOS, 2 Cores, 4GB RAM, 50GB Disk", conf: { cpu: 2, ram: 4, disk: 50, fw: "bios" } },
      { id: "winserver", label: "Windows Server 2022", desc: "UEFI, 4 Cores, 8GB RAM, 80GB Disk", conf: { cpu: 4, ram: 8, disk: 80, fw: "uefi" } },
      { id: "win7", label: "Windows 7 / 8.1 (x64)", desc: "BIOS, 2 Cores, 2GB RAM, 32GB Disk", conf: { cpu: 2, ram: 2, disk: 32, fw: "bios" } }
    ]
  },
  linux: {
    label: "Linux",
    versions: [
      { id: "ubuntu", label: "Ubuntu Desktop (x86_64)", desc: "BIOS, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ram: 2, disk: 25, fw: "bios" } },
      { id: "ubuntu_arm", label: "Ubuntu Server (ARM64)", desc: "UEFI, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ram: 2, disk: 25, fw: "uefi" } },
      { id: "debian", label: "Debian GNU/Linux (x86_64)", desc: "BIOS, 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ram: 2, disk: 20, fw: "bios" } },
      { id: "fedora", label: "Fedora Workstation (x86_64)", desc: "UEFI, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ram: 4, disk: 30, fw: "uefi" } },
      { id: "rhel", label: "RHEL / CentOS / AlmaLinux", desc: "BIOS, 2 Cores, 2GB RAM, 40GB Disk", conf: { cpu: 2, ram: 2, disk: 40, fw: "bios" } },
      { id: "arch", label: "Arch Linux (x86_64)", desc: "BIOS, 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ram: 2, disk: 20, fw: "bios" } },
      { id: "kali", label: "Kali Linux (x86_64)", desc: "BIOS, 2 Cores, 4GB RAM, 40GB Disk", conf: { cpu: 2, ram: 4, disk: 40, fw: "bios" } },
      { id: "alpine", label: "Alpine Linux (x86_64)", desc: "BIOS, 1 Core, 1GB RAM, 5GB Disk", conf: { cpu: 1, ram: 1, disk: 5, fw: "bios" } }
    ]
  },
  macos: {
    label: "macOS",
    versions: [
      { id: "macos_sequoia", label: "macOS 15 Sequoia (x86_64)", desc: "UEFI, 4 Cores, 8GB RAM, 80GB Disk", conf: { cpu: 4, ram: 8, disk: 80, fw: "uefi" } },
      { id: "macos_sonoma", label: "macOS 14 Sonoma (x86_64)", desc: "UEFI, 4 Cores, 8GB RAM, 80GB Disk", conf: { cpu: 4, ram: 8, disk: 80, fw: "uefi" } },
      { id: "macos_ventura", label: "macOS 13 Ventura (x86_64)", desc: "UEFI, 4 Cores, 4GB RAM, 60GB Disk", conf: { cpu: 4, ram: 4, disk: 60, fw: "uefi" } },
      { id: "macos_monterey", label: "macOS 12 Monterey (x86_64)", desc: "UEFI, 4 Cores, 4GB RAM, 60GB Disk", conf: { cpu: 4, ram: 4, disk: 60, fw: "uefi" } }
    ]
  },
  android: {
    label: "Android",
    versions: [
      { id: "android_11", label: "Android-x86 (11.0)", desc: "BIOS, 2 Cores, 2GB RAM, 16GB Disk", conf: { cpu: 2, ram: 2, disk: 16, fw: "bios" } },
      { id: "android_9", label: "Android-x86 (9.0 Pie)", desc: "BIOS, 2 Cores, 2GB RAM, 16GB Disk", conf: { cpu: 2, ram: 2, disk: 16, fw: "bios" } },
      { id: "bliss_os", label: "Bliss OS (Android 12/13)", desc: "UEFI, 4 Cores, 4GB RAM, 32GB Disk", conf: { cpu: 4, ram: 4, disk: 32, fw: "uefi" } },
      { id: "prime_os", label: "PrimeOS (x86_64)", desc: "BIOS, 2 Cores, 4GB RAM, 32GB Disk", conf: { cpu: 2, ram: 4, disk: 32, fw: "bios" } }
    ]
  },
  other: {
    label: "Other",
    versions: [
      { id: "custom", label: "Custom / Unknown OS", desc: "Configure everything manually", conf: { cpu: 1, ram: 1, disk: 20, fw: "bios" } },
      { id: "freebsd", label: "FreeBSD / OpenBSD (x64)", desc: "BIOS, 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ram: 2, disk: 20, fw: "bios" } }
    ]
  }
};

export default function VmManager() {




  // App State
  const [activeView, setActiveView] = useState<"create" | "manage">("create");
  const [selectedVmId, setSelectedVmId] = useState<string | null>(null);
  const [existingVms, setExistingVms] = useState<VM[]>([]);
  const [logs, setLogs] = useState<string>("");
  const [snapshots, setSnapshots] = useState<string[]>([]);
  const [newSnapshotName, setNewSnapshotName] = useState("");
  const [injectHostPath, setInjectHostPath] = useState("");
  const [injectGuestPath, setInjectGuestPath] = useState("");
  const logEndRef = useRef<HTMLDivElement>(null);
  
  // VNC State
  const [consoleTab, setConsoleTab] = useState<"vnc" | "logs">("vnc");
  const vncContainerRef = useRef<HTMLDivElement>(null);
  const rfbRef = useRef<any>(null);

  const refreshVms = async () => {
    try {
      const list = await invoke<any[]>("list_virtual_machines");
      setExistingVms(list.map(item => ({
        id: item.config.id,
        name: item.config.name,
        status: item.status,
        config: item.config
      })));
    } catch (err) {
      console.error("Failed to load virtual machines:", err);
    }
  };

  useEffect(() => {
    refreshVms();
  }, []);

  // Poll VM status & logs when running
  useEffect(() => {
    if (activeView === "manage" && selectedVmId) {
      const selected = existingVms.find(v => v.id === selectedVmId);
      if (selected && selected.status === "running") {
        const interval = setInterval(async () => {
          try {
            const currentLogs = await invoke<string>("get_virtual_machine_logs", { id: selectedVmId });
            setLogs(currentLogs);
            // Auto-scroll
            if (logEndRef.current) {
              logEndRef.current.scrollIntoView({ behavior: "smooth" });
            }
          } catch {}
        }, 1000);
        return () => clearInterval(interval);
      } else {
        setLogs("--- VM is stopped ---");
      }
    }
    return undefined;
  }, [activeView, selectedVmId, existingVms]);

  const currentVm = existingVms.find(v => v.id === selectedVmId);

  // Manage VNC Connection
  useEffect(() => {
    let connectTimeout: any;

    if (activeView === "manage" && currentVm && currentVm.status === "running" && consoleTab === "vnc") {
      // Add a delay to ensure QEMU's VNC server has bound to the port before connecting
      connectTimeout = setTimeout(() => {
        if (!rfbRef.current && vncContainerRef.current) {
          try {
            rfbRef.current = new RFB(vncContainerRef.current, "ws://127.0.0.1:5700");
            rfbRef.current.scaleViewport = true;
            rfbRef.current.resizeSession = true;
          } catch (e) {
            console.error("VNC Connection failed:", e);
          }
        }
      }, 1000);
    } else {
      if (rfbRef.current) {
        try { rfbRef.current.disconnect(); } catch (e) {}
        rfbRef.current = null;
      }
    }
    return () => {
      clearTimeout(connectTimeout);
      if (rfbRef.current) {
        try { rfbRef.current.disconnect(); } catch (e) {}
        rfbRef.current = null;
      }
    };
  }, [activeView, currentVm?.status, consoleTab]);

  // --- Creation Form State ---
  const [wizardStep, setWizardStep] = useState<1 | 2 | 3 | 4>(1);
  const [osFamily, setOsFamily] = useState<string>("linux");
  const [osType, setOsType] = useState<string>("ubuntu");
  const [vmName, setVmName] = useState("New-VM");
  const [vmDir, setVmDir] = useState("");
  const [cpuCores, setCpuCores] = useState(2);
  const [ramLimit, setRamLimit] = useState(2);
  const [networkType, setNetworkType] = useState("nat");
  const [isoPath, setIsoPath] = useState("");
  const [diskPath] = useState("");
  const [diskSizeGb, setDiskSizeGb] = useState<number>(25);
  const [firmware, setFirmware] = useState<"bios" | "uefi">("bios");

  const handleFamilyChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const family = e.target.value;
    setOsFamily(family);
    // @ts-ignore
    const firstVersion = OS_FAMILIES[family].versions[0];
    if (firstVersion) {
      applyOsTemplate(family, firstVersion.id);
    }
  };

  const applyOsTemplate = (family: string, versionId: string) => {
    setOsType(versionId);
    // @ts-ignore
    const ver = OS_FAMILIES[family]?.versions.find(v => v.id === versionId);
    if (ver) {
      setCpuCores(ver.conf.cpu);
      setRamLimit(ver.conf.ram);
      setDiskSizeGb(ver.conf.disk);
      setFirmware(ver.conf.fw as any);
    }
  };

  // --- Manage Form State ---
  const [editCpu, setEditCpu] = useState(1);
  const [editRam, setEditRam] = useState(1);
  const [editNetType, setEditNetType] = useState("nat");
  const [editIso, setEditIso] = useState("");
  const [editDisk, setEditDisk] = useState("");

  const refreshSnapshots = async (vmId: string) => {
    try {
      const list = await invoke<string[]>("list_vm_snapshots", { id: vmId });
      setSnapshots(list || []);
    } catch (err) {
      console.error("Failed to load snapshots:", err);
      setSnapshots([]);
    }
  };

  // Load configuration into manage tab when a VM is selected
  useEffect(() => {
    if (selectedVmId) {
      const selected = existingVms.find(v => v.id === selectedVmId);
      if (selected) {
        setEditCpu(selected.config.cpu_cores);
        setEditRam(Math.round(selected.config.ram_size_mb / 1024));
        setEditNetType(selected.config.network_mode);
        setEditIso(selected.config.iso_path || "");
        setEditDisk(selected.config.disk_path || "");
        refreshSnapshots(selectedVmId);
      }
    } else {
      setSnapshots([]);
    }
  }, [selectedVmId, existingVms]);

  const handleCreateVm = async () => {
    try {
      const config = {
        id: "",
        name: vmName,
        os_type: osType,
        cpu_cores: cpuCores,
        ram_size_mb: ramLimit * 1024,
        vm_dir: vmDir,
        iso_path: isoPath ? isoPath : null,
        disk_path: diskPath ? diskPath : null,
        disk_size_gb: diskSizeGb,
        network_mode: networkType,
        firmware: firmware
      };
      await invoke("save_virtual_machine", { config });
      alert("VM Created Successfully!");
      setActiveView("manage");
      await refreshVms();
    } catch (err) {
      alert("Failed to create VM: " + err);
    }
  };

  const handleStartVm = async (id: string) => {
    try {
      setLogs("--- Starting VM ---");
      await invoke("start_virtual_machine", { id });
      await refreshVms();
    } catch (err) {
      alert("Failed to start VM: " + err);
    }
  };

  const handleStopVm = async (id: string) => {
    try {
      await invoke("stop_virtual_machine", { id });
      await refreshVms();
    } catch (err) {
      alert("Failed to stop VM: " + err);
    }
  };

  const handleDeleteVm = async (id: string) => {
    if (confirm("Are you sure you want to delete this VM?")) {
      try {
        await invoke("delete_virtual_machine", { id });
        setSelectedVmId(null);
        setActiveView("create");
        await refreshVms();
      } catch (err) {
        alert("Failed to delete VM: " + err);
      }
    }
  };

  const handleApplyConfig = async () => {
    if (!selectedVmId) return;
    const selected = existingVms.find(v => v.id === selectedVmId);
    if (!selected) return;

    try {
      const updatedConfig = {
        ...selected.config,
        cpu_cores: editCpu,
        ram_size_mb: editRam * 1024,
        network_mode: editNetType,
        iso_path: editIso ? editIso : null,
        disk_path: editDisk ? editDisk : null,
      };
      await invoke("save_virtual_machine", { config: updatedConfig });
      alert("Configuration Saved!");
      await refreshVms();
    } catch (err) {
      alert("Failed to save config: " + err);
    }
  };

  const handleCreateSnapshot = async () => {
    if (!selectedVmId || !newSnapshotName.trim()) return;
    try {
      await invoke("create_vm_snapshot", { id: selectedVmId, name: newSnapshotName.trim() });
      setNewSnapshotName("");
      await refreshSnapshots(selectedVmId);
    } catch (err) {
      alert("Failed to create snapshot: " + err);
    }
  };

  const handleRestoreSnapshot = async (name: string) => {
    if (!selectedVmId) return;
    if (!confirm(`Are you sure you want to restore snapshot '${name}'? This will discard current state.`)) return;
    try {
      await invoke("restore_vm_snapshot", { id: selectedVmId, name });
      alert(`Snapshot '${name}' restored successfully!`);
    } catch (err) {
      alert("Failed to restore snapshot: " + err);
    }
  };

  const handleDeleteSnapshot = async (name: string) => {
    if (!selectedVmId) return;
    if (!confirm(`Are you sure you want to delete snapshot '${name}'?`)) return;
    try {
      await invoke("delete_vm_snapshot", { id: selectedVmId, name });
      await refreshSnapshots(selectedVmId);
    } catch (err) {
      alert("Failed to delete snapshot: " + err);
    }
  };

  const handleInjectFile = async () => {
    if (!selectedVmId || !injectHostPath || !injectGuestPath) {
      alert("Please specify both Host Path and Guest Destination.");
      return;
    }
    try {
      await invoke("inject_guest_file", { 
        id: selectedVmId, 
        hostPath: injectHostPath, 
        guestPath: injectGuestPath 
      });
      alert("File injected successfully!");
      setInjectHostPath("");
      setInjectGuestPath("");
    } catch (err) {
      alert("Failed to inject file: " + err);
    }
  };

  const handleBrowseFile = async (setter: (val: string) => void) => {
    try {
      const path = await invoke<string | null>("open_file_dialog");
      if (path) setter(path);
    } catch {}
  };

  const handleBrowseFolder = async (setter: (val: string) => void) => {
    try {
      const path = await invoke<string | null>("open_folder_dialog");
      if (path) setter(path);
    } catch {}
  };

  // Technical UI Theme Constants
  const THEME = {
    bgApp: "#1e1e1e",
    bgPanel: "#252526",
    bgInput: "#3c3c3c",
    bgHover: "#2a2d2e",
    border: "#3c3c3c",
    borderFocus: "#007acc",
    textMain: "#cccccc",
    textMuted: "#858585",
    accent: "#007acc",
    accentHover: "#005a9e",
    success: "#89d185",
    error: "#f48771",
  };

  const inputStyle: React.CSSProperties = {
    padding: "4px 8px",
    backgroundColor: THEME.bgInput,
    border: `1px solid ${THEME.border}`,
    color: THEME.textMain,
    outline: "none",
    fontSize: "12px",
    fontFamily: "monospace",
    width: "100%",
    boxSizing: "border-box"
  };

  const labelStyle: React.CSSProperties = {
    display: "block",
    fontSize: "11px",
    fontWeight: "bold",
    color: THEME.textMuted,
    marginBottom: "4px",
    textTransform: "uppercase",
  };

  const btnStyle: React.CSSProperties = {
    padding: "4px 12px",
    backgroundColor: THEME.bgInput,
    border: `1px solid ${THEME.border}`,
    color: THEME.textMain,
    cursor: "pointer",
    fontSize: "12px",
    display: "flex",
    alignItems: "center",
    gap: "6px"
  };

  const btnPrimaryStyle: React.CSSProperties = {
    ...btnStyle,
    backgroundColor: THEME.accent,
    border: `1px solid ${THEME.accentHover}`,
    color: "#fff",
    fontWeight: "bold"
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', backgroundColor: THEME.bgApp, color: THEME.textMain, fontFamily: "sans-serif", overflow: "hidden" }}>
      <WindowResizer />
      
      {/* Titlebar */}
      <div data-tauri-drag-region style={{ height: "30px", display: "flex", alignItems: "center", justifyContent: "space-between", backgroundColor: THEME.bgApp, borderBottom: `1px solid ${THEME.border}`, padding: "0 8px", userSelect: "none" }}>
        <div style={{ display: "flex", alignItems: "center", pointerEvents: "none" }}>
          <img src={brandIcon} alt="Logo" style={{ width: "14px", height: "14px", marginRight: "8px" }} />
          <span style={{ fontSize: "12px", fontWeight: "bold" }}>Alouette VMM</span>
        </div>
        <WindowControls />
      </div>
      
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        
        {/* Left Sidebar */}
        <div style={{ width: '240px', backgroundColor: THEME.bgPanel, borderRight: `1px solid ${THEME.border}`, display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '8px' }}>
            <button 
              onClick={() => { setActiveView("create"); setSelectedVmId(null); }}
              style={{ ...btnStyle, width: '100%', justifyContent: 'center', backgroundColor: THEME.bgApp }}
            >
              <Plus size={14} /> Create VM
            </button>
          </div>
          <div style={{ padding: '4px 8px', fontSize: '10px', fontWeight: 'bold', color: THEME.textMuted, borderBottom: `1px solid ${THEME.border}` }}>VIRTUAL MACHINES</div>
          <div style={{ flex: 1, overflowY: 'auto' }}>
            {existingVms.map(vm => (
              <div 
                key={vm.id}
                onClick={() => { setActiveView("manage"); setSelectedVmId(vm.id); }}
                style={{ 
                  padding: '6px 8px', 
                  cursor: 'pointer', 
                  display: 'flex', 
                  alignItems: 'center', 
                  justifyContent: 'space-between',
                  backgroundColor: (activeView === "manage" && selectedVmId === vm.id) ? THEME.bgHover : 'transparent',
                  borderLeft: (activeView === "manage" && selectedVmId === vm.id) ? `3px solid ${THEME.accent}` : '3px solid transparent'
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden' }}>
                  <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: vm.status === 'running' ? THEME.success : THEME.textMuted }} />
                  <span style={{ fontSize: '12px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{vm.name}</span>
                </div>
              </div>
            ))}
            {existingVms.length === 0 && (
              <div style={{ padding: '12px', fontSize: '11px', color: THEME.textMuted, textAlign: 'center' }}>No VMs Found.</div>
            )}
          </div>
        </div>

        {/* Main Content Area */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', backgroundColor: THEME.bgApp }}>
          
          {activeView === "manage" && currentVm ? (
            <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
              
              {/* Manage Toolbar */}
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 12px', backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}` }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <span style={{ fontSize: '14px', fontWeight: 'bold', marginRight: '16px' }}>{currentVm.name}</span>
                  {currentVm.status === 'running' ? (
                    <button onClick={() => handleStopVm(currentVm.id)} style={{ ...btnStyle, color: THEME.error, borderColor: THEME.error }}>
                      <Square size={12} fill="currentColor" /> Stop VM (Kill)
                    </button>
                  ) : (
                    <button onClick={() => handleStartVm(currentVm.id)} style={{ ...btnStyle, color: THEME.success, borderColor: THEME.success }}>
                      <Play size={12} fill="currentColor" /> Start VM
                    </button>
                  )}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <button onClick={handleApplyConfig} style={btnPrimaryStyle}><Save size={12} /> Save Config</button>
                  <button onClick={() => handleDeleteVm(currentVm.id)} style={{ ...btnStyle, color: THEME.error }}><Trash2 size={12} /> Delete</button>
                </div>
              </div>

              <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
                
                {/* Configuration Panel */}
                <div style={{ width: '400px', padding: '16px', overflowY: 'auto', borderRight: `1px solid ${THEME.border}` }}>
                  <h3 style={{ fontSize: '13px', borderBottom: `1px solid ${THEME.border}`, paddingBottom: '4px', marginBottom: '12px', color: THEME.accent }}>Hardware & Location</h3>
                  <div style={{ marginBottom: '12px' }}>
                    <label style={labelStyle}>VM Directory</label>
                    <input type="text" value={currentVm.config.vm_dir} readOnly style={{ ...inputStyle, backgroundColor: THEME.bgApp, color: THEME.textMuted }} />
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', marginBottom: '16px' }}>
                    <div>
                      <label style={labelStyle}>CPU Cores</label>
                      <input type="number" min="1" max="64" value={editCpu} onChange={(e) => setEditCpu(Number(e.target.value))} style={inputStyle} />
                    </div>
                    <div>
                      <label style={labelStyle}>RAM (GB)</label>
                      <input type="number" min="1" max="128" value={editRam} onChange={(e) => setEditRam(Number(e.target.value))} style={inputStyle} />
                    </div>
                  </div>

                  <h3 style={{ fontSize: '13px', borderBottom: `1px solid ${THEME.border}`, paddingBottom: '4px', marginBottom: '12px', color: THEME.accent }}>Storage</h3>
                  <div style={{ marginBottom: '12px' }}>
                    <label style={labelStyle}>Disk Image (.qcow2 / .img)</label>
                    <div style={{ display: 'flex', gap: '4px' }}>
                      <input type="text" value={editDisk} onChange={(e) => setEditDisk(e.target.value)} style={inputStyle} placeholder="Auto-created if empty" />
                      <button onClick={() => handleBrowseFile(setEditDisk)} style={btnStyle}><FolderOpen size={12} /></button>
                    </div>
                  </div>
                  <div style={{ marginBottom: '16px' }}>
                    <label style={labelStyle}>ISO Image (CD-ROM)</label>
                    <div style={{ display: 'flex', gap: '4px' }}>
                      <input type="text" value={editIso} onChange={(e) => setEditIso(e.target.value)} style={inputStyle} placeholder="Leave empty if not booting ISO" />
                      <button onClick={() => handleBrowseFile(setEditIso)} style={btnStyle}><FolderOpen size={12} /></button>
                    </div>
                  </div>

                  <h3 style={{ fontSize: '13px', borderBottom: `1px solid ${THEME.border}`, paddingBottom: '4px', marginBottom: '12px', color: THEME.accent }}>Network</h3>
                  <div>
                    <label style={labelStyle}>Adapter Type</label>
                    <select value={editNetType} onChange={(e) => setEditNetType(e.target.value)} style={inputStyle}>
                      <option value="nat">NAT (User Mode)</option>
                      <option value="bridged">Bridged Adapter</option>
                      <option value="host-only">Host-Only Network</option>
                    </select>
                  </div>

                  <h3 style={{ fontSize: '13px', borderBottom: `1px solid ${THEME.border}`, paddingBottom: '4px', marginBottom: '12px', color: THEME.accent, marginTop: '24px' }}>Snapshots</h3>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '24px' }}>
                    <div style={{ display: 'flex', gap: '4px' }}>
                      <input type="text" value={newSnapshotName} onChange={e => setNewSnapshotName(e.target.value)} placeholder="Snapshot Name" style={inputStyle} />
                      <button onClick={handleCreateSnapshot} style={btnStyle}><Plus size={12}/> Create</button>
                    </div>
                    {snapshots.map(s => (
                       <div key={s} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', backgroundColor: THEME.bgHover, padding: '4px 8px', border: `1px solid ${THEME.border}` }}>
                          <span style={{ fontSize: '12px', fontWeight: 'bold' }}>{s}</span>
                          <div style={{ display: 'flex', gap: '4px' }}>
                             <button onClick={() => handleRestoreSnapshot(s)} style={{...btnStyle, color: THEME.success}} title="Restore">Restore</button>
                             <button onClick={() => handleDeleteSnapshot(s)} style={{...btnStyle, color: THEME.error}} title="Delete"><Trash2 size={12}/></button>
                          </div>
                       </div>
                    ))}
                    {snapshots.length === 0 && <div style={{ fontSize: '11px', color: THEME.textMuted, fontStyle: 'italic' }}>No snapshots exist for this VM.</div>}
                  </div>

                  <h3 style={{ fontSize: '13px', borderBottom: `1px solid ${THEME.border}`, paddingBottom: '4px', marginBottom: '12px', color: THEME.accent, marginTop: '24px' }}>Guest File Injection (VMware Tools)</h3>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '24px' }}>
                    <div style={{ display: 'flex', gap: '4px' }}>
                      <input type="text" value={injectHostPath} onChange={e => setInjectHostPath(e.target.value)} placeholder="Host File Path (Select)" style={{...inputStyle, flex: 1}} />
                      <button onClick={() => handleBrowseFile(setInjectHostPath)} style={{...btnStyle, padding: '4px'}} title="Browse Host File"><Folder size={12}/></button>
                    </div>
                    <div style={{ display: 'flex', gap: '4px' }}>
                      <input type="text" value={injectGuestPath} onChange={e => setInjectGuestPath(e.target.value)} placeholder="Guest Destination (e.g., /root/file.txt)" style={{...inputStyle, flex: 1}} />
                    </div>
                    <button onClick={handleInjectFile} style={{...btnStyle, backgroundColor: THEME.accent, color: THEME.bgApp, fontWeight: 'bold'}}><ArrowRight size={12}/> Inject File</button>
                    <div style={{ fontSize: '11px', color: THEME.textMuted, fontStyle: 'italic', marginTop: '4px', lineHeight: 1.4 }}>
                      Requires <code>qemu-guest-agent</code> installed and running inside the VM.<br/>
                      e.g., Alpine: <code>apk add qemu-guest-agent && rc-service qemu-guest-agent start</code>
                    </div>
                  </div>
                </div>

                {/* Console Panel */}
                <div style={{ flex: 1, display: 'flex', flexDirection: 'column', backgroundColor: '#000' }}>
                  <div style={{ display: 'flex', backgroundColor: THEME.bgPanel, borderBottom: `1px solid ${THEME.border}` }}>
                    <button 
                      onClick={() => setConsoleTab("vnc")}
                      style={{ padding: '6px 12px', background: consoleTab === "vnc" ? THEME.bgApp : 'transparent', border: 'none', borderBottom: consoleTab === "vnc" ? `2px solid ${THEME.accent}` : '2px solid transparent', color: consoleTab === "vnc" ? THEME.accent : THEME.textMuted, fontSize: '11px', fontWeight: 'bold', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '6px' }}
                    >
                      <Monitor size={12} /> VM Display (VNC)
                    </button>
                    <button 
                      onClick={() => setConsoleTab("logs")}
                      style={{ padding: '6px 12px', background: consoleTab === "logs" ? THEME.bgApp : 'transparent', border: 'none', borderBottom: consoleTab === "logs" ? `2px solid ${THEME.accent}` : '2px solid transparent', color: consoleTab === "logs" ? THEME.accent : THEME.textMuted, fontSize: '11px', fontWeight: 'bold', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '6px' }}
                    >
                      <Terminal size={12} /> Serial Console
                    </button>
                    {consoleTab === "logs" && (
                      <button onClick={() => setLogs("")} style={{ marginLeft: 'auto', background: 'none', border: 'none', color: THEME.textMuted, fontSize: '11px', cursor: 'pointer', padding: '0 12px' }}>Clear</button>
                    )}
                  </div>
                  
                  {consoleTab === "vnc" ? (
                    <div style={{ flex: 1, position: 'relative', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center', backgroundColor: '#111' }}>
                      {currentVm.status === "running" ? (
                        <div ref={vncContainerRef} style={{ width: '100%', height: '100%' }} />
                      ) : (
                        <div style={{ color: THEME.textMuted, fontSize: '12px' }}>VM is not running</div>
                      )}
                    </div>
                  ) : (
                    <div style={{ flex: 1, padding: '8px', overflowY: 'auto', fontFamily: 'monospace', fontSize: '12px', color: '#00ff00', whiteSpace: 'pre-wrap' }}>
                      {logs}
                      <div ref={logEndRef} />
                    </div>
                  )}
                </div>

              </div>

            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '32px', alignItems: 'center', overflowY: 'auto' }}>
              
              <div style={{ width: '100%', maxWidth: '600px' }}>
                <h2 style={{ fontSize: '20px', marginBottom: '8px', color: THEME.textMain, display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <Plus size={20} color={THEME.accent} /> Create New Virtual Machine
                </h2>
                <p style={{ fontSize: '12px', color: THEME.textMuted, marginBottom: '24px' }}>Follow the wizard to set up your new environment.</p>
              
                <div style={{ backgroundColor: THEME.bgPanel, border: `1px solid ${THEME.border}`, borderRadius: '4px', overflow: 'hidden' }}>
                  
                  {/* Wizard Header */}
                  <div style={{ display: 'flex', backgroundColor: THEME.bgHover, borderBottom: `1px solid ${THEME.border}` }}>
                    {[
                      { step: 1, label: "OS Template" },
                      { step: 2, label: "Hardware" },
                      { step: 3, label: "Storage" },
                      { step: 4, label: "Summary" }
                    ].map(s => (
                      <div key={s.step} style={{ 
                        flex: 1, padding: '12px 8px', textAlign: 'center', fontSize: '12px', fontWeight: 'bold', 
                        color: wizardStep >= s.step ? THEME.accent : THEME.textMuted,
                        borderBottom: wizardStep === s.step ? `2px solid ${THEME.accent}` : '2px solid transparent'
                      }}>
                        Step {s.step}: {s.label}
                      </div>
                    ))}
                  </div>

                  <div style={{ padding: '24px', minHeight: '300px' }}>
                    
                    {/* STEP 1: OS Selection */}
                    {wizardStep === 1 && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                        <div>
                          <label style={labelStyle}>Virtual Machine Name</label>
                          <input type="text" value={vmName} onChange={(e) => setVmName(e.target.value)} style={{...inputStyle, fontSize: '14px', padding: '8px'}} />
                        </div>
                        
                        <div>
                          <label style={labelStyle}>Operating System Type</label>
                          <p style={{ fontSize: '11px', color: THEME.textMuted, marginBottom: '12px' }}>Selecting an OS auto-configures recommended hardware settings.</p>
                          
                          <div style={{ display: 'flex', gap: '12px', marginBottom: '16px' }}>
                            <div style={{ flex: 1 }}>
                              <label style={{...labelStyle, fontSize: '10px'}}>OS Family</label>
                              <select value={osFamily} onChange={handleFamilyChange} style={inputStyle}>
                                {Object.entries(OS_FAMILIES).map(([key, val]) => (
                                  <option key={key} value={key}>{val.label}</option>
                                ))}
                              </select>
                            </div>
                            <div style={{ flex: 1 }}>
                              <label style={{...labelStyle, fontSize: '10px'}}>Version</label>
                              <select 
                                value={osType} 
                                onChange={(e) => applyOsTemplate(osFamily, e.target.value)} 
                                style={inputStyle}
                              >
                                {/* @ts-ignore */}
                                {OS_FAMILIES[osFamily].versions.map(v => (
                                  <option key={v.id} value={v.id}>{v.label}</option>
                                ))}
                              </select>
                            </div>
                          </div>

                          <div style={{ padding: '12px', backgroundColor: THEME.bgHover, border: `1px solid ${THEME.border}`, borderRadius: '4px' }}>
                            <div style={{ fontSize: '12px', fontWeight: 'bold', color: THEME.accent, marginBottom: '4px' }}>Recommended Preset:</div>
                            {/* @ts-ignore */}
                            <div style={{ fontSize: '11px', color: THEME.textMuted }}>{OS_FAMILIES[osFamily].versions.find(v => v.id === osType)?.desc}</div>
                          </div>
                        </div>
                      </div>
                    )}

                    {/* STEP 2: Hardware */}
                    {wizardStep === 2 && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                          <div>
                            <label style={labelStyle}>CPU Cores</label>
                            <input type="number" min="1" max="64" value={cpuCores} onChange={(e) => setCpuCores(Number(e.target.value))} style={inputStyle} />
                          </div>
                          <div>
                            <label style={labelStyle}>RAM (GB)</label>
                            <input type="number" min="1" max="128" value={ramLimit} onChange={(e) => setRamLimit(Number(e.target.value))} style={inputStyle} />
                          </div>
                        </div>
                        
                        <div>
                          <label style={labelStyle}>Firmware Interface</label>
                          <div style={{ display: 'flex', gap: '16px', marginTop: '8px' }}>
                            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px', cursor: 'pointer' }}>
                              <input type="radio" name="firmware" checked={firmware === "bios"} onChange={() => setFirmware("bios")} /> BIOS (Legacy)
                            </label>
                            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px', cursor: 'pointer' }}>
                              <input type="radio" name="firmware" checked={firmware === "uefi"} onChange={() => setFirmware("uefi")} /> UEFI (OVMF)
                            </label>
                          </div>
                          <p style={{ fontSize: '11px', color: THEME.textMuted, marginTop: '8px' }}>UEFI is required for Windows 11 and modern macOS guests.</p>
                        </div>
                      </div>
                    )}

                    {/* STEP 3: Storage & Network */}
                    {wizardStep === 3 && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                        <div>
                          <label style={labelStyle}>Disk Size (GB)</label>
                          <input type="number" min="1" max="2000" value={diskSizeGb} onChange={(e) => setDiskSizeGb(Number(e.target.value))} style={inputStyle} />
                          <p style={{ fontSize: '11px', color: THEME.textMuted, marginTop: '4px' }}>A dynamic .qcow2 disk will be automatically created.</p>
                        </div>
                        
                        <div>
                          <label style={labelStyle}>Installer Image (ISO)</label>
                          <div style={{ display: 'flex', gap: '4px' }}>
                            <input type="text" value={isoPath} onChange={e => setIsoPath(e.target.value)} style={inputStyle} placeholder="Optional. Select an ISO to boot from." />
                            <button onClick={() => handleBrowseFile(setIsoPath)} style={btnStyle}><FolderOpen size={12} /></button>
                          </div>
                        </div>

                        <div>
                          <label style={labelStyle}>Network Adapter</label>
                          <select value={networkType} onChange={(e) => setNetworkType(e.target.value)} style={inputStyle}>
                            <option value="nat">NAT (User Mode) - Default</option>
                            <option value="bridged">Bridged Adapter</option>
                            <option value="host-only">Host-Only Network</option>
                          </select>
                        </div>
                        
                        <div>
                          <label style={labelStyle}>VM Directory (Location)</label>
                          <div style={{ display: 'flex', gap: '4px' }}>
                            <input type="text" value={vmDir} onChange={(e) => setVmDir(e.target.value)} style={inputStyle} placeholder="Leave empty for default location" />
                            <button onClick={() => handleBrowseFolder(setVmDir)} style={btnStyle}><FolderOpen size={12} /></button>
                          </div>
                        </div>
                      </div>
                    )}

                    {/* STEP 4: Summary */}
                    {wizardStep === 4 && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                        <div style={{ backgroundColor: THEME.bgApp, padding: '16px', borderRadius: '4px', border: `1px solid ${THEME.border}`, fontFamily: 'monospace', fontSize: '12px' }}>
                          <div style={{ marginBottom: '8px' }}><strong style={{ color: THEME.accent }}>Name:</strong> {vmName}</div>
                          <div style={{ marginBottom: '8px' }}><strong style={{ color: THEME.accent }}>OS Profile:</strong> {/* @ts-ignore */}{(OS_FAMILIES as any)[osFamily].versions.find((v: any) => v.id === osType)?.label}</div>
                          <div style={{ marginBottom: '8px' }}><strong style={{ color: THEME.accent }}>Hardware:</strong> {cpuCores} Cores, {ramLimit}GB RAM</div>
                          <div style={{ marginBottom: '8px' }}><strong style={{ color: THEME.accent }}>Firmware:</strong> {firmware.toUpperCase()}</div>
                          <div style={{ marginBottom: '8px' }}><strong style={{ color: THEME.accent }}>Storage:</strong> {diskSizeGb}GB Disk</div>
                          <div style={{ marginBottom: '8px' }}><strong style={{ color: THEME.accent }}>Installer:</strong> {isoPath || "None"}</div>
                          <div><strong style={{ color: THEME.accent }}>Network:</strong> {networkType}</div>
                        </div>
                        <p style={{ fontSize: '12px', color: THEME.success, display: 'flex', alignItems: 'center', gap: '6px', fontWeight: 'bold' }}>
                          <Check size={14} /> Ready to Create!
                        </p>
                      </div>
                    )}

                  </div>

                  {/* Wizard Footer / Controls */}
                  <div style={{ padding: '16px 24px', borderTop: `1px solid ${THEME.border}`, display: 'flex', justifyContent: 'space-between', backgroundColor: THEME.bgApp }}>
                    <button 
                      onClick={() => setWizardStep(prev => Math.max(1, prev - 1) as any)} 
                      disabled={wizardStep === 1}
                      style={{ ...btnStyle, opacity: wizardStep === 1 ? 0.5 : 1, width: '100px', justifyContent: 'center' }}
                    >
                      Back
                    </button>
                    
                    {wizardStep < 4 ? (
                      <button onClick={() => setWizardStep(prev => Math.min(4, prev + 1) as any)} style={{ ...btnPrimaryStyle, width: '100px', justifyContent: 'center' }}>
                        Next <ArrowRight size={14} />
                      </button>
                    ) : (
                      <button onClick={handleCreateVm} style={{ ...btnPrimaryStyle, width: '140px', justifyContent: 'center', backgroundColor: THEME.success, borderColor: THEME.success }}>
                        <Save size={14} /> Finish & Create
                      </button>
                    )}
                  </div>

                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
