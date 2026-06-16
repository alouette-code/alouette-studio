import React, { useState } from "react";
import { Play, Square, HardDrive, Cpu, Network, Monitor, FolderOpen, ShieldCheck, Minus, Square as SquareIcon, X, Plus, MoreVertical, Pause, RotateCw, Power, Save, History } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import WindowResizer from "./WindowResizer";
import brandIcon from "./logo_alouette.png";

export default function VmManager() {
  const appWindow = getCurrentWindow();

  const handleMinimize = async () => { try { await appWindow.minimize(); } catch {} };
  const handleMaximize = async () => { try { await appWindow.toggleMaximize(); } catch {} };
  const handleClose = async () => { try { await appWindow.close(); } catch {} };

  // App State
  const [activeView, setActiveView] = useState<"create" | "manage">("create");
  const [selectedVmId, setSelectedVmId] = useState<string | null>(null);

  // Mock Data for existing VMs
  const [existingVms] = useState([
    { id: "1", name: "Ubuntu-Dev", os: "Ubuntu Linux 24.04", status: "stopped", ip: "N/A" },
    { id: "2", name: "Windows-Test", os: "Windows 11 Pro", status: "running", ip: "192.168.1.45" },
    { id: "3", name: "Android-Emu", os: "Android 14", status: "stopped", ip: "N/A" },
  ]);

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
  const [ramLimit, setRamLimit] = useState(4); // GB
  const [cpuCores, setCpuCores] = useState(2);
  const [cpuArch, setCpuArch] = useState("x86_64");
  const [diskSize, setDiskSize] = useState(20); // GB
  const [diskType, setDiskType] = useState("nvme");
  const [networkType, setNetworkType] = useState("nat");
  const [macAddress, setMacAddress] = useState("auto");

  // Manage Form State (Direct Editing)
  const [editCpu, setEditCpu] = useState(4);
  const [editRam, setEditRam] = useState(8);
  const [editBootOrder, setEditBootOrder] = useState("disk,cdrom,net");
  const [editNetType, setEditNetType] = useState("nat");

  const [editIso, setEditIso] = useState("");

  const [isSnapshotModalOpen, setIsSnapshotModalOpen] = useState(false);

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
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{vm.status === 'running' ? vm.ip : vm.os}</span>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '4px', opacity: (activeView === "manage" && selectedVmId === vm.id) ? 1 : 0.4 }}>
                  {vm.status === 'running' ? (
                    <button style={{ background: 'none', border: 'none', color: 'var(--error)', cursor: 'pointer', padding: '2px' }} title="Stop"><Square size={12} fill="currentColor" /></button>
                  ) : (
                    <button style={{ background: 'none', border: 'none', color: 'var(--success)', cursor: 'pointer', padding: '2px' }} title="Start"><Play size={12} fill="currentColor" /></button>
                  )}
                  <button style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', padding: '2px' }} title="Options"><MoreVertical size={12} /></button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Main Content Area */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', backgroundColor: 'var(--bg-primary)', overflowY: 'auto' }}>
          
          {activeView === "manage" ? (
            <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
              
              {/* VMWare-style Toolbar (Thick Ribbon with inline inputs) */}
              <div style={{ display: 'flex', flexDirection: 'column', borderBottom: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-secondary)', userSelect: 'none' }}>
                
                {/* Top Row: Power Controls & Snapshots */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 16px', borderBottom: '1px solid var(--border-primary)' }}>
                  
                  <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                    {/* Power Controls Group */}
                    <div style={{ display: 'flex', alignItems: 'center', paddingRight: '16px', borderRight: '1px solid var(--border-primary)' }}>
                      {existingVms.find(v => v.id === selectedVmId)?.status === 'running' ? (
                        <>
                          <button style={toolbarBtnStyle} className="hover-bg-tertiary" title="Suspend this virtual machine">
                            <Pause size={22} color="#ffb700" fill="#ffb700" />
                            <span style={{ fontSize: '11px', fontWeight: 500 }}>Suspend</span>
                          </button>
                          <button style={toolbarBtnStyle} className="hover-bg-tertiary" title="Shut down the guest OS">
                            <Square size={22} color="var(--error)" fill="var(--error)" />
                            <span style={{ fontSize: '11px', fontWeight: 500 }}>Shut Down</span>
                          </button>
                          <button style={toolbarBtnStyle} className="hover-bg-tertiary" title="Restart the guest OS">
                            <RotateCw size={22} color="#00bfff" />
                            <span style={{ fontSize: '11px', fontWeight: 500 }}>Restart</span>
                          </button>
                          <button style={toolbarBtnStyle} className="hover-bg-tertiary" title="Power off immediately (Hard reset)">
                            <Power size={22} color="var(--error)" />
                            <span style={{ fontSize: '11px', fontWeight: 500 }}>Power Off</span>
                          </button>
                        </>
                      ) : (
                        <button style={toolbarBtnStyle} className="hover-bg-tertiary" title="Power on this virtual machine">
                          <Play size={22} color="var(--success)" fill="var(--success)" />
                          <span style={{ fontSize: '11px', fontWeight: 500 }}>Power On</span>
                        </button>
                      )}
                    </div>

                    {/* Snapshots Group */}
                    <div style={{ display: 'flex', alignItems: 'center' }}>
                      <button style={toolbarBtnStyle} className="hover-bg-tertiary" title="Lưu lại trạng thái hiện tại của máy ảo (Tạo Snapshot)">
                        <Save size={22} color="var(--text-secondary)" />
                        <span style={{ fontSize: '11px', fontWeight: 500 }}>Save State</span>
                      </button>
                      <button onClick={() => setIsSnapshotModalOpen(true)} style={toolbarBtnStyle} className="hover-bg-tertiary" title="Quản lý sơ đồ và khôi phục các trạng thái cũ (Cỗ máy thời gian)">
                        <History size={22} color="var(--text-secondary)" />
                        <span style={{ fontSize: '11px', fontWeight: 500 }}>Time Machine</span>
                      </button>
                    </div>
                  </div>

                  <button style={{ padding: '8px 16px', borderRadius: '4px', backgroundColor: 'var(--accent)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '13px', fontWeight: 500 }}>
                    Apply Configuration
                  </button>
                </div>

                {/* Bottom Row: Direct Configuration Inputs */}
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

                  {/* Boot Order Input */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Boot:</span>
                    <select value={editBootOrder} onChange={(e) => setEditBootOrder(e.target.value)} style={{ padding: '4px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '12px' }}>
                      <option value="disk,cdrom,net">Disk first</option>
                      <option value="cdrom,disk,net">CD-ROM first</option>
                      <option value="net,disk,cdrom">Network (PXE)</option>
                    </select>
                  </div>

                  <div style={{ width: '1px', height: '16px', backgroundColor: 'var(--border-primary)' }} />

                  {/* ISO/CD-ROM Input */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flex: 1, minWidth: '200px' }}>
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>ISO:</span>
                    <input type="text" placeholder="No ISO inserted" value={editIso} onChange={(e) => setEditIso(e.target.value)} style={{ flex: 1, padding: '4px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '12px' }} />
                    <button style={{ padding: '4px 12px', borderRadius: '4px', backgroundColor: 'var(--bg-tertiary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '12px' }}><FolderOpen size={12} style={{ marginRight: '4px', verticalAlign: 'middle' }} /> Browse</button>
                  </div>

                </div>
              </div>

              {/* VM Details Content -> Replaced by VM DISPLAY SCREEN */}
              <div style={{ flex: 1, backgroundColor: '#050505', position: 'relative', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', position: 'absolute', top: '16px', left: '16px', backgroundColor: 'rgba(255,255,255,0.1)', padding: '4px 12px', borderRadius: '20px', backdropFilter: 'blur(4px)' }}>
                  <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: existingVms.find(v => v.id === selectedVmId)?.status === 'running' ? 'var(--success)' : 'var(--error)' }} />
                  <span style={{ fontSize: '12px', color: '#fff', fontWeight: 500 }}>{existingVms.find(v => v.id === selectedVmId)?.status === 'running' ? 'VNC Connected' : 'VM Powered Off'}</span>
                </div>
                
                {existingVms.find(v => v.id === selectedVmId)?.status === 'running' ? (
                  <div style={{ textAlign: 'center' }}>
                    <Monitor size={64} color="rgba(255,255,255,0.1)" style={{ marginBottom: '16px' }} />
                    <h3 style={{ color: 'rgba(255,255,255,0.5)', margin: '0 0 8px 0', fontWeight: 400 }}>Guest OS Screen</h3>
                    <p style={{ color: 'rgba(255,255,255,0.3)', fontSize: '13px', maxWidth: '300px' }}>The virtual machine display (VNC/Spice) will be rendered here.</p>
                  </div>
                ) : (
                  <div style={{ textAlign: 'center' }}>
                    <Power size={64} color="rgba(255,255,255,0.1)" style={{ marginBottom: '16px' }} />
                    <h3 style={{ color: 'rgba(255,255,255,0.5)', margin: '0 0 8px 0', fontWeight: 400 }}>{existingVms.find(v => v.id === selectedVmId)?.name} is powered off</h3>
                    <button style={{ marginTop: '16px', padding: '8px 24px', borderRadius: '4px', backgroundColor: 'var(--success)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '14px', fontWeight: 500, display: 'flex', alignItems: 'center', gap: '8px', margin: '0 auto' }}>
                      <Play size={16} fill="currentColor" /> Power On VM
                    </button>
                  </div>
                )}
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
                            <optgroup label="Windows">
                              <option value="win11">Windows 11</option>
                              <option value="win10">Windows 10</option>
                              <option value="win8">Windows 8.1</option>
                              <option value="win7">Windows 7</option>
                              <option value="winxp">Windows XP</option>
                            </optgroup>
                            <optgroup label="Windows Server">
                              <option value="winsrv2022">Windows Server 2022</option>
                              <option value="winsrv2019">Windows Server 2019</option>
                              <option value="winsrv2016">Windows Server 2016</option>
                              <option value="winsrv2012">Windows Server 2012 R2</option>
                            </optgroup>
                            <optgroup label="Linux (Desktop & Server)">
                              <option value="ubuntu">Ubuntu (Debian-based)</option>
                              <option value="debian">Debian GNU/Linux</option>
                              <option value="centos">CentOS / Rocky / AlmaLinux</option>
                              <option value="rhel">Red Hat Enterprise Linux (RHEL)</option>
                              <option value="fedora">Fedora</option>
                              <option value="arch">Arch Linux / Manjaro</option>
                              <option value="suse">openSUSE / SLES</option>
                              <option value="alpine">Alpine Linux</option>
                              <option value="kali">Kali Linux</option>
                            </optgroup>
                            <optgroup label="macOS">
                              <option value="macos-sequoia">macOS 15 Sequoia</option>
                              <option value="macos-sonoma">macOS 14 Sonoma</option>
                              <option value="macos-ventura">macOS 13 Ventura</option>
                              <option value="macos-monterey">macOS 12 Monterey</option>
                              <option value="macos-bigsur">macOS 11 Big Sur</option>
                            </optgroup>
                            <optgroup label="Android x86 / Emulation">
                              <option value="android-16">Android 16</option>
                              <option value="android-15">Android 15</option>
                              <option value="android-14">Android 14</option>
                              <option value="android-13">Android 13</option>
                              <option value="android-12">Android 12 / 12L</option>
                              <option value="android-11">Android 11</option>
                              <option value="android-10">Android 10</option>
                              <option value="android-9">Android 9 (Pie)</option>
                              <option value="android-8">Android 8 (Oreo)</option>
                              <option value="android-7">Android 7 (Nougat)</option>
                            </optgroup>
                            <optgroup label="Other">
                              <option value="custom">Custom ISO / Raw Disk</option>
                            </optgroup>
                          </select>
                        </div>
                      </div>

                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>VM Save Location</label>
                        <div style={{ display: 'flex', gap: '8px' }}>
                          <input type="text" placeholder="~/VirtualMachines/" value={vmPath} onChange={e => setVmPath(e.target.value)} style={{ flex: 1, padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                          <button style={{ padding: '6px 12px', borderRadius: '4px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}>Browse</button>
                        </div>
                        <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>The directory where virtual disks and configuration files will be stored.</span>
                      </div>

                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Boot Image (ISO Path) - Optional</label>
                        <div style={{ display: 'flex', gap: '8px' }}>
                          <input type="text" placeholder="/path/to/image.iso" value={isoPath} onChange={e => setIsoPath(e.target.value)} style={{ flex: 1, padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                          <button style={{ padding: '6px 12px', borderRadius: '4px', backgroundColor: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}>Browse</button>
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
                          <option value="aarch64">ARM64 (aarch64)</option>
                          <option value="riscv64">RISC-V (64-bit)</option>
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
                          <option value="nvme">NVMe (Recommended / Fast)</option>
                          <option value="virtio">VirtIO Block</option>
                          <option value="sata">SATA / AHCI</option>
                          <option value="ide">IDE (Legacy)</option>
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
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>MAC Address</label>
                        <input type="text" placeholder="auto" value={macAddress} onChange={(e) => setMacAddress(e.target.value)} style={{ padding: '6px 10px', borderRadius: '4px', border: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-tertiary)', color: 'var(--text-primary)', outline: 'none', fontSize: '13px' }} />
                        <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Leave as 'auto' to generate a random MAC address.</span>
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
                          
                          <span style={{ color: 'var(--text-muted)' }}>Save Path:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{vmPath}</span>
                          
                          <span style={{ color: 'var(--text-muted)' }}>OS Type:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{osType}</span>
                          
                          <span style={{ color: 'var(--text-muted)' }}>ISO Path:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{isoPath || 'None (Disk only)'}</span>
                        </div>
                        
                        <div style={{ height: '1px', backgroundColor: 'var(--border-primary)' }} />
                        
                        <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: '8px', fontSize: '13px' }}>
                          <span style={{ color: 'var(--text-muted)' }}>Resources:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{cpuCores} Cores, {ramLimit} GB RAM ({cpuArch})</span>
                          
                          <span style={{ color: 'var(--text-muted)' }}>Storage:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{diskSize} GB ({diskType.toUpperCase()})</span>
                        </div>
                        
                        <div style={{ height: '1px', backgroundColor: 'var(--border-primary)' }} />
                        
                        <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: '8px', fontSize: '13px' }}>
                          <span style={{ color: 'var(--text-muted)' }}>Network:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{networkType.toUpperCase()}</span>
                          
                          <span style={{ color: 'var(--text-muted)' }}>MAC Address:</span>
                          <span style={{ color: 'var(--text-primary)' }}>{macAddress}</span>
                        </div>
                      </div>
                      
                      <p style={{ fontSize: '12px', color: 'var(--text-muted)', margin: 0 }}>
                        Click "Create & Boot VM" below to finalize and provision this virtual machine. The process may take a few moments depending on disk size.
                      </p>
                    </div>
                  )}

                </div>
              </div>

              {/* Bottom Footer Action */}
              <div style={{ padding: '16px 24px', borderTop: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-secondary)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <button style={{ padding: '6px 16px', borderRadius: '4px', backgroundColor: 'transparent', border: '1px solid var(--border-primary)', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}>Cancel</button>
                
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
                      onClick={() => alert("Creating Virtual Machine: " + vmName)}
                      style={{ padding: '6px 20px', borderRadius: '4px', backgroundColor: 'var(--success)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '13px', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px' }}
                    >
                      <Play size={14} fill="currentColor" /> Create & Boot VM
                    </button>
                  )}
                </div>
              </div>

            </div>
          )}
        </div>
      </div>

      {/* Snapshot Tree Manager Modal */}
      {isSnapshotModalOpen && (
        <div style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0,0,0,0.7)', zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center', backdropFilter: 'blur(4px)' }}>
          <div style={{ width: '650px', height: '500px', backgroundColor: 'var(--bg-primary)', borderRadius: '12px', border: '1px solid var(--border-primary)', display: 'flex', flexDirection: 'column', boxShadow: '0 20px 50px rgba(0,0,0,0.5)', overflow: 'hidden' }}>
            
            {/* Header */}
            <div style={{ padding: '16px 24px', borderBottom: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-secondary)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <History size={20} color="var(--accent)" />
                <h3 style={{ margin: 0, fontSize: '15px', fontWeight: 600, color: 'var(--text-primary)' }}>Time Machine (Snapshot Tree)</h3>
              </div>
              <button onClick={() => setIsSnapshotModalOpen(false)} style={{ background: 'transparent', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', padding: '4px' }} className="hover-bg-tertiary"><X size={18} /></button>
            </div>

            {/* Body: Diagram */}
            <div style={{ flex: 1, padding: '32px', overflowY: 'auto', backgroundColor: '#111' }}>
              <p style={{ color: 'var(--text-muted)', fontSize: '13px', marginBottom: '24px' }}>Sơ đồ cây (Tree) dưới đây thể hiện các điểm khôi phục của máy <b>{existingVms.find(v => v.id === selectedVmId)?.name}</b>. Bạn có thể rê nhánh ra từ bất kỳ điểm nào.</p>

              <div style={{ marginLeft: '12px', borderLeft: '2px solid var(--border-primary)', position: 'relative' }}>
                
                {/* Node 1: Base */}
                <div style={{ position: 'relative', paddingLeft: '24px', marginBottom: '16px' }}>
                  <div style={{ position: 'absolute', left: '-6px', top: '14px', width: '10px', height: '10px', borderRadius: '50%', backgroundColor: 'var(--text-secondary)' }} />
                  <div style={{ backgroundColor: 'var(--bg-secondary)', padding: '12px 16px', borderRadius: '8px', border: '1px solid var(--border-primary)', width: 'fit-content', cursor: 'pointer' }} className="hover-bg-tertiary">
                    <div style={{ fontWeight: 600, fontSize: '13px', color: 'var(--text-primary)' }}>Fresh Install (Hệ điều hành gốc)</div>
                    <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>Oct 12, 2023 - 2.4 GB</div>
                  </div>
                </div>

                {/* Node 2: Branch A */}
                <div style={{ position: 'relative', paddingLeft: '24px', marginBottom: '16px' }}>
                  <div style={{ position: 'absolute', left: '-6px', top: '14px', width: '10px', height: '10px', borderRadius: '50%', backgroundColor: 'var(--text-secondary)' }} />
                  <div style={{ backgroundColor: 'var(--bg-secondary)', padding: '12px 16px', borderRadius: '8px', border: '1px solid var(--border-primary)', width: 'fit-content', cursor: 'pointer' }} className="hover-bg-tertiary">
                    <div style={{ fontWeight: 600, fontSize: '13px', color: 'var(--text-primary)' }}>Cài đặt xong Docker & Node.js</div>
                    <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>Oct 15, 2023 - 800 MB</div>
                  </div>
                </div>

                {/* Sub-branch Level */}
                <div style={{ marginLeft: '40px', borderLeft: '2px solid var(--border-primary)', position: 'relative' }}>
                  {/* Node 3: Current State */}
                  <div style={{ position: 'relative', paddingLeft: '24px', marginBottom: '16px', paddingTop: '16px' }}>
                    <div style={{ position: 'absolute', left: '-20px', top: '30px', width: '20px', borderTop: '2px dashed var(--accent)' }} />
                    <div style={{ position: 'absolute', left: '-6px', top: '25px', width: '10px', height: '10px', borderRadius: '50%', backgroundColor: 'var(--accent)', boxShadow: '0 0 10px var(--accent)' }} />
                    <div style={{ backgroundColor: 'rgba(0,191,255,0.1)', padding: '12px 16px', borderRadius: '8px', border: '1px solid var(--accent)', width: 'fit-content' }}>
                      <div style={{ fontWeight: 600, fontSize: '13px', color: 'var(--accent)' }}>Current State (Bạn đang ở đây)</div>
                      <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>Trạng thái máy ảo hiện hành</div>
                    </div>
                  </div>
                </div>

                {/* Node 4: Branch B (Alternate timeline) */}
                <div style={{ position: 'relative', paddingLeft: '24px', paddingTop: '16px' }}>
                  <div style={{ position: 'absolute', left: '-20px', top: '30px', width: '20px', borderTop: '2px solid var(--border-primary)' }} />
                  <div style={{ position: 'absolute', left: '-6px', top: '25px', width: '10px', height: '10px', borderRadius: '50%', backgroundColor: 'var(--text-secondary)' }} />
                  <div style={{ backgroundColor: 'var(--bg-secondary)', padding: '12px 16px', borderRadius: '8px', border: '1px solid var(--border-primary)', width: 'fit-content', cursor: 'pointer' }} className="hover-bg-tertiary">
                    <div style={{ fontWeight: 600, fontSize: '13px', color: 'var(--text-primary)' }}>Dính Virus tống tiền (Test)</div>
                    <div style={{ fontSize: '11px', color: 'var(--error)', marginTop: '4px' }}>Oct 20, 2023 - Nhánh phụ rủi ro cao</div>
                  </div>
                </div>

              </div>
            </div>

            {/* Footer */}
            <div style={{ padding: '16px 24px', borderTop: '1px solid var(--border-primary)', backgroundColor: 'var(--bg-secondary)', display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
              <button style={{ padding: '8px 16px', borderRadius: '4px', backgroundColor: 'transparent', border: '1px solid var(--error)', color: 'var(--error)', cursor: 'pointer', fontSize: '13px', fontWeight: 500 }}>Delete Branch</button>
              <button style={{ padding: '8px 24px', borderRadius: '4px', backgroundColor: 'var(--accent)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: '13px', fontWeight: 500, display: 'flex', alignItems: 'center', gap: '6px' }}>
                <RotateCw size={14} /> Restore (Quay xe)
              </button>
            </div>

          </div>
        </div>
      )}
    </div>
  );
}
