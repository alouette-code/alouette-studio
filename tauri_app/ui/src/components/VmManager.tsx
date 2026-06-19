import React, { useState, useEffect } from "react";
import { Play, Square, HardDrive, Cpu, Network, Monitor, FolderOpen, ShieldCheck, Minus, Square as SquareIcon, X, Plus, MoreVertical, Pause, RotateCw, Power, Save, History, Terminal } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import WindowResizer from "./WindowResizer";
import brandIcon from "./logo_alouette.png";

interface VM {
  id: string;
  name: string;
  status: string;
  config: {
    id: string;
    name: string;
    cpu_cores: number;
    ram_size_mb: number;
    kernel_path: string | null;
    initrd_path: string | null;
    boot_args: string | null;
    disk_path: string | null;
    network_mode: string;
  };
}

export default function VmManager() {
  const appWindow = getCurrentWindow();

  const handleMinimize = async () => { try { await appWindow.minimize(); } catch {} };
  const handleMaximize = async () => { try { await appWindow.toggleMaximize(); } catch {} };
  const handleClose = async () => { try { await appWindow.close(); } catch {} };

  // App State
  const [activeView, setActiveView] = useState<"create" | "manage">("create");
  const [selectedVmId, setSelectedVmId] = useState<string | null>(null);
  const [existingVms, setExistingVms] = useState<VM[]>([]);
  const [logs, setLogs] = useState<string>("");

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
          } catch {}
        }, 1000);
        return () => clearInterval(interval);
      } else {
        setLogs("[VM is stopped]");
      }
    }
  }, [activeView, selectedVmId, existingVms]);

  const toolbarBtnStyle: React.CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '6px',
    padding: '8px 16px',
    background: 'transparent',
    border: '1px solid transparent',
    borderRadius: '6px',
    cursor: 'pointer',
    color: 'var(--text-primary)',
    minWidth: '80px'
  };

  // Creation Form State
  const [activeTab, setActiveTab] = useState<"general" | "system" | "storage" | "network" | "summary">("general");
  const [vmName, setVmName] = useState("New-VM");
  const [vmPath, setVmPath] = useState("~/VirtualMachines/");
  const [osType, setOsType] = useState("ubuntu");
  const [isoPath, setIsoPath] = useState("");
  const [ramLimit, setRamLimit] = useState(1); // GB
  const [cpuCores, setCpuCores] = useState(1);
  const [cpuArch, setCpuArch] = useState("x86_64");
  const [diskSize, setDiskSize] = useState(10); // GB
  const [diskType, setDiskType] = useState("nvme");
  const [networkType, setNetworkType] = useState("nat");
  const [macAddress, setMacAddress] = useState("auto");

  // Manage Form State (Direct Editing)
  const [editCpu, setEditCpu] = useState(1);
  const [editRam, setEditRam] = useState(1);
  const [editBootOrder, setEditBootOrder] = useState("disk,cdrom,net");
  const [editNetType, setEditNetType] = useState("nat");
  const [editIso, setEditIso] = useState("");
  const [isSnapshotModalOpen, setIsSnapshotModalOpen] = useState(false);

  // Load configuration into manage tab when a VM is selected
  useEffect(() => {
    if (selectedVmId) {
      const selected = existingVms.find(v => v.id === selectedVmId);
      if (selected) {
        setEditCpu(selected.config.cpu_cores);
        setEditRam(Math.round(selected.config.ram_size_mb / 1024));
        setEditNetType(selected.config.network_mode);
        setEditIso(selected.config.kernel_path || "");
      }
    }
  }, [selectedVmId, existingVms]);

  const handleCreateVm = async () => {
    try {
      const config = {
        id: "",
        name: vmName,
        cpu_cores: cpuCores,
        ram_size_mb: ramLimit * 1024,
        kernel_path: isoPath ? isoPath : null,
        initrd_path: null,
        boot_args: "console=ttyS0 quiet panic=1",
        disk_path: null,
        network_mode: networkType
      };
      await invoke("save_virtual_machine", { config });
      alert("Virtual Machine created successfully!");
      setActiveView("manage");
      await refreshVms();
    } catch (err) {
      alert("Failed to create VM: " + err);
    }
  };

  const handleStartVm = async (id: string) => {
    try {
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
        kernel_path: editIso ? editIso : null,
      };
      await invoke("save_virtual_machine", { config: updatedConfig });
      alert("Configuration updated!");
      await refreshVms();
    } catch (err) {
      alert("Failed to update config: " + err);
    }
  };

  const handleBrowseIso = async () => {
    try {
      const path = await invoke<string | null>("open_file_dialog");
      if (path) {
        setIsoPath(path);
      }
    } catch {}
  };

  const handleBrowseEditIso = async () => {
    try {
      const path = await invoke<string | null>("open_file_dialog");
      if (path) {
        setEditIso(path);
      }
    } catch {}
  };

  const currentVm = existingVms.find(v => v.id === selectedVmId);

  return (
    <div className="vm-manager" style={{ display: 'flex', flexDirection: 'column', height: '100vh', backgroundColor: 'var(--bg-primary)', color: 'var(--text-primary)', overflowY: 'hidden' }}>
      <WindowResizer />
      
      {/* Titlebar */}
      <div className="postman-window-titlebar" data-tauri-drag-region>
        <div className="titlebar-left" data-tauri-drag-region>
          <img src={brandIcon} alt="Logo" className="titlebar-icon" style={{ width: "14px", height: "14px", objectFit: "contain", marginRight: "4px" }} />
          <span className="titlebar-title">Virtual Machine</span>
          <span className="titlebar-subtitle">Manager</span>
        </div>
        <div className="titlebar-right">
          <button className="window-control-btn minimize" onClick={handleMinimize}><Minus size={13} /></button>
          <button className="window-control-btn maximize" onClick={handleMaximize}><SquareIcon size={10} /></button>
          <button className="window-control-btn close" onClick={handleClose}><X size={14} /></button>
        </div>
      </div>
      
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        
        {/* Left Sidebar (Taskbar) */}
        <div style={{ width: '220px', backgroundColor: 'var(--bg-secondary)', borderRight: '1px solid var(--border-primary)', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '12px' }}>
            <button 
              onClick={() => { setActiveView("create"); setSelectedVmId(null); }}
              style={{ width: '100%', padding: '8px', backgroundColor: 'var(--accent)', color: '#fff', border: 'none', borderRadius: '4px', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px', fontSize: '13px', fontWeight: 500 }}>
              <Plus size={14} /> New Virtual Machine
            </button>
          </div>
          <div style={{ padding: '0 12px 8px 12px', fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase' }}>Existing VMs</div>
          <div style={{ flex: 1, overflowY: 'auto' }}>
            {existingVms.map(vm => (
              <div 
                key={vm.id}
                onClick={() => { setActiveView("manage"); setSelectedVmId(vm.id); }}
                style={{ padding: '10px 12px', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderLeft: (activeView === "manage" && selectedVmId === vm.id) ? '3px solid var(--accent)' : '3px solid transparent', backgroundColor: (activeView === "manage" && selectedVmId === vm.id) ? 'var(--bg-tertiary)' : 'transparent', borderBottom: '1px solid rgba(255,255,255,0.02)' }}
                className="vm-sidebar-item"
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', overflow: 'hidden' }}>
                  <div style={{ width: '8px', height: '8px', borderRadius: '50%', flexShrink: 0, backgroundColor: vm.status === 'running' ? 'var(--success)' : 'var(--text-muted)', boxShadow: vm.status === 'running' ? '0 0 5px var(--success)' : 'none' }} />
                  <div style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
                    <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{vm.name}</span>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{vm.status === 'running' ? 'Running' : 'Stopped'}</span>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '4px', opacity: (activeView === "manage" && selectedVmId === vm.id) ? 1 : 0.4 }}>
                  {vm.status === 'running' ? (
                    <button onClick={(e) => { e.stopPropagation(); handleStopVm(vm.id); }} style={{ background: 'none', border: 'none', color: 'var(--error)', cursor: 'pointer', padding: '2px' }} title="Stop"><Square size={12} fill="currentColor" /></button>
                  ) : (
                    <button onClick={(e) => { e.stopPropagation(); handleStartVm(vm.id); }} style={{ background: 'none', border: 'none', color: 'var(--success)', cursor: 'pointer', padding: '2px' }} title="Start"><Play size={12} fill="currentColor" /></button>
                  )}
                  <button onClick={(e) => { e.stopPropagation(); handleDeleteVm(vm.id); }} style={{ background: 'none', border: 'none', color: 'var(--error)', cursor: 'pointer', padding: '2px' }} title="Delete"><X size={12} /></button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Main Content Area */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', backgroundColor: 'var(--bg-primary)', overflowY: 'auto' }}>
          
          {activeView === "manage" && currentVm ? (
            <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
              
              {/* Toolbar */}
              <div style={{ display: 'flex', flexDirection: 'column', borderBottom: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-secondary)', userSelect: 'none' }}>
                
                {/* Top Row: Power Controls */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 16px', borderBottom: '1px solid var(--border-primary)' }}>
                  
                  <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', paddingRight: '16px', borderRight: '1px solid var(--border-primary)' }}>
                      {currentVm.status === 'running' ? (
                        <>
                          <button onClick={() => handleStopVm(currentVm.id)} style={toolbarBtnStyle} className="hover-bg-tertiary" title="Shut down the VM">
                            <Square size={22} color="var(--error)" fill="var(--error)" />
                            <span style={{ fontSize: '11px', fontWeight: 500 }}>Shut Down</span>
                          </button>
                        </>
                      ) : (
                        <button onClick={() => handleStartVm(currentVm.id)} style={toolbarBtnStyle} className="hover-bg-tertiary" title="Power on this virtual machine">
                          <Play size={22} color="var(--success)" fill="var(--success)" />
                          <span style={{ fontSize: '11px', fontWeight: 500 }}>Power On</span>
                        </button>
                      )}
                    </div>
                  </div>

                  <button onClick={handleApplyConfig} style={{ padding: '8px 16px', borderRadius: '4px', backgroundColor: 'var(--accent)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '13px', fontWeight: 500 }}>
                    Apply Configuration
                  </button>
                </div>

                {/* Bottom Row: Configuration Inputs */}
                <div style={{ display: 'flex', alignItems: 'center', padding: '12px 16px', gap: '24px', flexWrap: 'wrap' }}>
                  
                  {/* CPU Input */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <Cpu size={16} color="var(--text-muted)" />
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>CPU:</span>
                    <input type="number" min="1" max="64" value={editCpu} onChange={(e) => setEditCpu(Number(e.target.value))} style={{ width: '50px', padding: '4px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '12px' }} />
                  </div>

                  {/* RAM Input */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <HardDrive size={16} color="var(--text-muted)" />
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>RAM:</span>
                    <input type="number" min="1" max="64" value={editRam} onChange={(e) => setEditRam(Number(e.target.value))} style={{ width: '50px', padding: '4px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '12px' }} />
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>GB</span>
                  </div>

                  <div style={{ width: '1px', height: '16px', backgroundColor: 'var(--border-primary)' }} />

                  {/* Network Input */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <Network size={16} color="var(--text-muted)" />
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Network:</span>
                    <select value={editNetType} onChange={(e) => setEditNetType(e.target.value)} style={{ padding: '4px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '12px' }}>
                      <option value="nat">NAT</option>
                      <option value="bridged">Bridged</option>
                      <option value="host-only">Host-Only</option>
                    </select>
                  </div>

                  <div style={{ width: '1px', height: '16px', backgroundColor: 'var(--border-primary)' }} />

                  {/* ISO/Kernel Input */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flex: 1, minWidth: '200px' }}>
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Kernel Path:</span>
                    <input type="text" placeholder="Default / Mock Program" value={editIso} onChange={(e) => setEditIso(e.target.value)} style={{ flex: 1, padding: '4px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '12px' }} />
                    <button onClick={handleBrowseEditIso} style={{ padding: '4px 12px', borderRadius: '4px', backgroundColor: 'var(--bg-tertiary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '12px' }}><FolderOpen size={12} style={{ marginRight: '4px', verticalAlign: 'middle' }} /> Browse</button>
                  </div>

                </div>
              </div>

              {/* Console logs */}
              <div style={{ flex: 1, backgroundColor: '#09090b', position: 'relative', display: 'flex', flexDirection: 'column', padding: '16px', fontFamily: 'monospace', fontSize: '13px', overflowY: 'auto' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px', borderBottom: '1px solid rgba(255,255,255,0.1)', paddingBottom: '8px' }}>
                  <Terminal size={16} color="var(--success)" />
                  <span style={{ color: '#e4e4e7', fontWeight: 600 }}>Guest Serial Console Output</span>
                  <div style={{ marginLeft: 'auto', width: '8px', height: '8px', borderRadius: '50%', backgroundColor: currentVm.status === 'running' ? 'var(--success)' : 'var(--error)' }} />
                </div>
                <pre style={{ margin: 0, color: '#a1a1aa', whiteSpace: 'pre-wrap', flex: 1 }}>
                  {logs || "[Waiting for console output...]"}
                </pre>
              </div>
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
              <div style={{ padding: '16px 24px', borderBottom: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-primary)' }}>
                <h2 style={{ fontSize: '18px', margin: 0, fontWeight: 500, color: 'var(--text-primary)' }}>Create New Virtual Machine</h2>
              </div>
              
              {/* Tabs */}
              <div style={{ display: 'flex', borderBottom: '1px solid var(--border-primary)', padding: '0 24px', gap: '24px', backgroundColor: 'var(--bg-secondary)', overflowX: 'auto' }}>
                {[
                  { id: 'general', label: 'General' },
                  { id: 'system', label: 'System' },
                  { id: 'storage', label: 'Storage' },
                  { id: 'network', label: 'Network' },
                  { id: 'summary', label: 'Summary' },
                ].map(tab => (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id as any)}
                    style={{ background: 'none', border: 'none', borderBottom: activeTab === tab.id ? '2px solid var(--accent)' : '2px solid transparent', padding: '12px 0', fontSize: '13px', fontWeight: activeTab === tab.id ? 600 : 400, color: activeTab === tab.id ? 'var(--accent)' : 'var(--text-secondary)', cursor: 'pointer', transition: 'all 0.2s' }}
                  >
                    {tab.label}
                  </button>
                ))}
              </div>

              {/* Tab Content */}
              <div style={{ flex: 1, padding: '24px', overflowY: 'auto' }}>
                <div style={{ maxWidth: '600px' }}>
                  
                  {activeTab === 'general' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                          <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Virtual Machine Name</label>
                          <input type="text" value={vmName} onChange={(e) => setVmName(e.target.value)} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                        </div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                          <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Operating System Type</label>
                          <select value={osType} onChange={(e) => setOsType(e.target.value)} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', cursor: 'pointer', fontSize: '13px' }}>
                            <option value="linux">Linux Kernel (MicroVM)</option>
                          </select>
                        </div>
                      </div>

                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>VM Save Location</label>
                        <div style={{ display: 'flex', gap: '8px' }}>
                          <input type="text" placeholder="~/VirtualMachines/" value={vmPath} onChange={e => setVmPath(e.target.value)} style={{ flex: 1, padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                          <button style={{ padding: '6px 12px', borderRadius: '4px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}>Browse</button>
                        </div>
                      </div>

                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Linux Kernel Path (Optional)</label>
                        <div style={{ display: 'flex', gap: '8px' }}>
                          <input type="text" placeholder="/path/to/vmlinux (leave empty for mock code)" value={isoPath} onChange={e => setIsoPath(e.target.value)} style={{ flex: 1, padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                          <button onClick={handleBrowseIso} style={{ padding: '6px 12px', borderRadius: '4px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}>Browse</button>
                        </div>
                      </div>
                    </div>
                  )}

                  {activeTab === 'system' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>CPU Architecture</label>
                        <select value={cpuArch} onChange={(e) => setCpuArch(e.target.value)} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', cursor: 'pointer', fontSize: '13px' }}>
                          <option value="x86_64">x86_64 (AMD64)</option>
                        </select>
                      </div>
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                          <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>CPU Cores</label>
                          <input type="number" min="1" max="64" value={cpuCores} onChange={(e) => setCpuCores(Number(e.target.value))} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                        </div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                          <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Memory / RAM (GB)</label>
                          <input type="number" min="1" max="128" value={ramLimit} onChange={(e) => setRamLimit(Number(e.target.value))} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                        </div>
                      </div>
                    </div>
                  )}

                  {activeTab === 'storage' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Primary Disk Size (GB)</label>
                        <input type="number" min="5" max="2000" value={diskSize} onChange={(e) => setDiskSize(Number(e.target.value))} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                      </div>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Disk Controller Type</label>
                        <select value={diskType} onChange={(e) => setDiskType(e.target.value)} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', cursor: 'pointer', fontSize: '13px' }}>
                          <option value="virtio">VirtIO Block</option>
                        </select>
                      </div>
                    </div>
                  )}

                  {activeTab === 'network' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Network Mode</label>
                        <select value={networkType} onChange={(e) => setNetworkType(e.target.value)} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', cursor: 'pointer', fontSize: '13px' }}>
                          <option value="nat">NAT (Share Host IP)</option>
                          <option value="bridged">Bridged Adapter</option>
                          <option value="host-only">Host-Only Network</option>
                        </select>
                      </div>
                    </div>
                  )}

                  {activeTab === 'summary' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)', marginBottom: '8px' }}>
                        <ShieldCheck size={18} color="var(--success)" />
                        <h3 style={{ fontSize: '15px', margin: 0, fontWeight: 600 }}>Review Configuration</h3>
                      </div>
                      
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', backgroundColor: 'var(--bg-tertiary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-primary)' }}>
                        <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: '8px', fontSize: '13px' }}>
                          <span style={{ color: 'var(--text-muted)' }}>VM Name:</span>
                          <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>{vmName || 'Unnamed'}</span>
                          
                          <span style={{ color: 'var(--text-muted)' }}>Kernel Path:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{isoPath || 'Mock Program'}</span>
                        </div>
                        
                        <div style={{ height: '1px', backgroundColor: 'var(--border-primary)' }} />
                        
                        <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: '8px', fontSize: '13px' }}>
                          <span style={{ color: 'var(--text-muted)' }}>Resources:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{cpuCores} Cores, {ramLimit} GB RAM ({cpuArch})</span>
                        </div>
                      </div>
                    </div>
                  )}

                </div>
              </div>

              {/* Bottom Footer Action */}
              <div style={{ padding: '16px 24px', borderTop: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-secondary)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <button onClick={() => { setActiveView("manage"); }} style={{ padding: '6px 16px', borderRadius: '4px', backgroundColor: 'transparent', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}>Cancel</button>
                
                <div style={{ display: 'flex', gap: '12px' }}>
                  {activeTab !== 'general' && (
                    <button 
                      onClick={() => {
                        const tabs = ['general', 'system', 'storage', 'network', 'summary'];
                        const idx = tabs.indexOf(activeTab);
                        if (idx > 0) setActiveTab(tabs[idx - 1] as any);
                      }}
                      style={{ padding: '6px 16px', borderRadius: '4px', backgroundColor: 'var(--bg-tertiary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px', fontWeight: 500 }}
                    >
                      Back
                    </button>
                  )}
                  
                  {activeTab !== 'summary' ? (
                    <button 
                      onClick={() => {
                        const tabs = ['general', 'system', 'storage', 'network', 'summary'];
                        const idx = tabs.indexOf(activeTab);
                        if (idx < tabs.length - 1) setActiveTab(tabs[idx + 1] as any);
                      }}
                      style={{ padding: '6px 24px', borderRadius: '4px', backgroundColor: 'var(--accent)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '13px', fontWeight: 500 }}
                    >
                      Next
                    </button>
                  ) : (
                    <button 
                      onClick={handleCreateVm}
                      style={{ padding: '6px 20px', borderRadius: '4px', backgroundColor: 'var(--success)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '13px', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px' }}
                    >
                      <Play size={14} fill="currentColor" /> Create VM
                    </button>
                  )}
                </div>
              </div>

            </div>
          )}
        </div>
      </div>
    </div>
  );
}
