import React, { useState } from "react";
import { Play, Square, Settings, HardDrive, Cpu, Network, Monitor, FolderOpen, ShieldCheck, Minus, Square as SquareIcon, X, Plus, Server, Database, Activity, MoreVertical } from "lucide-react";
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
  const [existingVms, setExistingVms] = useState([
    { id: "1", name: "Ubuntu-Dev", os: "Ubuntu Linux 24.04", status: "stopped", ip: "N/A" },
    { id: "2", name: "Windows-Test", os: "Windows 11 Pro", status: "running", ip: "192.168.1.45" },
    { id: "3", name: "Android-Emu", os: "Android 14", status: "stopped", ip: "N/A" },
  ]);

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
            <div style={{ padding: '32px', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-muted)' }}>
              <Server size={48} opacity={0.5} style={{ marginBottom: '16px' }} />
              <h2 style={{ color: 'var(--text-primary)', margin: '0 0 8px 0', fontWeight: 500 }}>{existingVms.find(v => v.id === selectedVmId)?.name} Dashboard</h2>
              <p style={{ fontSize: '14px', maxWidth: '400px', textAlign: 'center' }}>Giao diện quản lý chi tiết dự án máy ảo có sẵn sẽ được cập nhật sau. (UI Builder placeholder)</p>
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
    </div>
  );
}
