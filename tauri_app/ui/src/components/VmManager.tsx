import React, { useState, useEffect, useRef } from "react";
import { Play, Square, FolderOpen, Minus, Square as SquareIcon, X, Plus, Save, Terminal, Trash2, Monitor } from "lucide-react";
// @ts-ignore
import RFB from '@novnc/novnc';
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
    vm_dir: string;
    iso_path: string | null;
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
  const [activeTab, setActiveTab] = useState<"general" | "hardware" | "storage" | "network">("general");
  const [vmName, setVmName] = useState("New-VM");
  const [vmDir, setVmDir] = useState("");
  const [cpuCores, setCpuCores] = useState(1);
  const [ramLimit, setRamLimit] = useState(1);
  const [networkType, setNetworkType] = useState("nat");
  const [isoPath, setIsoPath] = useState("");
  const [diskPath, setDiskPath] = useState("");

  // --- Manage Form State ---
  const [editCpu, setEditCpu] = useState(1);
  const [editRam, setEditRam] = useState(1);
  const [editNetType, setEditNetType] = useState("nat");
  const [editIso, setEditIso] = useState("");
  const [editDisk, setEditDisk] = useState("");

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
        vm_dir: vmDir,
        iso_path: isoPath ? isoPath : null,
        disk_path: diskPath ? diskPath : null,
        network_mode: networkType
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
        <div style={{ display: "flex" }}>
          <button onClick={handleMinimize} style={{ background: "none", border: "none", color: THEME.textMain, padding: "4px 8px", cursor: "pointer" }}><Minus size={14} /></button>
          <button onClick={handleMaximize} style={{ background: "none", border: "none", color: THEME.textMain, padding: "4px 8px", cursor: "pointer" }}><SquareIcon size={12} /></button>
          <button onClick={handleClose} style={{ background: "none", border: "none", color: THEME.textMain, padding: "4px 8px", cursor: "pointer" }}><X size={14} /></button>
        </div>
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
            <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '24px', alignItems: 'flex-start' }}>
              <h2 style={{ fontSize: '16px', marginBottom: '24px', color: THEME.textMain }}>Create Virtual Machine</h2>
              
              <div style={{ width: '100%', maxWidth: '600px', backgroundColor: THEME.bgPanel, border: `1px solid ${THEME.border}`, padding: '16px' }}>
                
                <div style={{ display: 'flex', gap: '16px', marginBottom: '24px', borderBottom: `1px solid ${THEME.border}`, paddingBottom: '8px' }}>
                  {(['general', 'hardware', 'storage', 'network'] as const).map(tab => (
                    <button
                      key={tab}
                      onClick={() => setActiveTab(tab)}
                      style={{ background: 'none', border: 'none', color: activeTab === tab ? THEME.accent : THEME.textMuted, fontWeight: activeTab === tab ? 'bold' : 'normal', fontSize: '12px', cursor: 'pointer', textTransform: 'uppercase' }}
                    >
                      {tab}
                    </button>
                  ))}
                </div>

                <div style={{ minHeight: '200px' }}>
                  {activeTab === 'general' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div>
                        <label style={labelStyle}>Name</label>
                        <input type="text" value={vmName} onChange={(e) => setVmName(e.target.value)} style={inputStyle} />
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

                  {activeTab === 'hardware' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div>
                        <label style={labelStyle}>CPU Cores</label>
                        <input type="number" min="1" max="64" value={cpuCores} onChange={(e) => setCpuCores(Number(e.target.value))} style={inputStyle} />
                      </div>
                      <div>
                        <label style={labelStyle}>RAM (GB)</label>
                        <input type="number" min="1" max="128" value={ramLimit} onChange={(e) => setRamLimit(Number(e.target.value))} style={inputStyle} />
                      </div>
                    </div>
                  )}

                  {activeTab === 'storage' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div>
                        <label style={labelStyle}>Disk Image Path (.qcow2 / .img)</label>
                        <div style={{ display: 'flex', gap: '4px' }}>
                          <input type="text" value={diskPath} onChange={e => setDiskPath(e.target.value)} style={inputStyle} placeholder="Leave empty to auto-create inside VM Directory" />
                          <button onClick={() => handleBrowseFile(setDiskPath)} style={btnStyle}><FolderOpen size={12} /></button>
                        </div>
                      </div>
                      <div>
                        <label style={labelStyle}>ISO Image (CD-ROM)</label>
                        <div style={{ display: 'flex', gap: '4px' }}>
                          <input type="text" value={isoPath} onChange={e => setIsoPath(e.target.value)} style={inputStyle} placeholder="Path to .iso installer" />
                          <button onClick={() => handleBrowseFile(setIsoPath)} style={btnStyle}><FolderOpen size={12} /></button>
                        </div>
                      </div>
                    </div>
                  )}

                  {activeTab === 'network' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div>
                        <label style={labelStyle}>Network Adapter</label>
                        <select value={networkType} onChange={(e) => setNetworkType(e.target.value)} style={inputStyle}>
                          <option value="nat">NAT (User Mode)</option>
                          <option value="bridged">Bridged Adapter</option>
                          <option value="host-only">Host-Only Network</option>
                        </select>
                      </div>
                    </div>
                  )}
                </div>

                <div style={{ marginTop: '24px', paddingTop: '16px', borderTop: `1px solid ${THEME.border}`, display: 'flex', justifyContent: 'flex-end' }}>
                  <button onClick={handleCreateVm} style={btnPrimaryStyle}><Save size={14} /> Finish Creation</button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
